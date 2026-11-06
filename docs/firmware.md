# ND104 Firmware Sources

> Last checked: 2026-10-31. Versions read over the USB cable (the screen
> version can only be read there, see
> [Connection modes](../apps/nd104-studio/README.md#connection-modes)).
> This repository never contains vendor firmware binaries.

## TL;DR

- Chilkey publishes **no machine-readable version list** and no online update.
  The web driver's firmware page says *"Online upgrade feature not implemented"*
  and expects you to pick a local file.
- The version text on the support pages (**keyboard V3.0.4, screen V1.2.1**) has
  not changed since February 2026. It does **not** match the newest files.
  The newest firmware reports **keyboard V3.0.14** and **screen V1.2.4**
  (read from a real device, see below).
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

## Reported versions vs. files

Read with `nd104-probe version` on 2026-10-31 (wired USB):

| Device | Reported by the device | Matching Drive file | Up to date? |
|---|---|---|---|
| Keyboard | `V3.0.14 20260710_123331` | `VN001_314A108_1V0_RGB_TYPE1_20260710.bin` (same build date 2026-07-10) | ✅ yes |
| Screen MCU | `V1.2.4 Mar 17 2026 21:37:00` | `ND104_SCREEN_MCU_320_172_V1.0.0_20260317.bin` (24 h/°C, same build date 2026-03-17) | ✅ yes |
| Screen MCU, 12 h/°F | not on a device here; the file reports `V1.2.2 Mar  5 2026 15:39:11` | `ND104_SCREEN_MCU_CF_320_172_V1.0.0_20260305.bin` | – |

What this shows:
- The **build date** in the version answer matches the date in the Drive file
  name. The update check can compare these dates. The version numbers on the
  support page and in the file names (`V1.0.0`, `1V0`) are not useful for that.
- The screen MCU build date also reveals the **variant**. 2026-03-17 is the
  24 h/°C MCU; the 12 h/°F MCU is from 2026-03-05.
- The screen UI image (`…_UI_…`) is not part of the version answer, so its
  version cannot be read this way.

What the firmware files show (both screen MCU images compared, read only):
- Both builds contain the same code for °C/°F and 24 h/12 h (`%d.%d°%s`,
  `%02d:%02d %s`, `F`, `am`, `pm`). Two flags in RAM select the unit and the
  time format. The **°F build converts the °C values it receives**
  (`value × 9 / 5 + 32`) and shows the 24 h time it receives as 12 h with
  am/pm. Sending °C and 0–23 h is therefore right for both variants – the web
  driver does the same.
- The flags are set **once at start-up to a fixed value** (0 in the 24 h/°C
  build, 1 in the 12 h/°F build) and nothing else writes or reports them. There
  is **no USB command to read or change them**.
- The 12 h/°F screen firmware is an older code base: it reports **V1.2.2**
  (`Mar  5 2026 15:39:11`), the 24 h/°C one V1.2.4.
- So the only way to tell the variant over USB is the version answer
  (version + build date), matched against the files in the two Drive folders.

## Keyboard firmware analysis

`VN001_314A108_1V0_RGB_TYPE1_20260710.bin` (V3.0.14) was disassembled on
2026-11-02 (read only, file not kept in the repository):

- The chip is a **WCH CH585** (RISC-V, `CH585_BLE_LIB_V1.3`), QMK 0.19.2 based
  with VIA/Vial.
- The file is loaded at **`0x6000`** (a bootloader sits below). At start the
  bytes from `0x63EC` are copied to RAM `0x20000000` and the code from
  `0x65EC` to `0x12958` runs from RAM at `0x2000BC00`; `gp` = `0x20001410`.
- The code uses WCH's own compressed byte load/store instructions (in the
  places of `c.fld`/`c.fsd`) and Zba/Zbb/Zbs bit instructions; standard
  disassemblers show these wrongly.
- Results: the keyboard settings channel, including **keyboard colour** and
  **online RGB**, and the PC lighting protocols (OpenRGB, SignalRGB) – see
  [protocol.md § 5.2.1](protocol.md#521-what-the-firmware-does-with-each-id-v3014)
  and § 5.4.

## Consequences for ND104 Studio

- The firmware update check cannot rely on the version text on the support page.
  It compares the **build date** reported by the device with the date in the
  newest Drive file name, for the matching screen variant.
- Implemented in ND104 Studio (see its
  [firmware update check](../apps/nd104-studio/docs/firmware-safety.md#firmware-update-check)):
  1. The **firmware watch** action ([`firmware-watch.yml`](../.github/workflows/firmware-watch.yml),
     [`firmware/watch.py`](../firmware/watch.py)) checks the Drive folders and
     support pages every day and **opens an issue** when files, subfolders or
     version texts change. Support pages that block automated requests (the AU
     page answers HTTP 429) only produce warnings.
  2. A person updates [`firmware/versions.json`](../firmware/versions.json) after
     review ([process](../firmware/README.md)).
  3. **The app checks Chilkey's official Drive folders directly** (newest
     `…_YYYYMMDD.bin` per part) and uses `firmware/versions.json` only as a
     **fallback** when a folder cannot be read. The source is shown per part.
- The firmware checks have to recognise the **screen variant** too (°C or °F), not
  just the version.
