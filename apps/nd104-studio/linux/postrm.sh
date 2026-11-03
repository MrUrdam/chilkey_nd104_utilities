#!/bin/sh
# Runs after the package is removed. The package manager has already deleted the
# udev rule it installed; reload udev so the change takes effect.
set -e

if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
fi

exit 0
