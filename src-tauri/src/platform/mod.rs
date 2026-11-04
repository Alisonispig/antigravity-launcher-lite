use crate::core::{Result, error};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCandidate {
    pub path: PathBuf,
    pub executable: PathBuf,
}
pub fn validate(path: &str) -> Result<AppCandidate> {
    if path.is_empty() || path.len() > 4096 {
        return Err(error("INVALID_PATH", "app"));
    }
    let selected = Path::new(path);
    if !selected.is_absolute() {
        return Err(error("INVALID_PATH", "app"));
    }
    let path = selected
        .canonicalize()
        .map_err(|_| error("INVALID_PATH", "app"))?;
    let executable = resolve(&path)?;
    if !executable.is_file() {
        return Err(error("INVALID_APP", "app"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if executable
            .metadata()
            .map_err(|_| error("INVALID_APP", "app"))?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(error("NOT_EXECUTABLE", "app"));
        }
    }
    Ok(AppCandidate { path, executable })
}
#[cfg(target_os = "macos")]
fn resolve(path: &Path) -> Result<PathBuf> {
    if path.extension().and_then(|v| v.to_str()) != Some("app") {
        return Err(error("INVALID_APP", "app"));
    }
    let value = plist::Value::from_file(path.join("Contents/Info.plist"))
        .map_err(|_| error("INVALID_APP", "app"))?;
    let info = value
        .as_dictionary()
        .ok_or_else(|| error("INVALID_APP", "app"))?;
    if info.get("CFBundleIdentifier").and_then(|v| v.as_string()) != Some("com.google.antigravity")
    {
        return Err(error("INVALID_APP", "app"));
    }
    let name = info
        .get("CFBundleExecutable")
        .and_then(|v| v.as_string())
        .ok_or_else(|| error("INVALID_APP", "app"))?;
    if Path::new(name).components().count() != 1 || name == "." || name == ".." {
        return Err(error("INVALID_APP", "app"));
    }
    let root = path
        .join("Contents/MacOS")
        .canonicalize()
        .map_err(|_| error("INVALID_APP", "app"))?;
    let exe = root
        .join(name)
        .canonicalize()
        .map_err(|_| error("INVALID_APP", "app"))?;
    if !exe.starts_with(&root) {
        return Err(error("INVALID_APP", "app"));
    }
    Ok(exe)
}
#[cfg(target_os = "windows")]
fn resolve(path: &Path) -> Result<PathBuf> {
    if !path
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("antigravity.exe"))
    {
        return Err(error("INVALID_APP", "app"));
    }
    Ok(path.to_owned())
}
#[cfg(target_os = "linux")]
fn resolve(path: &Path) -> Result<PathBuf> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if name != "antigravity" && name != "Antigravity.AppImage" {
        return Err(error("INVALID_APP", "app"));
    }
    Ok(path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_relative_and_missing_paths() {
        assert_eq!(
            validate("relative/antigravity").err().unwrap().code,
            "INVALID_PATH"
        );
        assert!(validate("/does-not-exist/Antigravity.app").is_err());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn resolves_bundle_and_rejects_escape() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("测试 Antigravity.app");
        let root = bundle.join("Contents/MacOS");
        std::fs::create_dir_all(&root).unwrap();
        let executable = root.join("Antigravity");
        std::fs::write(&executable, b"test fixture").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let info = bundle.join("Contents/Info.plist");
        let write_info = |name: &str| {
            let mut d = plist::Dictionary::new();
            d.insert("CFBundleIdentifier".into(), "com.google.antigravity".into());
            d.insert("CFBundleExecutable".into(), name.into());
            plist::Value::Dictionary(d).to_file_xml(&info).unwrap();
        };
        write_info("Antigravity");
        assert_eq!(
            validate(bundle.to_str().unwrap()).unwrap().executable,
            executable.canonicalize().unwrap()
        );
        write_info("../../../escape");
        assert!(validate(bundle.to_str().unwrap()).is_err());
    }
}
