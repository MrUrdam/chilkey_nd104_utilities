// Credits the data providers ask for (their terms, checked 2026-11-02). The
// links are also on the app's list of addresses it may open
// (src-tauri/src/commands.rs, CREDIT_LINKS).

export interface Credit {
  /** Text to show, e.g. "Weather data by Open-Meteo.com". */
  text: string;
  /** Licence of the data, if any. */
  license?: string;
  url: string;
}

/** Weather data, by provider id. */
export const WEATHER_CREDITS: Record<string, Credit> = {
  "open-meteo": { text: "Weather data by Open-Meteo.com", license: "CC BY 4.0", url: "https://open-meteo.com/" },
  brightsky: {
    text: "Source: Deutscher Wetterdienst, via Bright Sky",
    license: "CC BY 4.0",
    url: "https://brightsky.dev/",
  },
  openweathermap: { text: "Weather data © OpenWeather", license: "ODbL", url: "https://openweathermap.org/" },
  weatherapi: { text: "Powered by WeatherAPI.com", url: "https://www.weatherapi.com/" },
};

/** Place search (Open-Meteo geocoding, based on GeoNames). */
export const PLACE_CREDIT: Credit = {
  text: "Place names: GeoNames, via Open-Meteo",
  license: "CC BY 4.0",
  url: "https://www.geonames.org/",
};

/** Licences the credits refer to. */
export const LICENSE_URLS: Record<string, string> = {
  "CC BY 4.0": "https://creativecommons.org/licenses/by/4.0/",
  ODbL: "https://opendatacommons.org/licenses/odbl/",
};
