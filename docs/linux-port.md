# ND104 Studio on Linux – implementation plan

> Branch: **`linux-port`** (from `main` at 2.3.1). Windows stays the reference
> platform; every step keeps the Windows build, tests and releases unchanged.

| Phase | Status | |
|---|---|---|
| 1 – it compiles on Linux | ✅ done (2026-11-03) | builds, lints and tests on Ubuntu 22.04 in CI |
| 2 – the keyboard works | ✅ done, tested in the VM | USB, 2.4 GHz, Bluetooth incl. battery; time, weather, picture, lighting, keymap, settings, OpenRGB |
| 3 – desktop integration | ✅ done | tray, launcher, autostart, Secret Service (0600 fallback + warning), pkexec rule button |
| 4 – packages and updates | ✅ done | **2.4.0-beta.1** published 2026-11-04 (.deb, AppImage, tarball + Windows assets) |
| 5 – install tests and release | ⏳ open | CI install tests, manual VM round, merge to `main` |

Open: `nd104-probe descriptors` stalls on Linux (diagnostic tool only).

Review 2026-11-06: four points fixed (see § 6); remaining: Linux CI package
build, beta channel.

<details>
<summary><b>Test log of phase 2 (2026-11-03)</b></summary>

VM: Ubuntu 24.04, Wayland, devices passed through by USB.

- **USB cable:** without the udev rule "found, no access"; with the rule
  firmware verified (keyboard V3.0.14, screen V1.2.4) without replugging.
  Time and weather sent, lighting changed and stored, keymap read (6 layers),
  keyboard settings written and read back. Reconnects by itself after
  re-attaching (udev rule applied to the new device). Start at login via
  `~/.config/autostart`.
- **2.4 GHz** (receiver passed through): connected 60 ms after the cable
  left; time, weather, lighting and settings sent and confirmed through the
  receiver.
- **Own picture** sent over USB (3 pages, stored on the screen; picked with
  the GTK file dialog).
- **Keymap** written and undone, each with an automatic backup and verified
  by reading back.
- **OpenRGB interface:** OpenRGB 1.1 (Linux AppImage) as SDK client on
  127.0.0.1:6743 colours the keyboard. Fix needed: on Linux one hidraw device
  holds all collections of a USB interface, so the report ID is now taken
  from the opened collection (interface 4: ID 5, not the first ID 3).
- **Bluetooth** (USB dongle in the VM): connected via BlueZ, firmware
  verified, time, weather and configuration sent while typing. Fix needed:
  one hidraw device carries the whole keyboard, so key presses (ID 1)
  reached the command channel and locked it; the channel now drops reports
  of other collections unlogged. BlueZ reports the battery
  (`Battery1.Percentage`), the app shows it.

</details>

## 1. Starting point – what was checked

A first analysis (another agent, by reading the code) was verified against
the repository on 2026-11-03.

| Finding | Verified | Where |
|---|---|---|
| `hidapi` is built with `windows-native` only – no Linux backend | ✅ | `apps/nd104-studio/Cargo.toml` (workspace deps) |
| `keyring` is built with `windows-native` only | ✅ | same |
| On non-Windows the device search keeps only paths that `ids_from_path()` understands – Windows paths only, so `/dev/hidrawN` is dropped: **no keyboard is ever found** | ✅ | `crates/nd104-core/src/hid.rs` (`hid_paths`, `enumerate`) |
| `is_bluetooth()` looks for the Windows GATT GUID in the path | ✅ | `hid.rs` |
| `installer/` uses `winreg` without a platform gate and is a workspace member → `cargo build --workspace` fails on Linux | ✅ | `installer/Cargo.toml`, `installer/src/system.rs` |
| Update asset name `_x64-setup.exe` hard-coded | ✅ | `crates/nd104-core/src/app_update.rs` |
| Bluetooth status (slot, battery) returns `None` off Windows | ✅ | `src-tauri/src/bluetooth.rs` |
| Protocol, engine, weather, keymap, picture, OpenRGB server and UI are platform-independent | ✅ (no `cfg` there) | |

Additions (not in the first analysis):

- **Self-update is already off on Linux** – `self_update::supported()` looks
  for the Windows setup next to the program, so it returns `false`; only the
  release check needs a Linux asset.
- **Interface numbers and the bus matter beyond Bluetooth:** the device
  manager and the OpenRGB writer pick the lighting channel by
  *interface 4* and "not Bluetooth". `HidInterfaceInfo` should get a `bus`
  field (USB / Bluetooth) filled from hidapi's `bus_type()` on Linux and from
  the path on Windows, instead of guessing from the path everywhere.
