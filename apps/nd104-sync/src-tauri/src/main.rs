//! nd104-sync – tray app (milestone M2).
//!
//! Runs the core engine in the background and shows its state in the system
//! tray. Settings are edited in the config file until the settings window
//! arrives with M3; the file is reloaded automatically when it changes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod i18n;
mod logging;
mod tray;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use nd104_core::app_update::UpdateStatus;
use nd104_core::config::{self, Config};
use nd104_core::device::{DeviceHandle, DeviceState, Trust};
use nd104_core::engine::{self, EngineCommand, Status};
use nd104_core::hid::{HidBackend, HidapiBackend};
use nd104_core::{model, APP_VERSION};
use tauri::menu::MenuEvent;
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{mpsc, watch};

use crate::i18n::I18n;
use crate::tray::{TrayItems, TrayState, TRAY_ID};

const PROJECT_URL: &str =
    "https://github.com/MrUrdam/chilkey_nd104_utilities/tree/main/apps/nd104-sync";
const CONFIG_POLL: Duration = Duration::from_secs(3);

struct AppState {
    config_path: PathBuf,
    config: Mutex<Config>,
    i18n: Mutex<I18n>,
    commands: mpsc::Sender<EngineCommand>,
    status: watch::Receiver<Status>,
    items: Mutex<Option<TrayItems>>,
    /// Show the result of the next app update check (manual check).
    manual_update_check: AtomicBool,
    /// Modification time of our own last config write (not a user edit).
    own_write: Mutex<Option<SystemTime>>,
}

fn main() {
    let _log_guard = logging::init();
    tracing::info!("nd104-sync {APP_VERSION} starting");

    let config_path = match config::default_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("no config directory: {e:#}");
            return;
        }
    };
    let (config, config_error) = match Config::load(&config_path) {
        Ok(c) => (c, None),
        Err(e) => {
            tracing::error!("{e:#}");
            (Config::default(), Some(format!("{e:#}")))
        }
    };
    if !config_path.exists() {
        if let Err(e) = config.save(&config_path) {
            tracing::warn!("cannot create config file: {e:#}");
        }
    }

    tauri::Builder::default()
        // must be registered first
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let text = app
                .state::<AppState>()
                .i18n
                .lock()
                .unwrap()
                .t("app.already_running");
            notify(app, "nd104-sync", &text);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            let device = DeviceHandle::spawn(
                || Ok(Box::new(HidapiBackend::new()?) as Box<dyn HidBackend>),
                model::registry(),
            );
            let engine =
                tauri::async_runtime::block_on(async { engine::start(config.clone(), device) });
            let i18n = I18n::new(&config.general.language);

            apply_autostart(&handle, config.general.autostart);
            let status = engine.status.borrow().clone();
            let (menu, items) = tray::build_menu(
                &handle,
                &i18n,
                &config.general.language,
                config.general.autostart,
                &status,
            )?;
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(TrayState::of(&status).icon())
                .tooltip(tray::tooltip(&i18n, &status))
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(on_menu_event)
                .build(app)?;

            if let Some(error) = &config_error {
                let title = i18n.t("notify.config_error_title");
                let body = i18n.tf("notify.config_error_body", &[("error", error)]);
                notify(&handle, &title, &body);
            }

            app.manage(AppState {
                own_write: Mutex::new(modified(&config_path)),
                config_path,
                config: Mutex::new(config.clone()),
                i18n: Mutex::new(i18n),
                commands: engine.commands.clone(),
                status: engine.status.clone(),
                items: Mutex::new(Some(items)),
                manual_update_check: AtomicBool::new(false),
            });

            tauri::async_runtime::spawn(watch_status(handle.clone(), engine.status.clone()));
            tauri::async_runtime::spawn(watch_config(handle));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("cannot start nd104-sync")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                // No windows: only an explicit "Quit" ends the app.
                if code.is_none() {
                    api.prevent_exit();
                } else {
                    let commands = app.state::<AppState>().commands.clone();
                    let _ = commands.try_send(EngineCommand::Shutdown);
                }
            }
        });
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notification failed: {e}");
    }
}

