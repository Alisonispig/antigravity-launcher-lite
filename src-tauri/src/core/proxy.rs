use super::{Result, error};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const TARGET: &str = "accounts.google.com";
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyInput {
    pub host: String,
    pub port: u16,
}
impl ProxyInput {
    pub fn url(&self) -> Result<String> {
        // 主机只能是裸 DNS 名或 IP，拒绝 URL、凭据、路径及控制字符。
        if self.port == 0 || self.host.is_empty() || self.host.len() > 253 {
            return Err(error("INVALID_PROXY", "input"));
        }
        if let Ok(ip) = self.host.parse::<std::net::IpAddr>() {
            return Ok(match ip {
                std::net::IpAddr::V4(_) => format!("http://{}:{}", ip, self.port),
                std::net::IpAddr::V6(_) => format!("http://[{}]:{}", ip, self.port),
            });
        }
        if !self.host.split('.').all(|s| {
            !s.is_empty()
                && s.len() <= 63
                && !s.starts_with('-')
                && !s.ends_with('-')
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }) {
            return Err(error("INVALID_PROXY", "input"));
        }
        Ok(format!("http://{}:{}", self.host, self.port))
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub target: &'static str,
    pub status: u16,
    pub latency_ms: u128,
}
async fn connect_handshake(proxy: &ProxyInput) -> Result<()> {
    let mut stream = tokio::net::TcpStream::connect((proxy.host.as_str(), proxy.port))
        .await
        .map_err(|_| error("TCP_FAILED", "tcp"))?;
    stream
        .write_all(
            format!("CONNECT {TARGET}:443 HTTP/1.1\r\nHost: {TARGET}:443\r\n\r\n").as_bytes(),
        )
        .await
        .map_err(|_| error("PROTOCOL_ERROR", "connect"))?;
    let mut header = Vec::new();
    loop {
        let b = stream
            .read_u8()
            .await
            .map_err(|_| error("PROTOCOL_ERROR", "connect"))?;
        header.push(b);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
        if header.len() >= 8192 {
            return Err(error("PROTOCOL_ERROR", "connect"));
        }
    }
    let first = std::str::from_utf8(&header)
        .map_err(|_| error("PROTOCOL_ERROR", "connect"))?
        .split("\r\n")
        .next()
        .unwrap_or_default();
    let mut fields = first.split_whitespace();
    let version = fields.next().unwrap_or_default();
    let status = fields.next().unwrap_or_default();
    if !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Err(error("PROTOCOL_ERROR", "connect"));
    }
    match status {
        "200" => Ok(()),
        "407" => Err(error("AUTH_REQUIRED", "connect")),
        _ => Err(error("CONNECT_REJECTED", "connect")),
    }
}
pub async fn probe(proxy: &ProxyInput) -> Result<ProbeResult> {
    let url = proxy.url()?;
    let started = Instant::now();
    tokio::time::timeout(Duration::from_secs(5), connect_handshake(proxy))
        .await
        .map_err(|_| error("TIMEOUT", "connect"))??;
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(url).map_err(|_| error("INVALID_PROXY", "input"))?)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| error("PROBE_FAILED", "https"))?;
    let response = client
        .get(format!("https://{TARGET}/"))
        .send()
        .await
        .map_err(|e| {
            error(
                if e.is_timeout() {
                    "TIMEOUT"
                } else {
                    "TLS_OR_HTTP_FAILED"
                },
                "https",
            )
        })?;
    Ok(ProbeResult {
        target: TARGET,
        status: response.status().as_u16(),
        latency_ms: started.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_validation() {
        for host in [
            "http://localhost",
            "a@b",
            "localhost\r\nX: y",
            "a/b",
            "-bad",
            "a b",
        ] {
            assert!(
                ProxyInput {
                    host: host.into(),
                    port: 80
                }
                .url()
                .is_err()
            );
        }
        assert_eq!(
            ProxyInput {
                host: "::1".into(),
                port: 7890
            }
            .url()
            .unwrap(),
            "http://[::1]:7890"
        );
    }
    async fn serve(reply: &'static [u8]) -> ProxyInput {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![];
            while !request.ends_with(b"\r\n\r\n") {
                request.push(socket.read_u8().await.unwrap());
            }
            assert!(request.starts_with(b"CONNECT accounts.google.com:443 HTTP/1.1\r\n"));
            socket.write_all(reply).await.unwrap();
        });
        ProxyInput {
            host: "127.0.0.1".into(),
            port,
        }
    }
    #[tokio::test]
    async fn connect_responses() {
        assert!(
            connect_handshake(&serve(b"HTTP/1.1 200 OK\r\n\r\n").await)
                .await
                .is_ok()
        );
        for (reply, code) in [
            (b"HTTP/1.1 407 Auth\r\n\r\n".as_slice(), "AUTH_REQUIRED"),
            (b"HTTP/1.1 403 No\r\n\r\n", "CONNECT_REJECTED"),
            (b"SOCKS 200 OK\r\n\r\n", "PROTOCOL_ERROR"),
            (b"HTTP/1.1 200", "PROTOCOL_ERROR"),
        ] {
            assert_eq!(
                connect_handshake(&serve(reply).await)
                    .await
                    .unwrap_err()
                    .code,
                code
            );
        }
    }
    #[tokio::test]
    async fn rejects_invalid_tls_after_connect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = ProxyInput {
            host: "127.0.0.1".into(),
            port: listener.local_addr().unwrap().port(),
        };
        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(socket.read_u8().await.unwrap());
                }
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\n\r\ninvalid tls")
                    .await
                    .unwrap();
            }
        });
        let result = probe(&proxy).await.err().unwrap();
        assert_eq!(result.code, "TLS_OR_HTTP_FAILED");
        assert_eq!(result.stage, "https");
    }
}