- **The Windows HID channel (`hid::win`) stays Windows-only**; Linux uses
  hidapi's `linux-native` (hidraw) backend, where creating a context is cheap
  (no minutes-long scan as on Windows).
- **`rebrand.rs`** migrates the old Windows `nd104-sync` folders – gate it to
  Windows (there were never Linux installs).
- **`tauri.conf.json`** still describes the app as "Wired USB connection
  only" and bundles only `msi` – fix the text, add Linux bundle targets.
- **`nd104-probe`** needs the same feature switch; on Linux it is the first
  tool to try with a new udev rule.

## 2. Decisions

| Topic | Decision | Why |
|---|---|---|
| Device access | hidapi `linux-native` (hidraw) | kernel driver, no libusb detach, works for USB and Bluetooth |
| Permissions | udev rule with `TAG+="uaccess"` (+ `plugdev` variant for non-systemd) | no root, the logged-in user gets access |
| No access | own device state "found, no access" with a hint (and, for the AppImage, an "Install rule" button via `pkexec`) | like the firmware lock: say what is wrong and how to fix it |
| API keys | `keyring` with Secret Service (`linux-native-sync-persistent` + `crypto-rust`) | GNOME Keyring / KWallet; on desktops without one: warn and offer a file with mode 0600 |
| Background | same single process as on Windows; autostart via XDG `.desktop` | one engine, one device owner |
| Tray | StatusNotifierItem (Tauri default); "Open ND104 Studio" as first menu item, status lines as disabled items | no left click and no tooltip there |
| Second start | `tauri-plugin-single-instance` opens the settings window | the GNOME way when there is no tray |
| Packages (phase 1) | `.deb`, `.rpm`, AppImage from the Tauri bundler, built on Ubuntu 22.04 | covers most users; glibc baseline 2.35 |
| Updates | AppImage: in-app (download, SHA-256 check, replace); packages: notify only | never overwrite files a package manager owns |
| SignalRGB | hide on Linux | Windows only |
| OpenRGB server | keep | OpenRGB is native on Linux |

## 3. Phases

Each phase ends with green CI (Windows **and** Linux) and is a commit on
`linux-port`. Merge to `main` only after phase 5 has been tested in the VM.

### Phase 1 – it compiles on Linux

- Move `hidapi` and `keyring` from workspace-wide features to target-specific
  dependencies of the crates that use them (`nd104-core`, `nd104-probe`):
  `windows-native` on Windows, `linux-native` / Secret Service on Linux.
- `installer/`: `#![cfg(windows)]` with a stub `main` elsewhere (or exclude it
  from the Linux CI job).
- Gate `rebrand.rs` and the Windows parts of `src-tauri` (`bluetooth.rs`,
  `signalrgb.rs`, `self_update.rs`) behind `cfg(windows)` with Linux stubs.
- CI: new job on `ubuntu-22.04` with `libwebkit2gtk-4.1-dev
  libayatana-appindicator3-dev librsvg2-dev libudev-dev libdbus-1-dev` –
  `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`
  (the mock HID backend runs there as it is), `npm run check`.

**Done:** the Linux CI job is green; Windows CI unchanged.

### Phase 2 – the keyboard works (the important one)

- `hid.rs`: on Linux, take VID/PID/usage/interface/bus from hidapi's
  `DeviceInfo` (`refresh_devices` is fast there); keep the Windows path code.
- `HidInterfaceInfo { bus }` replaces path guessing; `is_bluetooth()` reads it.
  Report IDs (Bluetooth 2, lighting channel 5) keep coming from the
  descriptor.
- Device state **"keyboard found, no access"** (permission denied on open):
  shown in tray, sidebar and Device page with the fix.
- udev rule `70-nd104-studio.rules` (keyboard `6D67:016C` over USB `0003`
  and Bluetooth `0005`, display `5542:0001`, receiver `6D66:8572`), plus a
  `plugdev` variant; ship it in `apps/nd104-studio/linux/`.
- Bluetooth status: BlueZ over D-Bus (`zbus`): `org.bluez.Device1`
  (Connected, Name → slot) and `org.bluez.Battery1` (Percentage).
- Tests: enumeration on Linux paths (mock), bus mapping, "no access" state.

**Done (in the VM):** keyboard by cable, receiver and – if a dongle is
available – Bluetooth are found without sudo; time, weather, lighting,
keymap, settings, picture and the OpenRGB interface work.

Implemented (2026-11-03):
- `HidInterfaceInfo.bus` (USB / Bluetooth); Windows from the path, Linux
  from hidapi's `bus_type()`. Linux enumeration straight from hidapi.
