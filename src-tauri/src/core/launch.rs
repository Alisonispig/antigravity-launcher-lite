use super::{Result, error, proxy::ProxyInput};
use crate::platform::AppCandidate;
use serde::Serialize;
use std::{
    ffi::{OsStr, OsString},
    process::{Command, Stdio},
    time::Duration,
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

fn proxy_key(key: &OsStr) -> bool {
    [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "FTP_PROXY",
    ]
    .iter()
    .any(|v| key.to_string_lossy().eq_ignore_ascii_case(v))
}
pub fn environment(
    parent: impl Iterator<Item = (OsString, OsString)>,
    url: &str,
) -> Vec<(OsString, OsString)> {
    let mut env: Vec<_> = parent.filter(|(k, _)| !proxy_key(k)).collect();
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        env.push((key.into(), url.into()));
    }
    for key in ["NO_PROXY", "no_proxy"] {
        env.push((key.into(), "localhost,127.0.0.1,::1".into()));
    }
    env
}
pub fn ensure_not_running(app: &AppCandidate) -> Result<()> {
    let mut system = System::new();
    // 只刷新可执行路径，避免读取可能包含凭据的 argv / environ。
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
    );
    if system
        .processes()
        .values()
        .any(|p| p.exe().is_some_and(|p| p == app.executable))
    {
        return Err(error("ALREADY_RUNNING", "launch"));
    }
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub pid: u32,
    pub early_exit: bool,
}
pub async fn launch(app: AppCandidate, proxy: ProxyInput) -> Result<LaunchResult> {
    let url = proxy.url()?;
    ensure_not_running(&app)?;
    let mut child = Command::new(&app.executable)
        .env_clear()
        .envs(environment(std::env::vars_os(), &url))
        .current_dir(
            app.executable
                .parent()
                .ok_or_else(|| error("INVALID_APP", "launch"))?,
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| error("SPAWN_FAILED", "launch"))?;
    let pid = child.id();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let early_exit = child
        .try_wait()
        .map_err(|_| error("PROCESS_STATUS_FAILED", "launch"))?
        .is_some();
    if !early_exit {
        // 独立运行，启动器退出不终止目标；后台回收子进程，避免僵尸。
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
    Ok(LaunchResult { pid, early_exit })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_is_scoped_and_clean() {
        let parent = vec![
            ("HtTp_PrOxY".into(), "secret".into()),
            ("NO_PROXY".into(), "*".into()),
            ("PATH".into(), "kept".into()),
        ];
        let env = environment(parent.into_iter(), "http://127.0.0.1:7890");
        assert!(!env.iter().any(|(_, v)| v == "secret" || v == "*"));
        assert!(env.contains(&("PATH".into(), "kept".into())));
        assert!(env.contains(&("HTTPS_PROXY".into(), "http://127.0.0.1:7890".into())));
        assert!(env.contains(&("NO_PROXY".into(), "localhost,127.0.0.1,::1".into())));
    }
    #[cfg(unix)]
    #[test]
    fn child_observes_env_and_argv_without_shell_interpolation() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("测试 helper");
        std::fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\n' \"$HTTPS_PROXY\" \"$NO_PROXY\" \"$1\" \"$PWD\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let before = std::env::var_os("HTTPS_PROXY");
        let output = Command::new(&executable)
            .arg("literal $(echo unsafe)")
            .env_clear()
            .envs(environment(std::env::vars_os(), "http://127.0.0.1:7890"))
            .current_dir(dir.path())
            .output()
            .unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.starts_with(
            "http://127.0.0.1:7890\nlocalhost,127.0.0.1,::1\nliteral $(echo unsafe)\n"
        ));
        assert_eq!(std::env::var_os("HTTPS_PROXY"), before);
        assert!(output.status.success());
    }
}
