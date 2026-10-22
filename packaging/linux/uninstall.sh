#!/bin/sh
# Removes what install.sh added. Drafts and settings in ~/.local/share/mcq-* are kept.
set -eu
bin="$HOME/.local/bin"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
rm -f "$bin/mcq-studio" "$bin/mcq-player" "$apps/mcq-advanced-studio.desktop" "$apps/mcq-advanced-player.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$apps" >/dev/null 2>&1 || true
fi
echo "MCQ Advanced removed."
