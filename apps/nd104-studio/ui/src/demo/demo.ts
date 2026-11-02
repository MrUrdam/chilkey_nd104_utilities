// Demo mode: the settings window runs in a plain browser with sample data,
// without the Tauri app and without a keyboard. Used for the README
// screenshots (scripts/screenshots.mjs). Only included in `--mode demo` builds.
//
// URL parameters: ?lang=en|de  ?theme=dark|light|auto

import { mockIPC } from "@tauri-apps/api/mocks";
import type { AppInfo, Config, FirmwareReport, Keymap, KeymapProfile, Status, WeatherReport } from "../lib/api";
import { layout } from "../lib/keyboard";
import en from "../../../locales/en.json";
import de from "../../../locales/de.json";

const params = new URLSearchParams(location.search);
const language = params.get("lang") === "de" ? "de" : "en";
const theme = params.get("theme") ?? "auto";
const style = params.get("style") === "gamer" ? "gamer" : "calm";

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
  general: { language, theme, style, autostart: true },
  device: { model: "chilkey-nd104", poll_interval_s: 3, openrgb_server: true, openrgb_port: 6743 },
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
    // ?conn=bluetooth / ?conn=wireless: connected without the cable
    connection: params.get("conn") === "bluetooth" ? "bluetooth" : params.get("conn") === "wireless" ? "wireless" : "usb",
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
    { at: iso(-31), level: "info", message: "ND104 Studio is up to date" },
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
    config_path: "C:\\Users\\demo\\AppData\\Roaming\\nd104-studio\\config.toml",
    log_dir: "C:\\Users\\demo\\AppData\\Local\\nd104-studio\\logs",
    project_url: "https://github.com/MrUrdam/chilkey_nd104_utilities",
    releases_url: "https://github.com/MrUrdam/chilkey_nd104_utilities/releases",
    changelog_url: "https://github.com/MrUrdam/chilkey_nd104_utilities/blob/main/apps/nd104-studio/CHANGELOG.md",
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

/** A short animated sunset loop drawn on a canvas, as if picked from disk. */
function demoPicture() {
  const W = 320;
  const H = 172;
  const FRAMES = 12;
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const g = canvas.getContext("2d")!;
  const frames: string[] = [];
  for (let f = 0; f < FRAMES; f++) {
    const sky = g.createLinearGradient(0, 0, 0, H * 0.62);
    sky.addColorStop(0, "#1b1435");
    sky.addColorStop(0.6, "#7a2e6b");
    sky.addColorStop(1, "#f08a4b");
    g.fillStyle = sky;
    g.fillRect(0, 0, W, H);
    // sun with bands that slide down
    const cx = W / 2;
    const cy = H * 0.62;
    const sun = g.createLinearGradient(0, cy - 52, 0, cy);
    sun.addColorStop(0, "#ffd36b");
    sun.addColorStop(1, "#ff6f61");
    g.save();
    g.beginPath();
    g.arc(cx, cy, 52, Math.PI, 0);
    g.clip();
    g.fillStyle = sun;
    g.fillRect(cx - 52, cy - 52, 104, 52);
    g.fillStyle = "#7a2e6b";
    for (let i = 0; i < 6; i++) {
      const y = cy - 30 + ((i * 9 + f * 0.75) % 30);
      g.fillRect(cx - 52, y, 104, 1 + i * 0.5);
    }
    g.restore();
    // ground with a moving grid
    g.fillStyle = "#120d24";
    g.fillRect(0, cy, W, H - cy);
    g.strokeStyle = "rgba(255, 140, 120, 0.55)";
    g.lineWidth = 1;
    for (let i = 0; i < 8; i++) {
      const t = (i + f / FRAMES) / 8;
      const y = cy + (H - cy) * t * t;
      g.beginPath();
      g.moveTo(0, y);
      g.lineTo(W, y);
      g.stroke();
    }
    for (let x = -10; x <= 10; x++) {
      g.beginPath();
      g.moveTo(cx + x * 6, cy);
      g.lineTo(cx + x * 40, H);
      g.stroke();
    }
    frames.push(canvas.toDataURL("image/jpeg", 0.85));
  }
  const jpegBytes = frames.reduce((sum, url) => sum + Math.round(((url.length - 23) * 3) / 4), 0);
  return {
    name: "sunset-loop.gif",
    file_bytes: 486_312,
    spec: { width: 320, height: 172, max_frames: 59, max_bytes: 2_097_152, max_file_bytes: 2_097_152 },
    problem: null,
    format: "GIF",
    width: 480,
    height: 258,
    frames: FRAMES,
    payload_bytes: 8 + FRAMES * 5 + jpegBytes,
    notes: [{ code: "scaled", width: 480, height: 258 }],
    preview_frames: frames,
    delays_ms: Array(FRAMES).fill(90),
  };
}

