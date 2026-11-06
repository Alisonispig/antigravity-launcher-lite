//! 仅跟踪本次启动器会话；安装内容变化 + 原进程消失 + 新主进程稳定后才恢复。
use super::{Result, error, launch, proxy};
use crate::platform::{self, AppCandidate};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::Manager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessId {
    pub pid: u32,
    pub started: u64,
}

// 不读取命令行、进程环境或客户端用户数据；排除共用 Electron 可执行文件的子进程。
pub fn processes(app: &AppCandidate) -> Vec<ProcessId> {
    snapshot(app, None)
}
fn snapshot(app: &AppCandidate, tracked: Option<ProcessId>) -> Vec<ProcessId> {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
    );
    let mut roots: Vec<_> = system
        .processes()
        .iter()
        .filter_map(|(pid, p)| {
            if !p
                .exe()
                .is_some_and(|exe| platform::same_executable(exe, &app.executable))
            {
                return None;
            }
            if p.parent()
                .and_then(|parent| system.process(parent))
                .is_some_and(|parent| {
                    parent
                        .exe()
                        .is_some_and(|exe| platform::same_executable(exe, &app.executable))
                })
            {
                return None;
            }
            Some(ProcessId {
                pid: pid.as_u32(),
                started: p.start_time(),
            })
        })
        .collect();
    // 更新可重命名/删除旧文件，旧进程的 exe 路径会变；不能把路径消失当成进程退出。
    if let Some(id) = tracked
        && system
            .process(sysinfo::Pid::from_u32(id.pid))
            .is_some_and(|p| p.start_time() == id.started)
        && !roots.contains(&id)
    {
        roots.push(id);
    }
    roots
}

#[derive(Clone, Debug, PartialEq)]
struct Stamp(Vec<(PathBuf, u64, SystemTime)>);
#[derive(Clone)]
pub struct Installation {
    stamp: Stamp,
    digest: [u8; 32],
}
impl Installation {
    pub fn read(app: &AppCandidate, cached: Option<&Self>) -> Result<Self> {
        let mut files = vec![app.executable.clone()];
        #[cfg(target_os = "macos")]
        files.push(app.path.join("Contents/Info.plist"));
        // Electron 的更新可能只替换资源，主可执行文件不变。
        let root = if cfg!(target_os = "macos") {
            app.path.join("Contents/Resources")
        } else {
            app.executable
                .parent()
                .ok_or_else(|| error("INVALID_APP", "recovery"))?
                .join("resources")
        };
        for relative in ["app/package.json", "app/product.json", "app.asar"] {
            let path = root.join(relative);
            if path.is_file() {
                files.push(path);
            }
        }
        let installation_root = if cfg!(target_os = "macos") {
            app.path.clone()
        } else {
            app.executable
                .parent()
                .ok_or_else(|| error("INVALID_APP", "recovery"))?
                .to_path_buf()
        };
        let installation_root = installation_root
            .canonicalize()
            .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?;
        for path in &files {
            if !path
                .canonicalize()
                .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?
                .starts_with(&installation_root)
            {
                return Err(error("UPDATE_READ_FAILED", "recovery"));
            }
        }
        let stamp = Stamp(
            files
                .iter()
                .map(|path| {
                    let meta = path
                        .metadata()
                        .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?;
                    Ok((
                        path.clone(),
                        meta.len(),
                        meta.modified()
                            .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?,
                    ))
                })
                .collect::<Result<_>>()?,
        );
        if let Some(cached) = cached.filter(|cached| cached.stamp == stamp) {
            return Ok(cached.clone());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        for path in &files {
            let mut file = File::open(path).map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?;
            hash.update(
                file.metadata()
                    .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?
                    .len()
                    .to_le_bytes(),
            );
            loop {
                let n = file
                    .read(&mut buffer)
                    .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
            }
        }
        // 更新正在写入时丢弃本轮结果，下一轮重试。
        for (path, len, modified) in &stamp.0 {
            let meta = path
                .metadata()
                .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))?;
            if meta.len() != *len || meta.modified().ok() != Some(*modified) {
                return Err(error("UPDATE_READ_FAILED", "recovery"));
            }
        }
        Ok(Self {
            stamp,
            digest: hash.finalize().into(),
        })
    }
}

