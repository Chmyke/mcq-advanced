#!/bin/sh
# Removes what install.sh added. Drafts and settings in ~/.local/share/mcq-* are kept.
set -eu
bin="$HOME/.local/bin"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
apps="$data/applications"
rm -f "$bin/mcq-studio" "$bin/mcq-player" "$apps/mcq-advanced-studio.desktop" "$apps/mcq-advanced-player.desktop"
rm -f "$data/icons/hicolor/256x256/apps/mcq-advanced.png" "$data/icons/hicolor/scalable/apps/mcq-advanced.svg"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$apps" >/dev/null 2>&1 || true
fi
echo "MCQ Advanced removed."
