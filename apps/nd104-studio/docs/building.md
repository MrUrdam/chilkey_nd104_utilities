# Building from source

[← Back to the README](../README.md) · [Installation](installation.md) · [Usage](usage.md)

The workspace builds on **Windows** (the reference platform) and **Linux**.
`npm run build` must run before any `cargo build` of the app, because the
settings window's files are compiled into the `nd104-studio` binary.

## Prerequisites

### Windows

| Tool | Version | Needed for | Notes |
|---|---|---|---|
| [Rust](https://rustup.rs/) | stable, `x86_64-pc-windows-msvc` | everything | install with `rustup-init.exe`, default options |
| Visual Studio Build Tools | 2022 | everything | workload **"Desktop development with C++"** (linker) |
| [Git](https://git-scm.com/) | any | getting the code | |
| [Node.js](https://nodejs.org/) | LTS (≥ 20) | settings window, Tauri CLI | the Tauri CLI comes via npm |
| WebView2 Runtime | current | app | pre-installed on Windows 10/11 |

Quick install with winget (Rust, then the Build Tools with the C++ workload):

```bash
winget install --id Rustlang.Rustup -e
```

```bash
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--passive --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

### Linux

| Tool | Version | Needed for |
|---|---|---|
| [Rust](https://rustup.rs/) | stable, `x86_64-unknown-linux-gnu` | everything |
| [Node.js](https://nodejs.org/) | LTS (≥ 20) | settings window, Tauri CLI |
| System libraries | – | WebKitGTK, tray, D-Bus (see below) |

Build on the **oldest** target you want to support (the releases use
Ubuntu 22.04, glibc 2.35). Install the development libraries:

```bash
sudo apt install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev libdbus-1-dev libssl-dev pkg-config build-essential
```

(On Fedora the equivalents are `webkit2gtk4.1-devel`,
`libayatana-appindicator-gtk3-devel`, `librsvg2-devel`, `dbus-devel`,
`openssl-devel`, `gcc`.)

## Build

```bash
git clone https://github.com/MrUrdam/chilkey_nd104_utilities.git
cd chilkey_nd104_utilities/apps/nd104-studio
npm ci                     # frontend + Tauri CLI
npm run build              # settings window → ui/dist (embedded into the app)
cargo test --workspace --exclude nd104-studio-setup   # unit tests, no hardware needed
```

**Windows** (adds the setup window and the MSI/setup):

```bash
npm run build:installer-ui # setup window → installer-ui/dist (embedded into the setup)
cargo build --release --workspace --exclude nd104-studio-setup
                           # → target/release/nd104-studio.exe, nd104-cli.exe, nd104-probe.exe
npm run tauri build        # app + MSI → target/release/bundle/msi/*.msi
cargo build --release -p nd104-studio-setup
                           # setup with the app inside → target/release/nd104-studio-setup.exe
```

**Linux** (the `installer/` crate is Windows-only and excluded):

```bash
cargo build --release -p nd104-cli -p nd104-probe
                           # → target/release/nd104-cli, nd104-probe
npx tauri build --bundles deb,rpm,appimage
                           # → target/release/bundle/{deb,rpm,appimage}/
```

- The setup (`installer/`) packs `target/release/nd104-studio.exe`, the console
  tools and the docs into itself, so build it **last** on Windows. Built
  without the app it shows its window but refuses to install.
- `npm run tauri build` downloads WiX (Windows) or linuxdeploy (Linux AppImage)
  on first use.
- The `.rpm` only builds for a plain `X.Y.Z` version; a pre-release version
  (e.g. `2.4.0-beta.1`) is rejected by RPM – build `--bundles deb,appimage`
  then.
- Development with hot reload: `npm run tauri dev`.
- Frontend type check: `npm run check`.

## Making a release (maintainers)

Stable releases follow SemVer (`MAJOR.MINOR.PATCH`); pre-releases use
`X.Y.Z-beta.N` or `X.Y.Z-rc.N`.

1. Set the version in [`Cargo.toml`](../Cargo.toml) (`[workspace.package]
   version`) and run `cargo build` (or `cargo update --workspace`) so
   `Cargo.lock` follows.
2. In [CHANGELOG.md](../CHANGELOG.md), move the `[Unreleased]` entries into a
   new section `## [X.Y.Z] – YYYY-MM-DD` and add the link at the bottom.
3. Commit, then tag and push:
   `git tag nd104-studio-vX.Y.Z` and `git push origin nd104-studio-vX.Y.Z`.
4. The workflow [`nd104-studio-release.yml`](../../../.github/workflows/nd104-studio-release.yml)
   runs as **prepare → build-windows + build-linux → publish**: it checks that
   the tag matches `Cargo.toml`, runs the tests, builds the Windows assets
   (setup, MSI, portable zip) and the Linux assets (`.deb`, `.rpm`, AppImage,
   portable tarball), writes one `SHA256SUMS.txt` over all of them and
   publishes the GitHub release. The notes are the CHANGELOG section for that
   version plus a download list of the files that were actually produced.
   - `X.Y.Z-beta.N` / `X.Y.Z-rc.N` → GitHub **pre-release**, ignored by the app
     update check. The Windows **MSI** is skipped (WiX needs a plain version)
     and the Linux **`.rpm`** is skipped (RPM's version grammar has no place
     for a pre-release); the `.deb`, AppImage, tarball, setup and zip are built.
   - `0.x` versions get a "(beta)" title but are **not** GitHub pre-releases, so
     the app update check still finds them.
   - Installed nd104-sync 1.x versions only look for `nd104-sync-v*` tags. For
     them there is one **bridge release** `nd104-sync-v2.0.0` with the setup
     named `nd104-sync_2.0.0_x64-setup.exe`; it updates them to ND104 Studio,
     which then finds all later releases by itself. No new bridge releases are
     made.

Every push to `main` or `linux-port` also runs
[`nd104-studio-ci.yml`](../../../.github/workflows/nd104-studio-ci.yml) on
Windows **and** Linux: frontend type check and build, Rust format check,
clippy (warnings are errors), tests, and the check that
`THIRD-PARTY-NOTICES.txt` is up to date (it covers both builds; regenerate it
with `python scripts/third_party_notices.py` after changing dependencies).

## Main dependencies

| Crate / package | Purpose |
|---|---|
| `tauri` v2 + plugins `autostart`, `single-instance`, `notification` | App shell, tray, window |
| `hidapi` | HID communication (Windows native / Linux hidraw) |
| `tokio` | Async runtime, scheduler |
| `reqwest` (rustls) | HTTP for weather / geocoding / manifest |
| `jiff` | Time zones and DST |
| (own SNTP client) | NTP time source, no extra dependency |
| `serde`, `toml`, `directories` | Configuration |
| `keyring` 3 | API keys (Windows Credential Manager / Linux Secret Service) |
| `dbus` | Linux: Bluetooth status and battery from BlueZ |
| `tracing`, `tracing-appender` | Logging |
| `sys-locale` | System language detection |
| `zip`, `winreg`, `windows` | Setup (Windows): embedded app files, "Apps" entry, shortcuts |
| `sha2` | Checking the downloaded setup / AppImage ("Install update") |
| Svelte 5 + TypeScript + Vite | Settings window (`ui/`) |
| `@tauri-apps/api`, `@tauri-apps/cli` | Window ↔ app calls, build tooling |
| `@fontsource/rajdhani` | Display font, bundled locally (OFL) |

## Project structure

```
apps/nd104-studio/
├── crates/
│   ├── nd104-protocol/   # packet building, CRC (pure, unit tested)
│   ├── nd104-probe/      # diagnostic CLI for hardware tests
│   ├── nd104-core/       # model interface, device + firmware guard,
│   │                     # clock, weather, scheduler, config, update check
│   └── nd104-cli/        # console runner for the core
├── src-tauri/            # Tauri tray app, icons, MSI config
├── ui/                   # Svelte settings window
├── installer/            # setup: install, update, uninstall (Windows, own window)
├── installer-ui/         # Svelte setup window
├── linux/                # udev rules, package maintainer scripts
└── locales/              # en.json, de.json (the setup has its own in installer/locales)
```

The Linux port is tracked in
[docs/linux-port.md](../../../docs/linux-port.md).
