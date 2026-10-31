# ND104 Firmware Sources

> Last checked: 2026-10-31. Wired USB connection only.
> This repository never contains vendor firmware binaries.

## TL;DR

- Chilkey publishes **no machine-readable version list** and no online update.
  The web driver's firmware page says *"Online upgrade feature not implemented"*
  and expects you to pick a local file.
- The version text on the support pages (**keyboard V3.0.4, screen V1.2.1**) has
  not changed since February 2026. It does **not** match the newest files.
- The **real "latest" source** is the set of file names in Chilkey's public
  Google Drive folders. Each name carries a build date.
- There are **two screen firmware variants**: 24 h + °C, and 12 h + °F.

## Official pages

| Page | Published | Content |
|---|---|---|
| [ND104 Support & Resources](https://chilkey.com/blogs/updates/nd104-faq-resources) | 2026-02-25 | "Keyboard firmware: V3.0.4", "Screen firmware: V1.2.1", links to the Drive folders |
| [ND104 Support & Resources (AU)](https://www.chilkey.com.au/blogs/news/nd104-support-resources) | 2026-05-08 | same versions and links |
| [Console / downloads](https://chilkey.com/pages/console) | – | "ND104 Keyboard Firmware" and "ND104 Screen Firmware" link to the Drive folders below; VIA JSON `nd104_via_v3_1.json` |
| [Firmware flashing guide](https://chilkey.com/pages/nd104_firmware_flashing_guide) | – | Flashing with `Wuque_toolbox.exe` / `ubest_toolbox.exe`, no versions |
| [Web driver](https://nd104.chilkey.com/) | – | Shows installed versions; firmware update only from a local file |

## Google Drive folders (the real source)

### Keyboard firmware
[Folder `1bBRxuqoCmtKGLFP1ONK2jDnFGpARMcl1`](https://drive.google.com/drive/folders/1bBRxuqoCmtKGLFP1ONK2jDnFGpARMcl1)

| File | Build date (from name) | Size |
|---|---|---|
| `VN001_314A108_1V0_RGB_TYPE1_20260710.bin` | **2026-07-10** | 310 KB |
| `Wuque_toolbox.exe` | – | 9 MB |
| `ND104_Firmware_Flashing_Guide.docx`, `Flashing Keyboard Firmware.mp4` | – | – |

### Screen firmware
[Folder `1OqgGn4GWbKE8EqnmPrfNkhQ2LkfSg_NS`](https://drive.google.com/drive/folders/1OqgGn4GWbKE8EqnmPrfNkhQ2LkfSg_NS), which has two subfolders:

**24-hour time format, Celsius** ([folder](https://drive.google.com/drive/folders/1E7kkBnX99qMY9dFOu-0rNqluetyRrFCO))

| File | Part | Build date (from name) |
|---|---|---|
| `ND104_SCREEN_MCU_320_172_V1.0.0_20260317.bin` | MCU | **2026-03-17** |
| `ND104_SCREEN_UI_320_172_V1.0.0_20251104.bin` | UI | 2025-11-04 |

**12-hour time format, Fahrenheit** ([folder](https://drive.google.com/drive/folders/1RNgHBBwdc8ynacXeRV3VMKW-bBQ4EJQX))

| File | Part | Build date (from name) |
|---|---|---|
| `ND104_SCREEN_MCU_CF_320_172_V1.0.0_20260305.bin` | MCU | **2026-03-05** |
| `ND104_SCREEN_UI_320_172_V1.0.0_20251230.bin` | UI | 2025-12-30 |

Both folders also contain `Wuque_toolbox.exe`, a flashing guide and a video.
The 12 h/°F folder warns that you must use the matching MCU and UI files.

## Open questions (answered by `nd104-probe version`)

- How do the version strings the devices report relate to the file names?
  - The support page says screen V1.2.1, while the files say V1.0.0 plus a date.
  - The keyboard file name has no V3.x at all (`314A108`, `1V0`).
- Can the reported version tell the 24 h/°C screen firmware from the 12 h/°F one
  (`MCU` vs `MCU_CF`)?
- Does the 12 h/°F variant expect temperatures in °F, or does it convert the °C
  values it receives?

## Consequences for nd104-sync

- The firmware update check cannot rely on the version text on the support page.
- Planned approach (see [PLAN.md § 5.4](PLAN.md#54-firmware-update-check-r4)):
  1. A **GitHub Action in this repo** checks the Drive folders and support pages
     every day. When a file name changes, it opens an issue or PR that updates
     `firmware/versions.json`.
  2. **nd104-sync only reads `firmware/versions.json`**, which is stable and
     reviewed, and never scrapes Google Drive itself.
- The firmware allowlist has to cover the **screen variant** too (°C or °F), not
  just the version.
