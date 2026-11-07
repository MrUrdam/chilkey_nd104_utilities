# Installation & updating

[← Back to the README](../README.md) · [Usage](usage.md) · [Building from source](building.md) · [Troubleshooting](troubleshooting.md)

> **Which file do I need?**
>
> | Your system | Take | Updates |
> |---|---|---|
> | 🪟 Windows 10/11 | `…_x64-setup.exe` → [Option A](#option-a--installer-recommended) | one click in the app |
> | 🐧 Debian, Ubuntu, Mint | `…_amd64.deb` → [Option A](#option-a--deb-debian-ubuntu-mint-) | package manager |
> | 🐧 Fedora, openSUSE | `…_x86_64.rpm` → [Option B](#option-b--rpm-fedora-opensuse-) (AppImage for betas) | package manager |
> | 🐧 any other distribution | `…_amd64.AppImage` → [Option C](#option-c--appimage-any-distribution) | one click in the app |
>
> Then connect the keyboard **once by USB cable** ([Connection modes](../README.md#connection-modes))
> and pick your city under **Weather → Location**.

<details>
<summary><b>All release files</b></summary>

Releases are published on the
[releases page](https://github.com/MrUrdam/chilkey_nd104_utilities/releases)
(tag `nd104-studio-vX.Y.Z`). Each release has:

| File | Platform | What | When to use |
|---|---|---|---|
| `nd104-studio_X.Y.Z_x64-setup.exe` | Windows | **Setup (recommended)** – install, update and uninstall; per user, no admin rights, English/German | normal use |
| `nd104-studio_X.Y.Z_x64_en-US.msi` | Windows | MSI installer – per machine, needs admin rights | managed PCs, software deployment |
| `nd104-studio-X.Y.Z-windows-x64.zip` | Windows | Portable: `nd104-studio.exe` (tray app), `nd104-cli.exe`, `nd104-probe.exe` | no installation, testing |
| `nd104-studio_X.Y.Z_amd64.deb` | Linux | Debian/Ubuntu package; installs the udev rule and the runtime dependencies | Debian, Ubuntu, Mint … |
| `nd104-studio_X.Y.Z_x86_64.rpm` | Linux | Fedora/openSUSE package; installs the udev rule | Fedora, openSUSE … (stable releases only, see note) |
| `nd104-studio_X.Y.Z_amd64.AppImage` | Linux | Portable, self-updating; runs on most distributions | no installation, other distros |
| `nd104-studio-X.Y.Z-linux-x64.tar.gz` | Linux | Portable: `nd104-studio`, `nd104-cli`, `nd104-probe` and the udev rule | no installation, testing |
| `SHA256SUMS.txt` | – | Checksums of all files | verifying downloads |
| `THIRD-PARTY-NOTICES.txt` | – | Licences of the bundled open-source software | – |

> **No `.rpm` on pre-releases.** RPM's version field cannot hold a semver
> pre-release (e.g. `2.4.0-beta.1`), so beta/rc releases ship the `.deb`, the
> AppImage and the tarball only. On Fedora/openSUSE use the **AppImage** for a
> pre-release.

</details>

---

## Windows

All Windows options need Windows 10 (21H2+) or Windows 11, 64-bit, and the
Microsoft Edge WebView2 Runtime (pre-installed on Windows 11 and current
Windows 10).

### Option A – installer (recommended)

1. Download `nd104-studio_X.Y.Z_x64-setup.exe` from the newest release.
2. Optional: compare its checksum with `SHA256SUMS.txt`:
   `certutil -hashfile nd104-studio_X.Y.Z_x64-setup.exe SHA256`
3. Run it. The setup is not code-signed, so Windows SmartScreen may warn:
   **More info → Run anyway**.
4. Choose whether ND104 Studio starts with Windows (on by default) and whether
   you want a desktop shortcut (off by default), then **Install**. A progress
   bar shows the installation; no admin rights are needed. ND104 Studio is
   installed to `%LOCALAPPDATA%\Programs\nd104-studio` and gets a Start menu
   entry and an entry in Windows **Apps**.
5. **Start ND104 Studio**. Its icon appears in the tray (maybe behind the **^**
   arrow; drag it onto the taskbar to keep it visible). The last page of the
   setup also shows how your keyboard is connected (and follows changes).
6. The settings window opens on first start: **Weather → Location** → search
   your city, check it on the map (🗺) if needed, pick it, **Apply**. Without
   this, Berlin is used.

If the Microsoft Edge WebView2 Runtime is missing (rare: it ships with
Windows 11 and current Windows 10), the setup offers to download and install
it from Microsoft first.

Unattended installation: `nd104-studio_X.Y.Z_x64-setup.exe --silent`
(add `--no-autostart` and/or `--desktop-shortcut`). The exit code is 0 on
success.

Installed nd104-sync 1.0.x with the old installer? The setup takes over that
installation as well (see the next section).

### Coming from nd104-sync

ND104 Studio was called **nd104-sync** up to version 1.5. Updating takes your
settings, profiles, backups and API keys over.

<details>
<summary><b>What exactly happens</b></summary>

Installed nd104-sync versions find ND104 Studio as a normal update (**Settings → General → Install
update**), or simply run the ND104 Studio setup. Either way it:

- closes nd104-sync, installs ND104 Studio to
  `%LOCALAPPDATA%\Programs\nd104-studio` and removes the old program folder,
  its "Apps" entry, Start menu/desktop shortcut and autostart entry
  (autostart and the desktop shortcut are set up again under the new name if
  you had them);
- moves your settings, keymap page settings and profiles from
  `%APPDATA%\nd104-sync` to `%APPDATA%\nd104-studio`, the automatic keymap
  backups from `%LOCALAPPDATA%\nd104-sync` to `%LOCALAPPDATA%\nd104-studio`,
  and the API keys in the Windows Credential Manager to the new name.
  Old logs are deleted.

The app does the same moves on its first start, so the portable zip and the
MSI take your data over too. Anything that already exists under the new name
is kept; old folders are only removed once they are empty. Keymap backups and
profile exports made with nd104-sync can still be restored and imported.

</details>

### Option B – portable zip

1. Download and extract `nd104-studio-X.Y.Z-windows-x64.zip`, e.g. to
   `%LOCALAPPDATA%\Programs\nd104-studio`.
2. Start `nd104-studio.exe` (tray app) – it enables "Start with Windows" for
   the exe at this location – or use the console runner `nd104-cli.exe`.

### Option C – your own build

1. [Build from source](building.md).
2. Run `target/release/nd104-studio-setup.exe` (the setup with your build
   inside), or run `target/release/nd104-studio.exe` directly.

In every case the configuration lives in `%APPDATA%\nd104-studio\config.toml`
and the logs in `%LOCALAPPDATA%\nd104-studio\logs` – see [Files](usage.md#files).

### Uninstall (Windows)

Windows Settings → **Apps** → ND104 Studio → **Uninstall**. The setup closes
ND104 Studio if it is running and removes the app, its shortcuts and the
autostart entry. **Keep my settings** is ticked by default: your
configuration, location and API keys stay for a later installation. Untick it
to delete the settings, the stored API keys and the logs as well.

Unattended: `"%LOCALAPPDATA%\Programs\nd104-studio\nd104-studio-setup.exe" --uninstall --silent`
(add `--remove-settings` to delete the settings too).

MSI installation: uninstall it in Windows **Apps** as usual; the configuration
and logs are kept. For the portable zip, untick "Start with Windows" first,
then delete the folder.

---

## Linux

ND104 Studio runs on x86-64 Linux with glibc 2.35 or newer (Ubuntu 22.04+,
Debian 12+, Fedora 41+, openSUSE …). It needs **WebKitGTK 4.1** and, for the
tray icon, an **AppIndicator** host; the `.deb`/`.rpm` pull these in, the
AppImage bundles what it can.

### Keyboard access – the udev rule

On Linux the keyboard is reached through `/dev/hidraw*`, which belongs to root.
A **udev rule** gives the logged-in user access without root. The **`.deb` and
`.rpm` install it for you** (to `/usr/lib/udev/rules.d/70-nd104-studio.rules`)
and reload udev. For the **AppImage and the tarball** install it by hand:

```bash
sudo cp linux/70-nd104-studio.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

(The AppImage can also do this for you from the app – it offers an *Install
access rule* button that uses `pkexec`.) Systems without systemd-logind use
[`linux/70-nd104-studio-plugdev.rules`](../linux/70-nd104-studio-plugdev.rules)
instead (group `plugdev`). Without a rule, ND104 Studio shows **"Found – no
access"** and connects by itself once the rule is in place; if it still cannot
open the device, unplug and replug the keyboard (and the 2.4 GHz receiver).

### Option A – `.deb` (Debian, Ubuntu, Mint …)

```bash
sudo apt install ./nd104-studio_X.Y.Z_amd64.deb
```

`apt` installs the runtime dependencies (`libwebkit2gtk-4.1-0`,
`libayatana-appindicator3-1`, `librsvg2-2`) and the udev rule. Then start
**ND104 Studio** from the application menu, or run `nd104-studio` /
`nd104-cli` from a terminal.

### Option B – `.rpm` (Fedora, openSUSE …)

```bash
sudo dnf install ./nd104-studio_X.Y.Z_x86_64.rpm      # Fedora
sudo zypper install ./nd104-studio_X.Y.Z_x86_64.rpm   # openSUSE
```

Same result as the `.deb`. The package declares its libraries by **soname**
(`libwebkit2gtk-4.1.so.0`, `libayatana-appindicator3.so.1`, `libgtk-3.so.0`,
`librsvg-2.so.2`) rather than by distribution-specific package names, so it
resolves on Fedora, openSUSE and the RHEL family alike. Not built for
pre-releases (see the note above) – use the AppImage there. It has not been
test-installed on a real Fedora/openSUSE system yet – please report problems.

### Option C – AppImage (any distribution)

```bash
chmod +x nd104-studio_X.Y.Z_amd64.AppImage
./nd104-studio_X.Y.Z_amd64.AppImage
```

The AppImage needs no installation and **updates itself** (see
[Updating](#updating)). Install the udev rule once (above). Keep the file where
you can write to it (e.g. `~/Applications/`) so the self-update can replace it.
It needs a desktop system with **GTK 3** (any GNOME, KDE, Xfce … desktop has
it): base libraries such as Wayland, fribidi and OpenGL come from your system,
not from the AppImage.

### Option D – portable tarball

Extract `nd104-studio-X.Y.Z-linux-x64.tar.gz`; it contains the tray app
`nd104-studio`, the console runner `nd104-cli`, the diagnostic tool
`nd104-probe` and the udev rule. Install the rule by hand (above) and run
`./nd104-studio`.

### First start

The tray icon appears (on GNOME only with the AppIndicator extension, which is
preinstalled on Ubuntu; otherwise open **ND104 Studio** from the application
menu). The settings window opens on first start: **Weather → Location** → pick
your city → **Apply**. Start at login can be switched on in **General**. On
Sway/Hyprland/i3 add `exec nd104-studio --minimized` to your config.

Configuration and logs live under XDG paths – see [Files](usage.md#files).

### Uninstall (Linux)

- `.deb`: `sudo apt remove nd104-studio`
- `.rpm`: `sudo dnf remove nd104-studio` / `sudo zypper remove nd104-studio`
- AppImage / tarball: delete the file or folder; remove
  `/etc/udev/rules.d/70-nd104-studio.rules` if you added it by hand.

Your configuration in `~/.config/nd104-studio` and data in
`~/.local/share/nd104-studio` are left in place; delete them too if you want a
clean removal. API keys are removed from the Secret Service when you delete
them in the app.

---

## Updating

**Update check:** ND104 Studio asks GitHub for the newest release 30 seconds
after start and then once a day. If a newer version exists, you get a
notification. Nothing is downloaded or installed automatically. Check by hand:
- Settings window → General → **Check for updates** (always shows a result)
- Console: `nd104-cli check-update`

Only releases tagged `nd104-studio-vX.Y.Z` count; **drafts and pre-releases
are ignored**, so a beta (e.g. `2.4.0-beta.1`) is not offered automatically –
get it from the [releases page](https://github.com/MrUrdam/chilkey_nd104_utilities/releases).

**How to update:**

| Install type | How |
|---|---|
| **Windows setup** (recommended) | Settings → General → **Install update** (one click): ND104 Studio downloads the new setup, checks it against `SHA256SUMS.txt`, closes itself and restarts when done. Or run the new `…-setup.exe` by hand – it offers **Update** and keeps your settings. |
| **Windows MSI** | Run the new `.msi`; it upgrades the installed version. |
| **Windows portable zip** | Quit ND104 Studio, replace the files in your folder. |
| **Linux AppImage** | Settings → General → **Install update** (one click): downloads the new AppImage, checks it against `SHA256SUMS.txt`, replaces the running file and restarts. The file must be writable (see Option C). |
| **Linux `.deb` / `.rpm`** | Update through your package manager / software centre (`apt`, `dnf`, `zypper`); the app only **notifies**, it never overwrites files the package manager owns. |
| **Linux tarball** | Quit ND104 Studio, replace the files. |
| **Own build** | `git pull`, build again, install or replace the binary. |

Configuration files are versioned (`schema = 1`). A newer app migrates older
files automatically. An older app refuses a file written by a newer schema
instead of misreading it.
