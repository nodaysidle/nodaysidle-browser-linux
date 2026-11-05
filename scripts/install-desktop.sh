#!/usr/bin/env bash
# Installs nodaysidle-browser for the current user so it shows in the Omarchy app menu:
#   ~/.local/bin/nodaysidle-browser
#   ~/.local/share/applications/com.nodaysidle.Browser.desktop
#   ~/.local/share/icons/hicolor/scalable/apps/nodaysidle-browser.svg (and PNG sizes with rsvg-convert)
#   ~/.config/fish/completions/nodaysidle-browser.fish
# The desktop file name matches the Wayland app_id the browser sets, so Hyprland
# window rules and the launcher match on it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
APP_DIR="${DATA_DIR}/applications"
ICON_DIR="${DATA_DIR}/icons/hicolor"
FISH_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/fish/completions"

"${ROOT}/bin/build"

install -d "${BIN_DIR}" "${APP_DIR}" "${ICON_DIR}/scalable/apps" "${FISH_DIR}"
install -m 0755 "${ROOT}/build/nodaysidle-browser" "${BIN_DIR}/nodaysidle-browser"
sed "s|^Exec=nodaysidle-browser|Exec=${BIN_DIR}/nodaysidle-browser|" \
  "${ROOT}/desktop/com.nodaysidle.Browser.desktop" > "${APP_DIR}/com.nodaysidle.Browser.desktop"
install -m 0644 "${ROOT}/assets/icon.svg" "${ICON_DIR}/scalable/apps/nodaysidle-browser.svg"
if command -v rsvg-convert >/dev/null 2>&1; then
  for size in 48 128 256; do
    install -d "${ICON_DIR}/${size}x${size}/apps"
    rsvg-convert -w "${size}" -h "${size}" "${ROOT}/assets/icon.svg" \
      -o "${ICON_DIR}/${size}x${size}/apps/nodaysidle-browser.png"
  done
fi
install -m 0644 "${ROOT}/completions/nodaysidle-browser.fish" "${FISH_DIR}/nodaysidle-browser.fish"

command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t -f "${ICON_DIR}" || true
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "${APP_DIR}" || true

echo "Installed ${BIN_DIR}/nodaysidle-browser"
echo "Launcher entry: ${APP_DIR}/com.nodaysidle.Browser.desktop"
