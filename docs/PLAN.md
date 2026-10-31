# nd104-sync – Project Plan

> **Status:** M0–M4 done (beta 0.4.0), M5 next. Last update: 2026-10-31
>
> User documentation: [apps/nd104-sync/README.md](../apps/nd104-sync/README.md)

`nd104-sync` is a background tool that keeps the display of a Chilkey ND104
keyboard supplied with **current time and weather**. The keyboard has no
internet connection of its own – all information has to come from the PC the
keyboard is plugged into.

---

## 1. Ground rules

These rules are non-negotiable and shape every design decision below.

| # | Rule | Consequence |
|---|---|---|
| R1 | **Wired USB only.** | No 2.4 GHz dongle, no Bluetooth. Wireless mode is out of scope and is not tested. |
| R2 | **Windows only (for now).** | Windows 10/11 is the only supported platform. The architecture keeps all OS-specific code behind a platform layer so Linux and macOS *can* be added later without touching the core. |
| R3 | **Protocol safety first.** | The tool also works with older or newer firmware than the verified ones, but only as long as the protocol behaves exactly as expected. As soon as anything looks different (missing or malformed answers, changed device layout, rejected writes) it **stops sending immediately**, locks itself and tells the user. When in doubt: stop. |
| R4 | **Read-only towards firmware.** | The tool never flashes, patches or modifies firmware. It only *checks* whether updates exist and tells the user. |
| R5 | **Extensible by design.** | Weather providers, clock sources, languages and platforms are plug-in points (traits / files), not hard-coded switches. |
| R6 | **Privacy.** | No telemetry. Network access only to services the user configured (weather, geocoding, NTP) plus the firmware manifest for the update check. |
| R7 | **ND104 first, other models possible.** | Only the Chilkey ND104 is implemented. Everything model-specific (USB IDs, packets, icons, verified firmware, firmware sources) sits behind a model interface, so other keyboards with displays can be added later without touching the core, scheduler, providers or UI (§ 4.3). |

---

## 2. Decisions log

| Topic | Decision |
|---|---|
| Connection | Wired USB only (R1) |
| Language / stack | **Rust** + **Tauri v2** (native backend, web-based UI) |
| Platforms | Windows now; Linux/macOS prepared, not implemented (R2) |
| Weather providers | Start with key-less providers (Open-Meteo, Bright Sky); key-based ones later; provider list documented and extensible |
| UI languages | German + English; default = system language; switchable in tray menu and settings; more languages via additional locale files |
| UI style | Modern, gamer-inspired but simple; dark/light following Windows, overridable (Auto / Dark / Light) |
| Firmware | Verified versions → normal; other versions → allowed while protocol checks pass ("compatible, unverified"); any protocol deviation → stop + lock + notification; update check after boot and on demand |
| Plan location | This file (`docs/PLAN.md`) |
| Unknown but well-formed messages | Log and ignore; damaged/unknown structure or a bad acknowledgement → lock (§ 5.3) |
| Keyboard models | Model interface in the architecture; only the ND104 is implemented (R7, § 4.3) |

---

## 3. Tech stack

| Area | Choice | Why |
|---|---|---|
| Language | Rust (stable, MSVC toolchain on Windows) | Small single binary, low background footprint, strong typing for the byte-level protocol |
| App shell | **Tauri v2** | Built-in tray icon, window management, plugins for autostart / single instance / notifications; the UI can be styled freely with HTML/CSS |
| Frontend | Svelte + TypeScript (Vite) | Lightweight, simple component model, no heavy runtime |
| HID | `hidapi` crate | Cross-platform; no driver needed for vendor-defined HID on Windows |
| HTTP | `reqwest` (rustls) | Weather / geocoding / manifest requests |
| Async runtime | `tokio` (bundled with Tauri) | Scheduler, network, device polling |
| Time & time zones | `jiff` | IANA time-zone database incl. DST rules, also on Windows |
| NTP | `sntpc` (or equivalent) | Optional time source |
| Config | `serde` + `toml`, paths via `directories` | Human-readable config in the user's app-data folder |
| Secrets | `keyring` crate | API keys in Windows Credential Manager (Keychain / Secret Service later) |
| Logging | `tracing` + `tracing-appender` | Rolling log files |
| i18n | JSON locale files shared by backend (tray menu) and frontend; system language via `sys-locale` | One source of truth for translations |
| Tauri plugins | `autostart`, `single-instance`, `notification` | Start with system, one instance only, toast notifications |
| Tests | `cargo test`, frontend unit tests via Vitest | Protocol tested byte-for-byte without hardware |
| CI / Release | GitHub Actions, Windows runner, MSI/NSIS installer from Tauri bundler | Reproducible builds |

