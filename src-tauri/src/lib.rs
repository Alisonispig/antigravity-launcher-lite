mod core;
mod desktop;
mod platform;
use core::{Result, error, proxy::ProxyInput};
use tokio::sync::Mutex;

#[derive(Default)]
struct Operation(Mutex<()>);
#[tauri::command]
fn validate_app(path: String) -> Result<platform::AppCandidate> {
    platform::validate(&path)
}
#[tauri::command]
async fn probe_proxy(
    proxy: ProxyInput,
    state: tauri::State<'_, Operation>,
) -> Result<core::proxy::ProbeResult> {
    let _guard = state.0.try_lock().map_err(|_| error("BUSY", "input"))?;
    core::proxy::probe(&proxy).await
}
#[tauri::command]
async fn launch_antigravity(
    path: String,
    proxy: ProxyInput,
    state: tauri::State<'_, Operation>,
    handle: tauri::AppHandle,
    recovery: tauri::State<'_, core::recovery::Recovery>,
) -> Result<core::launch::LaunchResult> {
    let _guard = state.0.try_lock().map_err(|_| error("BUSY", "input"))?;
    let app = platform::validate(&path)?;
    core::launch::ensure_not_running(&app)?;
    core::proxy::probe(&proxy).await?;
    // 检测后重验路径及旧实例，避免复用过期验证结果。
    let app = platform::validate(&path)?;
    let installation = if recovery.status().enabled && recovery.status().supported {
        let candidate = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            core::recovery::Installation::read(&candidate, None)
        })
        .await
        .ok()
        .and_then(|r| r.ok())
    } else {
        None
    };
    let result = core::launch::launch(app.clone(), proxy.clone()).await?;
    if !result.early_exit
        && let Some(installation) = installation
    {
        recovery.attach(handle, app, proxy, result.pid, installation);
    } else {
        recovery.unavailable(handle);
    }
    Ok(result)
}
#[tauri::command]
fn load_config(
    app: tauri::AppHandle,
    store: tauri::State<'_, core::config::ConfigStore>,
) -> Result<core::config::LoadedConfig> {
    let _guard = store
        .0
        .lock()
        .map_err(|_| error("CONFIG_READ_FAILED", "config"))?;
    let loaded = core::config::load(&core::config::config_path(&app)?)?;
    use tauri::Manager;
    app.state::<core::recovery::Recovery>()
        .configure(app.clone(), loaded.config.auto_restore_proxy);
    Ok(loaded)
}
#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    store: tauri::State<'_, core::config::ConfigStore>,
    mut config: core::config::AppConfig,
    recovery: tauri::State<'_, core::recovery::Recovery>,
) -> Result<()> {
    let _guard = store
        .0
        .lock()
        .map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    // 开关有独立保存入口，普通字段的迟到保存不能重新开启/关闭恢复。
    config.auto_restore_proxy = recovery.status().enabled;
    core::config::save(&core::config::config_path(&app)?, &config)?;
    recovery.configure(app, config.auto_restore_proxy);
    Ok(())
}
#[tauri::command]
fn set_auto_restore_proxy(
    app: tauri::AppHandle,
    store: tauri::State<'_, core::config::ConfigStore>,
    recovery: tauri::State<'_, core::recovery::Recovery>,
    enabled: bool,
) -> Result<()> {
    let _guard = store
        .0
        .lock()
        .map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    let path = core::config::config_path(&app)?;
    let mut loaded = core::config::load(&path)?;
    loaded.config.auto_restore_proxy = enabled;
    core::config::save(&path, &loaded.config)?;
    recovery.configure(app, enabled);
    Ok(())
}
#[tauri::command]
fn recovery_status(recovery: tauri::State<'_, core::recovery::Recovery>) -> core::recovery::Status {
    recovery.status()
}
#[tauri::command]
fn set_window_theme(app: tauri::AppHandle, theme: core::config::Theme) -> Result<()> {
    desktop::apply_theme(&app, theme).map_err(|_| error("THEME_FAILED", "window"))
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Operation::default())
        .manage(core::recovery::Recovery::default())
        .manage(core::config::ConfigStore::default())
        .setup(|app| {
            desktop::setup(app)?;
            Ok(())
        })
        .on_window_event(desktop::window_event)
        .invoke_handler(tauri::generate_handler![
            validate_app,
            probe_proxy,
            launch_antigravity,
            load_config,
            save_config,
            set_window_theme,
            recovery_status,
            set_auto_restore_proxy
        ])
        .build(tauri::generate_context!())
        .expect("运行启动器失败")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                let _ = desktop::show_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
