# nd104-sync

**English** · [Deutsch](README.de.md)

Background tool that keeps the display of the **Chilkey ND104** keyboard up to
date with the **current time and weather** – with a tray icon and a modern
settings/status window.

> 🚧 **Status: planning.** Nothing described here is implemented yet.
> The detailed plan lives in [docs/PLAN.md](../../docs/PLAN.md).

> 🔌 **Wired USB only.** nd104-sync only supports the keyboard connected with a
> **USB cable**. 2.4 GHz dongle and Bluetooth modes are **not supported** and
> not tested.

> 🪟 **Windows only (for now).** Windows 10/11 is the only supported platform.
> The architecture is prepared for Linux and macOS, but support is not
> implemented.

---

## Why?

The ND104 has no internet connection. Its display can show time and weather,
but only if a connected PC sends that information. The official solution is a
browser-based driver plus a weather plugin with a fixed weather backend.
nd104-sync replaces this with a small background app that you configure once:
your weather provider, your location, your clock source.

## What it does

- Starts with Windows and lives in the **system tray**
- **Syncs the clock** to the keyboard regularly, on connect, after sleep and
  exactly at summer/winter time changes
- **Fetches the weather** for your location and sends current, max and min
  temperature plus a condition icon to the display
- Lets you choose the **weather provider**, **location**, **clock source**,
  **time zone** and **DST mode** in a settings window
