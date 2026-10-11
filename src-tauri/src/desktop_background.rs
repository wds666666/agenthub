use agenthub_core::{
    background::{self, BackgroundSettings, BackgroundStatus},
    change_sync::LibraryChangeMonitor,
    manager, transaction,
};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize, Default)]
pub struct DesktopStatus {
    #[serde(flatten)]
    pub delivery: BackgroundStatus,
    pub tray_available: bool,
    pub watcher_available: bool,
    pub paused: bool,
}
#[derive(Clone)]
pub struct Runtime {
    pub settings: Arc<Mutex<BackgroundSettings>>,
    pub status: Arc<Mutex<DesktopStatus>>,
    sender: mpsc::Sender<Request>,
    stop: Arc<AtomicBool>,
}
enum Request {
    Refresh,
    Library,
    WatchError,
    Settings,
}
impl Runtime {
    pub fn refresh(&self) -> Result<(), String> {
        self.sender.send(Request::Refresh).map_err(super::err)
    }
    pub fn changed(&self) {
        let _ = self.sender.send(Request::Settings);
    }
}
pub fn maintain_manager(h: &agenthub_core::AgentHub) {
    if let Err(error) = manager::ensure(&h.paths, &h.store) {
        let error = super::err(format!("{error:#}"));
        let _ = h.store.set_meta("manager_error", &error);
        log::warn!("manager bundle: {error}");
    }
}
fn reveal(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn quit(app: tauri::AppHandle) {
    app.state::<Runtime>().stop.store(true, Ordering::Release);
    std::thread::spawn(move || {
        // Finish a running transaction before exiting.
        if let Ok(_h) = super::hub() {
            app.exit(0);
        }
    });
}
#[cfg(target_os = "linux")]
fn tray_host_available() -> bool {
    // A built indicator without a desktop watcher is not a usable escape route.
    std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.NameHasOwner",
            "org.kde.StatusNotifierWatcher",
        ])
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("true"))
}
#[cfg(not(target_os = "linux"))]
fn tray_host_available() -> bool {
    true
}
fn usable_tray(app: &tauri::AppHandle) -> bool {
    app.tray_by_id("agenthub").is_some() && tray_host_available()
}
fn build_tray(app: &tauri::AppHandle) -> anyhow::Result<bool> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    if !tray_host_available() {
        return Ok(false);
    }
    let labels: serde_json::Value = serde_json::from_str(include_str!("tray.i18n.json"))?;
    let language = super::hub()
        .map_err(anyhow::Error::msg)?
        .store
        .meta("ui_locale")?
        .unwrap_or_else(|| "zh-CN".into());
    let labels = labels.get(&language).unwrap_or(&labels["zh-CN"]);
    let text = |id: &str| labels[id].as_str().unwrap_or(id).to_string();
    let open = MenuItem::with_id(app, "open", text("open"), true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", text("refresh"), true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", text("pause"), true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", text("exit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &refresh, &pause, &exit])?;
    let mut builder = TrayIconBuilder::with_id("agenthub")
        .menu(&menu)
        .tooltip("AgentHub")
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => reveal(app),
            "refresh" => {
                let _ = app.state::<Runtime>().refresh();
            }
            "pause" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    if let Ok(h) = super::hub() {
                        if let Ok(mut value) = background::settings(&h.store) {
                            value.paused = !value.paused;
                            if background::save_settings(&h.store, &value).is_ok() {
                                *app.state::<Runtime>().settings.lock().unwrap() = value;
                                app.state::<Runtime>().changed();
                                let _ = app.emit("background-updated", ());
                            }
                        }
                    }
                });
            }
            "exit" => quit(app.clone()),
            _ => {}
        })
        .build(app)?;
    Ok(true)
}
fn delivery_due(status: &BackgroundStatus, paused: bool) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    !paused && status.due_timestamp.is_some_and(|due| due <= now)
}
pub fn setup(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let (sender, receiver) = mpsc::channel();
    let (prefs, initial) = {
        let h = super::hub().map_err(anyhow::Error::msg)?;
        (
            background::settings(&h.store)?,
            background::status(&h.store)?,
        )
    };
    let runtime = Runtime {
        settings: Arc::new(Mutex::new(prefs)),
        status: Arc::new(Mutex::new(DesktopStatus {
            delivery: initial,
            ..Default::default()
        })),
        sender,
        stop: Arc::new(AtomicBool::new(false)),
    };
    app.manage(runtime.clone());
    let tray = build_tray(app).unwrap_or_else(|e| {
        log::warn!("tray unavailable: {}", super::err(e));
        false
    });
    runtime.status.lock().unwrap().tray_available = tray;
    let handle = app.clone();
    std::thread::Builder::new()
        .name("background-coordinator".into())
        .spawn(move || {
            let mut monitor = LibraryChangeMonitor::default();
            let roots = if let Ok(h) = super::hub() {
                maintain_manager(&h);
                let _ = monitor.check(&h.paths, &h.store);
                vec![
                    h.paths.skills.clone(),
                    h.paths.mcp.clone(),
                    h.paths.rules.clone(),
                    h.paths.plugins.clone(),
                ]
            } else {
                vec![]
            };
            let tx = runtime.sender.clone();
            let skill_root = roots.first().cloned();
            let watched_roots = roots.clone();
            let mut watcher =
                notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                    let request = match event {
                        Ok(event)
                            if matches!(event.kind, notify::EventKind::Remove(_))
                                && event.paths.iter().any(|p| watched_roots.contains(p)) =>
                        {
                            Some(Request::WatchError)
                        }
                        Ok(event)
                            if !matches!(event.kind, notify::EventKind::Access(_))
                                && event.paths.iter().any(|p| {
                                    let Some(relative) = skill_root
                                        .as_ref()
                                        .and_then(|root| p.strip_prefix(root).ok())
                                    else {
                                        return true;
                                    };
                                    !relative.components().any(|part| {
                                        matches!(
                                            part.as_os_str().to_str(),
                                            Some(
                                                ".venv"
                                                    | "venv"
                                                    | ".git"
                                                    | "node_modules"
                                                    | "__pycache__"
                                                    | ".pytest_cache"
                                                    | ".mypy_cache"
                                                    | ".ruff_cache"
                                            )
                                        )
                                    }) && !relative
                                        .extension()
                                        .is_some_and(|ext| ext == "pyc" || ext == "pyo")
                                }) =>
                        {
                            Some(Request::Library)
                        }
                        Err(_) => Some(Request::WatchError),
                        _ => None,
                    };
                    if let Some(request) = request {
                        let _ = tx.send(request);
                    }
                })
                .ok();
            if let Some(w) = watcher.as_mut() {
                for root in roots {
                    if let Err(e) = w.watch(&root, RecursiveMode::Recursive) {
                        log::warn!("watch fallback: {}", super::err(e));
                        watcher = None;
                        break;
                    }
                }
            }
            runtime.status.lock().unwrap().watcher_available = watcher.is_some();
            let mut observe = 2u8;
            let mut manual = false;
            let mut last_observe = Instant::now();
            let mut last_tray_check = Instant::now();
            let mut last_check = Instant::now() - Duration::from_secs(1800);
            while !runtime.stop.load(Ordering::Acquire) {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(Request::Refresh) => manual = true,
                    Ok(Request::Library) => observe = 2,
                    Ok(Request::WatchError) => {
                        watcher = None;
                        observe = 2;
                        runtime.status.lock().unwrap().watcher_available = false;
                    }
                    Ok(Request::Settings) => {
                        last_check = Instant::now() - Duration::from_secs(1800);
                        observe = 2;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    _ => {}
                }
                // Drain bursts once; editor events never trigger one transaction per event.
                while let Ok(request) = receiver.try_recv() {
                    match request {
                        Request::Refresh => manual = true,
                        Request::Library => observe = 2,
                        Request::WatchError => {
                            watcher = None;
                            observe = 2;
                            runtime.status.lock().unwrap().watcher_available = false;
                        }
                        Request::Settings => observe = 2,
                    }
                }
                if last_tray_check.elapsed() >= Duration::from_secs(10) {
                    last_tray_check = Instant::now();
                    let was_available = runtime.status.lock().unwrap().tray_available;
                    if was_available && !usable_tray(&handle) {
                        runtime.status.lock().unwrap().tray_available = false;
                        reveal(&handle);
                        let _ = handle.emit("close-requested", ());
                    }
                }
                let prefs = runtime.settings.lock().unwrap().clone();
                let check =
                    manual || last_check.elapsed() >= Duration::from_secs(prefs.check_seconds);
                let metadata = observe > 0 && last_observe.elapsed() >= Duration::from_secs(2)
                    || last_observe.elapsed()
                        >= Duration::from_secs(if watcher.is_some() { 120 } else { 5 });
                if !manual
                    && !check
                    && !metadata
                    && !delivery_due(&runtime.status.lock().unwrap().delivery, prefs.paused)
                {
                    continue;
                }
                runtime.status.lock().unwrap().delivery.running = true;
                if let Ok(h) = super::hub() {
                    let result = (|| -> anyhow::Result<()> {
                        if check && h.store.initialized()? {
                            let report = super::SKILL_CHANGES
                                .lock()
                                .map_err(|e| anyhow::anyhow!(e.to_string()))?
                                .check(&h.paths, true)?;
                            let _ = handle.emit("skill-changes", &report);
                            last_check = Instant::now();
                        }
                        if manual {
                            let _guard = agenthub_core::versions::operation_lock(&h.paths)?;
                            let outcomes = transaction::run_auto_sync_pending(&h.paths, &h.store)?;
                            background::record(&h.store, &outcomes)?;
                        }
                        if metadata {
                            monitor.check(&h.paths, &h.store)?;
                            last_observe = Instant::now();
                            observe = if monitor.has_pending_observation() {
                                1
                            } else {
                                observe.saturating_sub(1)
                            };
                        }
                        background::run_due(&h.paths, &h.store)?;
                        Ok(())
                    })();
                    if let Err(error) = result {
                        let _ = h
                            .store
                            .set_meta("background_error", &super::err(format!("{error:#}")));
                    }
                    if let Ok(status) = background::status(&h.store) {
                        runtime.status.lock().unwrap().delivery = status;
                    }
                }
                runtime.status.lock().unwrap().delivery.running = false;
                let _ = handle.emit("background-updated", ());
                manual = false;
            }
        })?;
    Ok(())
}
#[tauri::command]
pub fn background_settings(app: tauri::AppHandle) -> BackgroundSettings {
    app.state::<Runtime>().settings.lock().unwrap().clone()
}
#[tauri::command(async)]
pub fn set_background_settings(
    app: tauri::AppHandle,
    settings: BackgroundSettings,
) -> Result<BackgroundSettings, String> {
    let h = super::hub()?;
    background::save_settings(&h.store, &settings).map_err(super::err)?;
    *app.state::<Runtime>().settings.lock().unwrap() = settings.clone();
    app.state::<Runtime>().changed();
    Ok(settings)
}
#[tauri::command]
pub fn background_status(app: tauri::AppHandle) -> DesktopStatus {
    let mut status = app.state::<Runtime>().status.lock().unwrap().clone();
    status.paused = app.state::<Runtime>().settings.lock().unwrap().paused;
    status
}
#[tauri::command]
pub fn background_refresh(app: tauri::AppHandle) -> Result<(), String> {
    app.state::<Runtime>().refresh()
}
#[tauri::command(async)]
pub fn close_choice(app: tauri::AppHandle, behavior: String, remember: bool) -> Result<(), String> {
    if !["exit", "tray"].contains(&behavior.as_str()) {
        return Err("invalid close choice".into());
    }
    if behavior == "tray" && !usable_tray(&app) {
        app.state::<Runtime>().status.lock().unwrap().tray_available = false;
        reveal(&app);
        return Err("system tray is unavailable; window remains open".into());
    }
    if remember {
        let mut prefs = background_settings(app.clone());
        prefs.close_behavior = behavior.clone();
        set_background_settings(app.clone(), prefs)?;
    }
    if behavior == "tray" {
        if let Some(w) = app.get_webview_window("main") {
            w.hide().map_err(super::err)?;
        }
    } else {
        quit(app);
    }
    Ok(())
}
#[derive(Debug, PartialEq, Eq)]
enum CloseAction {
    Quit,
    Hide,
    Prompt,
}
fn close_action(behavior: &str, tray_available: bool) -> CloseAction {
    match (behavior, tray_available) {
        ("exit", _) => CloseAction::Quit,
        ("tray", true) => CloseAction::Hide,
        _ => CloseAction::Prompt,
    }
}

pub fn window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let app = window.app_handle();
        let prefs = background_settings(app.clone());
        let tray = app.state::<Runtime>().status.lock().unwrap().tray_available;
        match close_action(&prefs.close_behavior, tray) {
            CloseAction::Quit => quit(app.clone()),
            CloseAction::Hide => {
                let app = app.clone();
                std::thread::spawn(move || {
                    if usable_tray(&app) {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    } else {
                        app.state::<Runtime>().status.lock().unwrap().tray_available = false;
                        let _ = app.emit("close-requested", ());
                        reveal(&app);
                    }
                });
            }
            CloseAction::Prompt => {
                let _ = app.emit("close-requested", ());
                reveal(app);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_never_hides_without_a_usable_tray() {
        assert_eq!(close_action("ask", true), CloseAction::Prompt);
        assert_eq!(close_action("exit", false), CloseAction::Quit);
        assert_eq!(close_action("tray", false), CloseAction::Prompt);
        assert_eq!(close_action("tray", true), CloseAction::Hide);
    }
}
