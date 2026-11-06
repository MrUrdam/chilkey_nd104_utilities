#!/usr/bin/env bash
# Installs a released ND104 Studio Linux build inside a distribution container
# and checks that it is usable. Run as root in the container:
#     install-test.sh <dir with the release assets> <version>
# Checks: the package installs (its dependencies resolve), nd104-cli reports
# the version, the udev rule is in place, nd104-studio has no missing shared
# library, and the tray app starts under Xvfb and keeps running. Distros
# without a package for this release (rpm is stable-only; Arch has none)
# test the AppImage instead. First version - meant to be refined on real runs.
set -uo pipefail

DIST=${1:?dir with the release assets}
VERSION=${2:?version}
. /etc/os-release
echo "=== $PRETTY_NAME - ND104 Studio $VERSION ==="

FAILED=0
pass() { echo "   ok   $*"; }
fail() { echo "   FAIL $*"; FAILED=1; }
check() { # check <description> <command...>
    local what=$1; shift
    if "$@" >/tmp/check.out 2>&1; then
        pass "$what"
    else
        fail "$what"
        sed 's/^/        /' /tmp/check.out | tail -5
    fi
}

# Starts a GUI command under Xvfb with a session bus; a tray app has to be
# still running when the timeout hits (rc 124), anything else is a crash.
runs_for_20s() { # runs_for_20s <description> <command...>
    local what=$1; shift
    timeout 20 xvfb-run -a dbus-run-session -- "$@" >/tmp/gui.out 2>&1
    local rc=$?
    if [ "$rc" -eq 124 ]; then
        pass "$what (still up after 20 s)"
    else
        fail "$what exited early (rc=$rc)"
        tail -15 /tmp/gui.out | sed 's/^/        /'
    fi
}

# --- 1. test tooling, and the package the distribution can take ------------
# Xvfb + a session bus for the GUI start; libGL and fontconfig are what
# linuxdeploy leaves to the system, so the AppImage needs them from here.
PKG=none
if command -v apt-get >/dev/null; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -q >/dev/null
    apt-get install -y -q xvfb dbus-x11 libgl1 fontconfig >/dev/null
    deb=$(ls "$DIST"/nd104-studio_*_amd64.deb 2>/dev/null | head -1)
    if [ -n "$deb" ]; then
        if apt-get install -y -q "$deb" >/tmp/install.out 2>&1; then
            PKG=deb; pass "apt install .deb"
        else
            fail "apt install .deb"; tail -15 /tmp/install.out
        fi
    fi
elif command -v dnf >/dev/null; then
    dnf install -y -q xorg-x11-server-Xvfb dbus-x11 mesa-libGL fontconfig >/dev/null 2>&1 || true
    rpm=$(ls "$DIST"/nd104-studio*.rpm 2>/dev/null | head -1)
    if [ -n "$rpm" ]; then
        if dnf install -y "$rpm" >/tmp/install.out 2>&1; then
            PKG=rpm; pass "dnf install .rpm"
        else
            fail "dnf install .rpm"; tail -15 /tmp/install.out
        fi
    else
        echo "   skip no .rpm in this release (pre-release) - testing the AppImage"
    fi
elif command -v zypper >/dev/null; then
    zypper --non-interactive --quiet install xorg-x11-server-Xvfb dbus-1-x11 Mesa-libGL1 fontconfig >/dev/null 2>&1 || true
    rpm=$(ls "$DIST"/nd104-studio*.rpm 2>/dev/null | head -1)
    if [ -n "$rpm" ]; then
        if zypper --non-interactive install --allow-unsigned-rpm "$rpm" >/tmp/install.out 2>&1; then
            PKG=rpm; pass "zypper install .rpm"
        else
            fail "zypper install .rpm"; tail -15 /tmp/install.out
        fi
    else
        echo "   skip no .rpm in this release (pre-release) - testing the AppImage"
    fi
elif command -v pacman >/dev/null; then
    pacman -Syu --noconfirm --quiet xorg-server-xvfb dbus mesa fontconfig >/dev/null 2>&1 || true
    echo "   skip no package for Arch - testing the AppImage"
fi

# --- 2. the installed package --------------------------------------------
if [ "$PKG" != none ]; then
    check "nd104-cli --version reports $VERSION" bash -c "nd104-cli --version | grep -F '$VERSION'"
    check "udev rule installed" test -f /usr/lib/udev/rules.d/70-nd104-studio.rules
    check "nd104-studio: no missing shared library" bash -c '! ldd "$(command -v nd104-studio)" | grep -q "not found"'
    runs_for_20s "nd104-studio runs under Xvfb" nd104-studio --minimized
fi

# --- 3. the AppImage (every distro; the only option where no package fits) -
app=$(ls "$DIST"/nd104-studio_*_amd64.AppImage 2>/dev/null | head -1)
if [ -n "$app" ]; then
    # no FUSE in containers: unpack it and run AppRun from the directory
    rm -rf /tmp/appimage && mkdir -p /tmp/appimage
    cp "$app" /tmp/appimage/app.AppImage && chmod +x /tmp/appimage/app.AppImage
    if (cd /tmp/appimage && ./app.AppImage --appimage-extract >/dev/null 2>&1); then
        pass "AppImage unpacks"
        runs_for_20s "AppImage runs under Xvfb" /tmp/appimage/squashfs-root/AppRun --minimized
    else
        fail "AppImage does not unpack"
    fi
else
    echo "   skip no AppImage in $DIST"
fi

if [ "$FAILED" -eq 0 ]; then
    echo "=== result: PASS ($PRETTY_NAME) ==="
else
    echo "=== result: FAIL ($PRETTY_NAME) ==="
fi
exit "$FAILED"
