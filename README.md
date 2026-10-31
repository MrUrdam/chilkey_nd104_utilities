# Chilkey ND104 Utilities

A collection of scripts, tools and (later) firmware experiments for the
Chilkey ND104 keyboard, with a focus on its built-in display.

The main goal is to use the display without the vendor software, for example
to always show the current weather for your own location, keep the clock in
sync and experiment with themes and images.

> ⚠️ Unofficial community project, not affiliated with Chilkey.
> Use at your own risk, especially when flashing firmware.

> 🔌 **Wired USB connection only.** All tools in this repository support the
> keyboard **only when it is connected with a USB cable**. The 2.4 GHz dongle
> and Bluetooth modes are not supported.

## Tools

| Tool | Description | Platform | Status |
|---|---|---|---|
| [nd104-sync](apps/nd104-sync/README.md) | Background tool with a tray icon for the ND104 (architecture open for other models): syncs time and weather to the display, settings/status window, firmware safety lock and firmware update notification | Windows (Linux/macOS prepared) | 🧪 Beta 0.5.0 (tray app, settings window, 5 weather providers, firmware update check, installer), see [releases](https://github.com/MrUrdam/chilkey_nd104_utilities/releases) and [docs/PLAN.md](docs/PLAN.md) |

## The keyboard

The [Chilkey ND104](https://chilkey.com/products/nd104-keyboard) is a full-size
keyboard (104 keys) with an aluminium case, hot-swap sockets, a rotary knob and
a 1.47" colour display.

- Connection: USB-C, 2.4 GHz, Bluetooth 5.4 (this repo: **USB-C only**)
- Display: 320×172 TFT, shows time, weather and images/GIFs
- Official software: [Chilkey web driver](https://nd104.chilkey.com/)
  (WebHID, Chromium-based browsers only) and the
  [Chilkey console / downloads](https://chilkey.com/pages/console)
- Support & FAQ: [ND104 Support & Resources](https://chilkey.com/blogs/updates/nd104-faq-resources)

### Hardware (from community reverse engineering)

| Component | Chip / info | USB VID:PID |
|---|---|---|
| Keyboard MCU | WCH CH585 (RISC-V, BLE) | `6d67:016c` |
| Display controller | U-BEST UB32F341S (ARM, RTX + LVGL) | `5542:0001` |
| Display in DFU mode | – | `5542:1000` |

Details: [docs/protocol.md](docs/protocol.md).

## Firmware

| Part | Support page says | Newest file on Chilkey's Drive | On my device (read 2026-10-31) |
|---|---|---|---|
| Keyboard | V3.0.4 | `VN001_314A108_1V0_RGB_TYPE1_20260710.bin` (2026-07-10) | **V3.0.14** `20260710_123331` ✅ latest, verified with nd104-sync |
| Screen, 24 h / °C | V1.2.1 | MCU `…_V1.0.0_20260317.bin`, UI `…_V1.0.0_20251104.bin` | **V1.2.4** `Mar 17 2026 21:37:00` ✅ latest (24 h / °C), verified with nd104-sync |
| Screen, 12 h / °F | V1.2.1 | MCU `…_CF_…_V1.0.0_20260305.bin`, UI `…_V1.0.0_20251230.bin` | – |

The support page text is outdated. The newest firmware is only visible through
the file names in Chilkey's Google Drive folders. Details and links:
[docs/firmware.md](docs/firmware.md).

The installed versions are read with `nd104-probe version`
([apps/nd104-sync/crates/nd104-probe](apps/nd104-sync/crates/nd104-probe/README.md)).
They are also shown in the [web driver](https://nd104.chilkey.com/) info dialog,
but the screen version is often wrong there; see
[docs/protocol.md § 3.3](docs/protocol.md#33-why-the-screen-version-looks-broken-on-the-web-driver).

## Repository layout

| Folder | Content |
|---|---|
| `apps/` | Standalone programs (e.g. `nd104-sync`) |
| `scripts/` | Small scripts and experiments for the display and keyboard |
| `docs/` | Project plan, protocol and hardware documentation |
| `firmware/` | Firmware version list (`versions.json`) and the firmware watcher – no vendor binaries ([details](firmware/README.md)) |
| `assets/` | Images and GIFs for the display (320×172) |

## Documentation

| Document | Content |
|---|---|
| [docs/PLAN.md](docs/PLAN.md) | Detailed plan and architecture of nd104-sync |
| [docs/protocol.md](docs/protocol.md) | USB HID protocol: devices, checksums, version queries, time and weather packets |
| [docs/firmware.md](docs/firmware.md) | Where the latest firmware really comes from, screen variants (°C / °F) |
| [nd104-probe](apps/nd104-sync/crates/nd104-probe/README.md) | M0 diagnostic CLI: read firmware versions, test time/weather packets |
| [apps/nd104-sync/README.md](apps/nd104-sync/README.md) | nd104-sync usage, build instructions, dependencies, limitations |

## Sources & credits

- [ND104 Reverse Engineering](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b) by borrougagnou
- [ND104 Firmware Upgrade Guide](https://gist.github.com/borrougagnou/ab98c68cb6812d76fc0357e57f8541d9) by borrougagnou
- Official [Chilkey web driver](https://nd104.chilkey.com/), analysed for the version queries
- [zoom-sync](https://github.com/ozboar/zoom-sync), a model for weather and time sync

## License

MIT, see [LICENSE](LICENSE).