---

## 4. Architecture

### 4.1 Layers

```
┌────────────────────────── nd104-sync (Tauri app) ──────────────────────────┐
│  Tray icon + menu          Settings / Status window (Svelte UI)            │
│         │                          │  Tauri commands & events              │
│         └──────────── App controller (state, events) ─────────────┘        │
│                                   │                                        │
│  ┌──────────────────────────── nd104-core ───────────────────────────────┐ │
│  │ Scheduler ─┬─ Clock service ── ClockSource trait (system, tz, ntp, …) │ │
│  │            ├─ Weather service ─ WeatherProvider trait + Location      │ │
│  │            └─ Firmware update check ── manifest source                │ │
│  │                         │                                             │ │
│  │ Firmware guard (protocol checks) ← must pass before ANY write         │ │
│  │                         │                                             │ │
│  │ Device manager (discover, reconnect, serialised writes)               │ │
│  │                         │                                             │ │
│  │ Platform layer (trait) – Windows impl │ Linux/macOS: stubs            │ │
│  └─────────────────────────┼─────────────────────────────────────────────┘ │
│              nd104-protocol (pure: packets, CRC, checksums)                │
└─────────────────────────────┼──────────────────────────────────────────────┘
                     hidapi → ND104 screen (USB 5542:0001, wired)
```

### 4.2 Crates / folders

```
apps/nd104-sync/
├── Cargo.toml                 # Cargo workspace
├── README.md
├── crates/
│   ├── nd104-protocol/        # pure packet building, no I/O, 100 % unit tested
│   └── nd104-core/            # device, firmware guard, clock, weather,
│                              # scheduler, config, secrets, platform trait
├── src-tauri/                 # Tauri app: tray, commands, wiring, bundling
│   ├── icons/                 # app + tray icons (light/dark/status variants)
│   └── tauri.conf.json
├── ui/                        # Svelte + TS frontend
│   └── src/
├── locales/                   # en.json, de.json (shared)
└── packaging/                 # installer config; later: udev rule, LaunchAgent
```

Separating `nd104-protocol` and `nd104-core` from the Tauri app means:
- protocol and logic are testable without UI or hardware,
- a headless CLI (e.g. `nd104-cli`) can reuse the core later,
- the UI framework could be swapped without touching the logic.

### 4.3 Keyboard models (R7)

The core never talks about "the ND104" directly. It works with a **keyboard
model**, and the ND104 is the first and, for now, only implementation.

```
                      ┌──────────────────────────────┐
 core (scheduler,     │  trait KeyboardModel         │
 firmware guard,  ───▶│  id, name, USB interfaces,   │
 device manager)      │  capabilities, firmware,     │
                      │  encode time / weather,      │
                      │  check answers               │
                      └──────────────┬───────────────┘
                                     │ implemented by
                 ┌───────────────────┼────────────────────┐
           Chilkey ND104       (later: other model) (later: …)
           nd104-protocol      <model>-protocol
```

Sketch of the interface (lives in `nd104-core`, will be renamed if a second
model arrives):

