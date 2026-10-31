//! [Open-Meteo](https://open-meteo.com/) – free, no API key, worldwide.

use anyhow::{anyhow, Context, Result};
use jiff::Timestamp;
use serde::Deserialize;

use super::{BoxFuture, Condition, FetchContext, WeatherProvider, WeatherReport};

pub struct OpenMeteo {
    base_url: String,
}

impl Default for OpenMeteo {
    fn default() -> Self {
        Self {
            base_url: "https://api.open-meteo.com/v1/forecast".into(),
        }
    }
}

impl OpenMeteo {
    pub const ID: &'static str = "open-meteo";
}

impl WeatherProvider for OpenMeteo {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn display_name(&self) -> &'static str {
        "Open-Meteo"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn fetch<'a>(&'a self, ctx: &'a FetchContext) -> BoxFuture<'a, Result<WeatherReport>> {
        Box::pin(async move {
            let location = ctx
                .location
                .as_ref()
                .ok_or_else(|| anyhow!("no location configured"))?;
            let body = ctx
                .client
                .get(&self.base_url)
                .query(&[
                    ("latitude", location.lat.to_string()),
                    ("longitude", location.lon.to_string()),
                    ("current", "temperature_2m,weather_code,is_day".into()),
                    ("daily", "temperature_2m_max,temperature_2m_min".into()),
                    ("timezone", "auto".into()),
                    ("forecast_days", "1".into()),
                ])
                .send()
                .await
                .map_err(super::without_url)
                .context("Open-Meteo request failed")?
                .error_for_status()
                .map_err(super::without_url)
                .context("Open-Meteo returned an error")?
                .text()
                .await
                .map_err(super::without_url)?;
            parse(&body)
        })
    }
}

#[derive(Deserialize)]
struct Response {
    current: Current,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f32,
    weather_code: u8,
    is_day: u8,
}

#[derive(Deserialize)]
struct Daily {
    temperature_2m_max: Vec<Option<f32>>,
    temperature_2m_min: Vec<Option<f32>>,
}

fn parse(body: &str) -> Result<WeatherReport> {
    let r: Response = serde_json::from_str(body).context("unexpected Open-Meteo answer")?;
    let max = r.daily.temperature_2m_max.first().copied().flatten();
    let min = r.daily.temperature_2m_min.first().copied().flatten();
    let current = r.current.temperature_2m;
    Ok(WeatherReport {
        condition: wmo_condition(r.current.weather_code),
        is_day: r.current.is_day != 0,
        current_c: current,
        max_c: max.unwrap_or(current).max(current),
        min_c: min.unwrap_or(current).min(current),
        observed_at: Timestamp::now(),
        provider: OpenMeteo::ID.into(),
    })
}

/// WMO weather interpretation codes as used by Open-Meteo.
fn wmo_condition(code: u8) -> Condition {
    match code {
        0 | 1 => Condition::Clear,
        2 => Condition::PartlyCloudy,
        3 => Condition::Overcast,
        45 | 48 => Condition::Fog,
        51..=57 => Condition::Drizzle,
        61..=67 | 80..=82 => Condition::Rain,
        71..=77 | 85 | 86 => Condition::Snow,
        95..=99 => Condition::Thunderstorm,
        _ => Condition::Cloudy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "latitude": 52.52, "longitude": 13.42, "utc_offset_seconds": 3600,
        "current": {"time": "2026-10-31T12:00", "interval": 900,
                    "temperature_2m": -3.4, "weather_code": 73, "is_day": 1},
        "daily": {"time": ["2026-10-31"],
                  "temperature_2m_max": [1.2], "temperature_2m_min": [-7.6]}
    }"#;

    #[test]
    fn parses_answer() {
        let r = parse(SAMPLE).unwrap();
        assert_eq!(r.condition, Condition::Snow);
        assert!(r.is_day);
        assert_eq!(r.current_c, -3.4);
        assert_eq!(r.max_c, 1.2);
        assert_eq!(r.min_c, -7.6);
        assert_eq!(r.provider, "open-meteo");
    }

    #[test]
    fn min_max_always_include_current() {
        let body = SAMPLE
            .replace("[1.2]", "[-5.0]")
            .replace("[-7.6]", "[null]");
        let r = parse(&body).unwrap();
        assert_eq!(r.max_c, -3.4);
        assert_eq!(r.min_c, -3.4);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("{}").is_err());
    }

    #[test]
    fn wmo_mapping() {
        assert_eq!(wmo_condition(0), Condition::Clear);
        assert_eq!(wmo_condition(2), Condition::PartlyCloudy);
        assert_eq!(wmo_condition(3), Condition::Overcast);
        assert_eq!(wmo_condition(48), Condition::Fog);
        assert_eq!(wmo_condition(55), Condition::Drizzle);
        assert_eq!(wmo_condition(81), Condition::Rain);
        assert_eq!(wmo_condition(86), Condition::Snow);
        assert_eq!(wmo_condition(95), Condition::Thunderstorm);
    }
}
