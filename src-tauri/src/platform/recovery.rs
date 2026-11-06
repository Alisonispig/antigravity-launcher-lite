//! 只发送正常关闭请求；不使用 kill、SIGTERM 或强制终止 API。
use super::AppCandidate;
use crate::core::{
    Result, error,
    recovery::{ProcessId, processes},
};

pub fn supported() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
        || (cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_some())
}

pub async fn request_quit(
    handle: &tauri::AppHandle,
    app: &AppCandidate,
    id: ProcessId,
    permitted: impl Fn() -> bool + Send + 'static,
) -> Result<()> {
    if !permitted() {
        return Err(error("RECOVERY_CANCELLED", "recovery"));
    }
    if processes(app) != [id] {
        return Err(error("UPDATE_PROCESS_CHANGED", "recovery"));
    }
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let app = app.clone();
        handle
            .run_on_main_thread(move || {
                let result = if !permitted() || processes(&app) != [id] {
                    false
                } else {
                    mac_quit(id.pid)
                };
                let _ = tx.send(result);
            })
            .map_err(|_| error("QUIT_REQUEST_FAILED", "recovery"))?;
        let sent = tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(false);
        if !sent {
            return Err(error("QUIT_REQUEST_FAILED", "recovery"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        let _ = handle;
        use windows_sys::Win32::{
            Foundation::{HWND, LPARAM},
            UI::WindowsAndMessaging::*,
        };
        struct Request {
            pid: u32,
            sent: bool,
        }
        unsafe extern "system" fn close(window: HWND, data: LPARAM) -> i32 {
            // EnumWindows 同步回调，data 指向调用期间有效的 Request。
            let request = unsafe { &mut *(data as *mut Request) };
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(window, &mut pid);
            }
            if pid == request.pid
                && unsafe { IsWindowVisible(window) } != 0
                && unsafe { GetWindow(window, GW_OWNER) }.is_null()
            {
                request.sent |= unsafe { PostMessageW(window, WM_CLOSE, 0, 0) } != 0;
            }
            1
        }
        let mut request = Request {
            pid: id.pid,
            sent: false,
        };
        unsafe {
            EnumWindows(Some(close), &mut request as *mut Request as LPARAM);
        }
        if !request.sent {
            return Err(error("QUIT_REQUEST_FAILED", "recovery"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        let _ = handle;
        // X11/EWMH 请求窗口管理器正常关闭目标窗口；原生 Wayland 不伪装支持。
        let sent = tauri::async_runtime::spawn_blocking(move || x11_quit(id.pid, permitted))
            .await
            .unwrap_or(false);
        if !sent {
            return Err(error("QUIT_REQUEST_FAILED", "recovery"));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn mac_quit(pid: u32) -> bool {
    objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid as i32)
        .is_some_and(|running| running.terminate())
}

#[cfg(target_os = "linux")]
fn x11_quit(pid: u32, permitted: impl Fn() -> bool) -> bool {
    use x11rb::{
        connection::Connection,
        protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask},
    };
    let send = || -> Option<bool> {
        let (connection, screen) = x11rb::connect(None).ok()?;
        let root = connection.setup().roots.get(screen)?.root;
        let atom = |name: &[u8]| {
            connection
                .intern_atom(true, name)
                .ok()?
                .reply()
                .ok()
                .map(|r| r.atom)
        };
        let clients = atom(b"_NET_CLIENT_LIST")?;
        let pid_atom = atom(b"_NET_WM_PID")?;
        let close = atom(b"_NET_CLOSE_WINDOW")?;
        if clients == 0 || pid_atom == 0 || close == 0 {
            return None;
        }
        let list = connection
            .get_property(false, root, clients, AtomEnum::WINDOW, 0, 4096)
            .ok()?
            .reply()
            .ok()?;
        let mut sent = false;
        for window in list.value32()? {
            if !permitted() {
                break;
            }
            let property = connection
                .get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)
                .ok()
                .and_then(|c| c.reply().ok());
            if property
                .as_ref()
                .and_then(|p| p.value32())
                .and_then(|mut p| p.next())
                != Some(pid)
            {
                continue;
            }
            let event = ClientMessageEvent::new(32, window, close, [0, 1, 0, 0, 0]);
            sent |= connection
                .send_event(
                    false,
                    root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                )
                .ok()
                .is_some_and(|cookie| cookie.check().is_ok());
        }
        connection.flush().ok()?;
        Some(sent)
    };
    send().unwrap_or(false)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };
    #[test]
    #[ignore = "需要 macOS 图形会话和 clang；仅启动测试自己的 Cocoa 辅助应用"]
    fn native_quit_respects_refusal_then_exits_normally() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("Recovery Fixture.app");
        let root = bundle.join("Contents/MacOS");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(bundle.join("Contents/Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>io.github.launcher.recovery-fixture</string><key>CFBundleExecutable</key><string>fixture</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#).unwrap();
        let source = dir.path().join("fixture.m");
        std::fs::write(&source, r#"
#import <Cocoa/Cocoa.h>
static NSString *allowPath;
static NSString *readyPath;
@interface Delegate : NSObject <NSApplicationDelegate>
@end
@implementation Delegate
- (void)applicationDidFinishLaunching:(NSNotification *)note {
    [@"ready" writeToFile:readyPath atomically:YES encoding:NSUTF8StringEncoding error:nil];
}
- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender {
    return [[NSFileManager defaultManager] fileExistsAtPath:allowPath] ? NSTerminateNow : NSTerminateCancel;
}
@end
int main(int argc, const char **argv) {
    @autoreleasepool {
        allowPath = [NSString stringWithUTF8String:argv[1]];
        readyPath = [NSString stringWithUTF8String:argv[2]];
        NSApplication *app = [NSApplication sharedApplication];
        [app setActivationPolicy:NSApplicationActivationPolicyAccessory];
        Delegate *delegate = [Delegate new];
        [app setDelegate:delegate];
        [app run];
    }
    return 0;
}
"#).unwrap();
        let executable = root.join("fixture");
        assert!(
            Command::new("clang")
                .arg("-w")
                .arg("-framework")
                .arg("Cocoa")
                .arg(&source)
                .arg("-o")
                .arg(&executable)
                .status()
                .unwrap()
                .success()
        );
        let allow = dir.path().join("allow");
        let ready = dir.path().join("ready");
        struct OwnChild(std::process::Child);
        impl Drop for OwnChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut child = OwnChild(
            Command::new(&executable)
                .arg(&allow)
                .arg(&ready)
                .spawn()
                .unwrap(),
        );
        let start = Instant::now();
        while !ready.exists() && start.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(ready.exists());
        assert!(mac_quit(child.0.id()));
        std::thread::sleep(Duration::from_millis(800));
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "拒绝正常退出时进程必须继续运行"
        );
        std::fs::write(&allow, b"saved").unwrap();
        assert!(mac_quit(child.0.id()));
        let start = Instant::now();
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "正常退出未完成");
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
