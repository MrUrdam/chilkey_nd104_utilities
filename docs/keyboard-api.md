# What the ND104 can do – capability list

> Status 2026-10-31. Sources: static analysis of the official web driver
> ([nd104.chilkey.com](https://nd104.chilkey.com/), bundle `index-BpBkeHgw.js`),
> the [community notes](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b)
> and our own hardware tests ([protocol.md](protocol.md)). Everything sent to the
> keyboard uses the same commands over the USB cable and Bluetooth
> ([protocol.md § 5.5](protocol.md#55-over-bluetooth-analysis-2026-11-02)). Which
> feature works on which connection: [Connection modes](../apps/nd104-studio/README.md#connection-modes).

Legend: ✅ in ND104 Studio · 🧪 verified on hardware, not in the app yet ·
📄 documented (web driver), not verified · ❌ not available / not planned

## Display (screen controller `5542:0001`, via keyboard relay or directly)

| Function | Command | Status | Notes |
|---|---|---|---|
| Read firmware version + build date | `11 02 …` | ✅ | P3 check, firmware update check |
| Set time | `1C 03 … A5 38` | ✅ | all clock sources, DST |
| Set weather (icon, now/max/min) | `1C 02 … A5 FE` | ✅ | 10 icons, negative temps |
| Theme + colours (3 themes, background/text RGB565) | `A5 FD 00 bgH bgL fgH fgL id` | ❌ rejected by screen V1.2.4 | tested 2026-10-31: answer status `FC`, display unchanged – the web driver's theme feature does not work with this firmware; not pursued |
| Menu navigation (up/down/switch/back) | `A5 39 00 01…04` | 📄 | could offer "next display page" buttons |
| Screen info / device address | `A5 FB` | 📄 | answer `1C … FB` with a MAC-like address and a number |
| Enter "test/upload mode" | `A5 34 00 01` | 📄 | sent by the web driver before image upload |
| **Upload image / GIF** (320×172, ≤60 frames, ≤2 MB) | header `55 42 01 …` (size, frame count, delays) + chunks `BA 06 AA B0 …` | 📄 | largest open block; chunk acknowledgements `BA ?? A5 B0/FC`; needs careful hardware tests |
| Reset display | `52 00 01` (community notes) | 📄 | not needed |
| Firmware update (DFU) | `5542:1000` + `dfu-util` | ❌ | ND104 Studio never flashes firmware |

## Keyboard (`6D67:016C`, vendor interface `FF60:0061`)

| Function | Command | Status | Notes |
|---|---|---|---|
| Read firmware version + build date | `11 02 …` | ✅ | |
| Protocol/layout version | `01` → `01 xx vv` | 📄 | selects the key-code table |
| Read keymap (layers) | `12 …` | 📄 | read-only display of the layout would be possible |
| Read knob mapping | `14 …` | 📄 | |
| RGB per-key lighting: brightness, effect, speed, colour | `07 03 01…04 …`, commit `09 03` | 📄 | "lighting" – stays with the Chilkey driver (scope) |
| Second light zone (backlight) brightness/effect/speed/colour | `07 02 01…04 …`, commit `09 02` | 📄 | same |
| Debounce, USB polling rate, sleep / deep-sleep time, "online RGB", keyboard colour | `07 00 04 …` / answers `08 00 04 …` | 📄 | settings – see [protocol.md § 5.2.1](protocol.md#521-what-the-firmware-does-with-each-id-v3014) for what each id does |
| Macros | `0D`, `0F`, `10` | 📄 | incomplete in the notes |
| Debug raw send | `58 …` | ❌ | no |
| Keyboard DFU | `13 …` | ❌ | never |

## Battery

| Function | Status | Notes |
|---|---|---|
| Battery level over the wired USB interfaces | ❌ **not available** | The web driver has no battery query; no command or report for it was found on either USB interface. When the keyboard is plugged in by cable, it is charging anyway. |
| Battery level over **Bluetooth** (standard BLE Battery Service) | ✅ | Read from Windows' device properties (the value of *Settings → Bluetooth & devices*); nothing is sent to the keyboard. Shown with the Bluetooth connection on the Dashboard and in the live preview. Over the cable the keyboard does not report it. |
| Battery over the 2.4 GHz dongle | ❌ not found | Windows has no battery property for the receiver, and no command for it is known ([protocol.md § 5.6](protocol.md#56-over-the-24-ghz-receiver-analysis-2026-11-02)) |

## Candidates for future milestones

| Idea | Effort | Risk | Notes |
|---|---|---|---|
| Display themes + colours | – | – | dropped: rejected by the current screen firmware (could be re-tested after a firmware update with `nd104-probe theme`) |
| "Next display page" buttons (menu navigation) | small | low | `39 00 0x`, needs a short hardware test |
| Image/GIF upload | large | medium | own milestone; chunked protocol, hardware tests, verified firmware only |
| System stats on the display (CPU temperature etc.) | medium | medium | only possible as generated images (no text command) → depends on image upload |
| Read-only keymap/knob overview in the window | medium | low | reading only, no writing |
| Battery via Bluetooth | medium | low | requires lifting R1 for this single read-only value |
| Lighting / key configuration | – | – | deliberately not planned – the official driver does this |
