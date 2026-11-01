"""Firmware watch: compares Chilkey's public firmware sources with versions.json.

Reads the public Google Drive folders and support pages listed under
"watch" in firmware/versions.json and reports new, changed or missing files
and version strings. It never changes versions.json – a person reviews the
report and updates the manifest (see docs/firmware.md).

Usage: python firmware/watch.py [--manifest PATH] [--report PATH]
Exit code 0 = no changes, 2 = changes found, 1 = a source could not be read.
Standard library only.
"""

import argparse
import json
import re
import sys
import urllib.request
from pathlib import Path

USER_AGENT = "Mozilla/5.0 (nd104-studio firmware watch; +https://github.com/MrUrdam/chilkey_nd104_utilities)"
DRIVE_FOLDER = "https://drive.google.com/drive/folders/{id}"

# File names appear JSON-escaped in the folder page: \x22NAME.bin\x22
FILE_RE = re.compile(r"\\x22([^\\\"<>]{1,200}?\.bin)\\x22")
SUBFOLDER_RE = re.compile(r'data-id="([A-Za-z0-9_-]{20,})"')
VERSION_RE = re.compile(r"\b(?:Keyboard|Screen)\s+firmware:\s*(V\d+(?:\.\d+)+)", re.IGNORECASE)


def fetch(url: str) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read().decode("utf-8", errors="replace")


def drive_files(html: str) -> set[str]:
    return set(FILE_RE.findall(html))


def drive_subfolders(html: str, own_id: str) -> set[str]:
    return {i for i in SUBFOLDER_RE.findall(html) if i != own_id and i != "_gd"}


def page_versions(html: str) -> set[str]:
    text = re.sub(r"<[^>]+>", " ", html)
    return set(VERSION_RE.findall(text))


def check_model(key: str, model: dict) -> tuple[list[str], list[str], list[str]]:
    """Returns (changes, errors, warnings) as markdown lines.

    Drive folders are the real source: problems there are errors. Support
    pages are secondary (and rate-limit bots): problems there are warnings.
    """
    changes: list[str] = []
    errors: list[str] = []
    warnings: list[str] = []
    watch = model.get("watch", {})

    for folder in watch.get("drive_folders", []):
        url = DRIVE_FOLDER.format(id=folder["id"])
        label = folder["label"]
        try:
            html = fetch(url)
        except Exception as e:  # noqa: BLE001 – report every failure
            errors.append(f"- **{label}**: cannot read [{url}]({url}): {e}")
            continue
        if "known_files" in folder:
            found = drive_files(html)
            known = set(folder["known_files"])
            if not found:
                errors.append(f"- **{label}**: no `.bin` files found – page layout changed? [{url}]({url})")
            for name in sorted(found - known):
                changes.append(f"- **{label}**: new file `{name}` – [folder]({url})")
            for name in sorted(known - found):
                changes.append(f"- **{label}**: file removed `{name}` – [folder]({url})")
        if "known_subfolders" in folder:
            found = drive_subfolders(html, folder["id"])
            known = set(folder["known_subfolders"])
            for sub in sorted(found - known):
                sub_url = DRIVE_FOLDER.format(id=sub)
                changes.append(f"- **{label}**: new subfolder [{sub}]({sub_url})")
            for sub in sorted(known - found):
                changes.append(f"- **{label}**: subfolder removed `{sub}`")

    for page in watch.get("support_pages", []):
        url = page["url"]
        try:
            found = page_versions(fetch(url))
        except Exception as e:  # noqa: BLE001
            warnings.append(f"- cannot read [{url}]({url}): {e}")
            continue
        known = set(page["known_versions"])
        if found and found != known:
            changes.append(
                f"- [{url}]({url}): versions now {', '.join(sorted(found))} "
                f"(known: {', '.join(sorted(known))})"
            )
        if not found:
            warnings.append(f"- [{url}]({url}): no version text found – page layout changed?")

    heading = [f"### {model.get('name', key)}", *changes] if changes else []
    return heading, errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    root = Path(__file__).resolve().parent
    parser.add_argument("--manifest", default=root / "versions.json", type=Path)
    parser.add_argument("--report", type=Path, help="write a markdown report to this file")
    args = parser.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    changes: list[str] = []
    errors: list[str] = []
    warnings: list[str] = []
    for key, model in manifest.get("models", {}).items():
        c, e, w = check_model(key, model)
        changes += c
        errors += e
        warnings += w

    lines = ["## Firmware watch", ""]
    if changes:
        lines += ["Chilkey's firmware sources changed. Please review and update "
                  "`firmware/versions.json` (latest build date, file names, `watch` lists) "
                  "and – after testing – the verified firmware of ND104 Studio.", "", *changes, ""]
    else:
        lines += ["No changes against `firmware/versions.json`.", ""]
    if errors:
        lines += ["### Problems reading the firmware folders", *errors, ""]
    if warnings:
        lines += ["### Warnings (support pages)", *warnings, ""]
    report = "\n".join(lines)
    print(report)
    if args.report:
        args.report.write_text(report, encoding="utf-8")
    if errors:
        return 1
    return 2 if changes else 0


if __name__ == "__main__":
    sys.exit(main())
