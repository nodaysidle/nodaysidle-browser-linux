#!/usr/bin/env bash
# Installs nodaysidle-browser for the current user:
#   ~/.local/bin/nodaysidle-browser
#   ~/.local/share/applications/com.nodaysidle.Browser.desktop
#   ~/.local/share/icons/hicolor/{scalable,48x48,128x128,256x256}/apps/nodaysidle-browser.*
# The desktop file is desktop/com.nodaysidle.Browser.desktop with only the
# Exec line pointing at the installed binary. Its name matches the GTK
# application ID, which is the Wayland app_id (Hyprland, GNOME, KDE).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
APP_DIR="${DATA_DIR}/applications"
ICON_SRC="${ROOT}/assets/icon.svg"
ICON_DIR="${DATA_DIR}/icons/hicolor"
DESKTOP_SRC="${ROOT}/desktop/com.nodaysidle.Browser.desktop"
DESKTOP_DST="${APP_DIR}/com.nodaysidle.Browser.desktop"
BIN="${BIN_DIR}/nodaysidle-browser"

# Quote a path for a desktop file Exec key: wrap it in double quotes,
# backslash-escape " ` $ \ inside, then double every backslash (the value is
# itself a string with escapes) and every % (field codes).
desktop_exec_quote() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//\`/\\\`}"
  s="${s//\$/\\\$}"
  s="${s//\\/\\\\}"
  s="${s//%/%%}"
  printf '"%s"' "$s"
}

echo "Building release binary..."
cargo build --release --manifest-path "${ROOT}/Cargo.toml"

install -d "${BIN_DIR}"
install -m 0755 "${ROOT}/target/release/nodaysidle-browser" "${BIN}"

install -d "${APP_DIR}"
EXEC_LINE="Exec=$(desktop_exec_quote "${BIN}") %u"
tmp="$(mktemp "${APP_DIR}/.com.nodaysidle.Browser.XXXXXX")"
while IFS= read -r line || [[ -n "${line}" ]]; do
  if [[ "${line}" == Exec=* ]]; then
    printf '%s\n' "${EXEC_LINE}"
  else
    printf '%s\n' "${line}"
  fi
done < "${DESKTOP_SRC}" > "${tmp}"
chmod 0644 "${tmp}"
mv -f "${tmp}" "${DESKTOP_DST}"

# Earlier versions installed nodaysidle-browser.desktop; remove it only if it
# is the one this script generated, so launchers do not list the app twice.
OLD_DESKTOP="${APP_DIR}/nodaysidle-browser.desktop"
if [[ -f "${OLD_DESKTOP}" ]] \
  && grep -qx 'Name=nodaysidle' "${OLD_DESKTOP}" \
  && grep -qx 'StartupWMClass=nodaysidle-browser' "${OLD_DESKTOP}"; then
  rm -f "${OLD_DESKTOP}"
  echo "Removed the old ${OLD_DESKTOP}"
fi

# The scalable SVG always; PNG sizes too when rsvg-convert is available.
install -d "${ICON_DIR}/scalable/apps"
install -m 0644 "${ICON_SRC}" "${ICON_DIR}/scalable/apps/nodaysidle-browser.svg"
if command -v rsvg-convert >/dev/null 2>&1; then
  for size in 48 128 256; do
    install -d "${ICON_DIR}/${size}x${size}/apps"
    rsvg-convert -w "${size}" -h "${size}" "${ICON_SRC}" \
      -o "${ICON_DIR}/${size}x${size}/apps/nodaysidle-browser.png"
  done
fi
# Earlier versions put the SVG in ~/.local/share/pixmaps, which icon themes
# do not search; drop that copy if it is ours.
OLD_PIXMAP="${DATA_DIR}/pixmaps/nodaysidle-browser.svg"
if [[ -f "${OLD_PIXMAP}" ]] && cmp -s "${OLD_PIXMAP}" "${ICON_SRC}"; then
  rm -f "${OLD_PIXMAP}"
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f "${ICON_DIR}" || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" || true
fi

echo "Installed nodaysidle-browser to ${BIN}"
echo "Desktop entry: ${DESKTOP_DST}"
