//! nd104-sync Setup – installs, updates and uninstalls nd104-sync for the
//! current user (no admin rights), with its own window.
//!
//! ```text
//! nd104-sync-setup.exe                      install, or update an existing installation
//! nd104-sync-setup.exe --update             update started from the app: progress only, restarts the app
//! nd104-sync-setup.exe --uninstall          uninstall (entry in Windows "Apps")
//! … --silent                                no window; exit code 0 = success
//! … --silent --no-autostart                 (install) do not start with Windows
//! … --silent --desktop-shortcut             (install) also create a desktop shortcut
//! … --uninstall --silent --remove-settings  also delete settings, API keys and logs
//! ```

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod payload;
mod system;
mod tasks;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

use crate::tasks::InstallOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Install,
    Update,
    Uninstall,
}

pub struct Args {
    pub mode: Mode,
    pub silent: bool,
    relaunched: bool,
    remove_settings: bool,
    no_autostart: bool,
    desktop_shortcut: bool,
}

impl Args {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let has = |flag: &str| args.iter().any(|a| a.eq_ignore_ascii_case(flag));
        Self {
            mode: if has("--uninstall") {
                Mode::Uninstall
            } else if has("--update") {
                Mode::Update
            } else {
                Mode::Install
            },
            silent: has("--silent") || has("/S"),
            relaunched: has("--relaunched"),
            remove_settings: has("--remove-settings"),
            no_autostart: has("--no-autostart"),
            desktop_shortcut: has("--desktop-shortcut"),
        }
    }
}

/// Per-run state shared with the commands.
pub struct SetupState {
    pub mode: Mode,
    pub texts: serde_json::Value,
    /// Running as the temporary copy: delete it on exit.
    pub cleanup_self: Mutex<bool>,
}

fn main() {
    let args = Args::parse();
    let texts = commands::load_texts();

    // The setup inside the installation folder cannot remove or replace it:
    // continue from a temporary copy (silent: wait for its result).
    if !args.relaunched && (args.mode == Mode::Uninstall || system::running_from_install_dir()) {
        let raw: Vec<String> = std::env::args().skip(1).collect();
        if let Ok(mut child) = system::relaunch_from_temp(&raw) {
            if args.silent {
                let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
                std::process::exit(code);
            }
            return;
        }
    }

    if args.silent {
        std::process::exit(match run_silent(&args) {
            Ok(()) => 0,
            Err(_) => 1,
        });
    }

    if !system::webview2_installed() {
        let t = |key: &str| commands::text(&texts, key);
        if !system::ask(&t("webview2.title"), &t("webview2.question")) {
            return;
        }
        if system::install_webview2().is_err() {
            system::ask(&t("webview2.title"), &t("webview2.failed"));
            return;
        }
    }

    let state = SetupState {
        mode: args.mode,
        texts,
        cleanup_self: Mutex::new(args.relaunched),
    };
    let title = commands::text(&state.texts, "window_title");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::setup_info,
            commands::texts,
            commands::run_install,
            commands::run_uninstall,
            commands::launch_app,
            commands::keyboard_connected,
            commands::open_link,
        ])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title(&title)
                .inner_size(620.0, 440.0)
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                .center()
                .visible(false)
                // keep the setup's browser data out of the user's profile
                .data_directory(std::env::temp_dir().join("nd104-sync-setup"))
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("cannot start the setup window");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            let state = handle.state::<SetupState>();
            if *state.cleanup_self.lock().unwrap() {
                system::delete_self_later();
            }
        }
    });
}

fn run_silent(args: &Args) -> anyhow::Result<()> {
    let quiet = |_| {};
    let result = match args.mode {
        Mode::Uninstall => tasks::uninstall(!args.remove_settings, &quiet),
        Mode::Install | Mode::Update => {
            if !system::webview2_installed() {
                system::install_webview2()?;
            }
            let existing = system::installed().is_some();
            let options = InstallOptions {
                autostart: !args.no_autostart && (!existing || system::autostart_enabled()),
                desktop_shortcut: args.desktop_shortcut || system::has_desktop_shortcut(),
                style: None,
            };
            let result = tasks::install(options, &quiet);
            if result.is_ok() && args.mode == Mode::Update {
                let _ = system::launch(&system::install_dir()?.join(system::APP_EXE));
            }
            result
        }
    };
    if args.relaunched {
        system::delete_self_later();
    }
    result
}
