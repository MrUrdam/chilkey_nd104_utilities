# ND104 Studio

[![Latest release](https://img.shields.io/github/v/release/MrUrdam/chilkey_nd104_utilities?filter=nd104-studio-v*&label=release&color=0969da)](https://github.com/MrUrdam/chilkey_nd104_utilities/releases)
![Windows 10 | 11](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6?logo=windows&logoColor=white)
![Linux x86-64 (beta)](https://img.shields.io/badge/Linux-x86--64%20(beta)-FCC624?logo=linux&logoColor=black)
[![Connection modes: USB, Bluetooth, 2.4 GHz](https://img.shields.io/badge/connection%20modes-matrix-blue)](#connection-modes)
[![CI](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/nd104-studio-ci.yml/badge.svg)](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/nd104-studio-ci.yml)
[![License: MIT](https://img.shields.io/github/license/MrUrdam/chilkey_nd104_utilities)](../../LICENSE)

Everything for the **Chilkey ND104** keyboard in one small tray app: time and
weather on the display, your own picture or GIF, lighting and the keymap.
Set it up once, then forget it.

<sub>Formerly nd104-sync. Updating from 1.x takes everything over, see
[Coming from nd104-sync](docs/installation.md#coming-from-nd104-sync).</sub>

<p align="center">
  <img src="../../docs/images/nd104-studio/dashboard.png" width="760" alt="ND104 Studio settings window, dashboard page with live preview of the keyboard display">
</p>

<p align="center">
  <b>⬇️ <a href="https://github.com/MrUrdam/chilkey_nd104_utilities/releases">Download</a></b> ·
  📦 <a href="docs/installation.md">Install</a> ·
  📖 <a href="docs/usage.md">Usage</a> ·
  🩺 <a href="docs/troubleshooting.md">Troubleshooting</a> ·
  📝 <a href="CHANGELOG.md">Changelog</a>
</p>

> [!WARNING]
> **Unofficial hobby project, use at your own risk.** Not affiliated with
> Chilkey, no warranty, no guaranteed support. Tested only with the firmware
> under [Supported firmware](docs/firmware-safety.md#supported-firmware), on
> Windows 10/11 and Linux (beta), with the connections marked ✅ in
> [Connection modes](#connection-modes). Full text: [disclaimer](../../README.md#disclaimer).

## Features

| | |
|---|---|
| 🕒 **Clock** | Synced on connect, after sleep and exactly at summer/winter time changes. Source: system, time zone, NTP or manual offset |
| 🌦️ **Weather** | Now, max, min and condition icon from 5 providers (2 without API key), with fallback and city search |
| 🖼️ **Screen picture** | Any PNG, JPEG, GIF or BMP, checked and scaled to 320 × 172 before sending |
| 💡 **Lighting** | Effect, brightness, speed, colour. Per-key colours from OpenRGB, Artemis, Aurora and SignalRGB |
| ⌨️ **Keymap** | Keys, knob and round button on all 6 layers, ISO/ANSI, automatic backups, profiles |
| 📶 **Connections** | USB cable, Bluetooth and 2.4 GHz receiver, battery level over Bluetooth |
| 🛡️ **Firmware safety** | Stops the moment the keyboard answers differently. Tells you about new firmware, never flashes |
| 🔄 **Updates** | Notifies about new versions, one-click install for the Windows setup and the AppImage |
| 🖥️ **App** | Tray app with live display preview, English/German, Calm/Gamer style in dark/light, no telemetry |

<details>
<summary><b>All features in detail</b></summary>

**Display**
- Syncs the clock regularly, on connect, after sleep and exactly at
  summer/winter time changes. Sources: system, time zone, NTP or manual
  offset, with automatic or forced DST ([clock sources](docs/usage.md#clock-sources)).
- Fetches the weather for your location (current, max, min and condition
  icon, all 10 icons of the display) from five providers, two of them without
  API key, with a fallback and a city search ([weather providers](docs/usage.md#weather-providers)).
- Own picture or GIF on the screen: any PNG/JPEG/GIF/BMP is checked and
  scaled to 320 × 172 (fill the screen or show the whole picture) before
  sending. USB cable only.

**Keymap**
- Remap keys, the knob and the round button on all 6 layers. Changes are
  collected, reviewed and written together (only the differences), then read
  back to verify.
- Automatic backup before every write (last 20 kept), manual backup and
  restore to a file, profiles (save, load, export, import).
- ISO or ANSI drawing with detection, several label sets, renamable layers.
  Pressed keys and the single-colour lighting light up in the drawing, next
  to the live screen.

**Lighting and keyboard settings**
- Background and bottom lighting: on/off, effect, brightness, speed, colour.
  Sent and stored in the keyboard right away.
- Keyboard settings (debounce mode and time, sleep and deep-sleep time) with
  exactly the values Chilkey's web driver offers.
- Per-key colours from PC lighting programs: OpenRGB, Artemis and Aurora
  through a built-in OpenRGB interface, SignalRGB (Windows) through a plugin.
  See [PC lighting software](#pc-lighting-software-openrgb-signalrgb).

**Connections**
- USB cable, Bluetooth (3 devices) and the 2.4 GHz receiver, with automatic
  switching to the best connection. What works where: [Connection modes](#connection-modes).
- Battery level on the dashboard and in the live preview while Bluetooth is
  connected.

**Safety and updates**
- Firmware safety lock: works with older and newer firmware too, but stops
  the moment the protocol behaves differently
  ([Firmware safety](docs/firmware-safety.md#firmware-safety)). Answers meant
  for other programs are ignored.
- Shows the installed firmware and checks for firmware updates against
  Chilkey's official folders. Never downloads or flashes anything.
- Checks for new ND104 Studio releases and installs them with one click
  (Windows setup, Linux AppImage), see [Updating](docs/installation.md#updating).

**App**
- Starts at login and lives in the system tray. Status dashboard with a live
  preview of the display.
- API keys in the system's secure store (Windows Credential Manager, Linux
  Secret Service).
- UI in English and German, two styles (Calm / Gamer) each in dark and light,
  collapsible sidebar.
- Console runner `nd104-cli` for scripting and testing.
- No telemetry ([Privacy](docs/troubleshooting.md#privacy--network-access)).

</details>

## Screenshots

<!-- page order as in the sidebar; 3 columns, 250 px each -->
<table>
  <tr>
    <td align="center" valign="top" width="33%"><a href="../../docs/images/nd104-studio/keymap.png"><img src="../../docs/images/nd104-studio/keymap.png" width="250" alt="Keymap page"></a><br><sub>Keymap: layers, ISO/ANSI, keys, knob, round button</sub></td>
    <td align="center" valign="top" width="33%"><a href="../../docs/images/nd104-studio/lighting.png"><img src="../../docs/images/nd104-studio/lighting.png" width="250" alt="Lighting page"></a><br><sub>Lighting: effects, colour, PC lighting software</sub></td>
    <td align="center" valign="top" width="33%"><a href="../../docs/images/nd104-studio/weather.png"><img src="../../docs/images/nd104-studio/weather.png" width="250" alt="Weather page"></a><br><sub>Weather: provider, location search, test</sub></td>
  </tr>
  <tr>
    <td align="center" valign="top" width="33%"><a href="../../docs/images/nd104-studio/clock.png"><img src="../../docs/images/nd104-studio/clock.png" width="250" alt="Clock page"></a><br><sub>Clock: source, time zone, summer/winter time</sub></td>
    <td align="center" valign="top" width="33%"><a href="../../docs/images/nd104-studio/picture.png"><img src="../../docs/images/nd104-studio/picture.png" width="250" alt="Screen picture page"></a><br><sub>Screen picture: checks, animated preview, send</sub></td>
    <td align="center" valign="top" width="33%"><a href="../../docs/images/nd104-studio/device.png"><img src="../../docs/images/nd104-studio/device.png" width="250" alt="Device and Firmware page"></a><br><sub>Device & Firmware: safety state, update check</sub></td>
  </tr>
  <tr>
    <td align="center" valign="top" colspan="3"><a href="../../docs/images/nd104-studio/general.png"><img src="../../docs/images/nd104-studio/general.png" width="250" alt="General page"></a><br><sub>General: autostart, language, theme, style, updates</sub></td>
  </tr>
</table>

## Quick start

| | 🪟 Windows 10 / 11 | 🐧 Linux x86-64 (beta) |
|---|---|---|
| **Download** | `…_x64-setup.exe` | `.deb` (Debian, Ubuntu), `.rpm` (Fedora, openSUSE) or the AppImage |
| **Install** | Run it. SmartScreen warns because it is not code-signed: *More info → Run anyway* | `.deb`: `sudo apt install ./….deb`. AppImage: install the [udev rule](docs/installation.md#keyboard-access--the-udev-rule) once |
| **Needs** | WebView2 (pre-installed) | glibc 2.35+ (Ubuntu 22.04+, Debian 12+, Fedora 41+), WebKitGTK 4.1, a tray host. The packages pull these in |

Then:
1. 🔌 Connect the keyboard **once by USB cable**. After that, Bluetooth and 2.4 GHz work too.
2. 🚫 Close Chilkey's web driver and weather plugin.
3. 📍 In the settings window: **Weather → Location**, search your city, **Apply**.

All install types (MSI, portable zip, tarball), updating and uninstalling:
**[Installation & updating](docs/installation.md)**.

## Documentation

| Guide | What's in it |
|---|---|
| 📦 [Installation & updating](docs/installation.md) | Every install type for Windows and Linux, the udev rule, updating, uninstalling |
| 📖 [Usage & reference](docs/usage.md) | Tray and settings window, `nd104-cli`, PC lighting setup, key combinations, files, weather providers, clock sources |
| 🛡️ [Firmware: safety & updates](docs/firmware-safety.md) | Supported versions, the safety lock, the firmware update check |
| 🩺 [Troubleshooting & privacy](docs/troubleshooting.md) | Common problems and fixes, what the app connects to |
| 🛠️ [Building from source](docs/building.md) | Build on Windows and Linux, release pipeline, project layout |
| 📝 [Changelog](CHANGELOG.md) | Changes per version |

## Connection modes

The ND104 connects by USB cable, Bluetooth (3 devices) or 2.4 GHz receiver.
**This table is the one place that says what works on which connection.** The
rest of the documentation, the app and the setup point here.

| Feature | 🔌 USB cable | 📶 Bluetooth | 📡 2.4 GHz |
|---|:-:|:-:|:-:|
| Time and weather on the display | ✅ | ✅ | ✅ |
| Own picture or GIF on the screen | ✅ | ❌ | ❌ |
| Lighting | ✅ | ✅ | ✅ |
| Per-key colours from PC lighting programs ([details](#pc-lighting-software-openrgb-signalrgb)) | ✅ | ❌ | ❌ |
| Keymap (remap, backup, profiles, pressed keys in the drawing) | ✅ | ✅ | ✅ |
| Keyboard settings (debounce, sleep times) | ✅ | ✅ | ✅ |
| Keyboard firmware version and firmware update check | ✅ | ✅ | ☑️ ¹ |
| Screen firmware version | ✅ | ☑️ ¹ | ☑️ ¹ |
| Battery level (app and live preview) | ☑️ ² | ✅ | ❌ |
| App updates, weather download, settings window | ✅ | ✅ | ✅ |

✅ works · ☑️ with a limitation (footnote) · ❌ not available

¹ Taken from the last USB connection (the keyboard version also from the last
Bluetooth connection). **Connect the keyboard once by cable** first. Until
then ND104 Studio waits ("Connecting …") and says so. It does not lock.<br>
² Over the cable the keyboard does not report its battery level. It is shown
as long as the keyboard is also connected via Bluetooth.

<details>
<summary><b>Priority, sleep and how it works</b></summary>

- **Priority:** with several connections at once, ND104 Studio uses the
  cable first, then Bluetooth, then 2.4 GHz (and still shows the battery
  level from Bluetooth). Switching to a higher one also ends a lock that
  happened on the lower one, and all checks run again.
- **Sleep and off:** when the keyboard sleeps, the system drops the Bluetooth
  link. Over 2.4 GHz the receiver stays but the keyboard stops answering.
  Either way ND104 Studio shows "Not connected" or "Connecting …" and connects
  again as soon as the keyboard answers (press a key). No lock: only a wrong
  answer locks, like over USB.
- **Display:** the sidebar shows the current mode in green ("USB",
  "Bluetooth", "2.4 GHz"), the tray status line adds "(Bluetooth)" or
  "(2.4 GHz)". The same overview is in the app: **Device → Connection modes**.
- **How it works:** over Bluetooth the keyboard offers the same command
  channel as over USB (HID over Bluetooth LE, one extra report ID byte). The
  2.4 GHz receiver offers it too and forwards every command over the air. In
  both cases the keyboard passes time and weather on to the screen as it
  does over the cable. The screen controller itself is a second USB device
  inside the keyboard and reachable by cable only. Hence picture and screen
  version need the cable, and over 2.4 GHz the version query reaches only the
  receiver. Details: [protocol.md § 5.5](../../docs/protocol.md#55-over-bluetooth-analysis-2026-11-02)
  and [§ 5.6](../../docs/protocol.md#56-over-the-24-ghz-receiver-analysis-2026-11-02).

</details>

## Platform support

ND104 Studio runs on **Windows 10/11** and, from 2.4.0, on **Linux (beta)**.
This table is the one place that says what differs between them. Everything
else works the same.

| Feature | 🪟 Windows 10/11 | 🐧 Linux (beta) |
|---|:-:|:-:|
| OpenRGB interface (OpenRGB, Artemis …) | ✅ | ✅ |
| SignalRGB plugin | ✅ | ❌ ¹ |
| Tray icon | ✅ | ☑️ ² |
| Start at login | ✅ | ✅ |
| API keys in the system's secure store | ✅ Credential Manager | ✅ Secret Service (GNOME Keyring, KWallet) |
| Keyboard access without admin rights | ✅ | ☑️ ³ |
| Bluetooth connection and battery | ✅ | ✅ (BlueZ) |
| Updates from within the app | ✅ | ☑️ ⁴ |

✅ works · ☑️ with a limitation (footnote) · ❌ not available

¹ SignalRGB (and Aurora) exist for Windows only. On Linux use the OpenRGB
interface.<br>
² GNOME shows tray icons only with the AppIndicator extension (preinstalled
on Ubuntu). On Linux a click opens the menu and there is no tooltip.
ND104 Studio can also be opened from the app menu.<br>
³ Needs the udev rule (the `.deb`/`.rpm` install it), see
[the udev rule](docs/installation.md#keyboard-access--the-udev-rule).<br>
⁴ The AppImage updates itself. `.deb`/`.rpm` packages are updated by the
system's package manager, see [Updating](docs/installation.md#updating).

The same overview is in the app: **Device → Operating systems**. Progress of
the Linux port: [linux-port.md](../../docs/linux-port.md).

## PC lighting software (OpenRGB, SignalRGB)

Besides its own effects, the ND104 can let a lighting program on the PC
colour all 130 LEDs individually: the keys, the light bar under
Del / End / Page Down and the edge glow. 🔌 USB cable only.

| Program | Works with the ND104 |
|---|---|
| **OpenRGB**, **Artemis**, **Aurora** and other OpenRGB SDK programs | ✅ through ND104 Studio's OpenRGB interface |
| **SignalRGB** (Windows only) | ✅ with the plugin from ND104 Studio |
| OpenRGB on its own (device detection) | ❌ it talks to the keyboard's VIA channel, not its lighting channel |
| Windows Dynamic Lighting | ❌ the keyboard does not answer Windows' lighting request (firmware V3.0.14) |

Switch it on with **Lighting → PC lighting software → *Allowed*** (or
Fn + Shift + L on the keyboard). Setup steps:
[Usage → PC lighting software](docs/usage.md#pc-lighting-software--setup).

## What it does NOT do

- ❌ **No firmware flashing.** It only tells you an update exists.
- ❌ **No risky communication.** If anything looks different, it stops and
  tells you why. There is no "ignore and continue" switch.
- ❌ **No macOS.** Windows 10/11 and Linux (beta) only.
- ❌ **No macros, tap/hold or combos.** Those stay with the official
  [Chilkey driver](https://chilkey.com/pages/console).
- ❌ **No other keyboard models.** Only the Chilkey ND104.

<details>
<summary><b>All limits in detail</b></summary>

- **Not every feature on every connection**, see [Connection modes](#connection-modes).
- **Macros:** recording or editing macros stays with the official Chilkey
  driver. Keymap backups and profiles do not contain them.
- **Tap/hold and combos:** keys get any single function. Existing
  combinations are shown and kept, but not created.
- **No automatic profile switching:** profiles are loaded by hand, because
  every switch wears the keyboard's memory.
- **No keyboard colour or USB polling rate setting.** Keyboard colour is
  stored but not used by the firmware (see
  [protocol.md § 5.2.1](../../docs/protocol.md#521-what-the-firmware-does-with-each-id-v3014)).
  Chilkey's "Online RGB" option is there, as the *PC lighting software* switch.
- **No display themes, no °C/°F or 12/24 h switching on the display:**
  Chilkey ships two screen firmware variants with the unit and time format
  fixed at build time. The keyboard converts the °C values and 24 h time it
  receives itself (see [docs/firmware.md](../../docs/firmware.md)).
- **Other keyboard models:** the code has a model interface (`KeyboardModel`
  in [`crates/nd104-core/src/model/mod.rs`](crates/nd104-core/src/model/mod.rs)),
  but only the ND104 is implemented.
- **Not meant to run alongside** the official Chilkey weather plugin. Don't
  change the keymap in Chilkey's web driver while ND104 Studio writes one.

</details>

## Versioning

[Semantic Versioning](https://semver.org/): MAJOR = incompatible changes,
MINOR = new features, PATCH = fixes. Release tags are `nd104-studio-vX.Y.Z`.

<details>
<summary><b>Details</b></summary>

- **One source of truth:** `version` in [`Cargo.toml`](Cargo.toml)
  (`[workspace.package]`). Every crate, the CLI (`nd104-cli --version`) and the
  app UI read it from there.
- **Tags:** `nd104-studio-vX.Y.Z` (up to 1.5.0: `nd104-sync-vX.Y.Z`). The
  prefix is needed because this repository hosts several tools.
- **Pre-releases** (`X.Y.Z-beta.N`, `X.Y.Z-rc.N`) are GitHub pre-releases.
  The in-app update check offers them only to an installed pre-release (beta
  channel), never to a stable install.
- **Changes:** listed per version in [CHANGELOG.md](CHANGELOG.md).

</details>

## Contributing

Issues and pull requests are welcome. Reviews and answers happen in my free
time, there is no guaranteed support (see the [disclaimer](../../README.md#disclaimer)).
Build instructions: [Building from source](docs/building.md).

- 🌦️ **New weather provider:** implement the `WeatherProvider` trait, register
  it, add locale strings and a row in the provider table.
- 🌐 **New language:** add `locales/<code>.json` with all keys of `en.json`.
- 🧪 **New firmware version:** verify the protocol on real hardware, document it
  in `docs/protocol.md`, add it to the verified list.

## Credits

Protocol knowledge is based on the
[ND104 reverse engineering notes](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b)
by borrougagnou and on an analysis of the official
[Chilkey web driver](https://nd104.chilkey.com/), collected in
[docs/protocol.md](../../docs/protocol.md). Inspired by
[zoom-sync](https://github.com/ozboar/zoom-sync). Weather and place data
sources and their required credits: [Weather providers](docs/usage.md#weather-providers).

<details>
<summary><b>Built with, and third-party licences</b></summary>

![Rust](https://img.shields.io/badge/Rust-1.80%2B-000000?logo=rust&logoColor=white)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)
![Vite](https://img.shields.io/badge/Vite-646CFF?logo=vite&logoColor=white)

ND104 Studio is built with open-source packages: Rust crates (Tauri, hidapi,
serde, reqwest …), Svelte, the [Rajdhani](https://fonts.google.com/specimen/Rajdhani)
font (SIL Open Font License 1.1) and icon shapes from [Lucide](https://lucide.dev) (ISC).
Their licences are listed in [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt),
which ships with every release and is shown in the app under **General →
About → Open-source licences**. It is generated by
[`scripts/third_party_notices.py`](scripts/third_party_notices.py). CI checks
that it is up to date, so run the script after changing dependencies.

</details>

Unofficial project, not affiliated with or endorsed by Chilkey. "Chilkey"
and "ND104" are names of their respective owners and are used only to say
which keyboard the app works with. MIT licensed, see [LICENSE](../../LICENSE).
