# Changelog – nd104-sync

All notable changes to nd104-sync are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/) (see [README → Versioning](README.md#versioning)).

- Release tags: `nd104-sync-vX.Y.Z`. Each tag builds a GitHub release whose
  notes are the matching section below.
- 0.1.0–0.5.0 were the beta releases of the milestones M1–M5.
- **1.0.0 is the first stable release**; from now on: MAJOR = incompatible
  changes, MINOR = new features, PATCH = fixes.

## [Unreleased]

## [1.0.0] – 2026-10-31 · Stable

> **First stable release.** nd104-sync keeps the display of the
> **Chilkey ND104** supplied with the current time and weather – from a small
> tray app with a settings window. **Wired USB only, Windows 10/11.**
> Unofficial project, not affiliated with Chilkey.
>
> This section summarises everything since the start of the project
> (0.1.0–0.5.0 were the betas of the milestones M1–M5; M0 and M6 were
> research). Details per beta follow below.

### Keyboard display
- **Time sync**: PC time, a chosen time zone (default Germany when needed),
  NTP or a manual offset; summer/winter time automatic or forced; re-sync
  every hour, on connect, after the PC wakes from sleep and exactly at DST
  changes.
- **Weather**: current, high and low temperature plus condition – all **10
  display icons** including thunderstorm and day/night variants, correct
  **negative temperatures** (sign + magnitude encoding found on hardware).
- **Five weather providers**: Open-Meteo (default), Bright Sky (DWD),
  OpenWeatherMap and WeatherAPI.com (free API keys), Manual/Test; optional
  fallback provider; retry with backoff; location via city search with an
  **OpenStreetMap check** or coordinates (default Berlin).

### Safety
- **Firmware and protocol checks P1–P6** on connect and on every packet:
  verified firmware runs normally, other firmware runs as *compatible* while
  every check passes, any deviation **locks immediately** – nothing is sent
  until the keyboard is re-plugged or re-checked. Unknown but well-formed
  messages are logged and ignored. No override.
- Verified on hardware: keyboard **V3.0.14**, display **V1.2.4** (24 h / °C).
- nd104-sync **never flashes firmware** and never changes keyboard settings.

### Updates
- **App update check** against the GitHub releases of this repository.
- **Firmware update check against Chilkey's official download folders**
  (build date per part, display per variant), with the reviewed list
  [`firmware/versions.json`](../../firmware/versions.json) as fallback; daily
  **firmware watch** on GitHub opens an issue when Chilkey publishes new files.
- One **Check for updates** button for app and firmware; notifications when
  something new is available.

### App
- **Tray app** with a colour-coded icon, tooltip with the **version**, a short
  menu (Sync now, Pause/Resume, Settings, Start with Windows, Quit) and
  **notifications** (lock, unverified firmware, updates).
- **Settings window**: Dashboard with a **live preview of the keyboard
  display**, Weather, Clock, Device & Firmware, General; Apply/Cancel with
  validation; **dark / light / like Windows**; **German and English**
  (system language by default).
- **Start with Windows**, single instance, automatic reload of the config
  file, rolling log files (7 days).
- **API keys in the Windows Credential Manager** – never in the config file
  or the log; request URLs (coordinates, keys) never appear in messages.

### Installation and tools
- Every release ships an **NSIS installer** (per user, no admin rights), an
  **MSI**, a **portable zip** and `SHA256SUMS.txt`, built by GitHub Actions.
- **`nd104-cli`** – the core on the console (`run`, `status`, `sync`,
  `weather`, `geocode`, `api-key`, `check-update`, `firmware-check`, `config`).
- **`nd104-probe`** – diagnostic tool for protocol tests on real hardware.
- Documentation: [protocol](../../docs/protocol.md),
  [firmware sources](../../docs/firmware.md),
  [keyboard API capabilities](../../docs/keyboard-api.md).

### Findings (M6)
- **Battery level is not available over the USB interfaces**; only Bluetooth
  could provide it, which is outside the wired-only scope.
- The display **theme command is rejected** by screen firmware V1.2.4
  (status `FC`); themes are not offered. `nd104-probe theme` remains for
  diagnosis.

### Known limitations
- Wired USB only (no 2.4 GHz, no Bluetooth); Windows only (Linux/macOS
  prepared, not implemented); Chilkey ND104 only (model interface prepared).
- No image/GIF upload yet; installers are not code-signed (SmartScreen asks).

### Beta history
| Version | Milestone | Highlights |
|---|---|---|
| [0.1.0](https://github.com/MrUrdam/chilkey_nd104_utilities/releases/tag/nd104-sync-v0.1.0) | M1 | Core, console runner, protocol checks |
| [0.2.0](https://github.com/MrUrdam/chilkey_nd104_utilities/releases/tag/nd104-sync-v0.2.0) | M2 | Tray app, installers |
| [0.3.0](https://github.com/MrUrdam/chilkey_nd104_utilities/releases/tag/nd104-sync-v0.3.0) | M3 | Settings window |
| [0.4.0](https://github.com/MrUrdam/chilkey_nd104_utilities/releases/tag/nd104-sync-v0.4.0) | M4 | More weather providers, API keys |
| [0.5.0](https://github.com/MrUrdam/chilkey_nd104_utilities/releases/tag/nd104-sync-v0.5.0) | M5 | Firmware update check, slim tray menu |

## [0.5.0] – 2026-10-31 · M5 · Beta

> **Beta – milestone M5: firmware update check.**
> **Chilkey ND104 only, wired USB only, Windows 10/11.**

### Added
- **Firmware update check against Chilkey's official sources:** nd104-sync
  reads Chilkey's public firmware download folders (Google Drive), takes the
  newest file per part (keyboard, display 24 h/°C or 12 h/°F) and compares its
  build date with the one reported by the keyboard. Only if a folder cannot be
  read, the reviewed version list
  [`firmware/versions.json`](../../firmware/versions.json) of this repository is
  used as a fallback – the source is shown per part. Runs after the keyboard
  is first connected, then daily, and with **Check for updates** (settings
  window) – one button checks both app and firmware.
- Notification when new keyboard firmware is available (once per new version;
  manual checks always show the result).
- **Device & Firmware page:** newest version and its source, state per part
  (up to date / update available / newer than the list / unknown), link to
  the download folder and to Chilkey's update instructions.
- General page: firmware state and a switch for the automatic firmware check.
- Console: `nd104-cli firmware-check`.
- **Firmware watch** (GitHub Action, daily): checks Chilkey's Google Drive
  folders and support pages and opens an issue when files or versions change.
  The version list is only updated after review (see
  [firmware/README.md](../../firmware/README.md)).
- `ND104_FIRMWARE_MANIFEST` environment variable to test a version list
  (file or URL) before it is published.

### Changed
- **Slimmer tray menu:** status, Sync now, Pause/Resume, Settings…, Start with
  Windows, Quit. Check for updates, Check keyboard again, Language, Open log
  folder and About are in the settings window only.

### Notes
- nd104-sync still never downloads or flashes firmware.
- The firmware check contacts `drive.google.com` (Chilkey's folders) and, as a
  fallback, `raw.githubusercontent.com`; no personal data is sent.

## [0.4.0] – 2026-10-31 · M4 · Beta

> **Beta – milestone M4: more weather providers and API keys.**
> **Chilkey ND104 only, wired USB only, Windows 10/11.**

### Added
- **Weather providers:**
  - **Bright Sky** (data of the German weather service DWD) – no API key,
    Germany and nearby.
  - **OpenWeatherMap** – free API key, worldwide; today's max/min from the
    forecast of the local day.
  - **WeatherAPI.com** – free API key, worldwide; same condition codes as the
    official Chilkey plugin.
- **API keys in the Windows Credential Manager** – never in the config file or
  the log. In the settings window (Weather): key field per provider with
  status (stored / not stored), Save, Delete and a link to get a free key. The
  stored key is never shown again. **Test** also works with a key that has not
  been saved yet.
- Fallback provider with its own key: if the main provider fails, the fallback
  is tried with its key.
- Console: `nd104-cli api-key set|delete|status` (the key is read from the
  console, not from the command line, so it does not end up in the shell
  history).
- Coverage hint per provider in the settings window.

### Changed
- HTTP errors name the provider and explain rejected keys (401/403) and rate
  limits (429). Request URLs – which contain the key or the coordinates – are
  never included in messages or logs.

## [0.3.0] – 2026-10-31 · M3 · Beta

> **Beta – milestone M3: settings window.** All settings can now be changed in
> a window – no more editing the configuration file by hand.
> **Chilkey ND104 only, wired USB only, Windows 10/11.**

### Added
- **Settings window** (tray → **Settings…**; also opens on first start, when
  nd104-sync is started a second time, or with `--settings`). Closing it keeps
  the tray app running.
  - **Dashboard:** live preview of the keyboard display (time keeps running
    from the last sync, weather icon and temperatures as sent), keyboard state
    with firmware badge, last/next syncs incl. the next DST change, weather,
    recent events; Sync now and Pause/Resume.
  - **Weather:** city search with a result list and an **OpenStreetMap link**
    per result to check the place, or coordinates by hand; provider and
    fallback provider, interval, °C/°F for the window, values for the manual
    test provider, and a **Test** button that fetches without sending.
  - **Clock:** source (System / Time zone / NTP / Manual), time zone with
    suggestions, NTP server test, summer/winter time mode, sync interval and a
    live **preview of the time sent to the keyboard**.
  - **Device & Firmware:** verified / unverified / locked state with the
    reason, installed vs. verified firmware, display variant, Check keyboard
    again, keyboard polling interval.
  - **General:** start with Windows, language, appearance (**like Windows /
    dark / light**), app update check with result, About with **version**,
    links, config file and log folder.
  - **Apply / Cancel** only appear when something changed; invalid fields are
    marked and block saving. Changes made in the tray or in the config file
    show up in the open window.
- Design: dark graphite with a cyan accent, keycap buttons and status LEDs,
  light theme with the same accent.

### Changed
- Tray menu **Settings…** opens the window instead of the config file (the
  file is still reachable from General → Config file).
- Starting nd104-sync a second time opens the window of the running instance.
- Release builds use the Tauri CLI from npm (much faster builds).

### Fixed
- The app could freeze when settings were applied while the tray updated
  (lock held during tray updates).

## [0.2.0] – 2026-10-31 · M2 · Beta

> **Beta – milestone M2: tray app and installer.** nd104-sync now runs in the
> background with a tray icon and starts with Windows. There is no settings
> window yet (M3): **Settings…** opens the configuration file, and changes are
> picked up automatically. **Chilkey ND104 only, wired USB only, Windows 10/11.**

### Added
- **Tray app `nd104-sync.exe`** built on the M1 core:
  - Tray icon colour shows the state: cyan = connected and synced, grey = not
    connected, amber = locked (protocol check failed), red = weather stale.
  - Tooltip with **app version**, keyboard, weather, firmware state and last sync.
  - Menu: status line, Sync now, Pause/Resume, Check keyboard again, Check for
    updates, Settings… (config file), Language, Start with Windows, Open log
    folder, About (version, opens the project page), Quit.
- **Notifications** when the keyboard locks, when unverified firmware is
  detected, when a new nd104-sync version is available, and for the result of
  a manual update check.
- **German and English**, following the system language; switchable in the
  tray menu (Language).
- **Start with Windows** (on by default, switchable in the tray menu).
- **Config file reload:** edits to `config.toml` take effect within seconds.
- **Log files** in `%LOCALAPPDATA%\nd104-sync\logs` (daily, kept 7 days; the
  first line of a run shows the version).
- **Single instance:** a second start shows a notification instead.
- **Installers** in every release: NSIS setup (`…_x64-setup.exe`, per user, no
  admin rights, English/German) and MSI (`…_x64_en-US.msi`). The portable zip
  now also contains the tray app. One `SHA256SUMS.txt` covers all downloads.
- App icon and tray icons.

### Known limitations
- No settings window yet – edit the configuration file via **Settings…** (M3).
- Installers are not code-signed: Windows SmartScreen asks for confirmation.
- When nd104-sync is started from a packaged (MSIX) app such as some terminals
  or IDE integrations, Windows redirects its AppData files into that app's
  package folder. Start it from the Start menu or via autostart instead.
- Key-based weather providers (M4) and the firmware update check (M5) are
  still missing.

## [0.1.0] – 2026-10-31 · M1 · Beta

> **Beta – milestone M1: core + console runner.** There is no tray app or
> settings window yet (that is M2/M3). This release contains the console
> runner `nd104-cli` and the diagnostic tool `nd104-probe`.
> **Chilkey ND104 only, wired USB only, Windows 10/11.**

### Added
- **Console runner `nd104-cli`**: `run` (keep time and weather in sync until
  Ctrl+C), `status`, `sync`, `weather`, `geocode` (city search, `--set` stores
  your location), `check-update`, `config path|show|init`.
- **Time sync** to the keyboard display: PC time, a chosen time zone, NTP or a
  manual offset; summer/winter time automatic or forced; re-sync exactly at
  DST changes, every hour, on connect and after the PC wakes from sleep.
- **Weather** from Open-Meteo (free, no API key) or manual test values, every
  15 minutes, with retry/backoff and an optional fallback provider. All 10
  display icons are used, including thunderstorm and day/night variants.
- **Firmware and protocol safety** (checks P1–P6): verified firmware runs
  normally, other firmware runs as "compatible" while every check passes, and
  any deviation locks immediately – nothing is sent until re-plug or re-check.
  Unknown but well-formed messages are logged and ignored.
- **App update check** against the GitHub releases of this repository.
- **Privacy:** your location is stored only in your local config; error
  messages in the log contain no request URLs (no coordinates).
- **Configuration** in `%APPDATA%\nd104-sync\config.toml`. Defaults: location
  Berlin, PC time zone (Germany when a time zone is required).
- **Diagnostic tool `nd104-probe`** (M0) to read firmware versions and test
  time, weather and icons by hand.

### Verified hardware
- Chilkey ND104, keyboard firmware **V3.0.14** (`20260710_123331`), screen
  firmware **V1.2.4** (`Mar 17 2026 21:37:00`, 24 h / °C variant).

### Known limitations
- No tray app, no settings window, no autostart yet – start `nd104-cli run`
  yourself (M2).
- Only Open-Meteo and manual weather; key-based providers come with M4.
- No firmware update check yet (M5); the app update check works.
- The keyboard occasionally sends status messages with unknown meaning
  (`0x35`, `0x3A`); they are logged and ignored.

[Unreleased]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.0.0...HEAD
[1.0.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.5.0...nd104-sync-v1.0.0
[0.5.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.4.0...nd104-sync-v0.5.0
[0.4.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.3.0...nd104-sync-v0.4.0
[0.3.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.2.0...nd104-sync-v0.3.0
[0.2.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.1.0...nd104-sync-v0.2.0
[0.1.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/releases/tag/nd104-sync-v0.1.0
