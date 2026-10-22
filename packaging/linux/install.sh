#!/bin/sh
# Installs MCQ Advanced (Studio and player) for the current user only: no root, nothing outside $HOME.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
bin="$HOME/.local/bin"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
apps="$data/applications"
icons="$data/icons/hicolor"
mkdir -p "$bin" "$apps" "$icons/256x256/apps" "$icons/scalable/apps"
install -m 755 "$here/mcq-studio" "$bin/mcq-studio"
install -m 755 "$here/mcq-player" "$bin/mcq-player"
install -m 644 "$here/icons/mcq-advanced.png" "$icons/256x256/apps/mcq-advanced.png"
install -m 644 "$here/icons/mcq-advanced.svg" "$icons/scalable/apps/mcq-advanced.svg"
entry() {
    cat <<ENTRY
[Desktop Entry]
Type=Application
Name=$2
Comment=$3
Exec=$bin/$1
Icon=mcq-advanced
Terminal=false
Categories=Education;
StartupWMClass=$1
ENTRY
}
entry mcq-studio "MCQ Advanced Studio" "Write, publish and correct multiple-choice papers" > "$apps/mcq-advanced-studio.desktop"
entry mcq-player "MCQ Advanced Player" "Take a multiple-choice paper" > "$apps/mcq-advanced-player.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$apps" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1 && [ -f "$icons/index.theme" ]; then
    gtk-update-icon-cache -q "$icons" 2>/dev/null || true
fi
echo "MCQ Advanced installed in $bin."
echo "Open it from your applications menu, or run: $bin/mcq-studio (teacher) or $bin/mcq-player (student)"
