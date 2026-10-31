//! Commands for the settings window (M3). Everything the UI shows or changes
//! goes through here; the window never touches files or devices itself.

use std::sync::atomic::Ordering;
use std::time::Duration;

use nd104_core::app_update;
use nd104_core::clock::{ntp_offset, ClockSettings};
use nd104_core::config::{self, Config};
use nd104_core::engine::{EngineCommand, Status};
use nd104_core::model::{self, VerifiedFirmware};
use nd104_core::weather::{self, geocoding, FetchContext, WeatherReport, WeatherSettings};
use nd104_core::APP_VERSION;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
pub struct Provider {
    id: &'static str,
    name: &'static str,
    requires_api_key: bool,
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
    providers: Vec<Provider>,
    models: Vec<ModelInfo>,
    min_weather_interval: u32,
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
        providers: weather::PROVIDERS
            .iter()
            .map(|(id, name, key)| Provider {
                id,
                name,
                requires_api_key: *key,
            })
            .collect(),
        models: model::registry()
            .iter()
            .map(|m| ModelInfo {
                id: m.id(),
                name: m.display_name(),
                verified_firmware: m.verified_firmware().to_vec(),
            })
            .collect(),
        min_weather_interval: WeatherSettings::MIN_INTERVAL_MIN,
    }
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
#[tauri::command]
pub async fn test_weather(settings: WeatherSettings) -> CmdResult<WeatherReport> {
    let provider = weather::create(&settings.provider)
        .ok_or_else(|| format!("unknown weather provider {:?}", settings.provider))?;
    let ctx = FetchContext {
        client: weather::new_http_client().map_err(err)?,
        location: settings.location.location(),
        api_key: None,
        settings: settings.clone(),
    };
    provider.fetch(&ctx).await.map_err(|e| format!("{e:#}"))
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
            state.manual_update_check.store(true, Ordering::SeqCst);
            EngineCommand::CheckAppUpdate
        }
        other => return Err(format!("unknown command {other:?}")),
    };
    state.commands.try_send(cmd).map_err(err)
}

/// Opens a known location. URLs are limited to this project on GitHub and to
/// OpenStreetMap markers (location check in the weather settings).
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
            || url.starts_with("https://www.openstreetmap.org/?mlat=") =>
        {
            use tauri_plugin_opener::OpenerExt;
            app.opener().open_url(url, None::<&str>).map_err(err)?;
        }
        other => return Err(format!("cannot open {other:?}")),
    }
    Ok(())
}
