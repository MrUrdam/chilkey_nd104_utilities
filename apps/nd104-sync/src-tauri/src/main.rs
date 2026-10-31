//! nd104-sync – tray app with settings window.
//!
//! Runs the core engine in the background, shows its state in the system
//! tray (M2) and offers a settings/status window (M3). The config file can
//! still be edited by hand; it is reloaded automatically when it changes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
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
use tauri::{AppHandle, Emitter, Manager, RunEvent, Theme, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{mpsc, watch};

use crate::i18n::I18n;
use crate::tray::{TrayItems, TrayState, TRAY_ID};

pub(crate) const PROJECT_URL: &str =
    "https://github.com/MrUrdam/chilkey_nd104_utilities/tree/main/apps/nd104-sync";
pub(crate) const CHANGELOG_URL: &str =
    "https://github.com/MrUrdam/chilkey_nd104_utilities/blob/main/apps/nd104-sync/CHANGELOG.md";
const CONFIG_POLL: Duration = Duration::from_secs(3);
const SETTINGS_WINDOW: &str = "settings";

pub(crate) struct AppState {
    pub(crate) config_path: PathBuf,
    pub(crate) config: Mutex<Config>,
    pub(crate) i18n: Mutex<I18n>,
    pub(crate) commands: mpsc::Sender<EngineCommand>,
    pub(crate) status: watch::Receiver<Status>,
    items: Mutex<Option<TrayItems>>,
    /// Show the result of the next app update check (manual check).
    pub(crate) manual_update_check: AtomicBool,
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
    let first_run = !config_path.exists();
    let (config, config_error) = match Config::load(&config_path) {
        Ok(c) => (c, None),
        Err(e) => {
            tracing::error!("{e:#}");
            (Config::default(), Some(format!("{e:#}")))
        }
    };
    if first_run {
        if let Err(e) = config.save(&config_path) {
            tracing::warn!("cannot create config file: {e:#}");
        }
    }
    let show_settings = first_run || std::env::args().any(|a| a == "--settings");

    tauri::Builder::default()
        // must be registered first: a second start opens the window of the first
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            open_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::texts,
            commands::get_status,
            commands::get_config,
            commands::save_config,
            commands::search_places,
            commands::test_weather,
            commands::api_key_status,
            commands::set_api_key,
            commands::delete_api_key,
            commands::test_ntp,
            commands::preview_time,
            commands::timezones,
            commands::engine_command,
            commands::open_target,
        ])
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
            tauri::async_runtime::spawn(watch_config(handle.clone()));
            if show_settings {
                open_settings(&handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("cannot start nd104-sync")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                // Closing the window keeps the tray app running; only "Quit" ends it.
                if code.is_none() {
                    api.prevent_exit();
                } else {
                    let commands = app.state::<AppState>().commands.clone();
                    let _ = commands.try_send(EngineCommand::Shutdown);
                }
            }
        });
}

fn window_theme(config: &Config) -> Option<Theme> {
    match config.general.theme.as_str() {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None, // follow Windows
    }
}

/// Shows the settings window, creating it if needed.
pub(crate) fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let theme = window_theme(&app.state::<AppState>().config.lock().unwrap());
    let result =
        WebviewWindowBuilder::new(app, SETTINGS_WINDOW, WebviewUrl::App("index.html".into()))
            .title(format!("nd104-sync {APP_VERSION}"))
            .inner_size(1000.0, 680.0)
            .min_inner_size(820.0, 560.0)
            .theme(theme)
            .center()
            .build();
    if let Err(e) = result {
        tracing::error!("cannot open settings window: {e}");
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notification failed: {e}");
    }
}

/// Opens a folder in the file manager or a file with its associated program.
/// On Windows Explorer is called directly: the opener plugin's shell "open"
/// reported existing paths as "not available".
pub(crate) fn open_in_shell(app: &AppHandle, path: &Path) {
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
        "settings" => open_settings(app),
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
            let mut config = state.config.lock().unwrap().clone();
            config.general.autostart = !config.general.autostart;
            if let Err(e) = apply_config(app, config) {
                tracing::warn!("{e:#}");
            }
        }
        "quit" => {
            send(app, EngineCommand::Shutdown);
            app.exit(0);
        }
        id => {
            if let Some(language) = id.strip_prefix("lang:") {
                let mut config = state.config.lock().unwrap().clone();
                config.general.language = language.to_string();
                if let Err(e) = apply_config(app, config) {
                    tracing::warn!("{e:#}");
                }
            }
        }
    }
}

/// Saves a changed configuration (tray or window) and applies it everywhere.
pub(crate) fn apply_config(app: &AppHandle, config: Config) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    config.save(&state.config_path)?;
    *state.own_write.lock().unwrap() = modified(&state.config_path);
    activate_config(app, config);
    Ok(())
}

/// Applies a configuration that is already on disk.
fn activate_config(app: &AppHandle, config: Config) {
    let state = app.state::<AppState>();
    let previous = std::mem::replace(&mut *state.config.lock().unwrap(), config.clone());
    send(app, EngineCommand::Reload(Box::new(config.clone())));
    if previous.general.autostart != config.general.autostart {
        apply_autostart(app, config.general.autostart);
    }
    if previous.general.language != config.general.language {
        *state.i18n.lock().unwrap() = I18n::new(&config.general.language);
    }
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = window.set_theme(window_theme(&config));
    }
    rebuild_menu(app);
    let _ = app.emit("config-changed", ());
}

// Locking rule: never hold a mutex while calling tray/menu/window functions.
// They run on the main thread, which may itself be waiting for that mutex
// (deadlock on Apply in 0.3.0 development builds). Copy what is needed first.

fn rebuild_menu(app: &AppHandle) {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    let status = state.status.borrow().clone();
    let i18n = state.i18n.lock().unwrap().clone();
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

/// Mirrors engine status into the tray and window, raises notifications.
async fn watch_status(app: AppHandle, mut status: watch::Receiver<Status>) {
    let mut previous = status.borrow().clone();
    let mut compatible_notified = false;
    let mut update_notified: Option<String> = None;

    while status.changed().await.is_ok() {
        let current = status.borrow().clone();
        let _ = app.emit("status", &current);
        let state = app.state::<AppState>();
        let i18n = state.i18n.lock().unwrap().clone();

        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            if TrayState::of(&current) != TrayState::of(&previous) {
                let _ = tray.set_icon(Some(TrayState::of(&current).icon()));
            }
            let _ = tray.set_tooltip(Some(tray::tooltip(&i18n, &current)));
        }
        let items = state.items.lock().unwrap().clone();
        if let Some(items) = items {
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

/// Reloads the config file when the user edits it by hand.
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
                activate_config(&app, config);
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
