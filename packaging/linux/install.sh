#!/bin/sh
# Installs MCQ Advanced (Studio and player) for the current user only: no root, nothing outside $HOME.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
bin="$HOME/.local/bin"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$bin" "$apps"
install -m 755 "$here/mcq-studio" "$bin/mcq-studio"
install -m 755 "$here/mcq-player" "$bin/mcq-player"
entry() {
    cat <<ENTRY
[Desktop Entry]
Type=Application
Name=$2
Comment=$3
Exec=$bin/$1
Icon=x-office-document
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
echo "MCQ Advanced installed in $bin."
echo "Open it from your applications menu, or run: $bin/mcq-studio (teacher) or $bin/mcq-player (student)"
