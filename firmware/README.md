# firmware/

Version information about Chilkey keyboard firmware. **No firmware binaries
are stored here** – they stay with Chilkey (see [docs/firmware.md](../docs/firmware.md)).

| File | Purpose |
|---|---|
| [`versions.json`](versions.json) | Reviewed version list: newest firmware per model and part (build date, file name, download folder, instructions) plus the sources the watcher checks. nd104-sync downloads this file for its firmware update check. |
| [`watch.py`](watch.py) | Firmware watcher: compares Chilkey's Google Drive folders and support pages with `versions.json`. Run by [`firmware-watch.yml`](../.github/workflows/firmware-watch.yml) every day; on changes it opens an issue. |

## When the watcher reports a change

1. Open the issue and the linked Drive folder; download the new file only if
   you want to look at it (never commit it).
2. Update `versions.json`:
   - the part's `latest` (`build_date` from the file name, `file`, `ui_file`,
     `version` if known),
   - the `watch` lists (`known_files`, `known_subfolders`, `known_versions`),
   - `updated` (today).
3. Check the list locally:
   - `python firmware/watch.py` → exit code 0, "No changes".
   - App side: `ND104_FIRMWARE_MANIFEST=firmware/versions.json nd104-cli firmware-check`.
4. Commit to `main` and close the issue. Installed apps pick up the new list
   with their next check (after start, daily, or "Check for updates").
5. After installing and testing the new firmware on a real keyboard (M0
   checklist in [docs/protocol.md § 6](../docs/protocol.md)), add the version to
   the verified list in `crates/nd104-core/src/model/nd104.rs` and release a new
   nd104-sync version.

## Format (schema 1)

```jsonc
{
  "schema": 1,
  "updated": "YYYY-MM-DD",
  "models": {
    "nd104": {                       // KeyboardModel::manifest_key()
      "name": "Chilkey ND104",
      "instructions_url": "…",       // Chilkey's flashing guide
      "support_url": "…",
      "parts": {                     // KeyboardModel::manifest_part()
        "keyboard":     { "label": "…", "latest": { "version": "3.0.14", "build_date": "2026-07-10", "file": "…" }, "download_url": "…" },
        "screen_24h_c": { "label": "…", "latest": { "version": "1.2.4", "build_date": "2026-03-17", "file": "…", "ui_file": "…" }, "download_url": "…" },
        "screen_12h_f": { "…": "…" }
      },
      "watch": {                     // used by watch.py only
        "drive_folders": [ { "id": "…", "label": "…", "known_files": ["…"] }, { "id": "…", "known_subfolders": ["…"] } ],
        "support_pages": [ { "url": "…", "known_versions": ["V3.0.4", "V1.2.1"] } ]
      }
    }
  }
}
```

The app compares the **build date** reported by the keyboard (keyboard
`20260710_123331`, display `Mar 17 2026 21:37:00`) with `latest.build_date` of
the matching part. The display part depends on its variant (24 h/°C or
12 h/°F); if the variant is not recognised, the display is reported as
"unknown" instead of guessing.