- Shows a **status dashboard** with a live preview of what the display shows
- **Protects your keyboard:** only talks to verified firmware versions
  (see [Firmware safety](#firmware-safety))
- **Checks for firmware updates** after boot and notifies you
- UI in **German and English** (follows the system language, switchable)
- **Dark / light mode** following Windows, overridable in settings

## What it does NOT do

- ❌ **No wireless support** – no 2.4 GHz dongle, no Bluetooth. USB cable only.
- ❌ **No Linux / macOS support** yet (possible in the future, not implemented).
- ❌ **No firmware flashing.** It never downloads, installs or modifies firmware;
  it only tells you an update exists.
- ❌ **No communication with unverified firmware.** If the firmware version is
  unknown or doesn't match, the tool sends nothing and tells you why.
- ❌ **No keyboard configuration** – keymap, RGB lighting, macros, knob etc.
  stay with the official [Chilkey driver](https://chilkey.com/pages/console).
- ❌ **No image/GIF upload** in the first version (planned later).
- ❌ **No °F on the display** – the display receives °C; °F is a UI-only option.
- ❌ **No 12/24 h switching** – that is a keyboard firmware setting.
- ❌ **No telemetry.** It only contacts the services you configured
  (see [Privacy](#privacy--network-access)).
- ❌ **Not meant to run alongside** the official Chilkey weather plugin – both
  would write to the display.

---

## Requirements

| | |
|---|---|
| OS | Windows 10 (21H2+) or Windows 11, 64-bit |
| Connection | Chilkey ND104 via **USB cable** |
| Firmware | Verified versions only, see table below |
| Runtime | Microsoft Edge WebView2 (pre-installed on Windows 11 and current Windows 10) |

### Supported firmware

| Part | Verified version | Status |
|---|---|---|
| Keyboard | V3.0.4 | to be verified (milestone M0) |
| Screen | V1.2.1 | to be verified (milestone M0) |

## Firmware safety

Before sending anything, nd104-sync reads the firmware versions of the keyboard
and of the display, using the same queries as the official web driver (see
[docs/protocol.md § 3](../../docs/protocol.md#3-firmware-version-queries)). It
then compares them with a list of versions it has been verified against.

| Situation | What happens |
|---|---|
| Verified version | Normal operation |
| Different or unknown version | **Locked mode:** nothing is sent. Tray icon turns amber, you get a notification and the status page explains what to do. |
| Version cannot be read | Treated as unknown → locked mode |

There is intentionally **no override**. New firmware versions get supported by
verifying them and releasing an updated nd104-sync.

## Firmware update check

Shortly after boot (and then once a day) nd104-sync checks a version manifest in
this repository ([`firmware/versions.json`](../../firmware/)). If Chilkey has
released a newer firmware, you get a notification with a link to the official
update instructions – and a hint whether nd104-sync already supports that
version. Updating stays your decision and is done with Chilkey's tools.

## Weather providers

| Provider | API key | Coverage | Status |
|---|---|---|---|
| [Open-Meteo](https://open-meteo.com/) (default) | no | worldwide | planned (M1) |
| Manual / Test | – | – | planned (M1) |
| [Bright Sky](https://brightsky.dev/) (DWD data) | no | Germany | planned (M4) |
| [OpenWeatherMap](https://openweathermap.org/) | yes | worldwide | planned (M4) |
| [WeatherAPI.com](https://www.weatherapi.com/) | yes | worldwide | planned (M4) |

Providers are plug-ins implementing one interface, so more can be added
(see [docs/PLAN.md § 5.6](../../docs/PLAN.md#56-weather-service)).
API keys are stored in the Windows Credential Manager, not in the config file.

## Clock sources

| Source | Description |
|---|---|
| System (default) | Local PC time |
| Time zone | PC time converted to a chosen time zone |
| NTP | Time from an NTP server (default `pool.ntp.org`) |
| Manual | System time plus a fixed offset |

Summer/winter time: **Auto** (from time-zone rules) · force summer · force winter.

---

## Installation

> Not available yet – releases will be published on the
> [GitHub releases page](https://github.com/MrUrdam/chilkey_nd104_utilities/releases)
> as a Windows installer (`.msi` / `.exe`).

## Usage

1. Connect the ND104 with a **USB cable**.
2. Close the official Chilkey web driver / weather plugin.
3. Start nd104-sync. On first start the settings window opens – choose your
   location (city search) and, if you like, another weather provider.
4. From then on nd104-sync starts with Windows and runs in the tray.

**Tray menu**

| Entry | Action |
|---|---|
| Status line | Connection, weather and last sync at a glance |
| Sync now | Send time and weather immediately |
| Pause / Resume | Stop / resume all updates |
| Settings… | Open the settings/status window |
| Language ▸ | Auto / Deutsch / English |
| Open log folder | For troubleshooting |
| Quit | Exit the app |

**Tray icon colours:** accent = connected & synced · grey = not connected ·
amber = locked (firmware) · red = error.

**Command-line options (planned)**

```
nd104-sync.exe                # normal start (tray)
nd104-sync.exe --settings     # open settings window directly
nd104-sync.exe --minimized    # start without window (used by autostart)
nd104-sync.exe --sync-once    # sync once and exit
```

**Files**

| What | Where |
|---|---|
| Configuration | `%APPDATA%\nd104-sync\config.toml` |
| Logs | `%LOCALAPPDATA%\nd104-sync\logs\` |
| API keys | Windows Credential Manager (`nd104-sync/<provider>`) |

## Privacy / network access

nd104-sync connects only to:
- the weather provider you selected (and Open-Meteo geocoding for city search),
- the NTP server, if you selected NTP as clock source,
- GitHub (raw file of this repo) for the firmware update check – can be disabled.

No telemetry, no analytics, no IP-based location unless you turn it on.

## Troubleshooting

| Problem | Fix |
|---|---|
| "Not connected" | Use a USB **cable** (wireless is not supported); try another port; avoid unpowered hubs |
| Amber icon / locked | Firmware not verified – see the Device & Firmware page; wait for an nd104-sync update or use a verified firmware |
| Display shows other values | The official Chilkey weather plugin is probably still running – close it |
| Wrong time after DST change | Check time zone / DST mode in the Clock tab |

---

## Building from source

### Prerequisites (Windows)

| Tool | Version | Notes |
|---|---|---|
| [Rust](https://rustup.rs/) | stable, `x86_64-pc-windows-msvc` | via `rustup` |
| Visual Studio Build Tools | 2022 | workload "Desktop development with C++" |
| [Node.js](https://nodejs.org/) | LTS (≥ 20) | for the UI build |
| WebView2 Runtime | current | pre-installed on Windows 11 |
| Tauri CLI | v2 | `cargo install tauri-cli --version "^2"` |

### Build

```bash
cd apps/nd104-sync
npm install --prefix ui
cargo tauri dev        # run in development mode with hot reload
cargo tauri build      # release build + installer in src-tauri/target/release/bundle
cargo test --workspace # run the unit tests (no hardware needed)
```

### Main dependencies

| Crate / package | Purpose |
|---|---|
| `tauri` v2 + plugins `autostart`, `single-instance`, `notification` | App shell, tray, window |
| `hidapi` | USB HID communication |
| `tokio` | Async runtime, scheduler |
| `reqwest` (rustls) | HTTP for weather / geocoding / manifest |
| `jiff` | Time zones and DST |
| `sntpc` | NTP client |
| `serde`, `toml`, `directories` | Configuration |
| `keyring` | API keys in the Credential Manager |
| `tracing`, `tracing-appender` | Logging |
| `sys-locale` | System language detection |
| Svelte + TypeScript + Vite | Settings UI |

### Project structure

```
apps/nd104-sync/
├── crates/
│   ├── nd104-protocol/   # packet building, CRC – pure, unit tested
│   └── nd104-core/       # device, firmware guard, clock, weather, scheduler, config
├── src-tauri/            # Tauri app: tray, commands, bundling
├── ui/                   # Svelte frontend
├── locales/              # en.json, de.json
└── packaging/            # installer configuration
```

## Contributing

- **New weather provider:** implement the `WeatherProvider` trait, register it,
  add locale strings and a row in the provider table above.
- **New language:** add `locales/<code>.json` with all keys of `en.json`.
- **New platform (Linux/macOS):** implement the `Platform` trait
  (see [docs/PLAN.md § 5.10](../../docs/PLAN.md#510-platform-layer-r2)).
- **New firmware version:** verify the protocol on real hardware, document it in
  `docs/protocol.md`, add it to the allowlist.

## Credits

Protocol knowledge is based on the
[ND104 reverse engineering notes](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b)
by borrougagnou and on an analysis of the official
[Chilkey web driver](https://nd104.chilkey.com/). Everything is collected in
[docs/protocol.md](../../docs/protocol.md). Inspired by
[zoom-sync](https://github.com/ozboar/zoom-sync).

Unofficial project, not affiliated with Chilkey. MIT licensed – see [LICENSE](../../LICENSE).