```rust
trait KeyboardModel {
    fn id(&self) -> &'static str;                 // "chilkey-nd104"
    fn display_name(&self) -> &'static str;       // "Chilkey ND104"
    fn interfaces(&self) -> &[InterfaceSpec];     // role, VID/PID, usage page/usage, report length
    fn capabilities(&self) -> Capabilities;       // time, weather, icon set, °C/°F variant, …
    fn read_firmware(&self, io: &mut dyn HidIo) -> Result<FirmwareInfo>;
    fn verified_firmware(&self) -> &[FirmwareId];
    fn encode_time(&self, time: &LocalTime) -> Result<Packet>;
    fn encode_weather(&self, weather: &WeatherReport) -> Result<Packet>; // maps neutral Condition → model icon
    fn check_answer(&self, sent: &Packet, report: &[u8]) -> AnswerCheck; // Ack | Ignored(logged) | Violation
    fn manifest_key(&self) -> &'static str;       // key in firmware/versions.json, e.g. "nd104"
}
```

- **Model registry:** a static list (`[Nd104]` for now). The device manager
  identifies the connected model by its USB IDs. Keyboards without a known model
  are shown as "not supported", and nothing is sent to them.
- **Per-model protocol crate:** `nd104-protocol` now; a second model gets its
  own crate. Shared helpers (e.g. CRC-16) move into a common crate once they
  are needed twice.
- **Neutral data types** (`WeatherReport` with a neutral `Condition`, local
  time) live in the core. Each model maps them to its own packet format and
  icon set.
- **Capabilities drive the UI:** pages and options only appear if the
  connected model supports them (e.g. icon preview, screen variant).
- **Firmware manifest and Drive watcher** are keyed by model.
- **Config:** `[device] model = "auto"` (auto-detect by USB IDs).
- **Naming:** the app stays `nd104-sync` for now. A neutral name is an open
  decision for when a second model is actually added.

---

## 5. Components

### 5.1 Protocol (`nd104-protocol`)
- Pure functions: `time_packet(datetime)`, `weather_packet(icon, current, max, min)`,
  later `theme_packet`, `osd_command`, `image_upload_*`.
- CRC-16/CCITT-FALSE (stored low byte first in every packet, as the official
  web driver does it) and the 8-bit payload checksum.
- Temperatures as °C × 10 in **sign + magnitude** encoding (bit 15 = negative),
  as the web driver does it (the gist assumes two's complement).
- Version queries for keyboard and screen (§ 5.3).
- Keyboard packets are 32 bytes, screen packets 64 bytes; see [protocol.md](protocol.md).
- Every packet builder is covered by golden tests (expected bytes).
- Protocol knowledge validated on real hardware is documented in `docs/protocol.md`.

### 5.2 Device manager
- Opens **both** vendor HID interfaces (usage page `0xFF60`, usage `0x61`):
  keyboard `6D67:016C` and screen `5542:0001`.
- Wired only: if one of them is missing, status = *"Not connected (USB cable
  required)"*.
- Time/weather packets go through the **keyboard** device, as the web driver
  does. M0 showed that both routes work; the screen route stays available
  internally as a fallback.
- Polls for plug/unplug (e.g. every 3 s); on (re)connect → firmware guard → initial sync.
- A single writer task serialises all writes (no interleaved packets).
- Detects a likely conflict with the official Chilkey web driver / weather plugin
  (e.g. device busy / unexpected screen changes) and warns.

### 5.3 Firmware guard (R3)
- Reads the keyboard and screen firmware versions **before** any write.
- Compares them with the list of versions verified on real hardware (filled in M0).
- Older or newer firmware is **allowed** as long as the protocol behaves exactly
  as expected. The guard continuously checks for **protocol fingerprints**:

