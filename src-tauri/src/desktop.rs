use tauri::{
    AppHandle, Manager, WindowEvent,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

fn background(theme: tauri::Theme) -> tauri::window::Color {
    match theme {
        tauri::Theme::Dark => tauri::window::Color(27, 32, 29, 255),
        _ => tauri::window::Color(255, 255, 255, 255),
    }
}

pub fn apply_theme(app: &AppHandle, theme: crate::core::config::Theme) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let theme = match theme {
        crate::core::config::Theme::Light => Some(tauri::Theme::Light),
        crate::core::config::Theme::Dark => Some(tauri::Theme::Dark),
        crate::core::config::Theme::System => None,
    };
    window.set_theme(theme)?;
    window.set_background_color(Some(background(theme.unwrap_or(window.theme()?))))
}

pub fn show_main(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular)?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()
}

pub fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    if let Some(window) = app.get_webview_window("main") {
        // 使用系统标题栏与窗口按钮，只让背景透出窗口底色。
        window.set_title_bar_style(tauri::TitleBarStyle::Transparent)?;
    }
    let open = MenuItem::with_id(app, "open-main", "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit-launcher", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &separator, &quit])?;
    #[cfg(target_os = "macos")]
    let icon = Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id("launcher-tray")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Antigravity Launcher Lite")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open-main" => {
                let _ = show_main(app);
            }
            "quit-launcher" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    // 托盘创建失败时 setup 返回错误，不留下无法重新打开的隐藏窗口。
    Ok(())
}

pub fn window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if let WindowEvent::ThemeChanged(theme) = event {
        let _ = window.set_background_color(Some(background(*theme)));
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window.app_handle().tray_by_id("launcher-tray").is_none() {
            return;
        }
        api.prevent_close();
        if window.hide().is_ok() {
            #[cfg(target_os = "macos")]
            if window
                .app_handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory)
                .is_err()
            {
                let _ = show_main(window.app_handle());
            }
        }
    }
}