/** A plausible keymap: layer 0 as printed, the Fn layers as Chilkey ships them. */
function demoKeymap(): Keymap {
  const layers = Array.from({ length: 6 }, (_, n) => Array<number>(128).fill(n === 0 ? 0 : 1));
  for (const key of layout("iso")) for (const [r, c] of key.pos) layers[0][r * 16 + c] = key.hid;
  layers[0][6 * 16 + 9] = 0xa8; // knob press: mute
  layers[0][7 * 16 + 9] = 0x7e35;
  layers[0][4 * 16 + 1] = 0x1c; // German QWERTZ habits: Y and Z swapped
  layers[0][2 * 16 + 6] = 0x1d;
  // Fn and Fn + Shift layers as Chilkey ships them: [layer, slot, code]
  const factory: [number, number, number][] = [
    [1, 33, 0x7e04], [1, 34, 0x7e05], [1, 42, 0x7823], [1, 43, 0x7825], [1, 44, 0x7e0e],
    [1, 48, 0x7e07], [1, 58, 0x7824], [1, 59, 0x7826], [1, 60, 0x7e17], [1, 65, 0x7e09],
    [1, 66, 0x7e0a], [1, 67, 0x7e0b], [1, 68, 0x7e0d], [1, 70, 0x7013], [1, 71, 0x7e14],
    [1, 74, 0x7e06], [1, 75, 0x5222], [1, 76, 0x7827], [1, 81, 0x7e0c], [1, 83, 0x7e08],
    [1, 84, 0x7e08], [1, 85, 0x7e08], [1, 86, 0x7e08], [1, 92, 0x7828], [1, 93, 0x7829],
    [1, 100, 0x7e0f], [1, 102, 0x7e10], [1, 105, 0x7e0e], [1, 106, 0x7821], [1, 107, 0x7820],
    [1, 108, 0x782a], [1, 120, 0x5222], [1, 121, 0x7e22], [1, 124, 0x7e21], [2, 42, 0x7823],
    [2, 43, 0x7825], [2, 44, 0x7e18], [2, 54, 0x7e15], [2, 57, 0x7e23], [2, 58, 0x7824],
    [2, 59, 0x7826], [2, 60, 0x7e16], [2, 75, 0x5222], [2, 76, 0x7827], [2, 92, 0x7828],
    [2, 93, 0x7829], [2, 100, 0x7e0f], [2, 102, 0x7e10], [2, 105, 0x7e18], [2, 106, 0x7821],
    [2, 107, 0x7820], [2, 108, 0x782a], [2, 120, 0x5222], [2, 121, 0x7e22],
  ];
  for (const [l, i, code] of factory) layers[l][i] = code;
  const knob = [
    [0xaa, 0xa9],
    [0x7e10, 0x7e0f],
    [0x7828, 0x7827],
    [0x7828, 0x7827],
  ];
  return { layers, knob };
}

let demoKeys = demoKeymap();
const demoProfiles: KeymapProfile[] = [
  { id: "p1", name: "Gaming", app: "Steam", created: "2026-10-28T19:40:00Z" },
  { id: "p2", name: "Photo editing", app: "Lightroom", created: "2026-10-21T08:05:00Z" },
];
let demoPage: unknown = null;
let demoSignalRgb = {
  signalrgb_found: true,
  installed: false,
  current: false,
  folder: String.raw`C:\Users\Demo\Documents\WhirlwindFX\Plugins`,
};
const demoKeyboardSettings = { debounce_mode: 1, debounce_time: 5, sleep_time: 10, deep_sleep_time: 30, pc_lighting: 1 };

