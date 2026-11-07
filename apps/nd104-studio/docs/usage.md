# Usage & reference

[← Back to the README](../README.md) · [Installation](installation.md) · [Troubleshooting](troubleshooting.md)

| | |
|---|---|
| 🚀 [Getting started](#getting-started) | first steps, tray menu, icon colours, notifications |
| 🖥️ [Settings window](#settings-window) | what each page does |
| ⌨️ [Console runner `nd104-cli`](#console-runner-nd104-cli) | all commands |
| 🎨 [PC lighting software – setup](#pc-lighting-software--setup) | OpenRGB, Artemis, Aurora, SignalRGB |
| 🔑 [Key combinations of the ND104](#key-combinations-of-the-nd104) | Fn shortcuts of the keyboard |
| ▶️ [App command-line options](#app-command-line-options) | `--minimized`, `--settings` |
| 📁 [Files](#files) | config, logs, backups per OS |
| 🌦️ [Weather providers](#weather-providers) | providers, API keys, data credits |
| 🕒 [Clock sources](#clock-sources) | system, time zone, NTP, manual |

## Getting started

1. Connect the ND104 – the first time with the **USB cable**, then any
   connection from [Connection modes](../README.md#connection-modes).
2. Close the official Chilkey web driver / weather plugin.
3. Start ND104 Studio (Start menu / application menu, or automatically at
   login). The tray icon appears and time and weather are sent right away.
4. Set your location once in the settings window (Weather → Location).

**Tray menu**

| Entry | Action |
|---|---|
| Status line | Keyboard, weather and state at a glance |
| Sync now | Send time and weather immediately (fetches fresh weather) |
| Pause / Resume updates | Stop / resume all updates |
| Settings… | Opens the settings window |
| Start with Windows / Start at login | Autostart on/off |
| Quit | Exit the app |

Everything else is in the settings window: **Check for updates** (app and
firmware), **language**, **Check keyboard again** (Device & Firmware), **log
folder** and **About** with the version (General).

**Tray icon colours:** cyan = connected & synced · grey = not connected ·
amber = locked (protocol check failed) · red = weather stale.
The **tooltip** shows the app version, keyboard, weather and the last sync.
(On GNOME there is no tooltip; a click opens the menu – see
[Troubleshooting](troubleshooting.md).)

**Notifications:** keyboard locked, unverified firmware detected, new
ND104 Studio version available, new keyboard firmware available, result of a
manual update check.

## Settings window

Opens via tray → **Settings…**, on first start, when ND104 Studio is started a
second time, or with `nd104-studio --settings`. Closing it keeps the app
running in the tray.

| Page | Contents |
|---|---|
| **Dashboard** | Large live preview of the keyboard display with keyboard state and last/next syncs (incl. next DST change) next to it; weather, firmware, recent events; Sync now, Pause/Resume |
| **Keymap** | Layers (renamable), ISO/ANSI drawing with detection, label sets, function picker for keys, knob and round button, list of changes, write with automatic backup; profiles; keymap backup and restore |
| **Lighting** | Background lighting (under the keys) and bottom lighting (around the keyboard + light bar below Del/End/PgDn): on/off, effect, brightness, speed, colour (sent and stored right away); *PC lighting software* switch, OpenRGB interface, SignalRGB plugin |
| **Weather** | Location via city search (with OpenStreetMap link to check a result) or coordinates; provider, fallback, interval, °C/°F in the window; Test button (fetches without sending) |
| **Clock** | Source (System / Time zone / NTP / Manual), time zone, NTP test, summer/winter time mode, sync interval; live preview of the time sent |
| **Screen picture** | Choose a picture or GIF, see the checks and a preview (fill the screen or show the whole picture), send it to the screen with a progress bar |
| **Device & Firmware** | Verified / unverified / locked state, installed vs. verified firmware, display variant, firmware update check, Check keyboard again; keyboard settings (debounce, sleep times); polling interval; overviews *Connection modes* and *Operating systems* |
| **General** | Start at login, language, appearance (like the system / dark / light), style (Calm / Gamer), update check, About with version, config file and log folder |

Changes take effect with **Apply** (shown only when something changed);
invalid fields are marked. The configuration file can still be edited by hand –
changes are picked up within seconds.

## Console runner `nd104-cli`

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

## PC lighting software – setup

Background on which programs work (and the connection limits) is in the README
under [PC lighting software](../README.md#pc-lighting-software-openrgb-signalrgb).
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

### SignalRGB (Windows only)

1. **Install plugin** in the *PC lighting software* card. It goes to
   `Documents\WhirlwindFX\Plugins\Chilkey_ND104.js`, where SignalRGB keeps
   it across updates.
2. Restart SignalRGB (or reload its plugins). The ND104 appears under
   *Devices*; place it on the canvas.

### Good to know

- The colours are not stored in the keyboard. Any change on the Lighting
  page also ends a program's control.
- Both use the same LED table, read from the keyboard:
  [`signalrgb/layout.json`](../signalrgb/layout.json), made by
  [`make_plugin.py`](../signalrgb/make_plugin.py) from
  [`leds.json`](../signalrgb/leds.json); the plugin is
  [`signalrgb/Chilkey_ND104.js`](../signalrgb/Chilkey_ND104.js).
- Technical details: [protocol.md § 5.4](../../../docs/protocol.md#54-other-keyboard-commands-not-used).

## Key combinations of the ND104

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

## App command-line options

```
nd104-studio                # normal start (tray)
nd104-studio --minimized    # used by autostart (tray only)
nd104-studio --settings     # open the settings window right away
```

(On Windows the binary is `nd104-studio.exe`.)

## Files

| What | Windows | Linux (XDG) |
|---|---|---|
| Configuration | `%APPDATA%\nd104-studio\config.toml` | `~/.config/nd104-studio/config.toml` |
| Keymap page settings, profiles | `%APPDATA%\nd104-studio\keymap-page.json`, `keymap-profiles\` | `~/.config/nd104-studio/keymap-page.json`, `keymap-profiles/` |
| Logs | `%LOCALAPPDATA%\nd104-studio\logs\` | `~/.local/share/nd104-studio/logs/` |
| Automatic keymap backups (last 20) | `%LOCALAPPDATA%\nd104-studio\keymap-backups\` | `~/.local/share/nd104-studio/keymap-backups/` |
| Remembered firmware (for Bluetooth / 2.4 GHz) | `%LOCALAPPDATA%\nd104-studio\screen-firmware.json`, `keyboard-firmware.json` | `~/.local/share/nd104-studio/…` |
| API keys | Windows Credential Manager (shown as `<provider>.nd104-studio`) | Secret Service (GNOME Keyring, KWallet); fallback file `~/.config/nd104-studio/secrets` (mode 0600) |
| Program | `%LOCALAPPDATA%\Programs\nd104-studio\` (incl. `nd104-studio-setup.exe` for uninstalling) | where the package manager put it, or your AppImage / tarball |

## Weather providers

| Provider | API key | Coverage |
|---|---|---|
| [Open-Meteo](https://open-meteo.com/) (default) | no | worldwide |
| [Bright Sky](https://brightsky.dev/) (DWD data) | no | Germany and nearby |
| [OpenWeatherMap](https://openweathermap.org/) | [free key](https://home.openweathermap.org/users/sign_up) | worldwide |
| [WeatherAPI.com](https://www.weatherapi.com/) | [free key](https://www.weatherapi.com/signup.aspx) | worldwide |
| Manual / Test | – | – |

<details>
<summary><b>Data credits and licences</b></summary>

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

</details>

**Default location:** Berlin, until you pick your own (city search in the
settings, or `nd104-cli geocode "<city>" --set 1`). Your location is stored only
in your local config file, never in this repository.

**Weather icons:** all ten icons of the display are used, including
thunderstorm and the night variants of clear and partly cloudy. Each provider
maps its own codes to one neutral condition, which the keyboard model turns
into the icon (table in [docs/protocol.md § 4.2](../../../docs/protocol.md#42-weather-command-0xfe-len-8)).
Open-Meteo and Bright Sky only report "cloudy or overcast"; ND104 Studio uses
their cloud cover to choose: below 88 % (7/8) the single cloud, from 88 %
the overcast icon.

Providers are plug-ins implementing one interface (`WeatherProvider` in
[`crates/nd104-core/src/weather/mod.rs`](../crates/nd104-core/src/weather/mod.rs)),
so more can be added.

**API keys** (OpenWeatherMap, WeatherAPI.com):
- Enter them in the settings window → **Weather** → *API key – …* → **Save key**,
  or on the console: `nd104-cli api-key set openweathermap` (the key is read
  from the console, not from the command line).
- They are stored in the system's secure store (service `nd104-studio`,
  account `<provider>`, see [Files](#files)), never in the config file or the
  log, and are only sent to that provider.
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
