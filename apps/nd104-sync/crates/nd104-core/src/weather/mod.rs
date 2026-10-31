//! Neutral weather data and providers (PLAN.md § 5.6).
//!
//! Providers return a neutral [`WeatherReport`]; each keyboard model maps the
//! [`Condition`] to its own icon set. Adding a provider = one module that
//! implements [`WeatherProvider`] plus an entry in [`create`] / [`PROVIDERS`].

mod bright_sky;
pub mod geocoding;
mod manual;
mod open_meteo;
mod openweathermap;
mod weatherapi;

use std::future::Future;
use std::pin::Pin;

use anyhow::{bail, Context, Result};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

pub use bright_sky::BrightSky;
pub use manual::ManualProvider;
pub use open_meteo::OpenMeteo;
pub use openweathermap::OpenWeatherMap;
pub use weatherapi::WeatherApi;

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

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocationSettings {
    pub mode: LocationMode,
    pub name: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

/// Berlin until the user picks a location.
impl Default for LocationSettings {
    fn default() -> Self {
        Self {
            mode: LocationMode::Search,
            name: "Berlin, Germany".into(),
            lat: Some(52.52),
            lon: Some(13.405),
        }
    }
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

/// Provider facts for the UI and the docs.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub requires_api_key: bool,
    /// Where the provider has data, e.g. `worldwide` or `germany`.
    pub coverage: &'static str,
    /// Where to get an API key (key-based providers).
    pub signup_url: Option<&'static str>,
}

/// All providers, in the order shown in the UI.
pub const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: OpenMeteo::ID,
        name: "Open-Meteo",
        requires_api_key: false,
        coverage: "worldwide",
        signup_url: None,
    },
    ProviderInfo {
        id: BrightSky::ID,
        name: "Bright Sky (DWD)",
        requires_api_key: false,
        coverage: "germany",
        signup_url: None,
    },
    ProviderInfo {
        id: OpenWeatherMap::ID,
        name: "OpenWeatherMap",
        requires_api_key: true,
        coverage: "worldwide",
        signup_url: Some("https://home.openweathermap.org/users/sign_up"),
    },
    ProviderInfo {
        id: WeatherApi::ID,
        name: "WeatherAPI.com",
        requires_api_key: true,
        coverage: "worldwide",
        signup_url: Some("https://www.weatherapi.com/signup.aspx"),
    },
    ProviderInfo {
        id: ManualProvider::ID,
        name: "Manual / Test",
        requires_api_key: false,
        coverage: "none",
        signup_url: None,
    },
];

pub fn info(id: &str) -> Option<&'static ProviderInfo> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// Creates a provider by id.
pub fn create(id: &str) -> Option<Box<dyn WeatherProvider>> {
    match id {
        OpenMeteo::ID => Some(Box::new(OpenMeteo::default())),
        BrightSky::ID => Some(Box::new(BrightSky)),
        OpenWeatherMap::ID => Some(Box::new(OpenWeatherMap)),
        WeatherApi::ID => Some(Box::new(WeatherApi)),
        ManualProvider::ID => Some(Box::new(ManualProvider)),
        _ => None,
    }
}

/// HTTP client used for all weather/geocoding requests.
pub type HttpClient = reqwest::Client;

/// Drops the request URL from an HTTP error: it contains the user's
/// coordinates, search text or API key, and logs may end up in bug reports.
pub(crate) fn without_url(e: reqwest::Error) -> reqwest::Error {
    e.without_url()
}

/// Sends a request and returns the body. Rejected keys and other HTTP errors
/// become readable messages; the URL (with key/coordinates) is never included.
pub(crate) async fn fetch_text(request: reqwest::RequestBuilder, provider: &str) -> Result<String> {
    let response = request
        .send()
        .await
        .map_err(without_url)
        .with_context(|| format!("{provider} request failed"))?;
    let status = response.status();
    let body = response.text().await.map_err(without_url)?;
    match status.as_u16() {
        200..=299 => Ok(body),
        401 | 403 => bail!(
            "{provider} rejected the API key (HTTP {status}). Check the key – new keys can take a while to activate."
        ),
        429 => bail!("{provider}: too many requests (HTTP 429) – increase the update interval"),
        _ => bail!("{provider} returned HTTP {status}"),
    }
}

/// Builds the request context for `provider`. Key-based providers get `key`
/// if given (e.g. a key typed into the window but not saved yet), otherwise the
/// key from the credential store.
pub fn context(
    provider: &dyn WeatherProvider,
    settings: &WeatherSettings,
    client: HttpClient,
    key: Option<String>,
) -> Result<FetchContext> {
    let api_key = match key {
        Some(k) if !k.trim().is_empty() => Some(k.trim().to_string()),
        _ if provider.requires_api_key() => crate::secrets::get(provider.id())?,
        _ => None,
    };
    Ok(FetchContext {
        client,
        location: settings.location.location(),
        api_key,
        settings: settings.clone(),
    })
}

/// The API key from the context, or an error naming the provider.
pub(crate) fn require_key<'a>(ctx: &'a FetchContext, provider: &str) -> Result<&'a str> {
    ctx.api_key
        .as_deref()
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("no API key stored for {provider} (Weather → API key)"))
}

pub(crate) fn require_location(ctx: &FetchContext) -> Result<&Location> {
    ctx.location
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no location configured"))
}

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
        for info in PROVIDERS {
            let p = create(info.id).unwrap();
            assert_eq!(p.id(), info.id);
            assert_eq!(p.requires_api_key(), info.requires_api_key);
            assert_eq!(info.requires_api_key, info.signup_url.is_some());
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
