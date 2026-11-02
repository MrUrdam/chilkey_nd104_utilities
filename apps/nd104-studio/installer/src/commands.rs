//! Commands for the setup window.

use std::cmp::Ordering;

use nd104_core::hid::{HidBackend, HidapiBackend};
use nd104_core::{app_update, config, model, APP_VERSION};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

use crate::tasks::{self, InstallOptions, Progress};
use crate::{payload, system, Mode, SetupState};

type CmdResult<T> = Result<T, String>;

const LINKS: &[(&str, &str)] = &[
    (
        "project",
        "https://github.com/MrUrdam/chilkey_nd104_utilities/tree/main/apps/nd104-studio",
    ),
    (
        "license",
        "https://github.com/MrUrdam/chilkey_nd104_utilities/blob/main/LICENSE",
    ),
    (
        "changelog",
        "https://github.com/MrUrdam/chilkey_nd104_utilities/blob/main/apps/nd104-studio/CHANGELOG.md",
    ),
];

// ---------------------------------------------------------------------------
// Texts (DE/EN): the app's language setting, else the system language

pub fn load_texts() -> serde_json::Value {
    let configured = config::default_path()
        .ok()
        .and_then(|p| config::Config::load(&p).ok())
        .map(|c| c.general.language)
        .filter(|l| l == "de" || l == "en");
    let lang = configured.unwrap_or_else(|| {
        let system = sys_locale::get_locale().unwrap_or_default().to_lowercase();
        if system.starts_with("de") { "de" } else { "en" }.into()
    });
    let raw = if lang == "de" {
        include_str!("../locales/de.json")
    } else {
        include_str!("../locales/en.json")
    };
    let mut texts: serde_json::Value = serde_json::from_str(raw).expect("valid locale file");
    texts["lang"] = lang.into();
    texts
}

/// Text for `key`; the key itself if missing.
pub fn text(texts: &serde_json::Value, key: &str) -> String {
    texts
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(key)
        .to_string()
}

#[tauri::command]
pub fn texts(state: State<'_, SetupState>) -> serde_json::Value {
    state.texts.clone()
}

// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Relation {
    Fresh,
    Upgrade,
    Same,
    Downgrade,
}

#[derive(Serialize)]
pub struct SetupInfo {
    mode: Mode,
    version: &'static str,
    installed: Option<String>,
    relation: Relation,
    /// An all-users MSI installation exists as well.
    msi: bool,
    /// The setup contains the app (false only for development builds).
    payload: bool,
    autostart: bool,
    desktop_shortcut: bool,
    style: String,
}

/// Also shows the window (it starts hidden to avoid a white flash).
#[tauri::command]
pub fn setup_info(window: WebviewWindow, state: State<'_, SetupState>) -> SetupInfo {
    let installed = system::installed();
    let relation = match &installed {
        None => Relation::Fresh,
        Some(i) => match app_update::compare(APP_VERSION, &i.version) {
            Ordering::Greater => Relation::Upgrade,
            Ordering::Equal => Relation::Same,
            Ordering::Less => Relation::Downgrade,
        },
    };
    let info = SetupInfo {
        mode: state.mode,
        version: APP_VERSION,
        relation,
        msi: system::msi_installed(),
        payload: payload::available(),
        autostart: installed.is_none() || system::autostart_enabled(),
        desktop_shortcut: system::has_desktop_shortcut(),
        style: config::default_path()
            .ok()
            .and_then(|p| config::Config::load(&p).ok())
            .map(|c| c.general.style)
            .unwrap_or_else(|| "calm".into()),
        installed: installed.map(|i| i.version),
    };
    let _ = window.show();
    let _ = window.set_focus();
    info
}

fn progress_sink(app: &AppHandle) -> impl Fn(Progress) + Sync + 'static {
    let app = app.clone();
    move |p: Progress| {
        let _ = app.emit("progress", p);
    }
}

#[tauri::command]
pub async fn run_install(app: AppHandle, options: InstallOptions) -> CmdResult<()> {
    let sink = progress_sink(&app);
    let update = app.state::<SetupState>().mode == Mode::Update;
    tauri::async_runtime::spawn_blocking(move || {
        tasks::install(options, &sink)?;
        if update {
            // an update started from the app brings it back right away
            system::launch(&system::install_dir()?.join(system::APP_EXE))?;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn run_uninstall(app: AppHandle, keep_settings: bool) -> CmdResult<()> {
    let sink = progress_sink(&app);
    tauri::async_runtime::spawn_blocking(move || tasks::uninstall(keep_settings, &sink))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn launch_app() -> CmdResult<()> {
    system::install_dir()
        .and_then(|d| system::launch(&d.join(system::APP_EXE)))
        .map_err(|e| format!("{e:#}"))
}

/// Is a supported keyboard connected, and how ("usb" or "bluetooth")? Only
/// for the hint on the last page, asked again every 2 s – nothing is sent
/// to the keyboard.
#[tauri::command]
pub async fn keyboard_connected() -> Option<&'static str> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut backend = HidapiBackend::new().ok()?;
        let found = backend.enumerate().ok()?;
        let ours: Vec<_> = found
            .iter()
            .filter(|f| {
                model::registry().iter().any(|m| {
                    m.interfaces()
                        .iter()
                        .any(|spec| f.vid == spec.vid && f.pid == spec.pid)
                })
            })
            .collect();
        // over Bluetooth the keyboard reports the same IDs
        if ours.iter().any(|f| !f.is_bluetooth()) {
            Some("usb")
        } else if !ours.is_empty() {
            Some("bluetooth")
        } else {
            None
        }
    })
    .await
    .ok()
    .flatten()
}

/// Opens one of the fixed project links in the browser.
#[tauri::command]
pub fn open_link(app: AppHandle, which: String) -> CmdResult<()> {
    let (_, url) = LINKS
        .iter()
        .find(|(name, _)| *name == which)
        .ok_or_else(|| format!("unknown link {which:?}"))?;
    app.opener()
        .open_url(*url, None::<&str>)
        .map_err(|e| e.to_string())
}
