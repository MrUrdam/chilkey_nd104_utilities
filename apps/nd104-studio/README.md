# ND104 Studio

[![Latest release](https://img.shields.io/github/v/release/MrUrdam/chilkey_nd104_utilities?filter=nd104-studio-v*&label=release&color=0969da)](https://github.com/MrUrdam/chilkey_nd104_utilities/releases)
[![CI](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/nd104-studio-ci.yml/badge.svg)](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/nd104-studio-ci.yml)
[![License: MIT](https://img.shields.io/github/license/MrUrdam/chilkey_nd104_utilities)](../../LICENSE)
![Windows 10 | 11](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6?logo=windows&logoColor=white)
[![Connection modes: USB, Bluetooth, 2.4 GHz](https://img.shields.io/badge/connection%20modes-matrix-blue)](#connection-modes)
<br>
![Rust](https://img.shields.io/badge/Rust-1.80%2B-000000?logo=rust&logoColor=white)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)
![Vite](https://img.shields.io/badge/Vite-646CFF?logo=vite&logoColor=white)

Everything for the **Chilkey ND104** keyboard in one small tray app: it keeps
the display up to date with the **current time and weather**, shows **your
own picture or GIF**, sets the **lighting** and lets you **remap keys, the
knob and the round button** on all layers. Set it up once, then forget it.

*Formerly **nd104-sync**. Updating from nd104-sync 1.x takes everything over
– see [Coming from nd104-sync](#coming-from-nd104-sync).*

<p align="center">
  <img src="../../docs/images/nd104-studio/dashboard.png" width="760" alt="ND104 Studio settings window, dashboard page with live preview of the keyboard display">
</p>

<!-- page order as in the sidebar -->
<table>
  <tr>
    <td align="center"><img src="../../docs/images/nd104-studio/keymap.png" width="370" alt="Keymap page"><br><sub>Keymap: layers, ISO/ANSI, labels, keys, knob and round button</sub></td>
    <td align="center"><img src="../../docs/images/nd104-studio/lighting.png" width="370" alt="Lighting page"><br><sub>Lighting: on/off, effect, brightness, speed, colour, PC lighting software</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="../../docs/images/nd104-studio/weather.png" width="370" alt="Weather page"><br><sub>Weather: provider, location search, test</sub></td>
    <td align="center"><img src="../../docs/images/nd104-studio/clock.png" width="370" alt="Clock page"><br><sub>Clock: source, time zone, summer/winter time</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="../../docs/images/nd104-studio/picture.png" width="370" alt="Screen picture page"><br><sub>Screen picture: checks, animated preview, send</sub></td>
    <td align="center"><img src="../../docs/images/nd104-studio/device.png" width="370" alt="Device and Firmware page"><br><sub>Device & Firmware: safety state, update check</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="../../docs/images/nd104-studio/general.png" width="370" alt="General page"><br><sub>General: autostart, language, theme, style, updates</sub></td>
  </tr>
</table>

**⬇️ [Download the installer](https://github.com/MrUrdam/chilkey_nd104_utilities/releases)** ·
[Installation](#installation) · [Usage](#usage) · [Changelog](CHANGELOG.md)

> ⚠️ **Use at your own risk.** Unofficial hobby project, not affiliated with
> Chilkey. No warranty and no guaranteed support. Only tested with the
> firmware versions listed under [Supported firmware](#supported-firmware),
> on Windows 10/11, with the connections marked ✅ in
> [Connection modes](#connection-modes). Full text: [disclaimer](../../README.md#disclaimer).

---

## Why?

The ND104 has no internet connection. Its display can show time and weather,
but only if a connected PC sends that information. The official solution is a
browser-based driver plus a weather plugin with a fixed weather backend.
ND104 Studio replaces this with a small background app that you configure
once – your weather provider, your location, your clock source – and adds
what you would otherwise open the web driver for: lighting, your own screen
picture and the keymap.

## Features

**Display**
- **Syncs the clock** to the keyboard regularly, on connect, after sleep and
  exactly at summer/winter time changes
- Clock sources: **system time, time zone, NTP or manual offset**, with
  automatic or forced summer/winter time
- **Fetches the weather** for your location and sends current, max and min
  temperature plus a condition icon to the display
- **Five weather providers** (two without API key) with a fallback provider
  and a city search for your location
- **Own picture or GIF** on the screen: any PNG/JPEG/GIF/BMP is checked
  (type, size, frames) and scaled to 320 × 172 before it is sent

**Keymap**
- **Remap keys, the knob and the round button** next to the screen on all
  6 layers; the knob turns left/right (layers 0–3, a keyboard limit) and is
  pressed like a key
- **Changes are collected** in a list (each one can be undone) and written
  together; only the differences are written and read back to check
- **Automatic backup** of the previous keymap before every write (the last
  20 are kept), plus **backup and restore** to a file of your choice
- **Renamable layers** ("Basic layer" and "Fn layer" by default); Fn on the
  basic layer stays protected so the other layers remain reachable
- **ISO or ANSI** drawing – switch, or detect it with one key press
- **Labels** ISO UK, ISO DE and ANSI US, plus your own copies; labels only
  change what the app shows
- **Profiles** with a name and an application: save, load (shown as changes
  to review), export and import
- **Pressed keys light up** in the drawing; an Fn combination shows the Fn
  layer while it is held
- The drawing shows the keyboard's **single-colour lighting** (keys, light
  bar, glow around it) and the **live screen**

**Lighting and keyboard settings**
- **Background lighting** (under the keys) and **bottom lighting** (around
  the keyboard and the light bar below Del / End / PgDn): on/off, effect,
  brightness, speed and colour –
  sent and stored in the keyboard right away
- **Keyboard settings** on the Device & Firmware page: debounce mode and
  time, sleep and deep-sleep time – with exactly the values Chilkey's web
  driver offers, stored in the keyboard right away

**Safety and updates**
- **Firmware safety lock:** works with older/newer firmware too, but stops
  immediately as soon as the protocol behaves differently
  (see [Firmware safety](#firmware-safety)); answers meant for other
  programs (e.g. Chilkey's web driver) are ignored
- **Shows the installed firmware** of keyboard and screen
- **Checks for firmware updates** against Chilkey's official folders and
  notifies you. It never downloads or flashes anything.
- **Checks for new ND104 Studio releases** on GitHub and installs them with
  one click (see [Updating](#updating))

**App**
- Starts with Windows and lives in the **system tray**
- **Status dashboard** with a live preview of what the display shows
- API keys stored in the **Windows Credential Manager**
- UI in **English and German** (follows the system language, switchable)
- **Two styles**, "Calm" (default) and "Gamer", each in **dark and light**
  following Windows; both can be changed in settings and in the setup
- **Collapsible sidebar** (icons only, the page name slides out on hover)
- **Console runner** `nd104-cli` for scripting and testing
- **No telemetry** (see [Privacy](#privacy--network-access))

## Connection modes

The ND104 connects by USB cable, Bluetooth (3 devices) or 2.4 GHz receiver.
**This table is the one place that says what works on which connection** –
the rest of the documentation, the app and the setup point here.

| Feature | USB cable | Bluetooth | 2.4 GHz |
|---|:-:|:-:|:-:|
| Time and weather on the display | ✅ | ✅ | ✅ |
| Own picture or GIF on the screen | ✅ | ❌ | ❌ |
| Lighting | ✅ | ✅ | ✅ |
| Per-key colours from PC lighting programs – OpenRGB, Artemis, Aurora, SignalRGB ([PC lighting software](#pc-lighting-software-openrgb-signalrgb)) | ✅ | ❌ | ❌ |
| Keymap (remap, backup, profiles, pressed keys in the drawing) | ✅ | ✅ | ✅ |
| Keyboard settings (debounce, sleep times) | ✅ | ✅ | ✅ |
| Keyboard firmware version and firmware update check | ✅ | ✅ | ☑️ ¹ |
| Screen firmware version | ✅ | ☑️ ¹ | ☑️ ¹ |
| Battery level (app and live preview) | ☑️ ² | ✅ | ❌ |
| App updates, weather download, settings window | ✅ | ✅ | ✅ |

✅ works · ☑️ with a limitation (footnote) · ❌ not available

¹ Taken from the last USB connection (the keyboard version also from the last
Bluetooth connection). **Connect the keyboard once by cable** first; until
then ND104 Studio waits ("Connecting …") and says so – it does not lock.
² Over the cable itself the keyboard does not report its battery level; it is
shown as long as the keyboard is also connected via Bluetooth.

- **Priority:** with several connections at once, ND104 Studio uses the
  cable first, then Bluetooth, then 2.4 GHz (and still shows the battery
  level from Bluetooth). Switching to a higher one also ends a lock that
  happened on the lower one – all checks run again.
- **Sleep and off:** when the keyboard sleeps, Windows drops the Bluetooth
  link; over 2.4 GHz the receiver stays but the keyboard stops answering.
  Either way ND104 Studio shows "Not connected" / "Connecting …" and connects
  again as soon as the keyboard answers (press a key) – no lock; only a wrong
  answer locks, like over USB.
- The sidebar shows the current mode in green ("USB", "Bluetooth",
  "2.4 GHz"), the tray status line adds "(Bluetooth)" or "(2.4 GHz)".
- **How it works:** over Bluetooth the keyboard offers the same command
  channel as over USB (HID over Bluetooth LE, one extra report ID byte); the
  2.4 GHz receiver offers it too and forwards every command over the air. In
  both cases the keyboard passes time and weather on to the screen as it
  does over the cable. The screen controller itself is a second USB device
  inside the keyboard and reachable by cable only – hence picture and
  screen version need the cable; over 2.4 GHz the version query reaches only
  the receiver. Details: [protocol.md § 5.5](../../docs/protocol.md#55-over-bluetooth-analysis-2026-11-02)
  and [§ 5.6](../../docs/protocol.md#56-over-the-24-ghz-receiver-analysis-2026-11-02).

The same overview is in the app: **Device → Connection modes**.

## PC lighting software (OpenRGB, SignalRGB)

Besides its own effects, the ND104 can let a lighting program on the PC
colour every LED individually: 130 LEDs – the keys, the light bar under
Del / End / Page Down and the edge glow. USB cable only (the lighting channel
is not available over Bluetooth or 2.4 GHz).

| Program | Works with the ND104 |
|---|---|
| **OpenRGB** | ✅ through ND104 Studio's OpenRGB interface (as an SDK client) |
| **Artemis**, **Aurora** and other OpenRGB SDK programs | ✅ through ND104 Studio's OpenRGB interface |
| **SignalRGB** | ✅ with the plugin from ND104 Studio |
| OpenRGB on its own (device detection) | ❌ it talks to the keyboard's VIA channel, not its lighting channel |
| Windows Dynamic Lighting | ❌ the keyboard does not answer Windows' lighting request (firmware V3.0.14) |

First, in either case: **Lighting → PC lighting software** set to *Allowed*
(Fn + Shift + L does the same on the keyboard).

### OpenRGB, Artemis, Aurora

ND104 Studio runs a small OpenRGB SDK server – only on this PC (127.0.0.1),
off until you switch it on:

1. **Lighting → OpenRGB interface:** switch on, keep port **6743** (OpenRGB's
   own server uses 6742), **Save**. The card shows whether it runs and how
   many programs are connected.
2. **OpenRGB:** tab *SDK Client* → IP `127.0.0.1`, port `6743` → *Connect*.
   **Artemis / Aurora:** add `127.0.0.1:6743` as an OpenRGB server.
3. The ND104 appears as a keyboard with three zones – *Keyboard* (a 24 × 6
   matrix, every key in its own cell), *Light bar* and *Edge glow* – and two
   modes: *Direct* (the program's colours) and *Keyboard effects* (the
   keyboard's own lighting).

The keyboard is offered only while it is connected by USB cable and its
firmware checks passed; programs are told when that changes. When the last
program disconnects (or the interface is switched off), the keyboard's own
lighting returns.

### SignalRGB

1. **Install plugin** in the *PC lighting software* card. It goes to
   `Documents\WhirlwindFX\Plugins\Chilkey_ND104.js`, where SignalRGB keeps
   it across updates.
2. Restart SignalRGB (or reload its plugins). The ND104 appears under
   *Devices*; place it on the canvas.

### Good to know

- The colours are not stored in the keyboard. Any change on the Lighting
  page also ends a program's control.
- Both use the same LED table, read from the keyboard:
  [`signalrgb/layout.json`](signalrgb/layout.json), made by
  [`make_plugin.py`](signalrgb/make_plugin.py) from
  [`leds.json`](signalrgb/leds.json); the plugin is
  [`signalrgb/Chilkey_ND104.js`](signalrgb/Chilkey_ND104.js).
- Technical details: [protocol.md § 5.4](../../docs/protocol.md#54-other-keyboard-commands-not-used).

## What it does NOT do

- ❌ **Not every feature on every connection** – see
  [Connection modes](#connection-modes).
- ❌ **No Linux / macOS support.** Windows 10/11 only.
- ❌ **No firmware flashing.** It never downloads, installs or modifies firmware.
  It only tells you an update exists.
- ❌ **No risky communication.** Time, weather, lighting, the own picture and
  keymap changes are only sent while every protocol check passes. If anything
  looks different, the tool stops sending immediately and tells you why. There
  is no "ignore and continue" switch.
- ❌ **No macros.** Recording or editing macros stays with the official
  [Chilkey driver](https://chilkey.com/pages/console); keymap backups and
  profiles do not contain them.
- ❌ **No tap/hold or combo editor.** Keys can get any single function
  (key, modifier, media, mouse, lighting, layer, display). Combinations set
  elsewhere (e.g. Ctrl+C on one key, tap for a key / hold for a layer) are
  shown and kept, but not created.
- ❌ **No key swapping or automatic profile switching.** A key is remapped on
  its own; profiles are loaded by hand, not when an application starts
  (every switch would wear the keyboard's memory).
- ❌ **No "Online RGB", keyboard colour or USB polling rate.** "Online RGB"
  only decides whether PC lighting programs (OpenRGB, SignalRGB) may take over
  the LEDs, and keyboard colour is stored but not used by the firmware (see
  [protocol.md § 5.2.1](../../docs/protocol.md#521-what-the-firmware-does-with-each-id-v3014)).
  The official driver even hides the polling rate.
- ❌ **No display themes.**
- ❌ **No °C/°F or 12/24 h switching on the display.** Chilkey ships two screen
  firmware variants (24 h + °C, 12 h + °F). The unit and the time format are
  fixed when the firmware is built and cannot be changed or read over USB; the
  keyboard converts the °C values and the 24 h time it receives itself (see
  [docs/firmware.md](../../docs/firmware.md)). The UI unit is independent of it.
- ❌ **No other keyboard models.** Only the Chilkey ND104 is supported. The
  code has a model interface (`KeyboardModel` in
  [`crates/nd104-core/src/model/mod.rs`](crates/nd104-core/src/model/mod.rs)),
  so other keyboards with displays could be added.
- ❌ **Not meant to run alongside** the official Chilkey weather plugin (both
  would write to the display), and don't change the keymap in Chilkey's web
  driver while ND104 Studio writes one.

---

## Requirements

| | |
|---|---|
| OS | Windows 10 (21H2+) or Windows 11, 64-bit |
| Connection | Chilkey ND104, connected as listed in [Connection modes](#connection-modes) |
| Firmware | Verified versions, or other versions while the protocol checks pass (see below) |
| Runtime | Microsoft Edge WebView2 (pre-installed on Windows 11 and current Windows 10) |

### Supported firmware

| Part | Verified version | Status |
|---|---|---|
| Keyboard | V3.0.14 (`20260710_123331`) | ✅ verified on hardware (2026-10-31) |
| Screen (24 h / °C) | V1.2.4 (`Mar 17 2026 21:37:00`) | ✅ verified on hardware (2026-10-31) |
| Screen (12 h / °F) | – | not verified (runs as "compatible" if all checks pass) |

Other versions run as **compatible (unverified)** as long as every protocol
check passes.

Firmware sources and the newest files: [docs/firmware.md](../../docs/firmware.md).

## Firmware safety

Before sending anything, ND104 Studio reads the firmware versions of the keyboard
and of the display, using the same queries as the official web driver (see
[docs/protocol.md § 3](../../docs/protocol.md#3-firmware-version-queries)). It
then compares them with a list of versions it has been verified against.

Older or newer firmware is welcome, but the protocol has to behave exactly as
expected. ND104 Studio checks this when the keyboard is connected and on every
write:
- both USB devices have the expected interfaces and report sizes,
- the version queries are answered correctly,
- writes are accepted,
- answers look the way they did on verified firmware.

| Situation | What happens |
|---|---|
| Verified version, all checks pass | **Verified:** normal operation |
| Older/newer version, all checks pass | **Compatible (unverified):** time and weather are synced; you get a one-time notification and the status page shows "unverified firmware" |
| Any check fails (before or during operation) | **Locked:** stops sending immediately. Tray icon turns amber, a notification names the failed check, and the status page explains what to do |
| Version cannot be read | Locked |

A lock is only lifted by re-connecting the keyboard or by
"Check again", and either way all checks run again. There is intentionally
**no "ignore and continue" override**.

Lighting and the own picture use the same checks. They are extras: if the
firmware answers "not supported" or does not answer, the page shows that and
time/weather keep running; a damaged answer locks like everywhere else.

## Firmware update check

Chilkey publishes firmware only as files in public Google Drive folders (the
version text on their support pages is outdated). ND104 Studio therefore checks
**Chilkey's official folders directly**: it takes the newest firmware file per
part – keyboard, display 24 h/°C or 12 h/°F – and compares the build date in
its name with the build date your keyboard reports.

**Fallback:** if a folder cannot be read (no network, Google changed the page,
folder moved), the reviewed version list
[`firmware/versions.json`](../../firmware/versions.json) of this repository is
used for that part. The settings window shows the source per part
(*Chilkey (official)* or *repository list*). A GitHub Action keeps that list
up to date: it checks Chilkey's folders every day and opens an issue on
changes ([details](../../firmware/README.md)).

The check runs:
- when the keyboard is first connected, then once a day,
- on **Check for updates** (settings window: General or Device & Firmware –
  the button checks app and firmware together),
- on the console: `nd104-cli firmware-check`.

The **Device & Firmware** page shows per part: installed, newest available
(with source), state (*up to date*, *update available*, *newer than the
newest file*, *unknown*), the download folder and Chilkey's update
instructions. If an update is available
you get a notification once. Updating stays your decision and is done with
Chilkey's tools – **ND104 Studio never downloads or flashes firmware**. After an
update, ND104 Studio checks the new firmware automatically (verified or
compatible, see [Firmware safety](#firmware-safety)).

The automatic check can be switched off (General → *Check for firmware updates
automatically*, or `[updates] firmware_check = false`).

## Weather providers

| Provider | API key | Coverage | Status |
|---|---|---|---|
| [Open-Meteo](https://open-meteo.com/) (default) | no | worldwide | ✅ available |
| [Bright Sky](https://brightsky.dev/) (DWD data) | no | Germany and nearby | ✅ available |
| [OpenWeatherMap](https://openweathermap.org/) | [free key](https://home.openweathermap.org/users/sign_up) | worldwide | ✅ available |
| [WeatherAPI.com](https://www.weatherapi.com/) | [free key](https://www.weatherapi.com/signup.aspx) | worldwide | ✅ available |
| Manual / Test | – | – | ✅ available |

**Data credits and licences:**

| Data | Source and licence | Credit shown in the app |
|---|---|---|
| Weather (default) | [Open-Meteo](https://open-meteo.com/), [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/); the free API is for non-commercial use | "Weather data by Open-Meteo.com" |
| Weather (Germany) | [Deutscher Wetterdienst](https://www.dwd.de/) open data via [Bright Sky](https://brightsky.dev/), CC BY 4.0 | "Source: Deutscher Wetterdienst, via Bright Sky" |
| Weather (with key) | [OpenWeather](https://openweathermap.org/), [ODbL](https://opendatacommons.org/licenses/odbl/) | "Weather data © OpenWeather" |
| Weather (with key) | [WeatherAPI.com](https://www.weatherapi.com/) | "Powered by WeatherAPI.com" |
| City search | [GeoNames](https://www.geonames.org/) via Open-Meteo geocoding, CC BY 4.0 | "Place names: GeoNames, via Open-Meteo" |

The app shows the credit of the selected provider on the Weather page, the
city search credit below the search, and all of them under **General →
About → Data sources**. Using a provider means accepting its terms.

**Default location:** Berlin, until you pick your own (city search in the
settings, or `nd104-cli geocode "<city>" --set 1`). Your location is stored only
in your local config file, never in this repository.

**Weather icons:** all ten icons of the display are used, including
thunderstorm and the night variants of clear and partly cloudy. Each provider
maps its own codes to one neutral condition, which the keyboard model turns
into the icon (table in [docs/protocol.md § 4.2](../../docs/protocol.md#42-weather-command-0xfe-len-8)).
Open-Meteo and Bright Sky only report "cloudy or overcast"; ND104 Studio uses
their cloud cover to choose: below 88 % (7/8) the single cloud, from 88 %
the overcast icon.

Providers are plug-ins implementing one interface (`WeatherProvider` in
[`crates/nd104-core/src/weather/mod.rs`](crates/nd104-core/src/weather/mod.rs)),
so more can be added.

**API keys** (OpenWeatherMap, WeatherAPI.com):
- Enter them in the settings window → **Weather** → *API key – …* → **Save key**,
  or on the console: `nd104-cli api-key set openweathermap` (the key is read
  from the console, not from the command line).
- They are stored in the **Windows Credential Manager** (entry
  `<provider>.nd104-studio`), never in the config file or the log, and are only
  sent to that provider.
- The window shows only *stored / not stored*; a stored key is never displayed
  again. **Test** can check a key before you save it.
- New OpenWeatherMap keys can take up to two hours to become active.

## Clock sources

| Source | Description |
|---|---|
| System (default) | Local PC time in the PC time zone |
| Time zone | PC time converted to a chosen time zone (default: Germany, `Europe/Berlin`) |
| NTP | Time from an NTP server (default `pool.ntp.org`) in the chosen time zone (default: Germany) |
| Manual | PC time plus a fixed offset, PC time zone |

Summer/winter time: **Auto** (from time-zone rules) · force summer · force winter.

---

## Installation

All options need Windows 10/11 (64-bit) and the keyboard connected (see
[Connection modes](#connection-modes)). Releases are published on the
[releases page](https://github.com/MrUrdam/chilkey_nd104_utilities/releases)
(tag `nd104-studio-vX.Y.Z`).
Each release has:

| File | What | When to use |
|---|---|---|
| `nd104-studio_X.Y.Z_x64-setup.exe` | **Setup (recommended)** – install, update and uninstall; per user, no admin rights, English/German | normal use |
| `nd104-studio_X.Y.Z_x64_en-US.msi` | MSI installer – per machine, needs admin rights | managed PCs, software deployment |
| `nd104-studio-X.Y.Z-windows-x64.zip` | Portable: `nd104-studio.exe` (tray app), `nd104-cli.exe`, `nd104-probe.exe` | no installation, testing |
| `SHA256SUMS.txt` | Checksums of all files | verifying downloads |

### Option A – installer (recommended)

1. Download `nd104-studio_X.Y.Z_x64-setup.exe` from the newest release.
2. Optional: compare its checksum with `SHA256SUMS.txt`:
   `certutil -hashfile nd104-studio_X.Y.Z_x64-setup.exe SHA256`
3. Run it. The setup is not code-signed, so Windows SmartScreen may warn:
   **More info → Run anyway**.
4. Choose whether ND104 Studio starts with Windows (on by default) and whether
   you want a desktop shortcut (off by default), then **Install**. A progress
   bar shows the installation; no admin rights are needed. ND104 Studio is
   installed to `%LOCALAPPDATA%\Programs\nd104-studio` and gets a Start menu
   entry and an entry in Windows **Apps**.
5. **Start ND104 Studio**. Its icon appears in the tray (maybe behind the **^**
   arrow; drag it onto the taskbar to keep it visible). The last page of the
   setup also shows how your keyboard is connected (and follows changes).
6. The settings window opens on first start: **Weather → Location** → search
   your city, check it on the map (🗺) if needed, pick it, **Apply**. Without
   this, Berlin is used.

If the Microsoft Edge WebView2 Runtime is missing (rare: it ships with
Windows 11 and current Windows 10), the setup offers to download and install
it from Microsoft first.

Unattended installation: `nd104-studio_X.Y.Z_x64-setup.exe --silent`
(add `--no-autostart` and/or `--desktop-shortcut`). The exit code is 0 on
success.

Installed nd104-sync 1.0.x with the old installer? The setup takes over that
installation as well (see the next section).

### Coming from nd104-sync

ND104 Studio was called **nd104-sync** up to version 1.5. Installed nd104-sync
versions find ND104 Studio as a normal update (**Settings → General → Install
update**), or simply run the ND104 Studio setup. Either way it:

- closes nd104-sync, installs ND104 Studio to
  `%LOCALAPPDATA%\Programs\nd104-studio` and removes the old program folder,
  its "Apps" entry, Start menu/desktop shortcut and autostart entry
  (autostart and the desktop shortcut are set up again under the new name if
  you had them);
- moves your settings, keymap page settings and profiles from
  `%APPDATA%\nd104-sync` to `%APPDATA%\nd104-studio`, the automatic keymap
  backups from `%LOCALAPPDATA%\nd104-sync` to `%LOCALAPPDATA%\nd104-studio`,
  and the API keys in the Windows Credential Manager to the new name.
  Old logs are deleted.

The app does the same moves on its first start, so the portable zip and the
MSI take your data over too. Anything that already exists under the new name
is kept; old folders are only removed once they are empty. Keymap backups and
profile exports made with nd104-sync can still be restored and imported.

### Option B – portable zip

1. Download and extract `nd104-studio-X.Y.Z-windows-x64.zip`, e.g. to
   `%LOCALAPPDATA%\Programs\nd104-studio`.
2. Start `nd104-studio.exe` (tray app) – it enables "Start with Windows" for
   the exe at this location – or use the console runner `nd104-cli.exe`.

### Option C – your own build

1. [Build from source](#building-from-source).
2. Run `target/release/nd104-studio-setup.exe` (the setup with your build
   inside), or run `target/release/nd104-studio.exe` directly.

In every case the configuration lives in `%APPDATA%\nd104-studio\config.toml`
and the logs in `%LOCALAPPDATA%\nd104-studio\logs` – see [Files](#files).

### Uninstall

Windows Settings → **Apps** → ND104 Studio → **Uninstall**. The setup closes
ND104 Studio if it is running and removes the app, its shortcuts and the
autostart entry. **Keep my settings** is ticked by default: your
configuration, location and API keys stay for a later installation. Untick it
to delete the settings, the stored API keys and the logs as well.

Unattended: `"%LOCALAPPDATA%\Programs\nd104-studio\nd104-studio-setup.exe" --uninstall --silent`
(add `--remove-settings` to delete the settings too).

MSI installation: uninstall it in Windows **Apps** as usual; the configuration
and logs are kept. For the portable zip, untick "Start with Windows" first,
then delete the folder.

## Updating

**Update check:** ND104 Studio asks GitHub for the newest release 30 seconds
after start and then once a day. If a newer version exists, you get a
notification. Nothing is downloaded or installed automatically. Check by
hand:
- Settings window → General → **Check for updates** (always shows a result)
- Console: `nd104-cli check-update`

Only releases tagged `nd104-studio-vX.Y.Z` count; drafts and pre-releases are
ignored.

**How to update:**
- **Setup, one click:** settings window → General → **Install update**.
  ND104 Studio downloads the new setup, checks it against the release's
  `SHA256SUMS.txt`, closes itself and shows a small progress window; it starts
  again on its own when the update is done.
- **Setup, by hand:** download the new `…-setup.exe` and run it. It recognises
  the installed version and offers **Update** (it closes ND104 Studio for you).
  Configuration, location and API keys stay. Running an *older* setup works
  too – it shows a hint and installs that version on request.
- **MSI:** run the new `.msi`; it upgrades the installed version.
- **Portable zip:** quit ND104 Studio, replace the files in your folder.
- **Own build:** `git pull`, build again, install or replace the exe.

Configuration files are versioned (`schema = 1`). A newer app migrates older
files automatically. An older app refuses a file written by a newer schema
instead of misreading it.

## Versioning

- ND104 Studio uses [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`.
  MAJOR = incompatible changes, MINOR = new features, PATCH = fixes.
- **One source of truth:** `version` in [`Cargo.toml`](Cargo.toml)
  (`[workspace.package]`). Every crate, the CLI (`nd104-cli --version`) and the
  app UI read it from there.
- **Release tags:** `nd104-studio-vX.Y.Z` (up to 1.5.0: `nd104-sync-vX.Y.Z`).
  The prefix is needed because this repository hosts several tools.
- **Changes:** listed per version in [CHANGELOG.md](CHANGELOG.md).
- **Where the version is shown:** settings window (General → About), dashboard
  footer, tray tooltip, `nd104-cli --version`, and the first line of each log file.

## Usage

1. Connect the ND104 – the first time with the **USB cable**, then any
   connection from [Connection modes](#connection-modes).
2. Close the official Chilkey web driver / weather plugin.
3. Start ND104 Studio (Start menu, or automatically with Windows). The tray icon
   appears and time and weather are sent right away.
4. Set your location once in the settings window (Weather → Location).

**Tray menu**

| Entry | Action |
|---|---|
| Status line | Keyboard, weather and state at a glance |
| Sync now | Send time and weather immediately (fetches fresh weather) |
| Pause / Resume updates | Stop / resume all updates |
| Settings… | Opens the settings window |
| Start with Windows | Autostart on/off |
| Quit | Exit the app |

Everything else is in the settings window: **Check for updates** (app and
firmware), **language**, **Check keyboard again** (Device & Firmware), **log
folder** and **About** with the version (General).

**Tray icon colours:** cyan = connected & synced · grey = not connected ·
amber = locked (protocol check failed) · red = weather stale.
The **tooltip** shows the app version, keyboard, weather and the last sync.

**Notifications:** keyboard locked, unverified firmware detected, new
ND104 Studio version available, new keyboard firmware available, result of a
manual update check.

### Settings window

Opens via tray → **Settings…**, on first start, when ND104 Studio is started a
second time, or with `nd104-studio.exe --settings`. Closing it keeps the app
running in the tray.

| Page | Contents |
|---|---|
| **Dashboard** | Large live preview of the keyboard display with keyboard state and last/next syncs (incl. next DST change) next to it; weather, firmware, recent events; Sync now, Pause/Resume |
| **Keymap** | Layers (renamable), ISO/ANSI drawing with detection, label sets, function picker for keys, knob and round button, list of changes, write with automatic backup; profiles; keymap backup and restore |
| **Lighting** | Background lighting (under the keys) and bottom lighting (around the keyboard + light bar below Del/End/PgDn): on/off, effect, brightness, speed, colour (sent and stored right away) |
| **Weather** | Location via city search (with OpenStreetMap link to check a result) or coordinates; provider, fallback, interval, °C/°F in the window; Test button (fetches without sending) |
| **Clock** | Source (System / Time zone / NTP / Manual), time zone, NTP test, summer/winter time mode, sync interval; live preview of the time sent |
| **Screen picture** | Choose a picture or GIF, see the checks and a preview (fill the screen or show the whole picture), send it to the screen with a progress bar |
| **Device & Firmware** | Verified / unverified / locked state, installed vs. verified firmware, display variant, Check keyboard again; keyboard settings (debounce, sleep times); polling interval |
| **General** | Start with Windows, language, appearance (like Windows / dark / light), style (Calm / Gamer), update check, About with version, config file and log folder |

Changes take effect with **Apply** (shown only when something changed);
invalid fields are marked. The configuration file can still be edited by hand –
changes are picked up within seconds.

### Console runner `nd104-cli`

Runs the same core on the console – useful for testing and for setting the
location. It uses the same configuration file as the app. Do not run it at the
same time as the tray app (both would send to the keyboard).

```
nd104-cli                            # run continuously (= nd104-cli run), Ctrl+C to stop
nd104-cli status                     # keyboard, firmware, protocol check result
nd104-cli sync                       # send time + weather once, then exit
nd104-cli geocode "Berlin"           # search a city
nd104-cli geocode "Berlin" --set 1   # store result 1 as your location
nd104-cli weather                    # fetch and print the weather (nothing sent)
nd104-cli check-update               # check GitHub for a newer release
nd104-cli firmware-check             # compare keyboard firmware with the version list
nd104-cli api-key status             # which providers have a stored key
nd104-cli api-key set <provider>     # store a key (prompted, not on the command line)
nd104-cli api-key delete <provider>  # delete a key
nd104-cli config path|show|init      # where the config is / print it / create it
nd104-cli --config other.toml …      # use another config file
nd104-cli --version
```

Set `RUST_LOG=debug` for more output.

### Key combinations of the ND104

Set in the keymap (and changeable on the Keymap page). Layer 2 is reached
with **Fn + right Shift**; there the same lighting keys control the bottom
lighting and the display keys the display brightness. ISO DE key names in
brackets.

| Fn + … | Fn + right Shift + … |
|---|---|
| ↑ / ↓: backlight brightness, ← / →: backlight speed | ↑ / ↓: bottom lighting brightness |
| `\|` (`#`): backlight on/off, `]` (`+`): next effect | `\|` (`#`): bottom lighting on/off, `]` (`+`): next effect |
| P / `;` (Ö): hue + / −, `[` (Ü) / `'` (Ä): saturation + / − | the same for the bottom lighting |
| Del or knob press: display on/off; PgUp / PgDn or knob turn: display page | Del or knob press: typed characters on the display; PgUp / PgDn: display brightness |
| Disp: restart display (hold long: display update mode, ends after 3 min idle) | Disp: display factory reset |
| Calc or Enter: display calculator | H: battery brightness limit (50 % / 100 %) |
| Q / W held 3 s: Windows / Mac key layout | |
| Z / X / C: Bluetooth 1–3, V: 2.4 GHz – short press switches, hold to pair; Space: USB | |
| Win: lock Win key, M: charging indicator, N: N-key rollover, Caps: sleep, `/`: lighting test (hold 3 s) | |

Built into the firmware (not in the keymap, so not changeable): **Fn +
Backspace** on battery – the LEDs show the battery level; **Space +
Backspace for 5 s** – keyboard factory reset.

### App command-line options

```
nd104-studio.exe                # normal start (tray)
nd104-studio.exe --minimized    # used by autostart (tray only)
nd104-studio.exe --settings     # open the settings window right away
```

### Files

| What | Where |
|---|---|
| Configuration | `%APPDATA%\nd104-studio\config.toml` |
| Logs | `%LOCALAPPDATA%\nd104-studio\logs\` |
| Keymap page settings, profiles | `%APPDATA%\nd104-studio\keymap-page.json`, `keymap-profiles\` |
| Automatic keymap backups | `%LOCALAPPDATA%\nd104-studio\keymap-backups\` (the last 20) |
| Firmware from the last USB/Bluetooth connection (for Bluetooth and 2.4 GHz) | `%LOCALAPPDATA%\nd104-studio\screen-firmware.json`, `keyboard-firmware.json` |
| API keys | Windows Credential Manager (`nd104-studio/<provider>`) |
| Program (setup) | `%LOCALAPPDATA%\Programs\nd104-studio\` (includes `nd104-studio-setup.exe` for uninstalling) |

## Privacy / network access

ND104 Studio connects only to:
- the weather provider you selected, plus the fallback provider if the first
  one fails (and Open-Meteo geocoding for city search) – API keys are sent only
  to their own provider,
- the NTP server, if you selected NTP as clock source,
- Chilkey's public Google Drive firmware folders (`drive.google.com`) and, as
  a fallback, the version list in this repository (`raw.githubusercontent.com`)
  for the firmware update check – can be disabled,
- the GitHub releases API of this repo for the app update check – can be disabled
  (`[updates] app_check = false`).

No telemetry, no analytics, no IP-based location unless you turn it on.

## Troubleshooting

| Problem | Fix |
|---|---|
| "Not connected" | Use a connection from [Connection modes](#connection-modes); try another USB port; avoid unpowered hubs; wake the keyboard by pressing a key |
| Over Bluetooth or 2.4 GHz it stays at "Connecting …" / "Not connected" | The first time, connect the keyboard once with the USB cable; ND104 Studio remembers the firmware, then Bluetooth and 2.4 GHz work. Over 2.4 GHz also check that the keyboard is switched to 2.4 GHz (Fn + V) and awake |
| Over Bluetooth it shows "Connecting …" now and then | The keyboard is asleep; it connects again as soon as you press a key |
| Amber icon / locked | A protocol check failed (the reason is in the tray status line, the notification and the log). Re-connect the cable or use "Check keyboard again"; if it stays locked, your firmware behaves differently – please open an issue with the log |
| No tray icon | It may be hidden behind the **^** arrow in the taskbar; drag it out. Is ND104 Studio running (Task Manager)? |
| SmartScreen blocks the setup | The setup is not code-signed: **More info → Run anyway** |
| "Install update" is missing | It is offered only for installations made by the setup (1.1.0+). With the MSI or the zip, download the new version from the release page |
| Config or log folder "not available" / settings not found | ND104 Studio was started from a packaged (MSIX) app, e.g. some terminals or IDE integrations; Windows then redirects its AppData into that app's package folder (`%LOCALAPPDATA%\Packages\<app>\LocalCache\…`). Start ND104 Studio from the Start menu or via autostart instead |
| Tray app and `nd104-cli run` at the same time | Only run one of them |
| Display shows other values | The official Chilkey weather plugin is probably still running – close it |
| Wrong time after DST change | Check time zone / DST mode in the Clock tab |

---

## Building from source

### Prerequisites (Windows)

| Tool | Version | Needed for | Notes |
|---|---|---|---|
| [Rust](https://rustup.rs/) | stable, `x86_64-pc-windows-msvc` | everything | install with `rustup-init.exe`, default options |
| Visual Studio Build Tools | 2022 | everything | workload **"Desktop development with C++"** (linker) |
| [Git](https://git-scm.com/) | any | getting the code | |
| [Node.js](https://nodejs.org/) | LTS (≥ 20) | settings window, Tauri CLI | the Tauri CLI comes via npm |
| WebView2 Runtime | current | app | pre-installed on Windows 10/11 |

Quick install with winget (Rust, then the Build Tools with the C++ workload):

```bash
winget install --id Rustlang.Rustup -e
```

```bash
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--passive --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

### Build

```bash
git clone https://github.com/MrUrdam/chilkey_nd104_utilities.git
cd chilkey_nd104_utilities/apps/nd104-studio
npm ci                     # frontend + Tauri CLI
npm run build              # settings window → ui/dist (embedded into the app)
npm run build:installer-ui # setup window → installer-ui/dist (embedded into the setup)
cargo test --workspace     # unit tests, no hardware needed
cargo build --release --workspace --exclude nd104-studio-setup
                           # → target/release/nd104-studio.exe, nd104-cli.exe, nd104-probe.exe
npm run tauri build        # app + MSI → target/release/bundle/msi/*.msi
cargo build --release -p nd104-studio-setup
                           # setup with the app inside → target/release/nd104-studio-setup.exe
```

- `npm run build` must run before any `cargo build` of the app, because the
  window's files are compiled into `nd104-studio.exe`.
- The setup (`installer/`) packs `target/release/nd104-studio.exe`, the console
  tools and the docs into itself, so build it **last**. Built without the app
  it shows its window but refuses to install.
- `npm run tauri build` downloads WiX on first use.
- Development with hot reload: `npm run tauri dev`.
- Frontend type check: `npm run check`.

### Making a release (maintainers)

Stable releases follow SemVer (MAJOR.MINOR.PATCH); pre-releases use `X.Y.Z-rc.N`.

1. Set the version in `Cargo.toml` (`[workspace.package] version`) and run
   `cargo build` so `Cargo.lock` follows.
2. In [CHANGELOG.md](CHANGELOG.md), move the `[Unreleased]` entries into a new
   section `## [X.Y.Z] – YYYY-MM-DD` and add the link at the bottom.
3. Commit, then tag and push:
   `git tag nd104-studio-vX.Y.Z` and `git push origin nd104-studio-vX.Y.Z`.
4. The workflow [`nd104-studio-release.yml`](../../.github/workflows/nd104-studio-release.yml)
   checks that the tag matches `Cargo.toml`, runs the tests, builds the
   console tools, the tray app and the **MSI**, then the **setup**, packs
   the portable zip, writes `SHA256SUMS.txt` and publishes the GitHub release.
   Its notes are the CHANGELOG section for that version plus a download list.
   - `X.Y.Z-rc.N` → GitHub pre-release without MSI (WiX needs a plain version),
     ignored by the app update check.
   - Installed nd104-sync 1.x versions only look for `nd104-sync-v*` tags.
     For them there is one **bridge release** `nd104-sync-v2.0.0` with the
     setup named `nd104-sync_2.0.0_x64-setup.exe`; it updates them to
     ND104 Studio, which then finds all later releases by itself (see
     [Coming from nd104-sync](#coming-from-nd104-sync)). No new bridge
     releases are made.

Every push to `main` also runs [`nd104-studio-ci.yml`](../../.github/workflows/nd104-studio-ci.yml):
frontend type check and build, Rust format check, clippy (warnings are errors)
and tests.

### Main dependencies

| Crate / package | Purpose |
|---|---|
| `tauri` v2 + plugins `autostart`, `single-instance`, `notification` | App shell, tray, window |
| `hidapi` | USB HID communication |
| `tokio` | Async runtime, scheduler |
| `reqwest` (rustls) | HTTP for weather / geocoding / manifest |
| `jiff` | Time zones and DST |
| (own SNTP client) | NTP time source, no extra dependency |
| `serde`, `toml`, `directories` | Configuration |
| `keyring` 3 | API keys in the Windows Credential Manager |
| `tracing`, `tracing-appender` | Logging |
| `sys-locale` | System language detection |
| `zip`, `winreg`, `windows` | Setup: embedded app files, Windows "Apps" entry, shortcuts |
| `sha2` | Checking the downloaded setup ("Install update") |
| Svelte 5 + TypeScript + Vite | Settings window (`ui/`) |
| `@tauri-apps/api`, `@tauri-apps/cli` | Window ↔ app calls, build tooling |
| `@fontsource/rajdhani` | Display font, bundled locally (OFL) |

### Project structure

```
apps/nd104-studio/
├── crates/
│   ├── nd104-protocol/   # packet building, CRC (pure, unit tested)
│   ├── nd104-probe/      # diagnostic CLI for hardware tests
│   ├── nd104-core/       # model interface, device + firmware guard,
│   │                     # clock, weather, scheduler, config, update check
│   └── nd104-cli/        # console runner for the core
├── src-tauri/            # Tauri tray app, icons, MSI config
├── ui/                   # Svelte settings window
├── installer/            # setup: install, update, uninstall (Tauri, own window)
├── installer-ui/         # Svelte setup window
└── locales/              # en.json, de.json (the setup has its own in installer/locales)
```

## Contributing

- **New weather provider:** implement the `WeatherProvider` trait, register it,
  add locale strings and a row in the provider table above.
- **New language:** add `locales/<code>.json` with all keys of `en.json`.
- **New platform (Linux/macOS):** not implemented. Please open an issue
  first to discuss it.
- **New firmware version:** verify the protocol on real hardware, document it in
  `docs/protocol.md`, add it to the verified list.

Contributions are welcome, but reviews and answers happen in my free time.
There is no guaranteed support (see the [disclaimer](../../README.md#disclaimer)).

## Credits

Protocol knowledge is based on the
[ND104 reverse engineering notes](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b)
by borrougagnou and on an analysis of the official
[Chilkey web driver](https://nd104.chilkey.com/). Everything is collected in
[docs/protocol.md](../../docs/protocol.md). Inspired by
[zoom-sync](https://github.com/ozboar/zoom-sync).

**Third-party software:** ND104 Studio is built with open-source packages –
Rust crates (Tauri, hidapi, serde, reqwest …), Svelte, the
[Rajdhani](https://fonts.google.com/specimen/Rajdhani) font (SIL Open Font
License 1.1) and icon shapes from [Lucide](https://lucide.dev) (ISC). Their
licences are listed in [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt),
which ships with every release and is shown in the app under **General →
About → Open-source licences**. It is generated by
[`scripts/third_party_notices.py`](scripts/third_party_notices.py); CI checks
that it is up to date (run the script after changing dependencies).

**Data:** see [Weather providers](#weather-providers) for the data sources
and the credits they require.

Unofficial project, not affiliated with or endorsed by Chilkey. "Chilkey"
and "ND104" are names of their respective owners and are used only to say
which keyboard the app works with. Use at your own risk, see the
[disclaimer](../../README.md#disclaimer). MIT licensed, see [LICENSE](../../LICENSE).
