"""Writes THIRD-PARTY-NOTICES.txt: licences of everything ND104 Studio ships.

- Rust crates: every crate of the Windows build and of the Linux build (both
  targets from `cargo metadata`, as one list; a crate used on one platform only
  is marked), with the licence files from the crate itself.
- JavaScript and font: the packages that end up in the built UI.
- Lucide: the icon shapes in ui/src/components/Icon.svelte.
- The shared libraries the Linux AppImage bundles (static text: they come from
  the build host, their licence texts ship inside the AppImage).

Usage (in apps/nd104-studio, after `npm ci` and with the crates fetched):
    python scripts/third_party_notices.py          # write the file
    python scripts/third_party_notices.py --check  # exit 1 if it is outdated
Standard library only. Works from Windows and Linux alike: `cargo metadata
--filter-platform` only needs the crate sources, not the target toolchain.
"""

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

APP = Path(__file__).resolve().parent.parent
OUT = APP / "THIRD-PARTY-NOTICES.txt"
TARGETS = [("Windows", "x86_64-pc-windows-msvc"), ("Linux", "x86_64-unknown-linux-gnu")]

# packages whose code is in the built UI (ui/ and installer-ui/): the Svelte
# runtime with its runtime imports, the Tauri API and the font
NPM_SHIPPED = ["svelte", "clsx", "esm-env", "@tauri-apps/api", "@fontsource/rajdhani"]

LICENSE_FILE = re.compile(r"^(licen[cs]e|copying|notice|unlicense|copyright)([-_. ].*)?$", re.I)

LUCIDE = """ISC License

Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE."""

# The AppImage is the one package that carries its libraries with it; the
# .deb, the .rpm and the tarball use the libraries of the user's system. This
# text is static: the libraries are whatever linuxdeploy copies from the
# release build host (Ubuntu 22.04 in CI), and their copyright files travel
# with them inside the AppImage.
APPIMAGE_LIBRARIES = """The Linux AppImage – and only the AppImage; the .deb, the .rpm and the
tarball use the libraries installed on your system – bundles unmodified
copies of the shared libraries the app links against, taken from the
Ubuntu 22.04 build host: WebKitGTK and JavaScriptCore, GTK 3, GLib/GIO/
GObject, Pango, Cairo, GdkPixbuf with its image loaders, librsvg, libsoup,
libsecret, GStreamer, ICU, GnuTLS, the X11 and Wayland client libraries,
and their dependencies. Most of them are licensed under the GNU LGPL
(2.0 or 2.1, or later); others under MIT, BSD, ICU, OpenSSL-style and
similar licences. They are used exactly as the distribution provides them.

The copyright and licence text of every bundled library ships inside the
AppImage at usr/share/doc/<package>/copyright, and the libraries themselves
are in usr/lib (unpack the AppImage with --appimage-extract to see both).
The source code of every bundled library is the distribution's source
package: https://packages.ubuntu.com/ (release "jammy"), or `apt source
<package>` on Ubuntu 22.04. As the LGPL requires, a bundled library can be
replaced: unpack the AppImage, exchange the file in usr/lib and run the app
from the unpacked directory, or re-pack it with appimagetool."""


def license_texts(folder: Path, extra: str | None = None) -> list[str]:
    try:
        files = [f for f in sorted(folder.iterdir()) if f.is_file() and LICENSE_FILE.match(f.name)]
    except OSError:
        # crate sources not on disk: listed without a text (see "missing")
        return []
    if extra:
        f = (folder / extra).resolve()
        if f.is_file() and f not in files:
            files.append(f)
    texts = []
    for f in files:
        try:
            text = f.read_text(encoding="utf-8", errors="replace").strip()
        except OSError:
            continue
        if text:
            texts.append(text.replace("\r\n", "\n"))
    return texts


