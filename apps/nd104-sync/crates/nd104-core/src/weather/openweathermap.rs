//! [OpenWeatherMap](https://openweathermap.org/) – worldwide, free API key.
//!
//! Uses the free "current weather" and "5 day / 3 hour forecast" endpoints;
//! today's max/min come from the forecast entries of the local day.

use anyhow::{Context, Result};
use jiff::Timestamp;
use serde::Deserialize;

use super::{
    fetch_text, require_key, require_location, BoxFuture, Condition, FetchContext, WeatherProvider,
    WeatherReport,
};

const BASE: &str = "https://api.openweathermap.org/data/2.5";

pub struct OpenWeatherMap;

impl OpenWeatherMap {
    pub const ID: &'static str = "openweathermap";
}

impl WeatherProvider for OpenWeatherMap {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn display_name(&self) -> &'static str {
        "OpenWeatherMap"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn fetch<'a>(&'a self, ctx: &'a FetchContext) -> BoxFuture<'a, Result<WeatherReport>> {
        Box::pin(async move {
            let key = require_key(ctx, "OpenWeatherMap")?;
            let location = require_location(ctx)?;
            let query = [
                ("lat", location.lat.to_string()),
                ("lon", location.lon.to_string()),
                ("units", "metric".to_string()),
                ("appid", key.to_string()),
            ];
            let current = fetch_text(
                ctx.client.get(format!("{BASE}/weather")).query(&query),
                "OpenWeatherMap",
            )
            .await?;
            let forecast = fetch_text(
                ctx.client.get(format!("{BASE}/forecast")).query(&query),
                "OpenWeatherMap",
            )
            .await?;
            parse(&current, &forecast)
        })
    }
}

#[derive(Deserialize)]
struct Current {
    weather: Vec<Condition0>,
    main: Main,
    dt: i64,
    timezone: i64,
}

#[derive(Deserialize)]
struct Condition0 {
    id: u16,
    icon: String,
}

#[derive(Deserialize)]
struct Main {
    temp: f32,
}

#[derive(Deserialize)]
struct Forecast {
    list: Vec<ForecastItem>,
}

#[derive(Deserialize)]
struct ForecastItem {
    dt: i64,
    main: Main,
}

fn parse(current: &str, forecast: &str) -> Result<WeatherReport> {
    let c: Current = serde_json::from_str(current).context("unexpected OpenWeatherMap answer")?;
    let f: Forecast = serde_json::from_str(forecast).context("unexpected OpenWeatherMap answer")?;
    let w = c
        .weather
        .first()
        .context("OpenWeatherMap answer without weather")?;

    // Local day at the location (OpenWeatherMap gives the UTC offset in seconds).
    let local_day = |ts: i64| (ts + c.timezone).div_euclid(86_400);
    let today = local_day(c.dt);
    let temps = f
        .list
        .iter()
        .filter(|i| local_day(i.dt) == today)
        .map(|i| i.main.temp)
        .chain([c.main.temp]);
    let max = temps.clone().fold(f32::MIN, f32::max);
    let min = temps.fold(f32::MAX, f32::min);

    Ok(WeatherReport {
        condition: id_condition(w.id),
        is_day: !w.icon.ends_with('n'),
        current_c: c.main.temp,
        max_c: max,
        min_c: min,
        observed_at: Timestamp::now(),
        provider: OpenWeatherMap::ID.into(),
    })
}

/// OpenWeatherMap weather condition ids.
fn id_condition(id: u16) -> Condition {
    match id {
        200..=299 => Condition::Thunderstorm,
        300..=399 => Condition::Drizzle,
        500..=599 => Condition::Rain,
        600..=699 => Condition::Snow,
        700..=799 => Condition::Fog,
        800 => Condition::Clear,
        801 | 802 => Condition::PartlyCloudy,
        803 => Condition::Cloudy,
        _ => Condition::Overcast, // 804
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // dt 2026-10-31T12:00Z, UTC+1
    const CURRENT: &str = r#"{"weather":[{"id":803,"main":"Clouds","icon":"04d"}],
        "main":{"temp":8.2,"temp_min":7.0,"temp_max":9.0},"dt":1793448000,"timezone":3600}"#;
    const FORECAST: &str = r#"{"list":[
        {"dt":1793451600,"main":{"temp":9.6}},
        {"dt":1793473200,"main":{"temp":3.1}},
        {"dt":1793487600,"main":{"temp":-1.5}}
    ],"city":{"timezone":3600}}"#;

    #[test]
    fn parses_and_limits_to_local_day() {
        let r = parse(CURRENT, FORECAST).unwrap();
        assert_eq!(r.condition, Condition::Cloudy);
        assert!(r.is_day);
        assert_eq!(r.current_c, 8.2);
        assert_eq!(r.max_c, 9.6);
        // 1793487600 = 2026-10-31T23:00Z = 00:00 local on the next day → excluded
        assert_eq!(r.min_c, 3.1);
    }

    #[test]
    fn condition_ids() {
        assert_eq!(id_condition(211), Condition::Thunderstorm);
        assert_eq!(id_condition(301), Condition::Drizzle);
        assert_eq!(id_condition(502), Condition::Rain);
        assert_eq!(id_condition(601), Condition::Snow);
        assert_eq!(id_condition(741), Condition::Fog);
        assert_eq!(id_condition(800), Condition::Clear);
        assert_eq!(id_condition(802), Condition::PartlyCloudy);
        assert_eq!(id_condition(804), Condition::Overcast);
    }
}