- Report ID from the descriptor for every collection (a proper item
  parser) – needed for the lighting channel (ID 5) over USB on Linux.
- `DeviceState::NoAccess { model_name, path }`: once in the log and as a
  notification, shown in tray, sidebar, Dashboard and Device page with the
  fix; retried every poll. Mock test.
- `apps/nd104-studio/linux/70-nd104-studio.rules` (+ `-plugdev` variant);
  a test checks that they cover every supported device.
- Bluetooth status from BlueZ (`dbus` crate, `GetManagedObjects`:
  `Device1.Connected/Alias/Modalias`, `Battery1.Percentage`); the
  selection logic is tested on every platform.

### Phase 3 – desktop integration

Done early (2026-11-03): platform-specific texts (`key@linux` variants in
the locale files, resolved by the backend for tray and UI – e.g. "Start at
login" instead of "Start with Windows", Secret Service instead of
Credential Manager); the in-app table **Operating systems**; SignalRGB
hidden on Linux. Tray on Linux: "Open ND104 Studio" first, the
tooltip's details (firmware trust, last sync) as a second status line;
WebKitGTK workaround `WEBKIT_DISABLE_DMABUF_RENDERER=1` when an NVIDIA
driver is present.

- Tray menu: "Open ND104 Studio" first; connection/time/weather as disabled
  lines (no tooltip on Linux).
- `.desktop` launcher (app menu/Activities); second start opens the window.
- Autostart: `tauri-plugin-autostart` (XDG); README note for Sway/Hyprland/i3
  (`exec nd104-studio --hidden`).
- Secret Service for API keys, with the **0600-file fallback and warning** done
  (2026-11-06): when the keyring call fails (no Secret Service on Sway/i3/
  Hyprland/headless), `secrets.rs` reads/writes `~/.config/nd104-studio/secrets`
  at mode 0600 and warns; `secure_store_available()` drives the UI warning.
- WebKitGTK on Wayland + NVIDIA: set `WEBKIT_DISABLE_DMABUF_RENDERER=1` at
  start when an NVIDIA driver is present.
- Hide the SignalRGB part of the PC lighting card on Linux; keep OpenRGB.
- Notification actions ("Open") for lock and update notices.

**Done:** on GNOME (with and without the AppIndicator extension) and KDE the
app is reachable, starts with the session and keeps its API key. The Secret
Service **0600 fallback + warning** and the AppImage **"Install access rule"
(`pkexec`) button** (Device page, shown on "no access" on Linux) are
implemented and match the docs (2026-11-06).

### Phase 4 – packages and updates

- `tauri.conf.json`: bundle targets per platform (`msi` on Windows; `deb`,
  `rpm`, `appimage` on Linux), Linux icons, package dependencies
  (`libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1`), the udev rule with
  `postinst`/`%post` `udevadm control --reload && udevadm trigger`; fix the
  "Wired USB connection only" description.
- `app_update.rs`: asset by install type – AppImage (`$APPIMAGE` set):
  download, check against `SHA256SUMS.txt`, replace, restart; `.deb`/`.rpm`
  (path under `/usr`): notify only ("update with your software centre").
- Release workflow: Linux job on `ubuntu-22.04` →
  `npx tauri build --bundles deb,rpm,appimage`, add the files to
  `SHA256SUMS.txt` and the release notes' download list.

**Done:** a tag builds Windows and Linux assets and publishes them under one
`SHA256SUMS.txt`; `2.4.0-beta.1` is live. The AppImage restart race is fixed
(`--after-update <pid>`, see § 6); the `.rpm` now depends by soname, so it
resolves on Fedora and openSUSE alike (see § 6); `THIRD-PARTY-NOTICES.txt`
covers both builds and the AppImage's bundled libraries (see § 6). **Still
open:** the AppImage self-update over two releases is still untested – betas
are pre-releases the update check ignores, so it needs the beta channel.

Implemented (2026-11-03), released as **2.4.0-beta.1**:
- `tauri.conf.json`: `deb`/`rpm`/`appimage` targets, runtime deps
  (`libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1`, `librsvg2-2`), the udev
  rule shipped to `/usr/lib/udev/rules.d` with `postinst`/`postrm` scripts
  (`udevadm control --reload && udevadm trigger`); the "Wired USB only"
  description fixed.
- `self_update.rs` / `app_update.rs`: install-type-aware update – the Windows
  setup and the Linux AppImage (`$APPIMAGE`) replace themselves and restart;
  `.deb`/`.rpm` notify only. New AppImage release asset
  (`nd104-studio_<version>_amd64.AppImage`).
- Release workflow rebuilt as `prepare → build-windows + build-linux →
  publish` with one `SHA256SUMS.txt` over both platforms and a download list
  built from the files actually produced.
