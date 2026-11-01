//! Commands for the settings window (M3). Everything the UI shows or changes
//! goes through here; the window never touches files or devices itself.

use std::sync::atomic::Ordering;
use std::time::Duration;

use nd104_core::app_update;
use nd104_core::clock::{ntp_offset, ClockSettings};
use nd104_core::config::{self, Config};
use nd104_core::engine::{EngineCommand, Status};
use nd104_core::model::{self, VerifiedFirmware};
use nd104_core::weather::{self, geocoding, WeatherReport, WeatherSettings};
use nd104_core::APP_VERSION;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
pub struct ModelInfo {
    id: &'static str,
    name: &'static str,
    verified_firmware: Vec<VerifiedFirmware>,
}

#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    /// Language actually used (`de` / `en`), after resolving `auto`.
    language: &'static str,
    languages: Vec<(&'static str, &'static str)>,
    config_path: String,
    log_dir: String,
    project_url: &'static str,
    releases_url: &'static str,
    changelog_url: &'static str,
    providers: Vec<weather::ProviderInfo>,
    models: Vec<ModelInfo>,
    min_weather_interval: u32,
    /// "Install update" is possible (installed by nd104-sync-setup).
    self_update: bool,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    let language = crate::i18n::resolve(&state.config.lock().unwrap().general.language);
    AppInfo {
        version: APP_VERSION,
        language,
        languages: crate::i18n::LANGUAGES.to_vec(),
        config_path: state.config_path.display().to_string(),
        log_dir: config::log_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_default(),
        project_url: crate::PROJECT_URL,
        releases_url: app_update::RELEASES_PAGE,
        changelog_url: crate::CHANGELOG_URL,
        providers: weather::PROVIDERS.to_vec(),
        models: model::registry()
            .iter()
            .map(|m| ModelInfo {
                id: m.id(),
                name: m.display_name(),
                verified_firmware: m.verified_firmware().to_vec(),
            })
            .collect(),
        min_weather_interval: WeatherSettings::MIN_INTERVAL_MIN,
        self_update: crate::self_update::supported(),
    }
}

/// Downloads the newest release's setup, verifies it and starts it in update
/// mode; then quits so the setup can replace the files.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> CmdResult<()> {
    if !crate::self_update::supported() {
        return Err("this installation cannot update itself".into());
    }
    let release = match &app.state::<AppState>().status.borrow().app_update {
        Some(Ok(app_update::UpdateStatus::Available(release))) => release.clone(),
        _ => return Err("no update available".into()),
    };
    crate::self_update::download_and_start(&release)
        .await
        .map_err(|e| format!("{e:#}"))?;
    crate::quit(&app);
    Ok(())
}

