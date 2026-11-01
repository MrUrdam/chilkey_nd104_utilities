# Changelog – ND104 Studio (formerly nd104-sync)

All notable changes to ND104 Studio are listed here; up to 1.5.0 the app was
called nd104-sync. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/) (see [README → Versioning](README.md#versioning)).

- Release tags: `nd104-studio-vX.Y.Z` (up to 1.5.0: `nd104-sync-vX.Y.Z`).
  Each tag builds a GitHub release whose notes are the matching section below.
- 0.1.0–0.5.0 were the beta releases of the milestones M1–M5.
- **1.0.0 is the first stable release**; from now on: MAJOR = incompatible
  changes, MINOR = new features, PATCH = fixes.

## [Unreleased]

## [2.0.0] – 2026-11-01

nd104-sync is now **ND104 Studio** – it long does more than syncing time and
weather. Installed nd104-sync versions find it as a
normal update; settings, keymap profiles and backups, API keys, autostart and
shortcuts are taken over. Plus keyboard settings (debounce and sleep times)
and a livelier keymap page: pressed keys light up and Fn combinations show
their layer. Update with **Settings → General → Install update**.

### Changed
- **New name ND104 Studio**, with the tagline "Display, lighting and keymap
  for your Chilkey ND104" in the setup and under Settings → General.
- **New icon:** a keycap with the screen, the knob and the light bar (amber
  to lime, the accents of the two styles). The tray icons have the same
  shape and keep their status colours.
- **New locations** (see [README → Files](README.md#files)):
  `%LOCALAPPDATA%\Programs\nd104-studio` (program),
  `%APPDATA%\nd104-studio` (settings, keymap page, profiles),
  `%LOCALAPPDATA%\nd104-studio` (logs, automatic keymap backups), Credential
  Manager entries `<provider>.nd104-studio`, "Apps" entry, autostart entry
  and shortcuts "ND104 Studio", program files `nd104-studio.exe` and
  `nd104-studio-setup.exe`.
- **Releases** are tagged `nd104-studio-vX.Y.Z`; the setup is
  `nd104-studio_X.Y.Z_x64-setup.exe`, the MSI
  `nd104-studio_X.Y.Z_x64_en-US.msi`. Each release also gets a bridge release
  `nd104-sync-vX.Y.Z` so installed nd104-sync 1.x versions find the update.
- The app folder in the repository is `apps/nd104-studio`; workflows and
  screenshots moved along.
- README: new intro, keymap screenshot first in the gallery and all
  screenshots in sidebar order, a **Keymap** section in the feature list,
  "Coming from nd104-sync", and a reworked "What it does NOT do" (no macros,
  no tap/hold or combo editor, no key swapping or automatic profile
  switching, no per-key colours, keyboard colour or polling rate).

### Added
- **Takeover from nd104-sync:** the setup closes nd104-sync, installs
  ND104 Studio and removes the old program folder, "Apps" entry, shortcuts
  and autostart entry (autostart and the desktop shortcut are set up again
  under the new name). Settings, keymap page settings, profiles, automatic
  keymap backups, window storage and API keys move to the new name – also on
  the app's first start, for the portable zip and the MSI. Existing data
  under the new name is never overwritten; old logs are deleted.
- Keymap backups and profile exports made with nd104-sync can still be
  restored and imported.
- **Keys light up in the keymap drawing while you press them** (looked up in
  the keyboard's keymap, so remapped keys light up where they are; knob
  turns flash the knob). Fn sends nothing itself, but a combination that
  only exists on a higher layer (e.g. Fn+F5 → play/pause) shows that layer
  while it is held. The layer stays put while a key is selected for
  remapping or a text field has the focus.
- The ISO Enter is drawn in its real shape (wide on the Tab row, narrower
  below).
- **Keyboard settings** on the Device & Firmware page: debounce mode (off /
  send first / debounce first), debounce time (1–200 ms), sleep and
  deep-sleep time (1–60 min). Only the values Chilkey's web driver offers;
  every change is stored in the keyboard right away and read back. Values
  the firmware does not report show "–"; unsupported commands are reported
  without locking. The USB polling rate (hidden in the driver), keyboard
  colour and online RGB are left out.

### Fixed
- **Light bar below Del / End / PgDn:** it belongs to the bottom lighting,
  not to the background lighting. The keymap drawing now lights it in the
  bottom lighting's solid colour (it showed the key lighting's colour,
  faded), and the Lighting page and README say so.

## [1.5.0] – 2026-11-01

A keymap page: change keys, the knob and the round button on all layers,
with label sets, profiles and backups – plus a collapsible sidebar and a fix
for locks while Chilkey's web driver is open. Update with
**Settings → General → Install update**.

### Added
- **Keymap page:** change what every key, the knob (turn left/right, press)
  and the round button left of the screen do, on all 6 layers. Layers can be
  renamed; the drawing follows ISO or ANSI (switch, or detect it with one key
  press) and scales with the window without growing beyond a comfortable
  size. Functions are chosen by category or search (keys, modifiers, media
  and apps, mouse, lighting, layers, display). Changes are collected and
  written together – only the differences, read back to check – and the
  current keymap is saved as an automatic backup first (the last 20 are
  kept). Fn on layer 0 stays protected. Knob turning exists on layers 0–3
  only (keyboard limit).
- **Key labels** on the keymap page: ISO UK, ISO DE and ANSI US built in;
  own label sets are always a copy of an existing one. Labels only change
  the display.
- **Keymap profiles:** save the current keymap with a name and an
  application, load it later (its differences appear as changes to review),
  export it to a file and import it again.
- **Keymap backup** (now on the keymap page): saves every key on all 6 layers
  and the knob actions (layers 0–3) to a JSON file, and restores such a file.
  Restoring first shows how many keys would change and warns if the backup
  comes from other keyboard firmware; it then writes only the differences
  and reads everything back to check. Macros are not included (the official
  tool never reads them back). The firmware guard knows the keymap answers;
  an unsupported command is reported without locking time and weather.
- **The keymap drawing shows the keyboard's lighting:** a single-colour
  background lighting tints the keys and the light bar below Del / End /
  PgDn, the bottom lighting glows softly around the keyboard. Animated
  effects are not drawn; a switch turns the lighting look off. The screen
  in the drawing shows the same live preview as the Dashboard.
- **Default layer names:** layer 0 is the "Basic layer", layer 1 the "Fn
  layer" (both can be renamed).
- **Collapsible sidebar:** shrinks to icons; the page name slides out next
  to the icon on hover.

### Changed
- **Sidebar order:** Dashboard, Keymap, Lighting, Weather, Clock, Screen
  picture, Device & Firmware, General.
- Drop-down lists use their own arrow that follows the theme.
- The keymap backup moved from the Device & Firmware page to the keymap page.
- Lighting page and README: the light bar sits below Del / End / PgDn (not
  Ins / Home / PgUp).

### Fixed
- **Lock while Chilkey's web driver (or VIA) is open:** Windows passes the
  keyboard's answers to every program that has it open, so nd104-sync also
  sees the answers meant for the web driver (e.g. its keymap read
  `12 01 C4 1C …` when it reconnects) and locked with "unknown keyboard
  report". Answers to the known VIA commands (`0x01`–`0x16`) and the screen's
  debug answer (`0x58`) are now logged as "answer to another program" and
  ignored. Damaged frames and unknown data still lock.

## [1.4.2] – 2026-11-01

All ten display weather icons now also appear with the default provider, plus
screenshots of the new pages in the README. Update with
**Settings → General → Install update**.

### Changed
- **Weather icon "cloudy" (single cloud)** now also appears with Open-Meteo
  (default) and Bright Sky. Both only report "cloudy or overcast"; their
  cloud cover now decides: below 88 % (7/8) the single cloud, from 88 % the
  overcast icon (two clouds). Before, these providers always showed
  overcast.
- README: screenshots of the Lighting and Screen picture pages; texts on
  lighting, own picture/GIF, styles and one-click updates.

## [1.4.1] – 2026-11-01

Fix for updating from the app.

### Fixed
- **Updating failed** with "The process cannot access the file because it
  is being used by another process (os error 32)". The setup renamed the
  whole installation folder, which Windows refuses while any file in it is
  still open for a moment after the app ends (virus scanner, Explorer
  reading the icon). It now replaces the files one by one: each old file is
  renamed to `*.old` (allowed even while it is open), the new one moved in,
  and the old files are deleted at the end (or by the next update if one is
  still locked). It waits up to about 10 seconds per file, a failure names
  the file, and the previous version is restored on any error.

The fix is in the setup itself, so it applies from this release on: an
installed 1.1.0–1.4.0 downloads the 1.4.1 setup when you choose
**Install update**.

## [1.4.0] – 2026-11-01

A new look for the settings window and the setup, with a choice of two
styles. Keyboard communication, protocol and configuration are unchanged
(the new style setting is added with its default). Update with
**Settings → General → Install update**.

### Changed
- **New look** for the settings window and the setup: calmer surfaces, no
  glow effects, line icons instead of symbols, status shown as quiet text
  with an LED dot, sentence-case headings. Keycap buttons and LED dots stay.
- **Two styles:** "Calm" (default, graphite/paper with an amber accent) and
  "Gamer" (dark blue-black with an acid-lime accent, Rajdhani headings).
  Both follow the Windows light/dark setting. Choose the style under
  General → Style or on the first page of the setup.
- **Dashboard:** the display replica is now the large centrepiece, with the
  keyboard state and the last/next syncs right next to it.

### Added
- Browser demo of the setup window (`npm run build:installer-demo`) for
  reviewing it without installing.

## [1.3.0] – 2026-11-01

Lighting control and your own picture or GIF on the screen. Time and weather
sync, protocol checks and configuration are unchanged. Update from 1.1.0 or
1.2.0 with **Settings → General → Install update**.

### Added
- **Lighting** page for the background lighting (keys and the light bar
  below Ins/Home/PgUp) and the bottom lighting: on/off, **effect** (all
  effects of the keyboard firmware, names as in Chilkey's web driver, in
  English and German), brightness, speed and colour. Speed is only active for
  animated effects, colour only for effects that use it (one click switches
  to solid colour). Every change is stored in the keyboard right away, like
  in the web driver; switching on restores the last effect.
- **Screen picture** page: send your own picture or animated GIF to the
  screen. The file is checked first (PNG/JPEG/GIF/BMP, up to 2 MB, GIF up to
  59 frames, size after conversion), scaled to 320 × 172 (fill the screen or
  show the whole picture) and previewed – GIFs play in the preview with the
  frame times the screen will use; sending shows a progress bar.

### Fixed
- Display preview: the weather icon was cut off (partly cloudy showed only
  half a sun) and white clouds were invisible on the light screen; clouds are
  now light blue like on the keyboard.
- Display preview in the settings window and the setup: corner radius like
  the real screen.

### Changed
- The firmware guard knows the keyboard's lighting answers and the screen's
  picture-transfer answers. An unsupported or unanswered lighting/picture
  command is reported on its page without locking time and weather.
- docs: picture transfer (protocol § 4.5), lighting and keyboard settings
  (§ 5), and why the 12 h/°F variant cannot be read over USB (firmware notes).

## [1.2.0] – 2026-11-01

The display preview now looks like the keyboard's real weather page.
Keyboard communication, protocol and configuration are unchanged.
If you run 1.1.0 (installed with the new setup), update with
**Settings → General → Install update**.

### Changed
- **Display preview** (dashboard and setup window) now looks like the real
  weather page of the keyboard: light screen, time top left, battery top
  right, weather icon left, temperature right with low ~ high below. The
  battery is a placeholder (nd104-sync cannot read battery level or charging
  state), noted below the preview. The icons stay nd104-sync's own drawings.
- README screenshots use a sunny summer day.

## [1.1.0] – 2026-11-01

New setup for install, update and uninstall, and one-click updates from the
app. Keyboard communication, protocol and configuration are unchanged.
**Run this setup once by hand** – an installation by the old installer
(1.0.x) is taken over automatically. From 1.1.0 on, later updates can be
installed from the settings window.

### Added
- **Own setup** (`nd104-sync_X.Y.Z_x64-setup.exe`) in the nd104-sync look,
  replacing the generic NSIS installer. One file for **install, update and
  uninstall**, per user without admin rights, English/German:
  - Options *Start with Windows* (on) and *Desktop shortcut* (off).
  - A simple progress bar instead of technical details.
  - Recognises an installed version and offers **Update**; reinstalling the
    same version repairs it; an older setup shows a hint but may install.
  - Closes a running nd104-sync before replacing files. The new files are
    swapped in only at the end, so a failed update keeps the old version.
  - The last page tells you whether the keyboard is connected by cable.
  - **Uninstall** via Windows *Apps*, with *Keep my settings* (on by
    default); unticked, settings, stored API keys and logs are deleted too.
  - Offers to install the WebView2 Runtime if it is missing.
  - Hint if an all-users MSI installation exists as well.
  - Unattended use: `--silent`, `--no-autostart`, `--desktop-shortcut`,
    `--uninstall --silent [--remove-settings]`.
- **Install update** in the settings window (General): downloads the new
  setup, checks it against the release's `SHA256SUMS.txt` and updates with a
  small progress window; nd104-sync restarts on its own. Offered for
  installations made by the new setup.

### Changed
- Installation folder of the setup: `%LOCALAPPDATA%\Programs\nd104-sync`.
  An installation by the old installer (1.0.x) is taken over automatically;
  settings, API keys, logs and the autostart choice stay.
- The MSI stays available for managed/per-machine installs.
- `nd104-sync.exe --quit` ends a running instance (used by the setup).

## [1.0.1] – 2026-11-01

Documentation and tooling release – **no changes to the app's behaviour**
(protocol, keyboard communication, configuration and settings are unchanged).
Updating from 1.0.0 is optional.

### Changed
- **Main README** reworked: tools shown as cards with screenshot and short
  description (ready for further tools), badges, and a full
  [disclaimer](../../README.md#disclaimer): use at your own risk, no
  warranty or liability, no guaranteed support, not affiliated with Chilkey,
  tested only with the listed firmware versions.
- **nd104-sync README**: badges and screenshots at the top, a current
  feature overview instead of roadmap, milestone and beta notes; the
  screenshot gallery shows the General page instead of the tray menu.
- The implemented project plan (`docs/PLAN.md`) was removed, together with
  all references to it (docs and code comments).

### Added
- **Screenshots** of all settings pages (Dashboard, Weather, Clock,
  Device & Firmware, General) in
  [`docs/images/nd104-sync/`](../../docs/images/nd104-sync/), with a list
  describing each image.
- **Demo mode** for the settings window (`npm run build:demo`): runs in a
  normal browser with sample data, without app or keyboard. It is built into
  a separate folder and is never part of the app.
- **Screenshot automation**: `npm run screenshots` (Playwright) and the
  workflow `nd104-sync-screenshots.yml`, which rebuilds the demo on Windows
  and commits updated screenshots.

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

[Unreleased]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-studio-v2.0.0...HEAD
[2.0.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.5.0...nd104-studio-v2.0.0
[1.5.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.4.2...nd104-sync-v1.5.0
[1.4.2]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.4.1...nd104-sync-v1.4.2
[1.4.1]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.4.0...nd104-sync-v1.4.1
[1.4.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.3.0...nd104-sync-v1.4.0
[1.3.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.2.0...nd104-sync-v1.3.0
[1.2.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.1.0...nd104-sync-v1.2.0
[1.1.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.0.1...nd104-sync-v1.1.0
[1.0.1]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v1.0.0...nd104-sync-v1.0.1
[1.0.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.5.0...nd104-sync-v1.0.0
[0.5.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.4.0...nd104-sync-v0.5.0
[0.4.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.3.0...nd104-sync-v0.4.0
[0.3.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.2.0...nd104-sync-v0.3.0
[0.2.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/compare/nd104-sync-v0.1.0...nd104-sync-v0.2.0
[0.1.0]: https://github.com/MrUrdam/chilkey_nd104_utilities/releases/tag/nd104-sync-v0.1.0
