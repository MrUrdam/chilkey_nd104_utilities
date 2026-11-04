# Troubleshooting & privacy

[← Back to the README](../README.md) · [Installation](installation.md) · [Usage](usage.md)

> **First aid:** the tray status line, the notification and the log name the
> reason (log folder: **General → About → Log folder**). For an issue, please
> attach the log.

## Troubleshooting

### 🔌 Connection

| Problem | Fix |
|---|---|
| "Not connected" | Use a connection from [Connection modes](../README.md#connection-modes), try another USB port, avoid unpowered hubs, wake the keyboard by pressing a key |
| Bluetooth or 2.4 GHz stays at "Connecting …" | Connect the keyboard **once by USB cable**. ND104 Studio remembers the firmware, then Bluetooth and 2.4 GHz work. Over 2.4 GHz also check that the keyboard is switched to 2.4 GHz (Fn + V) and awake |
| Bluetooth shows "Connecting …" now and then | The keyboard is asleep. It connects again as soon as you press a key |
| Amber icon / locked | A protocol check failed. Re-connect the cable or use **Check keyboard again**. If it stays locked, your firmware behaves differently: please open an issue with the log |
| Display shows other values | The official Chilkey weather plugin is probably still running. Close it |
| Tray app and `nd104-cli run` at the same time | Only run one of them |

### 💡 Lighting and display

| Problem | Fix |
|---|---|
| OpenRGB, Artemis or Aurora find nothing | Switch on **Lighting → OpenRGB interface**, connect to `127.0.0.1` port **6743** (not 6742, that is OpenRGB's own server), keyboard by USB cable, **PC lighting software** set to *Allowed* |
| SignalRGB does not show the ND104 | **Install plugin** on the Lighting page, then restart SignalRGB. USB cable only |
| PC program connected, LEDs unchanged | **PC lighting software** must be *Allowed* (Fn + Shift + L toggles it on the keyboard) |
| Wrong time after DST change | Check time zone and DST mode on the **Clock** page |

### 🪟 Windows

| Problem | Fix |
|---|---|
| No tray icon | It may be hidden behind the **^** arrow in the taskbar. Drag it out. Is ND104 Studio running (Task Manager)? |
| SmartScreen blocks the setup | The setup is not code-signed: **More info → Run anyway** |
| Config or log folder "not available", settings not found | ND104 Studio was started from a packaged (MSIX) app, e.g. some terminals or IDE integrations. Windows then redirects its AppData into that app's package folder (`%LOCALAPPDATA%\Packages\<app>\LocalCache\…`). Start ND104 Studio from the Start menu or via autostart instead |

### 🐧 Linux

| Problem | Fix |
|---|---|
| "Found – no access" | The udev rule is missing, see [the udev rule](installation.md#keyboard-access--the-udev-rule). ND104 Studio connects by itself once it is in place. If not, unplug and replug the keyboard (and the receiver) |
| No tray icon on GNOME | Vanilla GNOME hides tray icons. Install the **AppIndicator** extension (preinstalled on Ubuntu), or open **ND104 Studio** from the application menu |
| Blank or glitchy window on Wayland + NVIDIA | ND104 Studio sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically when an NVIDIA driver is present. If it persists, start it with that variable set, or run under X11 |

### 🔄 Updates

| Problem | Fix |
|---|---|
| "Install update" is missing | It is offered only for the Windows setup install (1.1.0+) and the Linux AppImage. With the MSI, the zip or a `.deb`/`.rpm`, update as in [Updating](installation.md#updating) |
| A beta is not offered | Pre-releases are ignored by the update check on purpose. Get them from the [releases page](https://github.com/MrUrdam/chilkey_nd104_utilities/releases) |

## Privacy / network access

ND104 Studio connects only to:

| Where | Why | Can be switched off |
|---|---|---|
| 🌦️ the weather provider you selected (plus the fallback, if the first one fails) | weather | – |
| 📍 Open-Meteo geocoding | city search, only when you search | – |
| 🕒 the NTP server | only if NTP is your clock source | choose another clock source |
| 💾 Chilkey's Google Drive folders (`drive.google.com`), fallback: the version list in this repository (`raw.githubusercontent.com`) | firmware update check | `[updates] firmware_check = false` |
| 🔄 the GitHub releases API of this repository | app update check | `[updates] app_check = false` |

API keys are sent only to their own provider. The OpenRGB interface listens
on `127.0.0.1` only and is off by default. **No telemetry, no analytics.**
Your location is stored only in your local config file.
