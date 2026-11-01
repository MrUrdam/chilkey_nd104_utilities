//! [Bright Sky](https://brightsky.dev/) – free JSON API for DWD (German
//! weather service) data. No API key; data for Germany and close surroundings.

use anyhow::{bail, Context, Result};
use jiff::Timestamp;
use serde::Deserialize;

use super::{
    fetch_text, require_location, BoxFuture, Condition, FetchContext, WeatherProvider,
    WeatherReport,
};

const BASE: &str = "https://api.brightsky.dev";

pub struct BrightSky;

impl BrightSky {
    pub const ID: &'static str = "brightsky";
}

impl WeatherProvider for BrightSky {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn display_name(&self) -> &'static str {
        "Bright Sky (DWD)"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn fetch<'a>(&'a self, ctx: &'a FetchContext) -> BoxFuture<'a, Result<WeatherReport>> {
        Box::pin(async move {
            let location = require_location(ctx)?;
            let (lat, lon) = (location.lat.to_string(), location.lon.to_string());
            let current = fetch_text(
                ctx.client
                    .get(format!("{BASE}/current_weather"))
                    .query(&[("lat", &lat), ("lon", &lon)]),
                "Bright Sky",
            )
            .await?;

            // Today's hourly records (local day of the PC) for max/min.
            let zone = jiff::tz::TimeZone::system();
            let today = Timestamp::now().to_zoned(zone.clone()).date();
            let tz = zone.iana_name().unwrap_or("Europe/Berlin").to_string();
            let day = fetch_text(
                ctx.client.get(format!("{BASE}/weather")).query(&[
                    ("lat", lat.as_str()),
                    ("lon", lon.as_str()),
                    ("date", &today.to_string()),
                    ("tz", &tz),
                ]),
                "Bright Sky",
            )
            .await?;
            parse(&current, &day)
        })
    }
}

#[derive(Deserialize)]
struct Current {
    weather: CurrentRecord,
}

#[derive(Deserialize)]
struct CurrentRecord {
    temperature: Option<f32>,
    icon: Option<String>,
    /// %, used to tell "cloudy" from "overcast"
    #[serde(default)]
    cloud_cover: Option<f32>,
}

#[derive(Deserialize)]
struct Day {
    weather: Vec<HourRecord>,
}

#[derive(Deserialize)]
struct HourRecord {
    temperature: Option<f32>,
}

fn parse(current: &str, day: &str) -> Result<WeatherReport> {
    let c: Current = serde_json::from_str(current).context("unexpected Bright Sky answer")?;
    let d: Day = serde_json::from_str(day).context("unexpected Bright Sky answer")?;
    let Some(now) = c.weather.temperature else {
        bail!("Bright Sky has no current temperature for this location (Germany only)");
    };
    let temps = d.weather.iter().filter_map(|r| r.temperature).chain([now]);
    let max = temps.clone().fold(f32::MIN, f32::max);
    let min = temps.fold(f32::MAX, f32::min);
    let icon = c.weather.icon.unwrap_or_default();
    Ok(WeatherReport {
        condition: icon_condition(&icon, c.weather.cloud_cover),
        is_day: !icon.ends_with("-night"),
        current_c: now,
        max_c: max,
        min_c: min,
        observed_at: Timestamp::now(),
        provider: BrightSky::ID.into(),
    })
}

/// Bright Sky icon names (Dark Sky style). "cloudy" covers both "cloudy"
/// and "overcast"; the cloud cover tells them apart.
fn icon_condition(icon: &str, cloud_cover: Option<f32>) -> Condition {
    match icon {
        "clear-day" | "clear-night" => Condition::Clear,
        "partly-cloudy-day" | "partly-cloudy-night" => Condition::PartlyCloudy,
        "cloudy" => super::cloudy_or_overcast(cloud_cover),
        "fog" => Condition::Fog,
        "rain" | "hail" => Condition::Rain,
        "sleet" | "snow" => Condition::Snow,
        "thunderstorm" => Condition::Thunderstorm,
        _ => Condition::Cloudy, // "wind" or no icon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT: &str = r#"{"weather":{"source_id":1,"timestamp":"2026-10-31T12:00:00+00:00",
        "temperature":8.3,"icon":"partly-cloudy-night","condition":"dry"},"sources":[]}"#;
    const DAY: &str = r#"{"weather":[{"temperature":2.1},{"temperature":null},
        {"temperature":9.4},{"temperature":5.0}],"sources":[]}"#;

    #[test]
    fn parses_current_and_day() {
        let r = parse(CURRENT, DAY).unwrap();
        assert_eq!(r.condition, Condition::PartlyCloudy);
        assert!(!r.is_day);
        assert_eq!((r.current_c, r.max_c, r.min_c), (8.3, 9.4, 2.1));
        assert_eq!(r.provider, "brightsky");
    }

    #[test]
    fn missing_temperature_is_an_error() {
        let current = CURRENT.replace("8.3", "null");
        assert!(parse(&current, DAY).is_err());
    }

    #[test]
    fn icons() {
        assert_eq!(icon_condition("cloudy", None), Condition::Overcast);
        assert_eq!(icon_condition("sleet", None), Condition::Snow);
        assert_eq!(
            icon_condition("thunderstorm", None),
            Condition::Thunderstorm
        );
        assert_eq!(icon_condition("wind", None), Condition::Cloudy);
    }

    #[test]
    fn cloud_cover_tells_cloudy_from_overcast() {
        assert_eq!(icon_condition("cloudy", Some(80.0)), Condition::Cloudy);
        assert_eq!(icon_condition("cloudy", Some(96.0)), Condition::Overcast);
        // only "cloudy" is refined; other icons keep their meaning
        assert_eq!(
            icon_condition("partly-cloudy-day", Some(80.0)),
            Condition::PartlyCloudy
        );
        let current = CURRENT.replace(
            r#""icon":"partly-cloudy-night""#,
            r#""icon":"cloudy","cloud_cover":75"#,
        );
        assert_eq!(parse(&current, DAY).unwrap().condition, Condition::Cloudy);
    }
}