def resolved_packages(target: str) -> dict[str, dict]:
    """Packages in the dependency graph for `target` (not the workspace itself)."""
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target],
            cwd=APP, capture_output=True, text=True, encoding="utf-8", check=True,
        ).stdout
    )
    used = {n["id"] for n in meta["resolve"]["nodes"]}
    own = set(meta["workspace_members"])
    return {p["id"]: p for p in meta["packages"] if p["id"] in used and p["id"] not in own}


def rust_crates() -> list[dict]:
    # one entry per crate across all targets, remembering where it is used
    crates: dict[str, dict] = {}
    for platform, target in TARGETS:
        for pid, p in resolved_packages(target).items():
            crate = crates.get(pid)
            if crate is None:
                folder = Path(p["manifest_path"]).parent
                crate = crates[pid] = {
                    "name": p["name"],
                    "version": p["version"],
                    "license": p.get("license") or "see licence file",
                    "url": f"https://crates.io/crates/{p['name']}",
                    "texts": license_texts(folder, p.get("license_file")),
                    "platforms": [],
                }
            crate["platforms"].append(platform)
    return sorted(crates.values(), key=lambda c: (c["name"], c["version"]))


def npm_packages() -> list[dict]:
    found = []
    for name in NPM_SHIPPED:
        folder = APP / "node_modules" / name
        meta = json.loads((folder / "package.json").read_text(encoding="utf-8"))
        lic = meta.get("license")
        if isinstance(lic, dict):
            lic = lic.get("type")
        found.append({
            "name": name,
            "version": meta.get("version", "?"),
            "license": lic or "see licence file",
            "url": f"https://www.npmjs.com/package/{name}",
            "texts": license_texts(folder),
        })
    found.append({
        "name": "lucide (icon shapes)",
        "version": "",
        "license": "ISC",
        "url": "https://lucide.dev",
        "texts": [LUCIDE],
    })
    return found


def render() -> str:
    groups = [("Rust crates", rust_crates()), ("JavaScript, font and icons", npm_packages())]
    # one copy of every distinct licence text, with the packages that use it
    texts: dict[str, tuple[str, list[str]]] = {}
    missing = []
    out = [
        "ND104 Studio – third-party notices",
        "=" * 34,
        "",
        "ND104 Studio is MIT licensed (see LICENSE). It contains the following",
        "third-party software, each under its own licence, across its Windows",
        "and Linux builds; a crate used on one platform only is marked. The",
        "licence texts follow the list; packages offered under several licences",
        "are used under any of them. MPL-2.0 packages are used unchanged; their",
        "source is available at the given address.",
        "Generated by scripts/third_party_notices.py – do not edit by hand.",
        "",
    ]
    for title, packages in groups:
        out += [title, "-" * len(title)]
        for p in packages:
            name = f"{p['name']} {p['version']}".strip()
            line = f"{name} – {p['license']} – {p['url']}"
            platforms = p.get("platforms") or []
            if platforms and len(platforms) < len(TARGETS):
                line += f" – {platforms[0]} only"
            out.append(line)
            if not p["texts"]:
                missing.append(name)
            for text in p["texts"]:
                # the same text with other line breaks counts once
                key = hashlib.sha256(" ".join(text.split()).encode()).hexdigest()
                texts.setdefault(key, (text, []))[1].append(name)
        out.append("")
    title = "Shared libraries bundled in the Linux AppImage"
    out += [title, "-" * len(title), APPIMAGE_LIBRARIES, ""]
    out += ["Licence texts", "-" * 13, ""]
    for text, users in texts.values():
        out += ["~" * 72, "Used by: " + ", ".join(users), "~" * 72, "", text, ""]
    if missing:
        out += [
            "Packages without a licence file of their own (licence as named in",
            "the list above, text as published by its project):",
            ", ".join(missing),
            "",
        ]
    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if the file is outdated")
    args = parser.parse_args()
    text = render()
    if args.check:
        current = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if current.replace("\r\n", "\n") != text:
            print(f"{OUT.name} is outdated – run: python scripts/third_party_notices.py")
            return 1
        print(f"{OUT.name} is up to date")
        return 0
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {OUT} ({len(text) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
