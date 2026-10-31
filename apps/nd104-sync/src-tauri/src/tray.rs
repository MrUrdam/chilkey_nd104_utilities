//! Tray icon, menu and status presentation (PLAN.md § 6.1).

use nd104_core::device::{DeviceState, Trust};
use nd104_core::engine::Status;
use nd104_core::weather::Condition;
use nd104_core::APP_VERSION;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};

use crate::i18n::{I18n, LANGUAGES};

pub const TRAY_ID: &str = "main";

/// Icon colour = state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    /// Connected and synced (accent).
    Ok,
    /// No keyboard (grey).
    Off,
    /// Protocol check failed (amber).
    Locked,
    /// Weather stale / errors (red).
    Error,
}

impl TrayState {
    pub fn of(status: &Status) -> Self {
        match &status.device {
            DeviceState::NotConnected => TrayState::Off,
            DeviceState::Locked { .. } => TrayState::Locked,
            DeviceState::Ready(_) if status.weather_stale => TrayState::Error,
            DeviceState::Ready(_) => TrayState::Ok,
        }
    }

    pub fn icon(self) -> Image<'static> {
        let bytes: &'static [u8] = match self {
            TrayState::Ok => include_bytes!("../icons/tray/ok.png"),
            TrayState::Off => include_bytes!("../icons/tray/off.png"),
            TrayState::Locked => include_bytes!("../icons/tray/locked.png"),
            TrayState::Error => include_bytes!("../icons/tray/error.png"),
        };
        Image::from_bytes(bytes).expect("embedded tray icons are valid PNGs")
    }
}

pub fn condition_text(i18n: &I18n, condition: Condition) -> String {
    let key = match condition {
        Condition::Clear => "weather.clear",
        Condition::PartlyCloudy => "weather.partly_cloudy",
        Condition::Cloudy => "weather.cloudy",
        Condition::Overcast => "weather.overcast",
        Condition::Fog => "weather.fog",
        Condition::Drizzle => "weather.drizzle",
        Condition::Rain => "weather.rain",
        Condition::Snow => "weather.snow",
        Condition::Thunderstorm => "weather.thunderstorm",
    };
    i18n.t(key)
}

/// One-line status for the first menu entry.
pub fn status_line(i18n: &I18n, status: &Status) -> String {
    let mut line = match &status.device {
        DeviceState::NotConnected => i18n.t("status.not_connected"),
        DeviceState::Locked { reason, .. } => i18n.tf("status.locked", &[("reason", reason)]),
        DeviceState::Ready(d) => {
            let model = d.model_name.trim_start_matches("Chilkey ");
            match &status.weather {
                Some(w) => {
                    let weather = format!(
                        "{} {:.1} °C",
                        condition_text(i18n, w.condition),
                        w.current_c
                    );
                    i18n.tf("status.ready", &[("model", model), ("weather", &weather)])
                }
                None => i18n.tf("status.ready_no_weather", &[("model", model)]),
            }
        }
    };
    if status.paused {
        line = format!("{line} ({})", i18n.t("status.paused"));
    }
    line
}

/// Tooltip: version, status, last sync. Windows limits it to 127 characters.
pub fn tooltip(i18n: &I18n, status: &Status) -> String {
    let mut lines = vec![
        format!("nd104-sync {APP_VERSION}"),
        status_line(i18n, status),
    ];
    if let DeviceState::Ready(d) = &status.device {
        lines.push(i18n.t(match d.trust {
            Trust::Verified => "status.verified",
            Trust::Compatible => "status.compatible",
        }));
    }
    if let Some(at) = status.last_time_sync {
        let time = at
            .to_zoned(jiff::tz::TimeZone::system())
            .strftime("%H:%M")
            .to_string();
        lines.push(i18n.tf("status.synced", &[("time", &time)]));
    }
    let text = lines.join("\n");
    text.chars().take(127).collect()
}

/// Menu entries that change at runtime.
#[derive(Clone)]
pub struct TrayItems {
    pub status: MenuItem<Wry>,
    pub pause: MenuItem<Wry>,
}

