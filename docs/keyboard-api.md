# What the ND104 can do – capability list

> Status 2026-11-04 (ND104 Studio 2.4.0-beta.1, keyboard V3.0.14, screen
> V1.2.4). Sources: static analysis of the official web driver
> ([nd104.chilkey.com](https://nd104.chilkey.com/), bundle `index-BpBkeHgw.js`),
> the [community notes](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b),
> the keyboard firmware and our own hardware tests ([protocol.md](protocol.md)).
> Which feature works on which connection:
> [Connection modes](../apps/nd104-studio/README.md#connection-modes).

**Legend:** ✅ in ND104 Studio · 🧪 verified on hardware, not in the app ·
📄 documented (web driver), not verified · ❌ not available or not planned

## At a glance

| Area | Status |
|---|---|
| 🕒 Time and 🌦️ weather on the display | ✅ |
| 🖼️ Own picture or GIF on the screen | ✅ (USB cable) |
| 💡 Lighting: both zones, effect, brightness, speed, colour | ✅ |
| 🎨 Per-key colours from PC programs (OpenRGB, Artemis, Aurora, SignalRGB) | ✅ (USB cable) |
| ⌨️ Keymap: keys, knob, round button, all layers | ✅ |
| ⚙️ Keyboard settings: debounce, sleep times | ✅ |
| 🔋 Battery level | ✅ over Bluetooth only |
| 🏷️ Firmware versions, update check | ✅ |
| 🎨 Display themes and colours | ❌ rejected by the screen firmware |
| 🎛️ Macros, tap/hold, combos | ❌ stays with Chilkey's driver |
| 💾 Firmware flashing | ❌ never |

## Display (screen controller `5542:0001`, via keyboard relay or directly)

| Function | Command | Status | Notes |
|---|---|---|---|
| Read firmware version + build date | `11 02 …` | ✅ | firmware checks, firmware update check |
| Set time | `1C 03 … A5 38` | ✅ | all clock sources, DST |
| Set weather (icon, now/max/min) | `1C 02 … A5 FE` | ✅ | 10 icons, negative temperatures |
| Upload picture / GIF (320×172, JPEG frames) | header `55 42 01 …` + chunks `BA 06 AA B0 …` | ✅ | [protocol.md § 4.5](protocol.md#45-own-picture--gif-raw-reports-to-the-screen--used-by-nd104-studio), USB cable only |
| Theme + colours (3 themes, background/text RGB565) | `A5 FD 00 bgH bgL fgH fgL id` | ❌ | rejected by screen V1.2.4 (status `FC`, display unchanged, tested 2026-10-31) |
| Menu navigation (up/down/switch/back) | `A5 39 00 01…04` | 📄 | could offer "next display page" buttons |
| Screen info / device address | `A5 FB` | 📄 | answer `1C … FB` with a MAC-like address and a number |
| Enter "test/upload mode" | `A5 34 00 01` | 📄 | sent by the web driver before an upload, not needed by ND104 Studio |
| Reset display | `52 00 01` (community notes) | 📄 | not needed |
| Firmware update (DFU) | `5542:1000` + `dfu-util` | ❌ | ND104 Studio never flashes firmware |

## Keyboard (`6D67:016C`, vendor interface `FF60:0061`)

| Function | Command | Status | Notes |
|---|---|---|---|
| Read firmware version + build date | `11 02 …` | ✅ | |
| Lighting: background (per key) and bottom zone | `07 03/02 01…04 …`, store `09 03/02` | ✅ | brightness, effect, speed, colour ([§ 5.1](protocol.md#51-lighting--used-by-nd104-studio-onoff-brightness-colour)) |
| Keymap: read, set keys | `12 …`, `05 …` | ✅ | 6 layers × 8 × 16, backups and profiles ([§ 5.3](protocol.md#53-keymap--used-by-nd104-studio-keymap-page-backup-and-restore)) |
| Knob mapping | `14 …`, `15 …` | ✅ | layers 0–3, left and right |
| Debounce, sleep / deep-sleep time | `07 00 04 …` / `08 00 04 …` | ✅ | exactly the values the web driver offers ([§ 5.2](protocol.md#52-keyboard-settings-channel-00--debounce-and-sleep-used-by-nd104-studio)) |
| "Online RGB" (PC lighting allowed) | `07 00 04 05 …` | ✅ | shown as *PC lighting software* switch ([§ 5.2.1](protocol.md#521-what-the-firmware-does-with-each-id-v3014)) |
| PC lighting channel (SignalRGB protocol) | interface 4, report ID `05`: `25` / `24` / `26` | ✅ | used by the SignalRGB plugin and the OpenRGB interface ([§ 5.4](protocol.md#54-other-keyboard-commands-not-used)) |
| USB polling rate, keyboard colour | `07 00 04 00/04 …` | ❌ | polling rate hidden in the web driver, keyboard colour stored but unused by the firmware |
| Protocol/layout version | `01` → `01 xx vv` | 📄 | answered as VIA (`01 00 0C`) |
| Macros | `0D`, `0F`, `10` | ❌ | stays with Chilkey's driver |
| Debug raw send | `58 …` | ❌ | no |
| Keyboard DFU | `13 …` | ❌ | never |

## Battery

| Function | Status | Notes |
|---|---|---|
| Over **Bluetooth** (standard BLE Battery Service) | ✅ | Windows: the device property behind *Settings → Bluetooth & devices*. Linux: BlueZ `Battery1.Percentage`. Nothing is sent to the keyboard. Shown on the dashboard and in the live preview |
| Over the USB cable | ❌ | no command or report found. Shown anyway while Bluetooth is connected too |
| Over the 2.4 GHz receiver | ❌ | no battery property and no command known ([§ 5.6](protocol.md#56-over-the-24-ghz-receiver-analysis-2026-11-02)) |

## Ideas for later

| Idea | Effort | Risk | Notes |
|---|---|---|---|
| "Next display page" buttons (menu navigation) | small | low | `39 00 0x`, needs a short hardware test |
| System stats on the display (CPU temperature etc.) | medium | medium | only as generated pictures (no text command), built on the picture upload |
| Display themes and colours | – | – | dropped: rejected by the current screen firmware. Could be re-tested after a firmware update with `nd104-probe theme` |
| Macros, tap/hold, combos | – | – | deliberately not planned, Chilkey's driver does this |