| # | Check | When |
|---|---|---|
| P1 | Both devices present with the expected VID/PID, usage page `FF60`, usage `61` | connect |
| P2 | Expected HID report lengths (keyboard 32, screen 64 bytes) | connect |
| P3 | Version query answered within the timeout with a well-formed `0x11` report and printable ASCII version | connect, after every re-connect |
| P4 | Writes accepted by the HID stack (no errors, no short writes) | every write |
| P5 | Every time/weather packet is acknowledged with the pattern recorded in M0 (`A5 <cmd> 00 02 FE 00 <chk>`, see [protocol.md § 4.1.1](protocol.md#411-answers-acknowledgement-)); every received report is well-formed (valid CRC/checksum, known structure) | every write |
| P6 | Screen variant (°C / °F) recognised | connect |

- Outcomes:

| Situation | Behaviour |
|---|---|
| Verified version, all checks pass | **Verified** – normal operation |
| Other version (older/newer), all checks pass | **Compatible (unverified)** – normal operation, one-time notification, status page shows "unverified firmware" |
| Any check fails – before or during operation | **Locked** – stop sending immediately, tray icon amber, notification with the reason (which check failed), status page explains it |
| Version cannot be read | Locked (P3 failed) |

- **Unknown but well-formed messages** (valid CRC and checksum, known
  structure, status OK, but an unknown command, e.g. the unsolicited
  `A5 35 … FE 00`) are **logged and ignored**. Nothing is sent in reply, and
  they appear in the log and in the "recent events" list on the status page.
  This is safe: the official web driver ignores such messages too, and not
  answering cannot change anything on the device. Decision 2026-10-31.
- Messages that are **damaged or structurally unknown**, a **missing
  acknowledgement**, or an acknowledgement with a **status other than OK**
  still lock immediately.
- **Only the basic data packets** (time, weather) are ever sent to unverified
  firmware. Anything else (themes, images, menu control, later features) is
  only allowed on verified firmware.
- Locked stays locked until the user runs a re-check ("Check again" button) or
  the device is re-connected, and then all checks run again from the start.
  There is no "ignore and continue" switch.
- A new firmware becomes *verified* once it has been tested with `nd104-probe`
  and added to the list in a new release.
- **How the version is read** (found in the official web driver, see
  [protocol.md § 3](protocol.md#3-firmware-version-queries)):
  - keyboard: send `11 02 … CRC` (32 bytes) → answer `0x11`, ASCII in bytes 8–14
  - screen: send `11 02 00 00 00 00 FD 1B …` (64 bytes) → answer `0x11`, ASCII in bytes 8–13
  - Only the ASCII answers count. The numeric byte in the screen's `0x1C … FB`
    answer is ignored; it is most likely why the web driver shows a broken
    screen version.
  - Timeout (e.g. 1 s, 3 retries) without an answer → P3 fails → locked.
- Fallback if the queries turn out unreliable in M0: the USB descriptor
  release number (`bcdDevice`).

### 5.4 Firmware update check (R4)
- Runs once after boot (with a short delay), then e.g. every 24 h, and
  **manually** via the "Check for updates" button (Device & Firmware page) or
  the tray entry *Check for firmware updates*. A manual check always shows a
  result, including "up to date" and "check failed".
- Chilkey has no machine-readable source. The real "latest" is only visible in
  the file names in their Google Drive folders, and the support page text is
  outdated (see [firmware.md](firmware.md)). Therefore:
  - a **GitHub Action** in this repo watches the Drive folders and support pages
    every day and opens an issue or PR when something changes,
  - nd104-sync reads only the reviewed manifest.
- Source for the tool: a small JSON manifest in this repository
  (`firmware/versions.json`), fetched from GitHub raw (the format below is a
  draft; it will also carry the screen variant °C/°F and the Drive file names):
  ```json
  {
    "nd104": {
      "keyboard": { "latest": "3.0.4", "verified": ["3.0.4"] },
      "screen":   { "latest": "1.2.1", "verified": ["1.2.1"] }
    },
    "source": "https://chilkey.com/blogs/updates/nd104-faq-resources"
  }
  ```
  The manifest is maintained in the repo and updated through the watcher
  action's issues/PRs. The tool itself never scrapes Chilkey or Google Drive.
- If an update is available → notification after boot + hint on the status page,
  including the hint *"not yet verified with nd104-sync – will run as
  compatible/unverified as long as the protocol checks pass"* when applicable,
  and a link to the official Chilkey instructions.
- The tool **never** downloads or installs firmware.
- Offline / manifest unreachable → silently retry later, show "last checked" date.

### 5.5 Clock service
**Sources** (`ClockSource` trait):

| Source | Description |
|---|---|
| `system` (default) | PC local time |
| `timezone` | PC time converted to a chosen IANA zone (e.g. `Europe/Berlin`) |
| `ntp` | Time from an NTP server (default `pool.ntp.org`), converted to the chosen zone |
| `manual` | System time + fixed offset in minutes (fully manual date/time for testing only) |

**DST mode:** `auto` (from tz rules, default) · `force_summer` · `force_winter`.

**Sync triggers:** app start · device (re)connect · every N minutes (default 60) ·
exactly at the next DST transition (pre-calculated) · resume from sleep ·
"Sync now".

12/24 h display is a firmware setting – the UI explains this, the tool does not change it.

### 5.6 Weather service
**Provider trait** (simplified):
```rust
trait WeatherProvider {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn requires_api_key(&self) -> bool;
    async fn fetch(&self, location: &Location, key: Option<&str>) -> Result<WeatherReport>;
}
struct WeatherReport { condition: Condition, current_c: f32, max_c: f32, min_c: f32, observed_at: Timestamp }
```
Each provider maps its own codes to a neutral `Condition` enum; one central
function maps `Condition` → firmware icon index (0–9, all verified in M0; day/night
variants for clear and partly cloudy, 9 = thunderstorm). See
[protocol.md § 4.2](protocol.md#42-weather-command-0xfe-len-8).

**Provider list:**

| Provider | API key | Milestone | Notes |
|---|---|---|---|
| Open-Meteo | no | M1 (default) | Worldwide, WMO codes |
| Bright Sky (DWD) | no | M4 | Germany |
| OpenWeatherMap | yes | M4 | |
| WeatherAPI.com | yes | M4 | Same code set as the official plugin |
| Manual / Test | – | M1 | Set icon + temperatures by hand |

Adding a provider = one new module implementing the trait + registry entry +
locale strings + table row in the README.

**Location:** city search via Open-Meteo geocoding (stores name + lat/lon) or
manual lat/lon. IP-based lookup optional and off by default.

**Behaviour:** default interval 15 min (min. 5) · exponential backoff on errors ·
keep last good values · "stale" warning after X hours · optional fallback provider.

The display receives °C × 10. A °F option only affects the UI.

### 5.7 Scheduler
Tokio-based job runner: `clock_sync`, `weather_fetch`, `weather_push`,
`firmware_update_check`, plus one-shot jobs (DST transition). Pausable from the tray.
All writes go through the firmware guard.

### 5.8 Config & secrets
- `%APPDATA%\nd104-sync\config.toml` (via `directories`), versioned schema with migrations.
- API keys in Windows Credential Manager via `keyring`, never in the TOML file.
- Logs in `%LOCALAPPDATA%\nd104-sync\logs` (rolling, 7 days).

Example:
```toml
schema = 1

[general]
language = "auto"        # auto | de | en
theme = "auto"           # auto | dark | light
autostart = true

[clock]
source = "system"        # system | timezone | ntp | manual
timezone = "Europe/Berlin"
dst_mode = "auto"        # auto | force_summer | force_winter
ntp_server = "pool.ntp.org"
manual_offset_min = 0
sync_interval_min = 60

[weather]
provider = "open-meteo"
fallback_provider = ""
interval_min = 15
ui_unit = "celsius"      # celsius | fahrenheit (UI only)

[weather.location]
mode = "search"          # search | coordinates | ip
name = "Berlin"
lat = 52.52
lon = 13.405

[firmware]
update_check = true
```

### 5.9 Internationalisation
- `locales/en.json`, `locales/de.json` – flat keys, shared by tray (Rust) and UI.
- `language = "auto"` → system language via `sys-locale`; fallback English.
- Switchable in the tray menu (submenu *Language*) and in settings; applied live.
- New language = new JSON file + entry in the language list. A test checks that
  all locale files contain the same keys.

### 5.10 Platform layer (R2)
```rust
trait Platform {
    fn autostart(&self) -> &dyn Autostart;          // enable / disable / is_enabled
    fn power_events(&self) -> PowerEventStream;     // resume from sleep
    fn system_theme(&self) -> Theme;                // + change notifications
    fn hid_access_hint(&self) -> Option<String>;    // e.g. udev rule on Linux
}
```
- `windows`: implemented (autostart via Tauri plugin / Run key, power events via
  window messages, theme via Tauri).
- `linux`, `macos`: compile-time stubs returning `Unsupported`. Planned notes:
  udev rule for `5542:0001`, XDG autostart, logind resume signal; LaunchAgent,
  menu-bar icon, possible HID permission prompt on macOS.

### 5.11 Versioning and app update check
- **SemVer** `MAJOR.MINOR.PATCH`; `0.x.y` during development. Current: `0.1.0`.
- **Single source of truth:** `[workspace.package] version` in
  `apps/nd104-sync/Cargo.toml`. The code reads it via `CARGO_PKG_VERSION`
  (`nd104_core::APP_VERSION`), so the CLI, the app, the logs and the HTTP user
  agent always show the same version.
- **Release tags** `nd104-sync-vX.Y.Z` (prefix because the repo hosts several
  tools); [CHANGELOG.md](../apps/nd104-sync/CHANGELOG.md) in Keep-a-Changelog format.
- **Version shown** in: settings window General → About, dashboard footer,
  tray tooltip, `nd104-cli --version`, first line of each log file.
- **App update check** (`nd104_core::app_update`, ✅ in M1):
  - GitHub releases API of this repository; only `nd104-sync-v*` tags,
    no drafts or pre-releases, numeric SemVer comparison
  - 30 s after start, then every `check_interval_h` (default 24 h), and
    manually: "Check for updates" (General → About, tray menu),
    `nd104-cli check-update`
  - Result: notification + About page ("up to date" / "X.Y.Z available" with a
    link / "check failed"). Nothing is downloaded or installed automatically.
  - Can be disabled: `[updates] app_check = false`.
- **Releases per milestone (beta):** M1 = 0.1.0, M2 = 0.2.0, …; a tag push runs
  `.github/workflows/nd104-sync-release.yml` (test, build, zip + SHA-256, release
  notes from the CHANGELOG section). 0.x = "(beta)" in the title but not a GitHub
  pre-release, so the update check finds it; `-rc.N` versions are pre-releases.
  CI (`nd104-sync-ci.yml`) runs fmt, clippy and tests on every push.
- **Updating:** run the newer installer (the config is kept). Config files carry
  `schema`, so newer apps migrate older files and older apps refuse newer files.

---

## 6. User interface

### 6.1 Tray
- Icon states: **connected & synced** (accent) · **not connected** (grey) ·
  **locked / firmware mismatch** (amber) · **error** (red). Light/dark variants
  for the Windows taskbar.
- Tooltip: `nd104-sync 0.1.0 · ND104 · 12.3 °C ☁ · synced 14:05` (includes the app version)
- Menu: status line · Sync now · Pause/Resume · **Check for updates** (firmware + app) · Settings… · Language ▸ (Auto / Deutsch / English) · Open log folder · Quit

### 6.2 Settings / status window
Tabs (left sidebar):
1. **Dashboard** – live preview of the keyboard display (320×172 mock-up showing
   what was sent), connection, **installed firmware versions (keyboard + screen,
   incl. screen variant °C/°F) with a verified / compatible (unverified) / locked badge**, last/next sync,
   recent errors, update hint.
2. **Weather** – provider, API key + *Test*, location search with result list,
   interval, fallback provider, °C/°F (UI).
3. **Clock** – source, time-zone picker, DST mode, NTP server + *Test*, offset,
   preview "time sent to keyboard", *Sync now*.
4. **Device & Firmware** – detected devices (USB IDs), **installed firmware
   versions**, verified versions of this nd104-sync release, latest known
   versions from the manifest (with Drive file names and dates), update status,
   "last checked" time, a **"Check for updates" button** (manual check, same as
   the tray entry), link to Chilkey's update instructions, wired-only notice.
5. **General** – autostart, language, theme (Auto/Dark/Light), log level,
   **About**: app version, build info, "Check for updates" button with the
   result (app + firmware), links to the repo, changelog and releases.

The **app version** is also shown in the dashboard footer.

Apply/Cancel with validation; *Test* buttons never save.

### 6.3 Design language
- **Gamer-inspired, but calm:** dark graphite surfaces, one neon accent
  (default cyan/teal; maybe selectable later), thin 1 px borders, subtle glow
  on active elements and status LEDs, rounded 10–12 px cards.
- **Typography:** a technical display font for headings/numbers (e.g. *Rajdhani*
  or *Orbitron*, OFL-licensed, bundled locally – no CDN), clean sans-serif for
  text, monospace for values.
- **Keyboard motifs:** keycap-style buttons, LED-dot status indicators, the
  display preview as the visual centre.
- **Theme:** `auto` follows the Windows setting live; manual Dark/Light in
  settings. Light theme = off-white surfaces with the same accent.
- Accessibility: sufficient contrast in both themes, keyboard navigation,
  no information carried by colour alone.
- Small window (~900×600), resizable, no animation overload.

---

## 7. Startup sequence

1. Single-instance check (second start → focus existing window).
2. Load config (create defaults on first run), set up logging.
3. Resolve language + theme, create tray icon.
4. Start device manager → find wired screen device.
5. Firmware guard → **verified** or **compatible (unverified)**: continue · **any protocol check fails**: locked + notification, stop here (keep polling).
6. Initial clock sync + weather fetch/push.
7. Start scheduler.
8. After ~30 s: firmware update check → notification if an update exists.
9. First run only: open settings window with a short welcome / location setup.

---

## 8. Milestones

| # | Milestone | Deliverable |
|---|---|---|
| **M0** | **Protocol & firmware verification** | Small Rust CLI (`nd104-probe`): list devices, read keyboard + screen firmware versions, send time, send test weather (incl. negative temperatures), cycle icons, compare keyboard vs. screen as target. Work through the checklist in [protocol.md § 6](protocol.md#6-open-points-for-m0-hardware-verification), record the answer patterns for the protocol checks, first verified firmware entry. **Gate for everything else.** |
| M1 | Core (headless) | `nd104-protocol` + `nd104-core`: `KeyboardModel` interface with the ND104 as the only model, device manager, firmware guard, clock (system/timezone/manual), Open-Meteo + manual provider, scheduler, config, **app update check**, console runner `nd104-cli`; unit tests |
| M2 | Tray app | Tauri shell, tray with states, autostart, single instance, notifications, i18n (DE/EN) |
| M3 | Settings UI | Dashboard, Weather, Clock, Device & Firmware, General; dark/light/auto; design system |
| M4 | Providers & secrets | Bright Sky, OpenWeatherMap, WeatherAPI, fallback, NTP, keyring |
| M5 | Firmware update check & installer | Manifest, Drive watcher action, firmware update notification, Windows installer in the release workflow |
| M6 | Extras | Theme colours, image/GIF upload, more languages, Linux/macOS implementation (if wanted) |

---

## 9. Risks & open questions

| Risk / question | Mitigation |
|---|---|
| Firmware version cannot be read | Version queries found in the web driver (protocol.md § 3); M0 verifies them; USB descriptor as fallback; otherwise the tool stays locked |
| Web driver sends to keyboard, gist to screen | M0 tests both paths; device manager supports both |
| Official Chilkey software writes to the display at the same time | Detect/warn; documentation says to close it |
| Firmware updates change the protocol | Protocol checks P1–P6 + immediate lock (R3); protocol crate versioned per firmware |
| Weather provider changes / rate limits | Provider trait, backoff, fallback provider |
| Tray behaviour on Linux desktops (later) | Platform layer; settings window also openable via command-line flag |
| Image/GIF upload protocol unverified | Not part of v1 |
