use super::{Result, error, proxy::ProxyInput};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_CONFIG_BYTES: u64 = 16 * 1024;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Zh,
    En,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    #[default]
    Light,
    Dark,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppConfig {
    pub schema_version: u32,
    pub client_path: String,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub language: Option<Language>,
    pub theme: Theme,
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            client_path: if cfg!(target_os = "macos") {
                "/Applications/Antigravity.app".into()
            } else {
                String::new()
            },
            proxy_host: "127.0.0.1".into(),
            proxy_port: 7890,
            language: None,
            theme: Theme::Light,
        }
    }
}
impl AppConfig {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(error("CONFIG_VERSION_UNSUPPORTED", "config"));
        }
        if self.client_path.len() > 4096 || self.client_path.chars().any(char::is_control) {
            return Err(error("INVALID_PATH", "config"));
        }
        // 不在保存时访问客户端文件；移动或暂时卸载客户端不应丢失用户设置。
        ProxyInput {
            host: self.proxy_host.clone(),
            port: self.proxy_port,
        }
        .url()?;
        Ok(())
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedConfig {
    pub config: AppConfig,
    pub recovered: bool,
}
#[derive(Default)]
pub struct ConfigStore(pub Mutex<()>);

pub fn load(path: &Path) -> Result<LoadedConfig> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LoadedConfig {
                config: AppConfig::default(),
                recovered: false,
            });
        }
        Err(_) => return Err(error("CONFIG_READ_FAILED", "config")),
    };
    let mut data = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|_| error("CONFIG_READ_FAILED", "config"))?;
    let parsed = if data.len() as u64 <= MAX_CONFIG_BYTES {
        serde_json::from_slice::<serde_json::Value>(&data).ok()
    } else {
        None
    };
    if let Some(version) = parsed
        .as_ref()
        .and_then(|v| v.get("schemaVersion"))
        .and_then(|v| v.as_u64())
    {
        // 较新版本必须保留原文件，不能作为“损坏”备份后覆盖。
        if version != 1 {
            return Err(error("CONFIG_VERSION_UNSUPPORTED", "config"));
        }
    }
    if let Some(config) = parsed
        .and_then(|v| serde_json::from_value::<AppConfig>(v).ok())
        .filter(|c| c.validate().is_ok())
    {
        return Ok(LoadedConfig {
            config,
            recovered: false,
        });
    }
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| error("CONFIG_READ_FAILED", "config"))?
        .as_nanos();
    fs::rename(
        path,
        path.with_file_name(format!("config.corrupt-{suffix}.json")),
    )
    .map_err(|_| error("CONFIG_BACKUP_FAILED", "config"))?;
    Ok(LoadedConfig {
        config: AppConfig::default(),
        recovered: true,
    })
}

pub fn save(path: &Path, config: &AppConfig) -> Result<()> {
    config.validate()?;
    // 保存前也检查磁盘版本，避免旧版应用覆盖较新版配置。
    match fs::File::open(path) {
        Ok(file) => {
            let mut data = Vec::new();
            file.take(MAX_CONFIG_BYTES + 1)
                .read_to_end(&mut data)
                .map_err(|_| error("CONFIG_READ_FAILED", "config"))?;
            if data.len() as u64 > MAX_CONFIG_BYTES {
                return Err(error("CONFIG_READ_FAILED", "config"));
            }
            let value: serde_json::Value =
                serde_json::from_slice(&data).map_err(|_| error("CONFIG_READ_FAILED", "config"))?;
            if value.get("schemaVersion").and_then(|v| v.as_u64()) != Some(1) {
                return Err(error("CONFIG_VERSION_UNSUPPORTED", "config"));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(error("CONFIG_READ_FAILED", "config")),
    }
    let parent = path
        .parent()
        .ok_or_else(|| error("CONFIG_WRITE_FAILED", "config"))?;
    fs::create_dir_all(parent).map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    let data =
        serde_json::to_vec_pretty(config).map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    temp.write_all(&data)
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    temp.persist(path)
        .map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    #[cfg(unix)]
    fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    Ok(())
}

pub fn config_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    use tauri::Manager;
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|_| error("CONFIG_READ_FAILED", "config"))?
        .join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_atomic_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("设置/config.json");
        assert!(!load(&path).unwrap().recovered);
        let mut config = AppConfig {
            client_path: "/测试路径/Antigravity.app".into(),
            proxy_port: 17891,
            ..AppConfig::default()
        };
        save(&path, &config).unwrap();
        assert_eq!(load(&path).unwrap().config, config);
        config.proxy_host = "::1".into();
        config.theme = Theme::Dark;
        save(&path, &config).unwrap();
        assert_eq!(load(&path).unwrap().config, config);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }
    #[test]
    fn corrupt_config_is_preserved_and_recovered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        for broken in [b"{broken".as_slice(), b"{\"schemaVersion\":1}"] {
            fs::write(&path, broken).unwrap();
            assert!(load(&path).unwrap().recovered);
            assert!(!path.exists());
            assert!(
                fs::read_dir(dir.path())
                    .unwrap()
                    .any(|entry| fs::read(entry.unwrap().path()).unwrap() == broken)
            );
        }
    }
    #[test]
    fn invalid_input_and_future_version_never_replace_saved_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = AppConfig::default();
        save(&path, &config).unwrap();
        let saved = fs::read(&path).unwrap();
        config.proxy_host = "user:password@localhost".into();
        assert!(save(&path, &config).is_err());
        assert_eq!(fs::read(&path).unwrap(), saved);
        let future = b"{\"schemaVersion\":2,\"newField\":true}";
        fs::write(&path, future).unwrap();
        assert_eq!(
            load(&path).err().unwrap().code,
            "CONFIG_VERSION_UNSUPPORTED"
        );
        assert!(save(&path, &AppConfig::default()).is_err());
        assert_eq!(fs::read(&path).unwrap(), future);
    }
    #[test]
    fn io_failure_does_not_silently_reset_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::create_dir(&path).unwrap();
        assert!(load(&path).is_err());
        assert!(save(&path, &AppConfig::default()).is_err());
        assert!(path.is_dir());
    }
}