- Verified in the VM: the `.deb` (ships the udev rule, maintainer scripts and
  deps) and the AppImage build at `2.4.0-beta.1`. The **`.rpm` is built only
  for plain `X.Y.Z` versions** – RPM's version field rejects a semver
  pre-release – so betas ship `.deb` + AppImage + tarball (use the AppImage on
  Fedora/openSUSE).

Open: AppImage self-update verified across two releases (beta.1 → a later
build). Betas are GitHub pre-releases, which the in-app check ignores, so a
**beta channel** (a pre-release install also sees newer pre-releases) is a
small follow-up for that test.

### Phase 5 – install tests and release

Status (2026-11-06): the container install tests exist –
[`nd104-studio-linux-install-tests.yml`](../.github/workflows/nd104-studio-linux-install-tests.yml)
runs [`linux/install-test.sh`](../apps/nd104-studio/linux/install-test.sh) in
`ubuntu:22.04/24.04`, `debian:12/13`, `fedora:41/42`, `opensuse/tumbleweed`
and `archlinux` (`.deb`/`.rpm` where the release has one, the AppImage
everywhere; `nd104-cli --version`, udev rule, `ldd`, GUI start under Xvfb).
On `linux-port` it is started by hand with a tag (`workflow_dispatch`);
the automatic run after each release only works from `main`. First run
against `2.4.0-beta.2` pending; the manual VM round and the merge are open.

- CI install tests in containers: `ubuntu:22.04/24.04`, `debian:12/13`,
  `fedora:41/42`, `opensuse/tumbleweed`, `archlinux` – `ldd` complete,
  `nd104-cli --version`, udev rule in place, GUI starts under `xvfb-run`.
- Manual test round in the VM (checklist in section 5), then merge
  `linux-port` → `main` and release (minor version: new platform).

### Later (own tasks)

- Repositories: openSUSE Build Service (Debian/Ubuntu/Fedora/openSUSE/Arch
  from one source) or Fedora COPR; AUR package `nd104-studio-bin` updated by
  a workflow on each tag.
- `systemd --user` unit for `nd104-cli run` (headless), with a lock file in
  `$XDG_RUNTIME_DIR` so GUI and service never open the keyboard together.
- Flatpak (needs the udev rule on the host anyway).

## 4. Documentation (part of every phase, not an afterthought)

| Document | Change |
|---|---|
| `README.md` (root) | Platforms: Windows 10/11 and Linux; disclaimer/test scope per platform |
| `apps/nd104-studio/README.md` | ✅ New **Platform support** matrix (Windows / Linux, like the connection matrix – the one place for it; in the app: Device → Operating systems); *Requirements* and *Installation* per platform (.deb, .rpm, AppImage, udev rule, `plugdev` variant); *Updating* per install type; *Files* (XDG paths: `~/.config/nd104-studio`, `~/.local/share/nd104-studio`); tray/launcher notes; *Troubleshooting*: "found, no access", missing tray on GNOME, NVIDIA/Wayland; *Building from source* on Linux |
| Connection matrix | unchanged rows; footnote where Linux differs (Bluetooth battery via BlueZ, SignalRGB Windows only) |
| `crates/nd104-probe/README.md` | Linux usage with the udev rule |
| `docs/protocol.md` | hidraw notes (report ID handling on Linux) |
| `CHANGELOG.md` | Linux entries in the release that ships it |
| `tauri.conf.json` | short/long description without "wired USB only" |
| Release notes template | Linux downloads in the list |
| This plan | kept up to date per phase; removed (or moved to `docs/`) when merged |

## 5. Test setup

**Linux VM** (set up in parallel, separate session): VMware Workstation Pro
or VirtualBox on the Windows 11 PC (Hyper-V cannot pass generic USB devices
through), guest **Ubuntu 24.04 LTS** (GNOME with AppIndicator), toolchain
for building, repository cloned in the guest. Keyboard (`6D67:016C` + display
`5542:0001`) and receiver (`6D66:8572`) are passed through by USB; Bluetooth
only with a separate USB dongle. Later VMs if needed: Fedora GNOME (no tray
extension), Kubuntu (KDE).

**Manual checklist per test round:**

- [ ] keyboard by cable found without sudo; "no access" shown correctly
      when the udev rule is missing
- [ ] time and weather on the display, picture upload
- [ ] lighting, PC lighting switch, OpenRGB interface (OpenRGB on Linux)
- [ ] keymap read/write/backup, keyboard settings
- [x] 2.4 GHz receiver; Bluetooth (with dongle) incl. battery
- [ ] unplug/replug, suspend/resume → time synced again
- [ ] tray visible (GNOME/Ubuntu) or launcher + single instance
- [ ] autostart after reboot
- [ ] API key kept after reboot (Secret Service)
- [ ] update path for the install type (AppImage replaces, .deb notifies)

