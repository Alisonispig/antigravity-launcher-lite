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
) -> Result<core::launch::LaunchResult> {
    let _guard = state.0.try_lock().map_err(|_| error("BUSY", "input"))?;
    let app = platform::validate(&path)?;
    core::launch::ensure_not_running(&app)?;
    core::proxy::probe(&proxy).await?;
    // 检测后重验路径及旧实例，避免复用过期验证结果。
    core::launch::launch(platform::validate(&path)?, proxy).await
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
    core::config::load(&core::config::config_path(&app)?)
}
#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    store: tauri::State<'_, core::config::ConfigStore>,
    config: core::config::AppConfig,
) -> Result<()> {
    let _guard = store
        .0
        .lock()
        .map_err(|_| error("CONFIG_WRITE_FAILED", "config"))?;
    core::config::save(&core::config::config_path(&app)?, &config)
}
#[tauri::command]
fn set_window_theme(app: tauri::AppHandle, theme: core::config::Theme) -> Result<()> {
    desktop::apply_theme(&app, theme).map_err(|_| error("THEME_FAILED", "window"))
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Operation::default())
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
            set_window_theme
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
