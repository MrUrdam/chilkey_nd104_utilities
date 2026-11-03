#!/bin/sh
# Runs after the .deb/.rpm is unpacked. Reloads udev so the ND104 access rule
# (70-nd104-studio.rules) takes effect without a reboot; the logged-in user can
# then open the keyboard without root. Harmless if udev is not present.
set -e

if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger --subsystem-match=hidraw || true
fi

exit 0
