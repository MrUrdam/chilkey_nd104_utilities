# Chilkey ND104 Utilities

[![nd104-sync CI](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/nd104-sync-ci.yml/badge.svg)](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/nd104-sync-ci.yml)
[![Firmware watch](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/firmware-watch.yml/badge.svg)](https://github.com/MrUrdam/chilkey_nd104_utilities/actions/workflows/firmware-watch.yml)
[![License: MIT](https://img.shields.io/github/license/MrUrdam/chilkey_nd104_utilities)](LICENSE)
![Unofficial](https://img.shields.io/badge/Chilkey-unofficial%20%C2%B7%20not%20affiliated-lightgrey)
![USB cable only](https://img.shields.io/badge/connection-USB%20cable%20only-orange)

Tools for the **Chilkey ND104** keyboard and its built-in 1.47" display, without
the vendor software: always show the current time and weather for your own
location, keep the clock in sync and know when new firmware is out.

> ⚠️ **Unofficial hobby project. Use at your own risk.** No warranty, no
> guaranteed support, not affiliated with Chilkey. Only tested with the
> firmware versions listed below. Please read the [disclaimer](#disclaimer)
> before you use any tool.

## Tools

<!--
  One block per tool. To add a tool, copy the table below and replace
  screenshot, name, description, badges and links.
  Screenshot: about 360 px wide, stored in docs/images/<tool>/.
-->

<table>
  <tr>
    <td width="380" valign="top">
      <a href="apps/nd104-sync/README.md">
        <img src="docs/images/nd104-sync/dashboard.png" width="360" alt="nd104-sync settings window, dashboard page">
      </a>
    </td>
    <td valign="top">
      <h3><a href="apps/nd104-sync/README.md">nd104-sync</a></h3>
      <p>
        Small tray app that keeps the keyboard display up to date with the
        <b>current time and weather</b>, controls the <b>lighting</b> and
        shows <b>your own picture or GIF</b> on the screen. Set it up once,
        then forget it.
      </p>
      <ul>
        <li>Time sync with time zones, NTP and summer/winter time</li>
        <li>Weather from 5 providers, 2 of them without API key</li>
        <li>Lighting: on/off, effects, brightness, speed, colour</li>
        <li>Own picture or animated GIF on the screen, checked and scaled first</li>
        <li>Firmware safety lock and firmware update notification</li>
        <li>Own setup with one-click updates from the app</li>
        <li>Settings window with live display preview, English/German, two styles in dark/light</li>
      </ul>
      <p>
        <img src="https://img.shields.io/github/v/release/MrUrdam/chilkey_nd104_utilities?filter=nd104-sync-v*&label=version&color=0969da" alt="version">
        <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6?logo=windows&logoColor=white" alt="Windows 10 | 11">
      </p>
      <p>
        📖 <a href="apps/nd104-sync/README.md">Documentation</a> ·
        ⬇️ <a href="https://github.com/MrUrdam/chilkey_nd104_utilities/releases">Download</a> ·
        📝 <a href="apps/nd104-sync/CHANGELOG.md">Changelog</a>
      </p>
    </td>
  </tr>
</table>

## The keyboard

The [Chilkey ND104](https://chilkey.com/products/nd104-keyboard) is a full-size
keyboard (104 keys) with an aluminium case, hot-swap sockets, a rotary knob and
a 1.47" colour display.

- Connection: USB-C, 2.4 GHz, Bluetooth 5.4 (this repo: **USB cable only**)
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

The tools are tested with the firmware versions marked ✅. Other versions may
work, but nobody has checked them (see [disclaimer](#disclaimer)).

| Part | Support page says | Newest file on Chilkey's Drive | Tested with (read 2026-10-31) |
|---|---|---|---|
| Keyboard | V3.0.4 | `VN001_314A108_1V0_RGB_TYPE1_20260710.bin` (2026-07-10) | **V3.0.14** `20260710_123331` ✅ |
| Screen, 24 h / °C | V1.2.1 | MCU `…_V1.0.0_20260317.bin`, UI `…_V1.0.0_20251104.bin` | **V1.2.4** `Mar 17 2026 21:37:00` ✅ |
| Screen, 12 h / °F | V1.2.1 | MCU `…_CF_…_V1.0.0_20260305.bin`, UI `…_V1.0.0_20251230.bin` | not tested |

The support page text is outdated. The newest firmware is only visible through
the file names in Chilkey's Google Drive folders. Details and links:
[docs/firmware.md](docs/firmware.md).

The installed versions are read with `nd104-probe version`
([apps/nd104-sync/crates/nd104-probe](apps/nd104-sync/crates/nd104-probe/README.md)).
They are also shown in the [web driver](https://nd104.chilkey.com/) info dialog,
but the screen version is often wrong there, see
[docs/protocol.md § 3.3](docs/protocol.md#33-why-the-screen-version-looks-broken-on-the-web-driver).

## Repository layout

| Folder | Content |
|---|---|
| `apps/` | Standalone programs (e.g. `nd104-sync`) |
| `scripts/` | Small scripts and experiments for the display and keyboard |
| `docs/` | Protocol, firmware and hardware documentation, screenshots |
| `firmware/` | Firmware version list (`versions.json`) and the firmware watcher, no vendor binaries ([details](firmware/README.md)) |
| `assets/` | Images and GIFs for the display (320×172) |

## Documentation

| Document | Content |
|---|---|
| [docs/protocol.md](docs/protocol.md) | USB HID protocol: devices, checksums, version queries, time and weather packets |
| [docs/firmware.md](docs/firmware.md) | Where the latest firmware really comes from, screen variants (°C / °F) |
| [docs/keyboard-api.md](docs/keyboard-api.md) | What the ND104 can do over USB, and what nd104-sync uses |
| [nd104-probe](apps/nd104-sync/crates/nd104-probe/README.md) | Diagnostic CLI: read firmware versions, test time/weather packets |
| [apps/nd104-sync/README.md](apps/nd104-sync/README.md) | nd104-sync features, installation, usage, building |

## Disclaimer

**Unofficial project.** This repository is a private hobby project. It is not
affiliated with, endorsed by or supported by Chilkey. "Chilkey" and "ND104" are
names of their respective owners and are used here only to describe what the
tools work with.

**Use at your own risk.** All tools, scripts and information in this repository
are provided "as is". Sending data to the keyboard over USB and, above all,
flashing firmware can make the keyboard or its display unusable and may void
your warranty. You decide what you run on your hardware, and you carry the
risk.

**No warranty, no liability.** The software comes without any warranty, as
stated in the [MIT license](LICENSE). The authors are not liable for damage to
hardware, loss of data or any other harm caused by using it.

**Limited scope.** The tools are only tested with:
- the Chilkey ND104,
- connected with a **USB cable** (no 2.4 GHz dongle, no Bluetooth),
- the firmware versions marked ✅ in [Firmware](#firmware),
- the platforms named in each tool's README (nd104-sync: Windows 10/11).

Everything else may work, but is untested. nd104-sync has a safety lock that
stops sending as soon as the keyboard behaves differently from what was tested,
but that is a precaution, not a promise.

**No guaranteed support.** Issues and pull requests are welcome. I answer them
in my free time, when and if I can. There is no promise of fixes, updates or
answers. For problems with the keyboard itself, please contact
[Chilkey support](https://chilkey.com/blogs/updates/nd104-faq-resources).

## Sources & credits

- [ND104 Reverse Engineering](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b) by borrougagnou
- [ND104 Firmware Upgrade Guide](https://gist.github.com/borrougagnou/ab98c68cb6812d76fc0357e57f8541d9) by borrougagnou
- Official [Chilkey web driver](https://nd104.chilkey.com/), analysed for the version queries
- [zoom-sync](https://github.com/ozboar/zoom-sync), a model for weather and time sync

## License

MIT, see [LICENSE](LICENSE).
