//! [WeatherAPI.com](https://www.weatherapi.com/) – worldwide, free API key.
//! Uses the same condition codes as the official Chilkey weather plugin.

use anyhow::{Context, Result};
use jiff::Timestamp;
use serde::Deserialize;

use super::{
    fetch_text, require_key, require_location, BoxFuture, Condition, FetchContext, WeatherProvider,
    WeatherReport,
};

const URL: &str = "https://api.weatherapi.com/v1/forecast.json";

pub struct WeatherApi;

impl WeatherApi {
    pub const ID: &'static str = "weatherapi";
}

impl WeatherProvider for WeatherApi {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn display_name(&self) -> &'static str {
        "WeatherAPI.com"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn fetch<'a>(&'a self, ctx: &'a FetchContext) -> BoxFuture<'a, Result<WeatherReport>> {
        Box::pin(async move {
            let key = require_key(ctx, "WeatherAPI.com")?;
            let location = require_location(ctx)?;
            let body = fetch_text(
                ctx.client.get(URL).query(&[
                    ("key", key.to_string()),
                    ("q", format!("{},{}", location.lat, location.lon)),
                    ("days", "1".into()),
                    ("aqi", "no".into()),
                    ("alerts", "no".into()),
                ]),
                "WeatherAPI.com",
            )
            .await?;
            parse(&body)
        })
    }
}

#[derive(Deserialize)]
struct Response {
    current: Current,
    forecast: Forecast,
}

#[derive(Deserialize)]
struct Current {
    temp_c: f32,
    is_day: u8,
    condition: Code,
}

#[derive(Deserialize)]
struct Code {
    code: u16,
}

#[derive(Deserialize)]
struct Forecast {
    forecastday: Vec<ForecastDay>,
}

#[derive(Deserialize)]
struct ForecastDay {
    day: Day,
}

#[derive(Deserialize)]
struct Day {
    maxtemp_c: f32,
    mintemp_c: f32,
}

fn parse(body: &str) -> Result<WeatherReport> {
    let r: Response = serde_json::from_str(body).context("unexpected WeatherAPI.com answer")?;
    let day = r
        .forecast
        .forecastday
        .first()
        .context("WeatherAPI.com answer without forecast")?;
    let now = r.current.temp_c;
    Ok(WeatherReport {
        condition: code_condition(r.current.condition.code),
        is_day: r.current.is_day != 0,
        current_c: now,
        max_c: day.day.maxtemp_c.max(now),
        min_c: day.day.mintemp_c.min(now),
        observed_at: Timestamp::now(),
        provider: WeatherApi::ID.into(),
    })
}

/// WeatherAPI.com condition codes.
fn code_condition(code: u16) -> Condition {
    match code {
        1000 => Condition::Clear,
        1003 => Condition::PartlyCloudy,
        1006 => Condition::Cloudy,
        1009 => Condition::Overcast,
        1030 | 1135 | 1147 => Condition::Fog,
        1063 | 1150 | 1153 | 1168 | 1171 | 1072 => Condition::Drizzle,
        1180..=1201 | 1240..=1246 => Condition::Rain,
        1066 | 1069 | 1114 | 1117 | 1204..=1237 | 1249..=1264 => Condition::Snow,
        1087 | 1273..=1282 => Condition::Thunderstorm,
        _ => Condition::Cloudy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{"location":{"name":"X"},
        "current":{"temp_c":-2.4,"is_day":0,"condition":{"text":"Light snow","code":1213}},
        "forecast":{"forecastday":[{"day":{"maxtemp_c":1.5,"mintemp_c":-5.0}}]}}"#;

    #[test]
    fn parses_answer() {
        let r = parse(BODY).unwrap();
        assert_eq!(r.condition, Condition::Snow);
        assert!(!r.is_day);
        assert_eq!((r.current_c, r.max_c, r.min_c), (-2.4, 1.5, -5.0));
    }

    #[test]
    fn codes() {
        assert_eq!(code_condition(1000), Condition::Clear);
        assert_eq!(code_condition(1009), Condition::Overcast);
        assert_eq!(code_condition(1153), Condition::Drizzle);
        assert_eq!(code_condition(1195), Condition::Rain);
        assert_eq!(code_condition(1258), Condition::Snow);
        assert_eq!(code_condition(1276), Condition::Thunderstorm);
        assert_eq!(code_condition(1087), Condition::Thunderstorm);
    }
}
