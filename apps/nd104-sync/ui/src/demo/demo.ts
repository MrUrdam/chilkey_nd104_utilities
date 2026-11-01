// Demo mode: the settings window runs in a plain browser with sample data,
// without the Tauri app and without a keyboard. Used for the README
// screenshots (scripts/screenshots.mjs). Only included in `--mode demo` builds.
//
// URL parameters: ?lang=en|de  ?theme=dark|light|auto

import { mockIPC } from "@tauri-apps/api/mocks";
import type { AppInfo, Config, FirmwareReport, Status, WeatherReport } from "../lib/api";
import en from "../../../locales/en.json";
import de from "../../../locales/de.json";

const params = new URLSearchParams(location.search);
const language = params.get("lang") === "de" ? "de" : "en";
const theme = params.get("theme") ?? "auto";

const now = Date.now();
const iso = (offsetMin: number) => new Date(now + offsetMin * 60_000).toISOString();

/** Wall-clock time without zone, as the engine reports `last_sent_time`. */
function civil(offsetMin: number): string {
  const d = new Date(now + offsetMin * 60_000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** Next last-Sunday-of-March/October 01:00 UTC change (EU rules). */
function nextDstChange(): string {
  const lastSunday = (year: number, month: number) => {
    const d = new Date(Date.UTC(year, month + 1, 0, 1));
    d.setUTCDate(d.getUTCDate() - d.getUTCDay());
    return d;
  };
  const year = new Date(now).getUTCFullYear();
  const candidates = [lastSunday(year, 2), lastSunday(year, 9), lastSunday(year + 1, 2)];
  return candidates.find((d) => d.getTime() > now)!.toISOString();
}

const config: Config = {
  schema: 1,
  general: { language, theme, autostart: true },
  device: { model: "chilkey-nd104", poll_interval_s: 3 },
  clock: {
    source: "timezone",
    timezone: "Europe/Berlin",
    dst_mode: "auto",
    ntp_server: "pool.ntp.org",
    manual_offset_min: 0,
    sync_interval_min: 60,
  },
  weather: {
    provider: "open-meteo",
    fallback_provider: "brightsky",
    interval_min: 15,
    ui_unit: "celsius",
    location: { mode: "search", name: "Berlin, Germany", lat: 52.52437, lon: 13.41053 },
    manual: { condition: "partly_cloudy", is_day: true, current_c: 12, max_c: 14, min_c: 6 },
  },
  updates: { firmware_check: true, app_check: true, check_interval_h: 24 },
};

const weather: WeatherReport = {
  condition: "clear",
  is_day: true,
  current_c: 26.4,
  max_c: 28.9,
  min_c: 17.2,
  observed_at: iso(-4),
  provider: "Open-Meteo",
};

const firmwareReport: FirmwareReport = {
  model_name: "Chilkey ND104",
  list_updated: "2026-10-31",
  instructions_url: "https://chilkey.com/pages/nd104_firmware_flashing_guide",
  parts: [
    {
      role: "keyboard",
      label: "Keyboard",
      installed: "3.0.14",
      installed_build: "2026-07-10",
      latest: { version: "3.0.14", build_date: "2026-07-10", file: "VN001_314A108_1V0_RGB_TYPE1_20260710.bin" },
      download_url: "https://drive.google.com/drive/folders/1bBRxuqoCmtKGLFP1ONK2jDnFGpARMcl1",
      source: "official",
      state: "up_to_date",
      official_error: null,
    },
    {
      role: "screen",
      label: "Display 24 h / °C",
      installed: "1.2.4",
      installed_build: "2026-03-17",
      latest: {
        version: "1.2.4",
        build_date: "2026-03-17",
        file: "ND104_SCREEN_MCU_320_172_V1.0.0_20260317.bin",
        ui_file: "ND104_SCREEN_UI_320_172_V1.0.0_20251104.bin",
      },
      download_url: "https://drive.google.com/drive/folders/1E7kkBnX99qMY9dFOu-0rNqluetyRrFCO",
      source: "official",
      state: "up_to_date",
      official_error: null,
    },
  ],
};

const status: Status = {
  device: {
    kind: "ready",
    model_id: "chilkey-nd104",
    model_name: "Chilkey ND104",
    firmware: [
      { role: "keyboard", raw: "V3.0.14 20260710_123331", version: "3.0.14", build: "20260710_123331" },
      { role: "screen", raw: "V1.2.4 Mar 17 2026 21:37:00", version: "1.2.4", build: "Mar 17 2026 21:37:00" },
    ],
    trust: "verified",
    screen_variant: "celsius24h",
  },
  paused: false,
  last_time_sync: iso(-2),
  last_sent_time: civil(-2),
  next_time_sync: iso(58),
  next_dst_change: nextDstChange(),
  weather,
  weather_stale: false,
  weather_error: null,
  last_weather_push: iso(-4),
  next_weather_fetch: iso(11),
  app_update: { Ok: { kind: "up_to_date", latest: null } },
  app_update_checked: iso(-30),
  firmware_update: { Ok: firmwareReport },
  firmware_update_checked: iso(-30),
  // Messages as the engine writes them (crates/nd104-core/src/engine.rs, device.rs).
  events: [
    { at: iso(-31), level: "info", message: "nd104-sync is up to date" },
    {
      at: iso(-32),
      level: "info",
      message: "Chilkey ND104 connected, firmware verified (keyboard V3.0.14 20260710_123331, screen V1.2.4 Mar 17 2026 21:37:00)",
    },
  ],
};

function appInfo(version: string): AppInfo {
  return {
    version,
    language,
    languages: [
      ["de", "Deutsch"],
      ["en", "English"],
    ],
    config_path: "C:\\Users\\demo\\AppData\\Roaming\\nd104-sync\\config.toml",
    log_dir: "C:\\Users\\demo\\AppData\\Local\\nd104-sync\\logs",
    project_url: "https://github.com/MrUrdam/chilkey_nd104_utilities",
    releases_url: "https://github.com/MrUrdam/chilkey_nd104_utilities/releases",
    changelog_url: "https://github.com/MrUrdam/chilkey_nd104_utilities/blob/main/apps/nd104-sync/CHANGELOG.md",
    providers: [
      { id: "open-meteo", name: "Open-Meteo", requires_api_key: false, coverage: "worldwide", signup_url: null },
      { id: "brightsky", name: "Bright Sky (DWD)", requires_api_key: false, coverage: "germany", signup_url: null },
      {
        id: "openweathermap",
        name: "OpenWeatherMap",
        requires_api_key: true,
        coverage: "worldwide",
        signup_url: "https://home.openweathermap.org/users/sign_up",
      },
      {
        id: "weatherapi",
        name: "WeatherAPI.com",
        requires_api_key: true,
        coverage: "worldwide",
        signup_url: "https://www.weatherapi.com/signup.aspx",
      },
      { id: "manual", name: "Manual / Test", requires_api_key: false, coverage: "none", signup_url: null },
    ],
    models: [
      {
        id: "chilkey-nd104",
        name: "Chilkey ND104",
        verified_firmware: [
          { role: "keyboard", version: "3.0.14" },
          { role: "screen", version: "1.2.4" },
        ],
      },
    ],
    min_weather_interval: 5,
    self_update: false,
  };
}

const TIMEZONES = [
  "America/New_York",
  "Asia/Tokyo",
  "Australia/Sydney",
  "Europe/Berlin",
  "Europe/London",
  "Europe/Vienna",
  "Europe/Zurich",
  "UTC",
];

export function installDemo(version: string) {
  const texts: Record<string, string> = { ...en, ...(language === "de" ? de : {}) };

  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "app_info":
        return appInfo(version);
      case "texts":
        return texts;
      case "get_status":
        return status;
      case "get_config":
        return structuredClone(config);
      case "save_config":
        Object.assign(config, a.config);
        return null;
      case "search_places":
        return [
          { name: "Berlin", latitude: 52.52437, longitude: 13.41053, country: "Germany", admin1: "Berlin", timezone: "Europe/Berlin" },
        ];
      case "test_weather":
        return weather;
      case "api_key_status":
        return { openweathermap: false, weatherapi: false };
      case "test_ntp":
        return 23;
      case "preview_time":
        return { local: civil(0), next_dst_change: nextDstChange() };
      case "timezones":
        return TIMEZONES;
      case "plugin:event|listen":
        return 1;
      default:
        // set_api_key, delete_api_key, engine_command, open_target, plugin:event|unlisten …
        return null;
    }
  });
}
