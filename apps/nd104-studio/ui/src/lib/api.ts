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
  general: { language: string; theme: string; style: "calm" | "gamer"; autostart: boolean };
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
  setup_url: string | null;
  checksums_url: string | null;
}

export type UpdateStatus =
  | { kind: "up_to_date"; latest: Release | null }
  | ({ kind: "available" } & Release);

export type FirmwarePartState = "up_to_date" | "update_available" | "newer" | "unknown";

export interface FirmwarePart {
  role: "keyboard" | "screen";
  label: string | null;
  installed: string;
  installed_build: string | null;
  latest: { version: string | null; build_date: string; file: string; ui_file?: string | null } | null;
  download_url: string | null;
  source: "official" | "repository" | null;
  state: FirmwarePartState;
  official_error: string | null;
}

export interface FirmwareReport {
  model_name: string;
  list_updated: string | null;
  instructions_url: string;
  parts: FirmwarePart[];
}

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
  firmware_update: { Ok: FirmwareReport } | { Err: string } | null;
  firmware_update_checked: string | null;
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
  self_update: boolean;
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

export type LightZone = "per_key" | "bottom";

export interface ZoneState {
  effect: number | null;
  brightness: number | null;
  speed: number | null;
  hue: number | null;
  sat: number | null;
}

export type LightChange =
  | { field: "brightness"; value: number }
  | { field: "effect"; value: number }
  | { field: "speed"; value: number }
  | { field: "color"; hue: number; sat: number };

export type Fit = "fill" | "contain";

export type PictureProblem =
  | { code: "file_too_large"; bytes: number; max: number }
  | { code: "unsupported_format" }
  | { code: "damaged"; detail: string }
  | { code: "too_many_frames"; frames: number; max: number }
  | { code: "too_large_after_conversion"; bytes: number; max: number };

export type PictureNote =
  | { code: "scaled"; width: number; height: number }
  | { code: "enlarged" }
  | { code: "cropped" }
  | { code: "bars" }
  | { code: "transparency" }
  | { code: "delay_adjusted" }
  | { code: "reduced_quality" };

export interface PictureSpec {
  width: number;
  height: number;
  max_frames: number;
  max_bytes: number;
  max_file_bytes: number;
}

export interface PictureCheck {
  name: string;
  file_bytes: number;
  spec: PictureSpec;
  problem: PictureProblem | null;
  format: string | null;
  width: number;
  height: number;
  frames: number;
  payload_bytes: number;
  notes: PictureNote[];
  /** Frames as sent (JPEG data URLs) and their display times. */
  preview_frames: string[];
  delays_ms: number[];
}

export type EngineAction = "sync_now" | "pause" | "resume" | "recheck" | "check_updates";
export type OpenTarget = "config" | "logs" | string;

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  installUpdate: () => invoke<void>("install_update"),
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
  lightRead: (zone: LightZone) => invoke<ZoneState>("light_read", { zone }),
  lightApply: (zone: LightZone, changes: LightChange[], save: boolean) =>
    invoke<void>("light_apply", { zone, changes, save }),
  picturePick: (fit: Fit) => invoke<PictureCheck | null>("picture_pick", { fit }),
  pictureRefit: (fit: Fit) => invoke<PictureCheck>("picture_refit", { fit }),
  pictureUpload: () => invoke<void>("picture_upload"),
  keymapBackup: () =>
    invoke<{ path: string; layers: number; assigned_keys: number } | null>("keymap_backup"),
  keymapRestoreCheck: () => invoke<KeymapRestoreCheck | null>("keymap_restore_check"),
  keymapRestoreApply: () => invoke<number>("keymap_restore_apply"),
  keymapRestoreCancel: () => invoke<void>("keymap_restore_cancel"),
  keymapRead: () => invoke<Keymap>("keymap_read"),
  keyboardSettingsRead: () => invoke<KeyboardSettings>("keyboard_settings_read"),
  keyboardSettingsWrite: (setting: KeyboardSetting, value: number) =>
    invoke<KeyboardSettings>("keyboard_settings_write", { setting, value }),
  keymapWrite: (target: Keymap) => invoke<number>("keymap_write", { target }),
  keymapPageGet: () => invoke<unknown>("keymap_page_get"),
  keymapPageSet: (settings: unknown) => invoke<void>("keymap_page_set", { settings }),
  keymapProfiles: () => invoke<KeymapProfile[]>("keymap_profiles"),
  keymapProfileSave: (name: string, application: string, keymap: Keymap) =>
    invoke<KeymapProfile>("keymap_profile_save", { name, application, keymap }),
  keymapProfileGet: (id: string) => invoke<Keymap>("keymap_profile_get", { id }),
  keymapProfileDelete: (id: string) => invoke<void>("keymap_profile_delete", { id }),
  keymapProfileExport: (id: string) => invoke<string | null>("keymap_profile_export", { id }),
  keymapProfileImport: () => invoke<KeymapProfile | null>("keymap_profile_import"),
};

/** All layers (6 × 128 slots, row × 16 + column) and the knob (4 layers × left/right). */
export interface Keymap {
  layers: number[][];
  knob: number[][];
}

export type KeyboardSetting = "debounce_mode" | "debounce_time" | "sleep_time" | "deep_sleep_time";

/** Keyboard settings as read; `null`: not reported by this firmware. */
export interface KeyboardSettings {
  debounce_mode: number | null;
  debounce_time: number | null;
  sleep_time: number | null;
  deep_sleep_time: number | null;
}

export interface KeymapProfile {
  id: string;
  name: string;
  app: string;
  created: string;
}

export interface KeymapRestoreCheck {
  name: string;
  created: string;
  backup_firmware: string;
  keyboard_firmware: string;
  keys: number;
  knob: number;
}

export function onKeymapProgress(handler: (progress: number) => void): Promise<UnlistenFn> {
  return listen<number>("keymap-progress", (event) => handler(event.payload));
}

export function onPictureProgress(handler: (progress: number) => void): Promise<UnlistenFn> {
  return listen<number>("picture-progress", (event) => handler(event.payload));
}

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
