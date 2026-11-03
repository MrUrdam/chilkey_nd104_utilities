//! ND104 Studio – tray app with settings window.
//!
//! Runs the core engine in the background, shows its state in the system
//! tray (M2) and offers a settings/status window (M3). The config file can
//! still be edited by hand; it is reloaded automatically when it changes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bluetooth;
mod commands;
mod hardware;
mod i18n;
mod keymap_backup;
mod keymap_page;
mod logging;
mod openrgb;
mod self_update;
mod signalrgb;
mod tray;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use nd104_core::app_update::UpdateStatus;
use nd104_core::config::{self, Config};
use nd104_core::device::{DeviceHandle, DeviceState, Trust};
use nd104_core::engine::{self, EngineCommand, Status};
use nd104_core::firmware_update::PartState;
use nd104_core::hid::{HidBackend, HidapiBackend};
use nd104_core::{model, APP_VERSION};
use tauri::menu::MenuEvent;
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, RunEvent, Theme, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;
#[cfg(not(windows))]
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{mpsc, watch};

use crate::i18n::I18n;
use crate::tray::{TrayItems, TrayState, TRAY_ID};

pub(crate) const PROJECT_URL: &str =
    "https://github.com/MrUrdam/chilkey_nd104_utilities/tree/main/apps/nd104-studio";
pub(crate) const CHANGELOG_URL: &str =
    "https://github.com/MrUrdam/chilkey_nd104_utilities/blob/main/apps/nd104-studio/CHANGELOG.md";
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
    /// Show the result of the next firmware update check (manual check).
    pub(crate) manual_firmware_check: AtomicBool,
    /// Modification time of our own last config write (not a user edit).
    own_write: Mutex<Option<SystemTime>>,
    /// Picture picked on the Screen picture page.
    pub(crate) picture: Mutex<Option<hardware::PendingPicture>>,
    /// Keymap backup chosen for restoring (checked, not written yet).
    pub(crate) keymap_restore: Mutex<Option<keymap_backup::PendingRestore>>,
}

