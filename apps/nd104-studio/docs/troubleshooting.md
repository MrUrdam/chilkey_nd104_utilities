# Troubleshooting & privacy

[← Back to the README](../README.md) · [Installation](installation.md) · [Usage](usage.md)

## Troubleshooting

| Problem | Fix |
|---|---|
| "Not connected" | Use a connection from [Connection modes](../README.md#connection-modes); try another USB port; avoid unpowered hubs; wake the keyboard by pressing a key |
| "Found – no access" (Linux) | The udev rule is missing – see [Installation → the udev rule](installation.md#keyboard-access--the-udev-rule); ND104 Studio connects by itself once it is in place |
| No tray icon on GNOME (Linux) | Vanilla GNOME hides tray icons; install the **AppIndicator** extension (preinstalled on Ubuntu), or open **ND104 Studio** from the application menu |
| Blank or glitchy window on Wayland + NVIDIA (Linux) | ND104 Studio sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically when an NVIDIA driver is present; if it persists, start it with that variable set, or run under X11 |
| Over Bluetooth or 2.4 GHz it stays at "Connecting …" / "Not connected" | The first time, connect the keyboard once with the USB cable; ND104 Studio remembers the firmware, then Bluetooth and 2.4 GHz work. Over 2.4 GHz also check that the keyboard is switched to 2.4 GHz (Fn + V) and awake |
| Over Bluetooth it shows "Connecting …" now and then | The keyboard is asleep; it connects again as soon as you press a key |
| Amber icon / locked | A protocol check failed (the reason is in the tray status line, the notification and the log). Re-connect the cable or use "Check keyboard again"; if it stays locked, your firmware behaves differently – please open an issue with the log |
| No tray icon (Windows) | It may be hidden behind the **^** arrow in the taskbar; drag it out. Is ND104 Studio running (Task Manager)? |
| SmartScreen blocks the setup (Windows) | The setup is not code-signed: **More info → Run anyway** |
| "Install update" is missing | It is offered only for the Windows setup install (1.1.0+) and the Linux AppImage. With the MSI, the zip or a `.deb`/`.rpm`, update as in [Installation → Updating](installation.md#updating) |
| Config or log folder "not available" / settings not found (Windows) | ND104 Studio was started from a packaged (MSIX) app, e.g. some terminals or IDE integrations; Windows then redirects its AppData into that app's package folder (`%LOCALAPPDATA%\Packages\<app>\LocalCache\…`). Start ND104 Studio from the Start menu or via autostart instead |
| Tray app and `nd104-cli run` at the same time | Only run one of them |
| Display shows other values | The official Chilkey weather plugin is probably still running – close it |
| Wrong time after DST change | Check time zone / DST mode in the Clock tab |

## Privacy / network access

ND104 Studio connects only to:
- the weather provider you selected, plus the fallback provider if the first
  one fails (and Open-Meteo geocoding for city search) – API keys are sent only
  to their own provider,
- the NTP server, if you selected NTP as clock source,
- Chilkey's public Google Drive firmware folders (`drive.google.com`) and, as
  a fallback, the version list in this repository (`raw.githubusercontent.com`)
  for the firmware update check – can be disabled,
- the GitHub releases API of this repo for the app update check – can be disabled
  (`[updates] app_check = false`).

No telemetry, no analytics, no IP-based location unless you turn it on.