pub fn build_menu(
    app: &AppHandle,
    i18n: &I18n,
    language: &str,
    autostart: bool,
    status: &Status,
) -> tauri::Result<(Menu<Wry>, TrayItems)> {
    let status_item = MenuItem::with_id(
        app,
        "status",
        status_line(i18n, status),
        false,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        i18n.t(if status.paused {
            "menu.resume"
        } else {
            "menu.pause"
        }),
        true,
        None::<&str>,
    )?;

    let auto = CheckMenuItem::with_id(
        app,
        "lang:auto",
        i18n.t("menu.language_auto"),
        true,
        language == "auto",
        None::<&str>,
    )?;
    let mut languages = vec![auto];
    for (code, name) in LANGUAGES {
        languages.push(CheckMenuItem::with_id(
            app,
            format!("lang:{code}"),
            *name,
            true,
            language == *code,
            None::<&str>,
        )?);
    }
    let language_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = languages
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<Wry>)
        .collect();
    let language_menu = Submenu::with_id_and_items(
        app,
        "language",
        i18n.t("menu.language"),
        true,
        &language_refs,
    )?;

    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "sync_now", i18n.t("menu.sync_now"), true, None::<&str>)?,
            &pause,
            &MenuItem::with_id(app, "recheck", i18n.t("menu.recheck"), true, None::<&str>)?,
            &MenuItem::with_id(
                app,
                "check_updates",
                i18n.t("menu.check_updates"),
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "settings", i18n.t("menu.settings"), true, None::<&str>)?,
            &language_menu,
            &CheckMenuItem::with_id(
                app,
                "autostart",
                i18n.t("menu.autostart"),
                true,
                autostart,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "open_logs",
                i18n.t("menu.open_logs"),
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "about",
                i18n.tf("menu.about", &[("version", APP_VERSION)]),
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "quit", i18n.t("menu.quit"), true, None::<&str>)?,
        ],
    )?;
    Ok((
        menu,
        TrayItems {
            status: status_item,
            pause,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nd104_core::device::ConnectedDevice;
    use nd104_core::model::ScreenVariant;
    use nd104_core::weather::WeatherReport;

    fn ready(trust: Trust) -> Status {
        Status {
            device: DeviceState::Ready(ConnectedDevice {
                model_id: "chilkey-nd104",
                model_name: "Chilkey ND104",
                firmware: vec![],
                trust,
                screen_variant: ScreenVariant::Celsius24h,
            }),
            weather: Some(WeatherReport {
                condition: Condition::PartlyCloudy,
                is_day: true,
                current_c: 8.24,
                max_c: 9.1,
                min_c: 2.1,
                observed_at: jiff::Timestamp::UNIX_EPOCH,
                provider: "open-meteo".into(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn status_lines() {
        let en = I18n::new("en");
        assert_eq!(
            status_line(&en, &ready(Trust::Verified)),
            "ND104 · Partly cloudy 8.2 °C"
        );
        assert_eq!(
            status_line(&I18n::new("de"), &ready(Trust::Verified)),
            "ND104 · Teils bewölkt 8.2 °C"
        );
        assert_eq!(
            status_line(&en, &Status::default()),
            "Not connected – USB cable required"
        );
        let mut paused = ready(Trust::Verified);
        paused.paused = true;
        assert!(status_line(&en, &paused).ends_with("(paused)"));
    }

    #[test]
    fn tooltip_starts_with_version_and_fits() {
        let mut s = ready(Trust::Compatible);
        s.device = DeviceState::Locked {
            model_name: Some("Chilkey ND104"),
            firmware: vec![],
            reason: "P5: ".to_string() + &"x".repeat(300),
        };
        let t = tooltip(&I18n::new("en"), &s);
        assert!(t.starts_with(&format!("nd104-sync {APP_VERSION}")));
        assert!(t.chars().count() <= 127);
        assert!(
            tooltip(&I18n::new("en"), &ready(Trust::Compatible)).contains("unverified firmware")
        );
    }

    #[test]
    fn tray_states() {
        assert_eq!(TrayState::of(&Status::default()), TrayState::Off);
        assert_eq!(TrayState::of(&ready(Trust::Verified)), TrayState::Ok);
        let mut stale = ready(Trust::Verified);
        stale.weather_stale = true;
        assert_eq!(TrayState::of(&stale), TrayState::Error);
    }
}
