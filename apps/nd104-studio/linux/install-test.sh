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
APP_VERSION=${2:?version}
PRETTY_NAME=$(. /etc/os-release; echo "$PRETTY_NAME")  # os-release sets VERSION too
echo "=== $PRETTY_NAME - ND104 Studio $APP_VERSION ==="

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

# Starts a GUI command on its own Xvfb display with a session bus; a tray
# app has to be still running when the timeout hits (rc 124), anything else
# is a crash. Xvfb is started directly - the xvfb-run wrapper is not packaged
# the same way everywhere.
runs_for_20s() { # runs_for_20s <description> <command...>
    local what=$1; shift
    if ! command -v Xvfb >/dev/null; then
        fail "$what: Xvfb not installed (test tooling)"
        return
    fi
    Xvfb :99 -screen 0 1280x800x24 >/tmp/xvfb.out 2>&1 &
    local xvfb=$!
    sleep 2
    # X11 explicitly (GDK_BACKEND): Xvfb is an X server, and X11 is what
    # desktops not yet on Wayland run - this must keep working.
    local bus=()
    if command -v dbus-run-session >/dev/null; then bus=(dbus-run-session --)
    elif command -v dbus-launch >/dev/null; then bus=(dbus-launch --exit-with-session)
    fi
    DISPLAY=:99 GDK_BACKEND=x11 timeout 20 "${bus[@]}" "$@" >/tmp/gui.out 2>&1
    local rc=$?
    kill "$xvfb" 2>/dev/null
    wait "$xvfb" 2>/dev/null
    if [ "$rc" -eq 124 ]; then
        pass "$what (still up after 20 s)"
    else
        fail "$what exited early (rc=$rc)"
        tail -15 /tmp/gui.out | sed 's/^/        /'
    fi
}

# --- 1. test tooling, and the package the distribution can take ------------
# Xvfb + a session bus for the GUI start. GTK 3 is the desktop baseline the
# AppImage expects from the host: linuxdeploy deliberately leaves base
# libraries such as libwayland-client, fribidi and libGL to the system, and
# any desktop that can show a tray app has them - minimal containers do not.
# Errors of the tooling install are shown, not hidden.
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
    # one by one: dnf installs all or nothing, one unknown name dropped the rest
    for p in xorg-x11-server-Xvfb dbus-daemon dbus-x11 mesa-libGL mesa-libEGL libglvnd-gles \n             libgpg-error fontconfig gtk3; do
        dnf install -y -q "$p" >/dev/null 2>&1 || echo "   note no package $p here"
    done
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
    for p in xorg-x11-server-Xvfb dbus-1 dbus-1-tools dbus-1-x11 Mesa-libGL1 Mesa-libEGL1              libGLESv2-2 libgpg-error0 fontconfig libgtk-3-0; do
        zypper --non-interactive --quiet install "$p" >/dev/null 2>&1 || echo "   note no package $p here"
    done
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
    pacman -Syu --noconfirm --quiet xorg-server-xvfb dbus mesa fontconfig gtk3 >/dev/null || true
    echo "   skip no package for Arch - testing the AppImage"
fi

# --- 2. the installed package --------------------------------------------
if [ "$PKG" != none ]; then
    # the .deb/.rpm ship the GUI only; nd104-cli is in the tarball (known gap)
    if command -v nd104-cli >/dev/null; then
        check "nd104-cli --version reports $APP_VERSION" bash -c "nd104-cli --version | grep -F '$APP_VERSION'"
    else
        echo "   skip nd104-cli not in the package (tarball only)"
    fi
    check "udev rule installed" test -f /usr/lib/udev/rules.d/70-nd104-studio.rules
    check "nd104-studio: no missing shared library" bash -c '! ldd "$(command -v nd104-studio)" | grep -q "not found"'
    runs_for_20s "nd104-studio runs on X11 (Xvfb)" nd104-studio --minimized
fi

# --- 3. the AppImage (every distro; the only option where no package fits) -
app=$(ls "$DIST"/nd104-studio_*_amd64.AppImage 2>/dev/null | head -1)
if [ -n "$app" ]; then
    # no FUSE in containers: unpack it and run AppRun from the directory
    rm -rf /tmp/appimage && mkdir -p /tmp/appimage
    cp "$app" /tmp/appimage/app.AppImage && chmod +x /tmp/appimage/app.AppImage
    if (cd /tmp/appimage && ./app.AppImage --appimage-extract >/dev/null 2>&1); then
        pass "AppImage unpacks"
        runs_for_20s "AppImage runs on X11 (Xvfb)" /tmp/appimage/squashfs-root/AppRun --minimized
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
