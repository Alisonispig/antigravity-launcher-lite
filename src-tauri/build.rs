fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "validate_app",
            "probe_proxy",
            "launch_antigravity",
            "load_config",
            "save_config",
            "set_window_theme",
            "recovery_status",
            "set_auto_restore_proxy",
            "set_close_after_launch",
        ]),
    ))
    .expect("构建 Tauri 权限清单失败");
}