/// Opens a folder in the file manager or a file with its associated program.
/// On Windows Explorer is called directly: the opener plugin's shell "open"
/// reported existing paths as "not available".
fn open_in_shell(app: &AppHandle, path: &Path) {
    #[cfg(windows)]
    {
        let _ = app;
        if let Err(e) = std::process::Command::new("explorer.exe").arg(path).spawn() {
            tracing::warn!("cannot open {}: {e}", path.display());
        }
    }
    #[cfg(not(windows))]
    if let Err(e) = app.opener().open_path(path.to_string_lossy(), None::<&str>) {
        tracing::warn!("cannot open {}: {e}", path.display());
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Enabling always rewrites the entry, so it points to the exe that runs now
/// (e.g. after moving from a test build to the installed app).
fn apply_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    let result = match (enabled, launcher.is_enabled().unwrap_or(false)) {
        (true, _) => launcher.enable(),
        (false, true) => launcher.disable(),
        (false, false) => Ok(()),
    };
    if let Err(e) = result {
        tracing::warn!("autostart: {e}");
    }
}

fn send(app: &AppHandle, command: EngineCommand) {
    if let Err(e) = app.state::<AppState>().commands.try_send(command) {
        tracing::warn!("engine busy or stopped: {e}");
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let state = app.state::<AppState>();
    match event.id.as_ref() {
        "sync_now" => send(app, EngineCommand::SyncNow),
        "pause" => {
            let paused = state.status.borrow().paused;
            send(
                app,
                if paused {
                    EngineCommand::Resume
                } else {
                    EngineCommand::Pause
                },
            );
        }
        "recheck" => send(app, EngineCommand::Recheck),
        "check_updates" => {
            state.manual_update_check.store(true, Ordering::SeqCst);
            send(app, EngineCommand::CheckAppUpdate);
        }
        "settings" => {
            if !state.config_path.exists() {
                update_config(app, |_| {});
            }
            open_in_shell(app, &state.config_path);
        }
        "open_logs" => {
            if let Ok(dir) = config::log_dir() {
                let _ = std::fs::create_dir_all(&dir);
                open_in_shell(app, &dir);
            }
        }
        "about" => {
            let _ = app.opener().open_url(PROJECT_URL, None::<&str>);
        }
        "autostart" => {
            let enabled = !state.config.lock().unwrap().general.autostart;
            apply_autostart(app, enabled);
            update_config(app, |c| c.general.autostart = enabled);
            rebuild_menu(app);
        }
        "quit" => {
            send(app, EngineCommand::Shutdown);
            app.exit(0);
        }
        id => {
            if let Some(language) = id.strip_prefix("lang:") {
                let language = language.to_string();
                update_config(app, |c| c.general.language = language.clone());
                *state.i18n.lock().unwrap() = I18n::new(&language);
                rebuild_menu(app);
            }
        }
    }
}

/// Changes and saves the config from the tray (not treated as a user edit).
fn update_config(app: &AppHandle, change: impl FnOnce(&mut Config)) {
    let state = app.state::<AppState>();
    let mut config = state.config.lock().unwrap();
    change(&mut config);
    match config.save(&state.config_path) {
        Ok(()) => *state.own_write.lock().unwrap() = modified(&state.config_path),
        Err(e) => tracing::warn!("cannot save config: {e:#}"),
    }
}

fn rebuild_menu(app: &AppHandle) {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    let status = state.status.borrow().clone();
    let i18n = state.i18n.lock().unwrap();
    match tray::build_menu(
        app,
        &i18n,
        &config.general.language,
        config.general.autostart,
        &status,
    ) {
        Ok((menu, items)) => {
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                let _ = tray.set_menu(Some(menu));
                let _ = tray.set_tooltip(Some(tray::tooltip(&i18n, &status)));
            }
            *state.items.lock().unwrap() = Some(items);
        }
        Err(e) => tracing::warn!("cannot build menu: {e}"),
    }
}

/// Mirrors engine status into the tray and raises notifications.
async fn watch_status(app: AppHandle, mut status: watch::Receiver<Status>) {
    let mut previous = status.borrow().clone();
    let mut compatible_notified = false;
    let mut update_notified: Option<String> = None;

    while status.changed().await.is_ok() {
        let current = status.borrow().clone();
        let state = app.state::<AppState>();
        let i18n = state.i18n.lock().unwrap();

        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            if TrayState::of(&current) != TrayState::of(&previous) {
                let _ = tray.set_icon(Some(TrayState::of(&current).icon()));
            }
            let _ = tray.set_tooltip(Some(tray::tooltip(&i18n, &current)));
        }
        if let Some(items) = state.items.lock().unwrap().as_ref() {
            let _ = items.status.set_text(tray::status_line(&i18n, &current));
            let _ = items.pause.set_text(i18n.t(if current.paused {
                "menu.resume"
            } else {
                "menu.pause"
            }));
        }

        // Notifications on transitions only.
        match (&previous.device, &current.device) {
            (DeviceState::Locked { .. }, DeviceState::Locked { .. }) => {}
            (_, DeviceState::Locked { reason, .. }) => notify(
                &app,
                &i18n.t("notify.locked_title"),
                &i18n.tf("notify.locked_body", &[("reason", reason)]),
            ),
            (_, DeviceState::Ready(d)) if d.trust == Trust::Compatible && !compatible_notified => {
                compatible_notified = true;
                let firmware: Vec<String> = d.firmware.iter().map(|f| f.raw.clone()).collect();
                notify(
                    &app,
                    &i18n.t("notify.compatible_title"),
                    &i18n.tf(
                        "notify.compatible_body",
                        &[("model", d.model_name), ("firmware", &firmware.join(", "))],
                    ),
                );
            }
            _ => {}
        }

        if current.app_update_checked != previous.app_update_checked {
            let manual = state.manual_update_check.swap(false, Ordering::SeqCst);
            match &current.app_update {
                Some(Ok(UpdateStatus::Available(r)))
                    if manual || update_notified.as_deref() != Some(&r.version) =>
                {
                    update_notified = Some(r.version.clone());
                    notify(
                        &app,
                        &i18n.t("notify.update_title"),
                        &i18n.tf(
                            "notify.update_body",
                            &[("version", &r.version), ("url", &r.url)],
                        ),
                    );
                }
                Some(Ok(UpdateStatus::UpToDate { .. })) if manual => notify(
                    &app,
                    &i18n.t("notify.up_to_date_title"),
                    &i18n.tf("notify.up_to_date_body", &[("version", APP_VERSION)]),
                ),
                Some(Err(error)) if manual => notify(
                    &app,
                    &i18n.t("notify.update_failed_title"),
                    &i18n.tf("notify.update_failed_body", &[("error", error)]),
                ),
                _ => {}
            }
        }

        drop(i18n);
        previous = current;
    }
}