**CI:** Linux job (phase 1), install tests (phase 5).

## 6. Risks and open points

- **Firmware safety on Linux:** the same checks run (protocol unchanged);
  nothing is ever flashed. hidraw report-ID behaviour must be verified on
  the keyboard before the first write (read-only checks first, as on
  Windows).
- **Bluetooth over hidraw:** report ID 2 of the command channel works as on
  Windows; reports of the other collections (key presses) on the same
  hidraw device are dropped (tested with a dongle).
- **WebKitGTK differences** (fonts, scrolling, `color-mix()`): the UI needs
  one review pass in the VM.
- **Tray on vanilla GNOME** stays invisible without the extension – the
  launcher must be enough.
- Distribution beyond .deb/.rpm/AppImage (OBS, AUR, Flatpak) is separate
  work after the first Linux release.

### Found in the 2026-11-06 repo review (close before merge to `main`)

Code vs. documentation — **all fixed 2026-11-06** (clippy/tests green on
Windows and Linux):
- ✅ **Secret Service fallback.** `secrets.rs` now falls back to
  `~/.config/nd104-studio/secrets` (mode 0600) when the keyring is unavailable,
  and warns via `secure_store_available()` → `AppInfo.secure_store` and an
  `ApiKeyField` note (EN/DE).
- ✅ **AppImage "Install access rule" button.** `install_udev_rule` command
  (writes the embedded rule, `pkexec` installs it and reloads udev); the Device
  page shows the button on "no access" on Linux.
- ✅ **Stale "wired USB only" texts** removed from `src-tauri/Cargo.toml`,
  `nd104-cli`, `nd104-probe`, `nd104-protocol/src/lib.rs`, `nd104-core/src/lib.rs`.
- ✅ **`hid.rs` comment** on `HidapiChannel.report_id` corrected (notes ID 5
  over USB for the lighting channel).

Packaging / release:
- ✅ **AppImage self-update restart race** – fixed 2026-11-06. The old
  instance now starts the new AppImage with `--after-update <pid>`;
  `main.rs::wait_for_previous_instance` polls `/proc/<pid>` (bounded to 30 s)
  *before* the single-instance plugin runs, so the new instance only registers
  once the old one is gone instead of handing its arguments to it and exiting.
  (Clippy green on Linux; a real two-release update still needs the beta
  channel to be exercised end to end.)
- ✅ **`.rpm` dependencies are Fedora-only** – fixed 2026-11-06. A real
  `.rpm` built in the VM (plain version `2.4.0`) showed Tauri's own defaults are
  already **soname** requires (`libwebkit2gtk-4.1.so.0()(64bit)`,
  `libayatana-appindicator3.so.1()(64bit)`, `libgtk-3.so.0()(64bit)`), which
  resolve on any rpm distro; the only Fedora-name entries were our own
  `depends`. Replaced them with the one thing Tauri does not add,
  `librsvg-2.so.2()(64bit)` (and dropped the matching redundant `.deb` names).
  The package now *declares* cross-distro deps; an actual install on
  Fedora/openSUSE is still to be exercised by the Phase 5 container tests.
- ✅ **`THIRD-PARTY-NOTICES.txt` covers the Windows build only** – fixed
  2026-11-06. `scripts/third_party_notices.py` now resolves both targets
  (`cargo metadata --filter-platform`, which needs no target toolchain, so it
  runs from Windows and Linux alike) into one list: 487 crates, 127 marked
  "Linux only" (`dbus`, `secret-service`, `linux-keyutils`, `basic-udev`, the
  gtk-rs/webkit2gtk stack …), 22 "Windows only". A static section describes
  the ~190 shared libraries the AppImage bundles from the Ubuntu 22.04 build
  host (mostly LGPL), where their copyright files sit inside the AppImage
  (`usr/share/doc/<package>/copyright` – verified in an unpacked build),
  where the source packages are, and that a bundled library may be replaced.
  The Linux CI job now runs the `--check` as well.
- **Linux CI builds no packages.** A broken `.deb`/AppImage only surfaces at
  the release tag. → add `npx tauri build --bundles deb` (or deb+appimage) to
  the Linux CI job on `linux-port`.

Still open from earlier:
- `nd104-probe descriptors` hangs on Linux (diagnostic tool only).
- **Beta/pre-release update channel** missing, so the AppImage self-update
  cannot be exercised beta → beta.