/// Linux + NVIDIA driver: WebKitGTK may show a blank window under Wayland;
/// the known workaround is to switch off its DMA-BUF renderer. Set before
/// any thread starts, and only if the user did not choose otherwise.
#[cfg(target_os = "linux")]
fn webkit_workarounds() {
    if std::path::Path::new("/proc/driver/nvidia").exists()
        && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    webkit_workarounds();
    // before anything reads or writes data: take over nd104-sync's folders and keys
    let migrated = nd104_core::rebrand::migrate();
    let _log_guard = logging::init();
    tracing::info!("ND104 Studio {APP_VERSION} starting");
    for line in &migrated {
        tracing::info!("taken over from nd104-sync – {line}");
    }

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
    // `--quit` (sent by the setup before an update) ends a running instance;
    // without one there is nothing to do.
    let quit_request = std::env::args().any(|a| a == "--quit");

    tauri::Builder::default()
        // must be registered first: a second start opens the window of the first
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|a| a == "--quit") {
                quit(app);
            } else {
                open_settings(app);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::third_party_notices,
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
            commands::install_update,
            hardware::light_read,
            hardware::light_apply,
            bluetooth::bluetooth_status,
            signalrgb::signalrgb_status,
            openrgb::openrgb_status,
            signalrgb::signalrgb_install,
            signalrgb::signalrgb_remove,
            hardware::keyboard_settings_read,
            hardware::keyboard_settings_write,
            hardware::picture_pick,
            hardware::picture_refit,
            hardware::picture_upload,
            keymap_backup::keymap_backup,
            keymap_backup::keymap_restore_check,
            keymap_backup::keymap_restore_apply,
            keymap_backup::keymap_restore_cancel,
            keymap_page::keymap_read,
            keymap_page::keymap_write,
            keymap_page::keymap_page_get,
            keymap_page::keymap_page_set,
            keymap_page::keymap_profiles,
            keymap_page::keymap_profile_save,
            keymap_page::keymap_profile_get,
            keymap_page::keymap_profile_delete,
            keymap_page::keymap_profile_export,
            keymap_page::keymap_profile_import,
        ])
        .setup(move |app| {
            if quit_request {
                std::process::exit(0);
            }
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
            let (menu, items) =
                tray::build_menu(&handle, &i18n, config.general.autostart, &status)?;
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
                picture: Mutex::new(None),
                keymap_restore: Mutex::new(None),
                config_path,
                config: Mutex::new(config.clone()),
                i18n: Mutex::new(i18n),
                commands: engine.commands.clone(),
                status: engine.status.clone(),
                items: Mutex::new(Some(items)),
                manual_update_check: AtomicBool::new(false),
                manual_firmware_check: AtomicBool::new(false),
            });

            app.manage(openrgb::OpenRgbServer::default());
            openrgb::apply(
                &handle,
                config.device.openrgb_server,
                config.device.openrgb_port,
            );

            tauri::async_runtime::spawn(watch_status(handle.clone(), engine.status.clone()));
            tauri::async_runtime::spawn(watch_config(handle.clone()));
            if show_settings {
                open_settings(&handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("cannot start ND104 Studio")
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

/// Stops the engine and ends the app.
pub(crate) fn quit(app: &AppHandle) {
    send(app, EngineCommand::Shutdown);
    app.exit(0);
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
            .title(format!("ND104 Studio {APP_VERSION}"))
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
        "settings" => open_settings(app),
        "autostart" => {
            let mut config = state.config.lock().unwrap().clone();
            config.general.autostart = !config.general.autostart;
            if let Err(e) = apply_config(app, config) {
                tracing::warn!("{e:#}");
            }
        }
        "quit" => {
            quit(app);
        }
        _ => {}
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
    openrgb::apply(
        app,
        config.device.openrgb_server,
        config.device.openrgb_port,
    );
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
    match tray::build_menu(app, &i18n, config.general.autostart, &status) {
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
    let mut firmware_notified: Option<String> = None;

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
            if let Some(detail) = &items.detail {
                let _ = detail.set_text(tray::detail_line(&i18n, &current));
            }
            let _ = items.pause.set_text(i18n.t(if current.paused {
                "menu.resume"
            } else {
                "menu.pause"
            }));
        }

        // Notifications on transitions only.
        match (&previous.device, &current.device) {
            (DeviceState::Locked { .. }, DeviceState::Locked { .. }) => {}
            (DeviceState::NoAccess { .. }, DeviceState::NoAccess { .. }) => {}
            (_, DeviceState::NoAccess { path, .. }) => notify(
                &app,
                &i18n.t("notify.no_access_title"),
                &i18n.tf("notify.no_access_body", &[("path", path)]),
            ),
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

        if current.firmware_update_checked != previous.firmware_update_checked {
            let manual = state.manual_firmware_check.swap(false, Ordering::SeqCst);
            match &current.firmware_update {
                Some(Ok(report)) if report.update_available() => {
                    let parts: Vec<String> = report
                        .parts
                        .iter()
                        .filter(|p| p.state == PartState::UpdateAvailable)
                        .map(|p| {
                            let label = p.label.clone().unwrap_or_else(|| p.role.to_string());
                            let date = p.latest.as_ref().map(|l| l.build_date.to_string());
                            format!("{label} ({})", date.unwrap_or_default())
                        })
                        .collect();
                    let key = parts.join(", ");
                    if manual || firmware_notified.as_deref() != Some(&key) {
                        notify(
                            &app,
                            &i18n.t("notify.firmware_title"),
                            &i18n.tf(
                                "notify.firmware_body",
                                &[("model", &report.model_name), ("parts", &key)],
                            ),
                        );
                        firmware_notified = Some(key);
                    }
                }
                Some(Ok(_)) if manual => notify(
                    &app,
                    &i18n.t("notify.firmware_ok_title"),
                    &i18n.t("notify.firmware_ok_body"),
                ),
                Some(Err(error)) if manual => notify(
                    &app,
                    &i18n.t("notify.firmware_failed_title"),
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