/// Reloads the config file when the user edits it.
async fn watch_config(app: AppHandle) {
    let mut last_seen = modified(&app.state::<AppState>().config_path);
    loop {
        tokio::time::sleep(CONFIG_POLL).await;
        let state = app.state::<AppState>();
        let now = modified(&state.config_path);
        if now == last_seen {
            continue;
        }
        last_seen = now;
        if now == *state.own_write.lock().unwrap() {
            continue;
        }
        match Config::load(&state.config_path) {
            Ok(config) => {
                tracing::info!("config file changed, reloading");
                let language_changed =
                    config.general.language != state.config.lock().unwrap().general.language;
                let autostart = config.general.autostart;
                *state.config.lock().unwrap() = config.clone();
                send(&app, EngineCommand::Reload(Box::new(config.clone())));
                apply_autostart(&app, autostart);
                if language_changed {
                    *state.i18n.lock().unwrap() = I18n::new(&config.general.language);
                }
                rebuild_menu(&app);
            }
            Err(e) => {
                let i18n = state.i18n.lock().unwrap();
                let (title, body) = (
                    i18n.t("notify.config_error_title"),
                    i18n.tf("notify.config_error_body", &[("error", &format!("{e:#}"))]),
                );
                drop(i18n);
                notify(&app, &title, &body);
            }
        }
    }
}
