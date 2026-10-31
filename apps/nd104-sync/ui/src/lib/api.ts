// Types mirror the serde output of nd104-core / src-tauri; calls go through
// the Tauri commands in src-tauri/src/commands.rs.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Condition =
  | "clear"
  | "partly_cloudy"
  | "cloudy"
  | "overcast"
  | "fog"
  | "drizzle"
  | "rain"
  | "snow"
  | "thunderstorm";

export const CONDITIONS: Condition[] = [
  "clear",
  "partly_cloudy",
  "cloudy",
  "overcast",
  "fog",
  "drizzle",
  "rain",
  "snow",
  "thunderstorm",
];

export type ClockSource = "system" | "timezone" | "ntp" | "manual";
export type DstMode = "auto" | "force_summer" | "force_winter";

export interface Config {
  schema: number;
  general: { language: string; theme: string; autostart: boolean };
  device: { model: string; poll_interval_s: number };
  clock: {
    source: ClockSource;
    timezone: string;
    dst_mode: DstMode;
    ntp_server: string;
    manual_offset_min: number;
    sync_interval_min: number;
  };
  weather: WeatherSettings;
  updates: { firmware_check: boolean; app_check: boolean; check_interval_h: number };
}

export interface WeatherSettings {
  provider: string;
  fallback_provider: string;
  interval_min: number;
  ui_unit: "celsius" | "fahrenheit";
  location: { mode: "search" | "coordinates"; name: string; lat: number | null; lon: number | null };
  manual: { condition: Condition; is_day: boolean; current_c: number; max_c: number; min_c: number };
}

export interface FirmwareInfo {
  role: "keyboard" | "screen";
  raw: string;
  version: string;
  build: string;
}

export type DeviceState =
  | { kind: "not_connected" }
  | {
      kind: "ready";
      model_id: string;
      model_name: string;
      firmware: FirmwareInfo[];
      trust: "verified" | "compatible";
      screen_variant: "celsius24h" | "fahrenheit12h" | "unknown";
    }
  | { kind: "locked"; model_name: string | null; firmware: FirmwareInfo[]; reason: string };

export interface WeatherReport {
  condition: Condition;
  is_day: boolean;
  current_c: number;
  max_c: number;
  min_c: number;
  observed_at: string;
  provider: string;
}

export interface Release {
  version: string;
  tag: string;
  url: string;
  published_at: string | null;
}

export type UpdateStatus =
  | { kind: "up_to_date"; latest: Release | null }
  | ({ kind: "available" } & Release);

export interface AppEvent {
  at: string;
  level: "info" | "warning" | "error";
  message: string;
}

export interface Status {
  device: DeviceState;
  paused: boolean;
  last_time_sync: string | null;
  last_sent_time: string | null;
  next_time_sync: string | null;
  next_dst_change: string | null;
  weather: WeatherReport | null;
  weather_stale: boolean;
  weather_error: string | null;
  last_weather_push: string | null;
  next_weather_fetch: string | null;
  app_update: { Ok: UpdateStatus } | { Err: string } | null;
  app_update_checked: string | null;
  events: AppEvent[];
}

export interface AppInfo {
  version: string;
  language: string;
  languages: [string, string][];
  config_path: string;
  log_dir: string;
  project_url: string;
  releases_url: string;
  changelog_url: string;
  providers: ProviderInfo[];
  models: { id: string; name: string; verified_firmware: { role: string; version: string }[] }[];
  min_weather_interval: number;
}

export interface ProviderInfo {
  id: string;
  name: string;
  requires_api_key: boolean;
  coverage: "worldwide" | "germany" | "none";
  signup_url: string | null;
}

export interface Place {
  name: string;
  latitude: number;
  longitude: number;
  country: string | null;
  admin1: string | null;
  timezone: string | null;
}

export interface TimePreview {
  local: string;
  next_dst_change: string | null;
}

export type EngineAction = "sync_now" | "pause" | "resume" | "recheck" | "check_updates";
export type OpenTarget = "config" | "logs" | string;

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  texts: () => invoke<Record<string, string>>("texts"),
  status: () => invoke<Status>("get_status"),
  config: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  searchPlaces: (query: string, language: string) =>
    invoke<Place[]>("search_places", { query, language }),
  testWeather: (settings: WeatherSettings, apiKey: string | null = null) =>
    invoke<WeatherReport>("test_weather", { settings, apiKey }),
  apiKeyStatus: () => invoke<Record<string, boolean>>("api_key_status"),
  setApiKey: (provider: string, key: string) => invoke<void>("set_api_key", { provider, key }),
  deleteApiKey: (provider: string) => invoke<void>("delete_api_key", { provider }),
  testNtp: (server: string) => invoke<number>("test_ntp", { server }),
  previewTime: (clock: Config["clock"]) => invoke<TimePreview>("preview_time", { clock }),
  timezones: () => invoke<string[]>("timezones"),
  engine: (command: EngineAction) => invoke<void>("engine_command", { command }),
  open: (target: OpenTarget) => invoke<void>("open_target", { target }),
};

/** OpenStreetMap page with a marker, to check a location in the browser. */
export function osmUrl(lat: number, lon: number): string {
  const la = lat.toFixed(5);
  const lo = lon.toFixed(5);
  return `https://www.openstreetmap.org/?mlat=${la}&mlon=${lo}#map=13/${la}/${lo}`;
}

export function onStatus(handler: (status: Status) => void): Promise<UnlistenFn> {
  return listen<Status>("status", (event) => handler(event.payload));
}

export function onConfigChanged(handler: () => void): Promise<UnlistenFn> {
  return listen("config-changed", () => handler());
}
