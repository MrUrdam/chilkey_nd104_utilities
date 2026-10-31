//! City search via the free Open-Meteo geocoding API.

use anyhow::{Context, Result};
use serde::Deserialize;

use super::Location;

const URL: &str = "https://geocoding-api.open-meteo.com/v1/search";

/// One search hit.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub country: Option<String>,
    /// State / region.
    #[serde(default)]
    pub admin1: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
}

impl Place {
    /// Label like `Berlin, State of Berlin, Germany`.
    pub fn label(&self) -> String {
        [
            Some(self.name.as_str()),
            self.admin1.as_deref(),
            self.country.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
    }

    pub fn to_location(&self) -> Location {
        Location {
            name: self.label(),
            lat: self.latitude,
            lon: self.longitude,
        }
    }
}

/// Searches places by name. `language` e.g. `en` or `de`.
pub async fn search(query: &str, language: &str) -> Result<Vec<Place>> {
    let body = super::new_http_client()?
        .get(URL)
        .query(&[
            ("name", query),
            ("count", "10"),
            ("language", language),
            ("format", "json"),
        ])
        .send()
        .await
        .context("geocoding request failed")?
        .error_for_status()?
        .text()
        .await?;
    parse(&body)
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    results: Vec<Place>,
}

fn parse(body: &str) -> Result<Vec<Place>> {
    Ok(serde_json::from_str::<Response>(body)
        .context("unexpected geocoding answer")?
        .results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_results() {
        let body = r#"{"results":[{"id":2950159,"name":"Berlin","latitude":52.52437,
            "longitude":13.41053,"country":"Germany","admin1":"State of Berlin",
            "timezone":"Europe/Berlin"}],"generationtime_ms":0.5}"#;
        let places = parse(body).unwrap();
        assert_eq!(places.len(), 1);
        assert_eq!(places[0].label(), "Berlin, State of Berlin, Germany");
        assert_eq!(places[0].to_location().lat, 52.52437);
    }

    #[test]
    fn no_results() {
        assert!(parse(r#"{"generationtime_ms":0.5}"#).unwrap().is_empty());
    }
}
