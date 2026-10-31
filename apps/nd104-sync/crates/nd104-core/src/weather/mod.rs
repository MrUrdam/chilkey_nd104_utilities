//! Neutral weather data and providers (PLAN.md § 5.6).
//!
//! Providers return a neutral [`WeatherReport`]; each keyboard model maps the
//! [`Condition`] to its own icon set. Adding a provider = one module that
//! implements [`WeatherProvider`] plus an entry in [`create`] / [`PROVIDERS`].

pub mod geocoding;
mod manual;
mod open_meteo;

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

pub use manual::ManualProvider;
pub use open_meteo::OpenMeteo;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Weather condition independent of provider and keyboard model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    #[default]
    Clear,
    PartlyCloudy,
    Cloudy,
    Overcast,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Thunderstorm,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeatherReport {
    pub condition: Condition,
    /// Day or night at the location (for day/night icons).
    pub is_day: bool,
    pub current_c: f32,
    pub max_c: f32,
    pub min_c: f32,
    pub observed_at: Timestamp,
    /// Provider id that delivered the report.
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LocationMode {
    /// Found via city search; name + coordinates stored.
    #[default]
    Search,
    /// Coordinates entered by hand.
    Coordinates,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LocationSettings {
    pub mode: LocationMode,
    pub name: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

impl LocationSettings {
    pub fn location(&self) -> Option<Location> {
        Some(Location {
            name: self.name.clone(),
            lat: self.lat?,
            lon: self.lon?,
        })
    }
}

/// Values for the manual/test provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManualWeather {
    pub condition: Condition,
    pub is_day: bool,
    pub current_c: f32,
    pub max_c: f32,
    pub min_c: f32,
}

impl Default for ManualWeather {
    fn default() -> Self {
        Self {
            condition: Condition::Clear,
            is_day: true,
            current_c: 20.0,
            max_c: 24.0,
            min_c: 12.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherSettings {
    pub provider: String,
    /// Tried when `provider` fails; empty = none.
    pub fallback_provider: String,
    pub interval_min: u32,
    /// Unit shown in the UI only; the display always gets °C.
    pub ui_unit: UiUnit,
    pub location: LocationSettings,
    pub manual: ManualWeather,
}

impl Default for WeatherSettings {
    fn default() -> Self {
        Self {
            provider: OpenMeteo::ID.into(),
            fallback_provider: String::new(),
            interval_min: 15,
            ui_unit: UiUnit::Celsius,
            location: LocationSettings::default(),
            manual: ManualWeather::default(),
        }
    }
}

impl WeatherSettings {
    /// Smallest allowed interval, to protect the free services.
    pub const MIN_INTERVAL_MIN: u32 = 5;

    pub fn interval_min(&self) -> u32 {
        self.interval_min.max(Self::MIN_INTERVAL_MIN)
    }
}

/// Everything a provider may need for one request.
pub struct FetchContext {
    pub client: HttpClient,
    pub location: Option<Location>,
    pub api_key: Option<String>,
    pub settings: WeatherSettings,
}

pub trait WeatherProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn requires_api_key(&self) -> bool;
    fn fetch<'a>(&'a self, ctx: &'a FetchContext) -> BoxFuture<'a, Result<WeatherReport>>;
}

/// Provider list for the UI: (id, name, needs API key).
pub const PROVIDERS: &[(&str, &str, bool)] = &[
    (OpenMeteo::ID, "Open-Meteo", false),
    (ManualProvider::ID, "Manual / Test", false),
];

/// Creates a provider by id.
pub fn create(id: &str) -> Option<Box<dyn WeatherProvider>> {
    match id {
        OpenMeteo::ID => Some(Box::new(OpenMeteo::default())),
        ManualProvider::ID => Some(Box::new(ManualProvider)),
        _ => None,
    }
}

/// HTTP client used for all weather/geocoding requests.
pub type HttpClient = reqwest::Client;

pub fn new_http_client() -> Result<HttpClient> {
    Ok(reqwest::Client::builder()
        .user_agent(crate::user_agent())
        .timeout(std::time::Duration::from_secs(15))
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_creates_every_listed_provider() {
        for (id, _, _) in PROVIDERS {
            let p = create(id).unwrap();
            assert_eq!(p.id(), *id);
        }
        assert!(create("nope").is_none());
    }

    #[test]
    fn interval_has_a_minimum() {
        let s = WeatherSettings {
            interval_min: 1,
            ..Default::default()
        };
        assert_eq!(s.interval_min(), 5);
    }
}
