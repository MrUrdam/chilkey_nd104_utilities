# Changelog – nd104-sync

All notable changes to nd104-sync are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/) (see [README → Versioning](README.md#versioning)).
Release tags: `nd104-sync-vX.Y.Z`.

## [Unreleased] – 0.1.0

### Added
- **M0:** `nd104-protocol` (packet building, CRC, checksums, firmware version
  queries) and the diagnostic CLI `nd104-probe`. Protocol verified on
  hardware: keyboard V3.0.14, screen V1.2.4 (24 h / °C).
- **M1 core** (`nd104-core`):
  - Keyboard model interface with the Chilkey ND104 as the only model
  - Device manager with the protocol checks P1–P6: verified / compatible
    (unverified) / locked; unknown well-formed messages are logged and ignored
  - Clock sources: system, time zone, NTP (own SNTP client), manual offset;
    DST auto / force summer / force winter; sync exactly at DST changes
  - Weather: Open-Meteo (default) and manual/test provider, fallback provider,
    backoff on errors, city search (Open-Meteo geocoding)
  - All 10 display icons used, including thunderstorm and day/night variants
  - Scheduler with device polling, resume-from-sleep detection, pause/resume
  - TOML configuration (`%APPDATA%\nd104-sync\config.toml`, schema 1)
  - App update check against the GitHub releases of this repository
- Console runner `nd104-cli` (`run`, `status`, `sync`, `weather`, `geocode`,
  `check-update`, `config`).
