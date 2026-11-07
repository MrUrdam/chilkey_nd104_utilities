# Firmware: supported versions, safety, update check

[← Back to the README](../README.md) · [Usage](usage.md) · [Troubleshooting](troubleshooting.md)

## Supported firmware

| Part | Verified version | Status |
|---|---|---|
| Keyboard | V3.0.14 (`20260710_123331`) | ✅ verified on hardware (2026-10-31) |
| Screen (24 h / °C) | V1.2.4 (`Mar 17 2026 21:37:00`) | ✅ verified on hardware (2026-10-31) |
| Screen (12 h / °F) | – (newest file: V1.2.2) | not verified (runs as "compatible" if all checks pass) |

Other versions run as **compatible (unverified)** as long as every protocol
check passes. Firmware sources and the newest files:
[docs/firmware.md](../../../docs/firmware.md).

## Firmware safety

Before sending anything, ND104 Studio reads the firmware versions of the keyboard
and of the display, using the same queries as the official web driver (see
[docs/protocol.md § 3](../../../docs/protocol.md#3-firmware-version-queries)). It
then compares them with a list of versions it has been verified against.

Older or newer firmware is welcome, but the protocol has to behave exactly as
expected. ND104 Studio checks this when the keyboard is connected and on every
write:
- both USB devices have the expected interfaces and report sizes,
- the version queries are answered correctly,
- writes are accepted,
- answers look the way they did on verified firmware.

| Situation | What happens |
|---|---|
| Verified version, all checks pass | **Verified:** normal operation |
| Older/newer version, all checks pass | **Compatible (unverified):** time and weather are synced; you get a one-time notification and the status page shows "unverified firmware" |
| Any check fails (before or during operation) | **Locked:** stops sending immediately. Tray icon turns amber, a notification names the failed check, and the status page explains what to do |
| Version cannot be read | Locked |

A lock is only lifted by re-connecting the keyboard or by **Check keyboard
again** (tray menu or Device & Firmware), and either way all checks run again. There is intentionally
**no "ignore and continue" override**.

Lighting, keymap, keyboard settings and the own picture use the same checks. They are extras: if the
firmware answers "not supported" or does not answer, the page shows that and
time/weather keep running; a damaged answer locks like everywhere else.

**Bluetooth and 2.4 GHz:** the screen version can only be read over the
cable, and over 2.4 GHz the keyboard version only answers from the receiver.
ND104 Studio therefore remembers the versions from the last USB (or
Bluetooth) connection and checks against those. Until the keyboard was
connected once by cable it waits ("Connecting …") instead of locking, see
[Connection modes](../README.md#connection-modes).

On Linux the same checks run (the protocol is unchanged); nothing is ever
flashed.

## Firmware update check

Chilkey publishes firmware only as files in public Google Drive folders (the
version text on their support pages is outdated). ND104 Studio therefore checks
**Chilkey's official folders directly**: it takes the newest firmware file per
part – keyboard, display 24 h/°C or 12 h/°F – and compares the build date in
its name with the build date your keyboard reports.

**Fallback:** if a folder cannot be read (no network, Google changed the page,
folder moved), the reviewed version list
[`firmware/versions.json`](../../../firmware/versions.json) of this repository is
used for that part. The settings window shows the source per part
(*Chilkey (official)* or *repository list*). A GitHub Action keeps that list
up to date: it checks Chilkey's folders every day and opens an issue on
changes ([details](../../../firmware/README.md)).

The check runs:
- when the keyboard is first connected, then once a day,
- on **Check for updates** (settings window: General or Device & Firmware –
  the button checks app and firmware together),
- on the console: `nd104-cli firmware-check`.

The **Device & Firmware** page shows per part: installed, newest available
(with source), state (*up to date*, *update available*, *newer than the
newest file*, *unknown*), the download folder and Chilkey's update
instructions. If an update is available
you get a notification once. Updating stays your decision and is done with
Chilkey's tools – **ND104 Studio never downloads or flashes firmware**. After an
update, ND104 Studio checks the new firmware automatically (verified or
compatible, see [Firmware safety](#firmware-safety)).

The automatic check can be switched off (General → *Check for firmware updates
automatically*, or `[updates] firmware_check = false`).