/// Locale texts for the UI (same files as the tray).
#[tauri::command]
pub fn texts(state: State<'_, AppState>) -> std::collections::HashMap<String, String> {
    state.i18n.lock().unwrap().all()
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Status {
    state.status.borrow().clone()
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

/// Validates, saves and applies a configuration from the window.
#[tauri::command]
pub async fn save_config(app: AppHandle, config: Config) -> CmdResult<()> {
    let checked = Config::from_toml(&config.to_toml().map_err(err)?).map_err(err)?;
    if weather::create(&checked.weather.provider).is_none() {
        return Err(format!(
            "unknown weather provider {:?}",
            checked.weather.provider
        ));
    }
    if !checked.weather.fallback_provider.is_empty()
        && weather::create(&checked.weather.fallback_provider).is_none()
    {
        return Err(format!(
            "unknown fallback provider {:?}",
            checked.weather.fallback_provider
        ));
    }
    checked
        .clock
        .local_time(jiff::Timestamp::now())
        .map_err(|e| format!("{e:#}"))?;
    crate::apply_config(&app, checked).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn search_places(query: String, language: String) -> CmdResult<Vec<geocoding::Place>> {
    geocoding::search(&query, &language)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// Fetches weather with unsaved settings (nothing is sent to the keyboard).
/// `api_key` = a key typed but not saved yet; otherwise the stored key is used.
#[tauri::command]
pub async fn test_weather(
    settings: WeatherSettings,
    api_key: Option<String>,
) -> CmdResult<WeatherReport> {
    let provider = weather::create(&settings.provider)
        .ok_or_else(|| format!("unknown weather provider {:?}", settings.provider))?;
    let client = weather::new_http_client().map_err(err)?;
    let ctx = weather::context(provider.as_ref(), &settings, client, api_key)
        .map_err(|e| format!("{e:#}"))?;
    provider.fetch(&ctx).await.map_err(|e| format!("{e:#}"))
}

fn key_provider(provider: &str) -> CmdResult<()> {
    match weather::info(provider) {
        Some(p) if p.requires_api_key => Ok(()),
        _ => Err(format!("{provider:?} takes no API key")),
    }
}

/// Which key-based providers have a stored key. Keys never leave the backend.
#[tauri::command]
pub async fn api_key_status() -> std::collections::HashMap<String, bool> {
    weather::PROVIDERS
        .iter()
        .filter(|p| p.requires_api_key)
        .map(|p| (p.id.to_string(), nd104_core::secrets::has(p.id)))
        .collect()
}

#[tauri::command]
pub async fn set_api_key(app: AppHandle, provider: String, key: String) -> CmdResult<()> {
    key_provider(&provider)?;
    nd104_core::secrets::set(&provider, &key).map_err(|e| format!("{e:#}"))?;
    // Fetch again right away with the new key.
    let _ = app
        .state::<AppState>()
        .commands
        .try_send(EngineCommand::SyncNow);
    Ok(())
}

#[tauri::command]
pub async fn delete_api_key(provider: String) -> CmdResult<()> {
    key_provider(&provider)?;
    nd104_core::secrets::delete(&provider).map_err(|e| format!("{e:#}"))
}

/// Offset of the NTP server against the PC clock, in milliseconds.
#[tauri::command]
pub async fn test_ntp(server: String) -> CmdResult<i64> {
    tauri::async_runtime::spawn_blocking(move || ntp_offset(&server, Duration::from_secs(5)))
        .await
        .map_err(err)?
        .map(|d| d.as_millis() as i64)
        .map_err(|e| format!("{e:#}"))
}

#[derive(Serialize)]
pub struct TimePreview {
    /// Wall-clock time that would be sent, e.g. `2026-10-31T14:05:09`.
    local: String,
    next_dst_change: Option<String>,
}

#[tauri::command]
pub fn preview_time(clock: ClockSettings) -> CmdResult<TimePreview> {
    let now = jiff::Timestamp::now();
    let local = clock.local_time(now).map_err(|e| format!("{e:#}"))?;
    Ok(TimePreview {
        local: local.strftime("%Y-%m-%dT%H:%M:%S").to_string(),
        next_dst_change: clock.next_transition(now).map(|t| t.to_string()),
    })
}

/// All IANA time-zone names.
#[tauri::command]
pub fn timezones() -> Vec<String> {
    let mut zones: Vec<String> = jiff::tz::db().available().map(|n| n.to_string()).collect();
    zones.sort();
    zones
}

#[tauri::command]
pub async fn engine_command(app: AppHandle, command: String) -> CmdResult<()> {
    let state = app.state::<AppState>();
    let cmd = match command.as_str() {
        "sync_now" => EngineCommand::SyncNow,
        "pause" => EngineCommand::Pause,
        "resume" => EngineCommand::Resume,
        "recheck" => EngineCommand::Recheck,
        "check_updates" => {
            // One button for both: app release and keyboard firmware.
            state.manual_update_check.store(true, Ordering::SeqCst);
            state.manual_firmware_check.store(true, Ordering::SeqCst);
            state
                .commands
                .try_send(EngineCommand::CheckFirmwareUpdate)
                .map_err(err)?;
            EngineCommand::CheckAppUpdate
        }
        other => return Err(format!("unknown command {other:?}")),
    };
    state.commands.try_send(cmd).map_err(err)
}

/// Opens a known location. URLs are limited to this project on GitHub,
/// OpenStreetMap markers (location check), the providers' sign-up pages and
/// Chilkey's firmware pages/folders (firmware update hints).
#[tauri::command]
pub async fn open_target(app: AppHandle, target: String) -> CmdResult<()> {
    let state = app.state::<AppState>();
    match target.as_str() {
        "config" => crate::open_in_shell(&app, &state.config_path),
        "logs" => {
            let dir = config::log_dir().map_err(err)?;
            let _ = std::fs::create_dir_all(&dir);
            crate::open_in_shell(&app, &dir);
        }
        url if url.starts_with("https://github.com/MrUrdam/chilkey_nd104_utilities")
            || url.starts_with("https://www.openstreetmap.org/?mlat=")
            || weather::PROVIDERS.iter().any(|p| p.signup_url == Some(url))
            || url.starts_with("https://chilkey.com/")
            || url.starts_with("https://www.chilkey.com.au/")
            || url.starts_with("https://drive.google.com/drive/folders/") =>
        {
            use tauri_plugin_opener::OpenerExt;
            app.opener().open_url(url, None::<&str>).map_err(err)?;
        }
        other => return Err(format!("cannot open {other:?}")),
    }
    Ok(())
}