export function installDemo(version: string) {
  const texts: Record<string, string> = { ...en, ...(language === "de" ? de : {}) };

  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "third_party_notices":
        return "ND104 Studio – third-party notices\n\nRust crates\n-----------\nserde 1.0 – MIT OR Apache-2.0 – https://crates.io/crates/serde\n\nLicence texts\n-------------\n(demo)";
      case "app_info":
        return appInfo(version);
      case "texts":
        return texts;
      case "get_status":
        // ?usb=0: no cable (e.g. together with ?bt=1)
        return params.get("usb") === "0" ? { ...status, device: { kind: "not_connected" } } : status;
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
      case "light_read":
        // key lighting: solid amber; bottom lighting: a rainbow effect
        return a.zone === "bottom"
          ? { effect: 9, brightness: 170, speed: 190, hue: 170, sat: 255 }
          : { effect: 1, brightness: 210, speed: 128, hue: 24, sat: 210 };
      case "picture_pick":
      case "picture_refit":
        return demoPicture();
      case "picture_upload":
        return null;
      case "keymap_backup":
        return { path: "C:\\Users\\demo\\Documents\\nd104-keymap-2026-11-01.json", layers: 6, assigned_keys: 214 };
      case "keymap_restore_check":
        return {
          name: "nd104-keymap-2026-10-30.json",
          created: "2026-10-30T18:12:00Z",
          backup_firmware: "V3.0.14 20260710_123331",
          keyboard_firmware: "V3.0.14 20260710_123331",
          keys: 12,
          knob: 2,
        };
      case "keymap_restore_apply":
        return 14;
      case "keymap_read":
        return structuredClone(demoKeys);
      case "keymap_write": {
        const target = a.target as Keymap;
        let n = 0;
        target.layers.forEach((l, i) => l.forEach((c, j) => (n += c !== demoKeys.layers[i][j] ? 1 : 0)));
        target.knob.forEach((l, i) => l.forEach((c, j) => (n += c !== demoKeys.knob[i][j] ? 1 : 0)));
        demoKeys = structuredClone(target);
        return new Promise((resolve) => setTimeout(() => resolve(n), 700));
      }
      case "openrgb_status":
        return { running: true, port: 6743, clients: 1, offered: true, error: null };
      case "signalrgb_status":
        return { ...demoSignalRgb };
      case "signalrgb_install":
        demoSignalRgb = { ...demoSignalRgb, installed: true, current: true };
        return new Promise((resolve) => setTimeout(() => resolve({ ...demoSignalRgb }), 300));
      case "signalrgb_remove":
        demoSignalRgb = { ...demoSignalRgb, installed: false, current: false };
        return { ...demoSignalRgb };
      case "bluetooth_status":
        // ?bt=1: keyboard connected over Bluetooth instead of the cable
        return params.get("bt") === "1" || params.get("conn") === "bluetooth"
          ? { name: "ND104 BT2", slot: 2, battery: 87 }
          : null;
      case "keyboard_settings_read":
        return { ...demoKeyboardSettings };
      case "keyboard_settings_write":
        (demoKeyboardSettings as Record<string, number>)[String(a.setting)] = Number(a.value);
        return new Promise((resolve) => setTimeout(() => resolve({ ...demoKeyboardSettings }), 300));
      case "keymap_page_get":
        return demoPage;
      case "keymap_page_set":
        demoPage = a.settings;
        return null;
      case "keymap_profiles":
        return demoProfiles;
      case "keymap_profile_get": {
        const k = demoKeymap();
        k.layers[0][3 * 16 + 0] = 0xe0; // profile: Caps Lock → Ctrl, Win key off
        k.layers[0][5 * 16 + 1] = 0;
        return k;
      }
      case "keymap_profile_save": {
        const p = { id: `p${Date.now()}`, name: String(a.name), app: String(a.application), created: new Date().toISOString() };
        demoProfiles.push(p);
        return p;
      }
      case "keymap_profile_delete":
        demoProfiles.splice(demoProfiles.findIndex((p) => p.id === a.id), 1);
        return null;
      case "keymap_profile_export":
        return "C:\\Users\\demo\\Documents\\nd104-profile-Gaming.json";
      case "keymap_profile_import":
        return null;
      case "plugin:event|listen":
        return 1;
      default:
        // set_api_key, delete_api_key, engine_command, open_target, plugin:event|unlisten …
        return null;
    }
  });
}