#[derive(Default)]
struct Gate {
    missing_since: Option<u64>,
    candidate: Option<(ProcessId, u64)>,
}
#[derive(Debug, PartialEq)]
enum Decision {
    Watch,
    Wait,
    Recover(ProcessId),
    Stop,
}
impl Gate {
    fn step(
        &mut self,
        now: u64,
        original: ProcessId,
        roots: &[ProcessId],
        changed: bool,
    ) -> Decision {
        if roots.contains(&original) {
            self.missing_since = None;
            self.candidate = None;
            return Decision::Watch;
        }
        let since = *self.missing_since.get_or_insert(now);
        if now.saturating_sub(since) >= 120 {
            return Decision::Stop;
        }
        if !changed || roots.len() != 1 {
            self.candidate = None;
            return Decision::Wait;
        }
        let id = roots[0];
        match self.candidate {
            Some((old, first)) if old == id && now.saturating_sub(first) >= 6 => {
                Decision::Recover(id)
            }
            Some((old, _)) if old == id => Decision::Wait,
            _ => {
                self.candidate = Some((id, now));
                Decision::Wait
            }
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub supported: bool,
    pub phase: &'static str,
    pub code: Option<&'static str>,
    pub revision: u64,
}
#[derive(Clone)]
struct Session {
    app: AppCandidate,
    proxy: proxy::ProxyInput,
    original: ProcessId,
    installation: Installation,
}
struct Inner {
    epoch: u64,
    session: Option<Session>,
    status: Status,
}
#[derive(Clone)]
pub struct Recovery(Arc<Mutex<Inner>>);
impl Default for Recovery {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            epoch: 0,
            session: None,
            status: Status {
                enabled: false,
                supported: platform::recovery::supported(),
                phase: "off",
                code: None,
                revision: 0,
            },
        })))
    }
}
impl Recovery {
    pub fn status(&self) -> Status {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
    }
    fn active(&self, epoch: u64) -> bool {
        let inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        inner.epoch == epoch && inner.status.enabled
    }
    fn report(&self, epoch: u64, phase: &'static str, code: Option<&'static str>) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if inner.epoch != epoch {
            return;
        }
        if inner.status.phase != phase || inner.status.code != code {
            inner.status.phase = phase;
            inner.status.code = code;
            inner.status.revision += 1;
        }
    }
    pub fn configure(&self, handle: tauri::AppHandle, enabled: bool) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if inner.status.enabled == enabled {
            return;
        }
        inner.status.enabled = enabled;
        drop(inner);
        self.restart_watch(handle);
    }
    pub fn unavailable(&self, handle: tauri::AppHandle) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).session = None;
        self.restart_watch(handle);
        let inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let epoch = inner.epoch;
        let enabled = inner.status.enabled;
        drop(inner);
        if enabled {
            self.report(epoch, "failed", Some("UPDATE_PROCESS_CHANGED"));
        }
    }
    pub fn attach(
        &self,
        handle: tauri::AppHandle,
        app: AppCandidate,
        proxy: proxy::ProxyInput,
        pid: u32,
        installation: Installation,
    ) {
        let original = processes(&app).into_iter().find(|p| p.pid == pid);
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        inner.session = original.map(|original| Session {
            app,
            proxy,
            original,
            installation,
        });
        drop(inner);
        self.restart_watch(handle);
        let inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let epoch = inner.epoch;
        let unobserved = inner.status.enabled && inner.status.supported && inner.session.is_none();
        drop(inner);
        if unobserved {
            self.report(epoch, "failed", Some("UPDATE_PROCESS_CHANGED"));
        }
    }
    fn restart_watch(&self, handle: tauri::AppHandle) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        inner.epoch += 1;
        inner.status.revision += 1;
        inner.status.code = None;
        let enabled = inner.status.enabled && inner.status.supported;
        inner.status.phase = if !inner.status.enabled {
            "off"
        } else if !inner.status.supported {
            "unsupported"
        } else if inner.session.is_some() {
            "watching"
        } else {
            "armed"
        };
        let session = inner.session.clone();
        let epoch = inner.epoch;
        drop(inner);
        if let Some(session) = session.filter(|_| enabled) {
            let this = self.clone();
            tauri::async_runtime::spawn(async move {
                this.watch(handle, epoch, session).await;
            });
        }
    }
    async fn watch(&self, handle: tauri::AppHandle, epoch: u64, mut session: Session) {
        let mut start = Instant::now();
        let mut gate = Gate::default();
        let mut baseline = session.installation.digest;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if !self.active(epoch) {
                return;
            }
            let app = session.app.clone();
            let cached = session.installation.clone();
            let tracked = session.original;
            let observation = tauri::async_runtime::spawn_blocking(move || {
                let validated = platform::validate(&app.path.to_string_lossy());
                let installation = validated
                    .as_ref()
                    .ok()
                    .and_then(|a| Installation::read(a, Some(&cached)).ok());
                (snapshot(&app, Some(tracked)), installation)
            })
            .await;
            let Ok((roots, installation)) = observation else {
                self.report(epoch, "failed", Some("UPDATE_READ_FAILED"));
                return;
            };
            let readable = installation.is_some();
            if let Some(installation) = installation {
                session.installation = installation;
            }
            match gate.step(
                start.elapsed().as_secs(),
                session.original,
                &roots,
                readable && session.installation.digest != baseline,
            ) {
                Decision::Watch => self.report(epoch, "watching", None),
                Decision::Wait => self.report(epoch, "waiting", None),
                Decision::Stop => {
                    self.finish(epoch, "stopped", None);
                    return;
                }
                Decision::Recover(id) => {
                    self.report(epoch, "recovering", None);
                    let result = self.restore(&handle, epoch, &session, id).await;
                    match result {
                        Ok(Some(next)) => {
                            session = next;
                            baseline = session.installation.digest;
                            gate = Gate::default();
                            start = Instant::now();
                            self.report(epoch, "restored", None);
                        }
                        Ok(None) => return,
                        Err(err) => {
                            self.finish(epoch, "failed", Some(err.code));
                            return;
                        }
                    }
                }
            }
        }
    }
    fn finish(&self, epoch: u64, phase: &'static str, code: Option<&'static str>) {
        self.report(epoch, phase, code);
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if inner.epoch == epoch {
            inner.session = None;
        }
    }
    async fn restore(
        &self,
        handle: &tauri::AppHandle,
        epoch: u64,
        session: &Session,
        id: ProcessId,
    ) -> Result<Option<Session>> {
        let operation = handle.state::<crate::Operation>();
        let _guard = operation.0.lock().await;
        if !self.active(epoch) {
            return Ok(None);
        }
        let app = platform::validate(&session.app.path.to_string_lossy())?;
        if processes(&app) != [id] {
            return Err(error("UPDATE_PROCESS_CHANGED", "recovery"));
        }
        // 先确认原代理仍可用，避免退出后无法恢复。
        proxy::probe(&session.proxy).await?;
        if !self.active(epoch) {
            return Ok(None);
        }
        if processes(&app) != [id] {
            return Err(error("UPDATE_PROCESS_CHANGED", "recovery"));
        }
        let permit = self.clone();
        platform::recovery::request_quit(handle, &app, id, move || permit.active(epoch)).await?;
        self.report(epoch, "quitting", None);
        let start = Instant::now();
        let mut empty_polls = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if !self.active(epoch) {
                return Ok(None);
            }
            let roots = processes(&app);
            if roots.is_empty() {
                empty_polls += 1;
            } else {
                empty_polls = 0;
            }
            // 两次空快照，避免进程交接间隙重复启动。
            if empty_polls >= 2 {
                break;
            }
            if roots.iter().any(|p| *p != id) {
                return Err(error("UPDATE_PROCESS_CHANGED", "recovery"));
            }
            if start.elapsed() >= Duration::from_secs(120) {
                return Err(error("QUIT_NOT_COMPLETED", "recovery"));
            }
        }
        if !self.active(epoch) {
            return Ok(None);
        }
        let app = platform::validate(&session.app.path.to_string_lossy())?;
        let candidate = app.clone();
        let cached = session.installation.clone();
        let installation = tauri::async_runtime::spawn_blocking(move || {
            Installation::read(&candidate, Some(&cached))
        })
        .await
        .map_err(|_| error("UPDATE_READ_FAILED", "recovery"))??;
        if !self.active(epoch) {
            return Ok(None);
        }
        // 正常 launch 再次校验已有实例；不绕过原有代理约束。
        let result = launch::launch(app.clone(), session.proxy.clone()).await?;
        if result.early_exit {
            return Err(error("SPAWN_FAILED", "recovery"));
        }
        let next = processes(&app)
            .into_iter()
            .find(|p| p.pid == result.pid)
            .map(|original| Session {
                app,
                proxy: session.proxy.clone(),
                original,
                installation,
            });
        if next.is_none() {
            return Err(error("UPDATE_PROCESS_CHANGED", "recovery"));
        }
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if inner.epoch != epoch {
            return Ok(None);
        }
        inner.session = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const OLD: ProcessId = ProcessId {
        pid: 10,
        started: 1,
    };
    const NEW: ProcessId = ProcessId {
        pid: 20,
        started: 2,
    };
    #[test]
    fn stale_watch_cannot_act_or_overwrite_new_status() {
        let recovery = Recovery::default();
        {
            let mut inner = recovery.0.lock().unwrap();
            inner.status.enabled = true;
            inner.epoch = 2;
        }
        assert!(!recovery.active(1));
        assert!(recovery.active(2));
        recovery.report(1, "quitting", None);
        assert_eq!(recovery.status().phase, "off");
        recovery.report(2, "watching", None);
        recovery.0.lock().unwrap().status.enabled = false;
        assert!(!recovery.active(2));
    }
    #[test]
    fn installation_compares_content_and_tracks_electron_resource_updates() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Antigravity.app");
        let executable = root.join("Contents/MacOS/Antigravity");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"executable fixture").unwrap();
        std::fs::write(root.join("Contents/Info.plist"), b"version fixture").unwrap();
        let resources = if cfg!(target_os = "macos") {
            root.join("Contents/Resources")
        } else {
            executable.parent().unwrap().join("resources")
        };
        std::fs::create_dir_all(&resources).unwrap();
        let asar = resources.join("app.asar");
        std::fs::write(&asar, b"resource version 1").unwrap();
        let app = AppCandidate {
            path: root,
            executable,
        };
        let before = Installation::read(&app, None).unwrap();
        assert_eq!(
            Installation::read(&app, Some(&before)).unwrap().digest,
            before.digest
        );
        // 相同内容即便重新写入也不能当成更新。
        std::fs::write(&asar, b"resource version 1").unwrap();
        assert_eq!(
            Installation::read(&app, None).unwrap().digest,
            before.digest
        );
        std::fs::write(&asar, b"resource version 2 updated").unwrap();
        assert_ne!(
            Installation::read(&app, Some(&before)).unwrap().digest,
            before.digest
        );
        std::fs::remove_file(&app.executable).unwrap();
        assert!(Installation::read(&app, Some(&before)).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn follows_real_process_replacement_at_the_same_path() {
        use std::process::Command;
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("client fixture");
        std::fs::copy("/bin/sleep", &executable).unwrap();
        let executable = executable.canonicalize().unwrap();
        let app = AppCandidate {
            path: executable.clone(),
            executable,
        };
        let mut old = Command::new(&app.executable).arg("30").spawn().unwrap();
        let original = processes(&app)
            .into_iter()
            .find(|p| p.pid == old.id())
            .unwrap();
        // 安装器移动旧文件时，旧进程仍活着，不能提前认为已退出。
        std::fs::rename(&app.executable, dir.path().join("old client")).unwrap();
        std::fs::copy("/bin/sleep", &app.executable).unwrap();
        assert!(snapshot(&app, Some(original)).contains(&original));
        // 仅结束本测试创建的辅助进程，绝不操作真实 Antigravity。
        old.kill().unwrap();
        old.wait().unwrap();
        assert!(processes(&app).is_empty());
        let mut new = Command::new(&app.executable).arg("30").spawn().unwrap();
        let roots = processes(&app);
        new.kill().unwrap();
        new.wait().unwrap();
        assert_eq!(roots.len(), 1);
        assert_ne!(roots[0], original);
        let mut gate = Gate::default();
        assert_eq!(gate.step(0, original, &roots, true), Decision::Wait);
        assert_eq!(
            gate.step(6, original, &roots, true),
            Decision::Recover(roots[0])
        );
    }
    #[test]
    fn only_changed_installation_and_stable_replacement_can_recover() {
        let mut g = Gate::default();
        assert_eq!(g.step(0, OLD, &[OLD], true), Decision::Watch);
        assert_eq!(g.step(2, OLD, &[], true), Decision::Wait);
        assert_eq!(g.step(4, OLD, &[NEW], true), Decision::Wait);
        assert_eq!(g.step(8, OLD, &[NEW], true), Decision::Wait);
        assert_eq!(g.step(10, OLD, &[NEW], true), Decision::Recover(NEW));
    }
    #[test]
    fn ordinary_exit_restart_pid_reuse_and_ambiguous_roots_do_not_recover() {
        let mut g = Gate::default();
        assert_eq!(g.step(0, OLD, &[], false), Decision::Wait);
        assert_eq!(g.step(8, OLD, &[NEW], false), Decision::Wait);
        assert_eq!(g.step(120, OLD, &[NEW], false), Decision::Stop);
        let reused = ProcessId {
            pid: OLD.pid,
            started: 99,
        };
        let mut g = Gate::default();
        assert_eq!(g.step(0, OLD, &[reused], true), Decision::Wait);
        assert_eq!(g.step(8, OLD, &[reused, NEW], true), Decision::Wait);
        assert_eq!(g.step(10, OLD, &[reused], true), Decision::Wait);
        assert_eq!(g.step(16, OLD, &[reused], true), Decision::Recover(reused));
    }
    #[test]
    fn unobservable_or_changing_replacement_resets_stability() {
        let mut g = Gate::default();
        assert_eq!(g.step(0, OLD, &[NEW], true), Decision::Wait);
        assert_eq!(g.step(6, OLD, &[NEW], false), Decision::Wait);
        assert_eq!(g.step(8, OLD, &[NEW], true), Decision::Wait);
        assert_eq!(g.step(10, OLD, &[OLD, NEW], true), Decision::Watch);
        assert_eq!(g.step(12, OLD, &[NEW], true), Decision::Wait);
        assert_eq!(g.step(16, OLD, &[NEW], true), Decision::Wait);
    }
}
