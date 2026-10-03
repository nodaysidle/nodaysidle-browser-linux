#!/usr/bin/env bash
# Installs nodaysidle-browser for the current user:
#   ~/.local/bin/nodaysidle-browser
#   ~/.local/share/applications/com.nodaysidle.Browser.desktop
#   ~/.local/share/icons/hicolor/{scalable,48x48,128x128,256x256}/apps/nodaysidle-browser.*
# The desktop file is desktop/com.nodaysidle.Browser.desktop with only the
# Exec line pointing at the installed binary. Its name matches the
# application ID, which the browser also uses as its Wayland app_id and X11
# WM_CLASS (Hyprland, GNOME, KDE, docks and taskbars match on it).
# Upgrading from nodaysidle-browser.desktop moves default-browser and MIME
# associations to the new name and removes the old launcher.
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

# Quote a path for a desktop file Exec key. Paths made only of safe
# characters are written bare: xdg-settings (xdg-utils 1.2.1) takes the first
# word of Exec literally, quotes included, and cannot set a browser whose
# Exec is quoted. Other paths are wrapped in double quotes, with " ` $ \
# backslash-escaped inside, then every backslash doubled (the value is itself
# a string with escapes) and every % doubled (field codes).
desktop_exec_quote() {
  local s="$1"
  if [[ "${s}" =~ ^[A-Za-z0-9/._+,:@=-]+$ ]]; then
    printf '%s' "${s}"
    return
  fi
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

# Earlier versions installed nodaysidle-browser.desktop (always under
# $HOME/.local/share, whatever XDG_DATA_HOME said). Before removing it, carry
# the user's default-browser and MIME choices over to the new name (V-1).
OLD_NAME="nodaysidle-browser.desktop"
NEW_NAME="com.nodaysidle.Browser.desktop"

# True if the file is a launcher this script used to generate.
is_our_old_launcher() {
  [[ -f "$1" ]] \
    && grep -qx 'Name=nodaysidle' "$1" \
    && grep -qx 'StartupWMClass=nodaysidle-browser' "$1"
}
OLD_APP_DIRS=("${APP_DIR}")
[[ "${APP_DIR}" != "${HOME}/.local/share/applications" ]] && OLD_APP_DIRS+=("${HOME}/.local/share/applications")
# A nodaysidle-browser.desktop that this script did not write belongs to the
# user; then their associations really point at that file and stay as they are.
migrate_defaults=1
for app_dir in "${OLD_APP_DIRS[@]}"; do
  if [[ -e "${app_dir}/${OLD_NAME}" ]] && ! is_our_old_launcher "${app_dir}/${OLD_NAME}"; then
    migrate_defaults=0
    echo "Keeping ${app_dir}/${OLD_NAME} and the associations that use it: not created by this script"
  fi
done

old_default_browser=""
if (( migrate_defaults )) && command -v xdg-settings >/dev/null 2>&1; then
  old_default_browser="$(env -u BROWSER xdg-settings get default-web-browser 2>/dev/null || true)"
fi

# Rewrites OLD_NAME to NEW_NAME in the values of one mimeapps.list. Only
# key=value lines whose ;-separated list contains OLD_NAME change (without
# duplicating NEW_NAME); every other line is copied unchanged. The original
# is kept as <file>.nodaysidle-backup.
migrate_mimeapps() {
  local file="$1" tmp
  [[ -f "${file}" ]] || return 0
  grep -q "${OLD_NAME}" "${file}" || return 0
  tmp="$(mktemp "${file}.XXXXXX")"
  awk -v old="${OLD_NAME}" -v new="${NEW_NAME}" '
    /^[[:space:]]*[#[]/ || index($0, "=") == 0 { print; next }
    {
      eq = index($0, "=")
      key = substr($0, 1, eq - 1)
      value = substr($0, eq + 1)
      n = split(value, items, ";")
      found = 0
      for (i = 1; i <= n; i++) if (items[i] == old) found = 1
      if (!found) { print; next }
      out = ""; seen_new = 0; count = 0
      for (i = 1; i <= n; i++) {
        item = items[i]
        if (item == old) item = new
        if (item == "") continue
        if (item == new) { if (seen_new) continue; seen_new = 1 }
        out = out item ";"; count++
      }
      if (substr(value, length(value)) != ";" && count > 0) out = substr(out, 1, length(out) - 1)
      print key "=" out
    }' "${file}" > "${tmp}"
  if cmp -s "${file}" "${tmp}"; then
    rm -f "${tmp}"
    return 0
  fi
  cp -p "${file}" "${file}.nodaysidle-backup"
  chmod --reference="${file}" "${tmp}" 2>/dev/null || chmod 0644 "${tmp}"
  mv -f "${tmp}" "${file}"
  echo "Updated ${file}: ${OLD_NAME} -> ${NEW_NAME} (backup: ${file}.nodaysidle-backup)"
}

CONFIG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}"
declare -A seen_mimeapps=()
(( migrate_defaults )) && for list in "${CONFIG_DIR}"/mimeapps.list "${CONFIG_DIR}"/*-mimeapps.list \
  "${DATA_DIR}/applications/mimeapps.list" "${HOME}/.local/share/applications/mimeapps.list"; do
  [[ -f "${list}" ]] || continue
  real="$(realpath "${list}")"
  [[ -n "${seen_mimeapps[${real}]:-}" ]] && continue
  seen_mimeapps[${real}]=1
  # Rewrite the target, so a symlinked mimeapps.list (dotfiles) stays a link.
  migrate_mimeapps "${real}"
done

# Remove the old launcher only if this script generated it, so launchers
# do not list the app twice; look in both possible data directories.
for app_dir in "${OLD_APP_DIRS[@]}"; do
  old_desktop="${app_dir}/${OLD_NAME}"
  if is_our_old_launcher "${old_desktop}"; then
    rm -f "${old_desktop}"
    echo "Removed the old ${old_desktop}"
    if [[ "${app_dir}" != "${APP_DIR}" ]] && command -v update-desktop-database >/dev/null 2>&1; then
      update-desktop-database "${app_dir}" || true
    fi
  fi
done

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

# Desktops that keep the default browser outside mimeapps.list (KDE, GNOME
# via gio) are updated through xdg-settings, but only if nodaysidle was the
# default before.
if [[ "${old_default_browser}" == "${OLD_NAME}" ]]; then
  if env -u BROWSER xdg-settings set default-web-browser "${NEW_NAME}" 2>/dev/null; then
    echo "Default web browser: ${OLD_NAME} -> ${NEW_NAME}"
  else
    echo "Could not update the default web browser; run:"
    echo "  xdg-settings set default-web-browser ${NEW_NAME}"
  fi
fi

echo "Installed nodaysidle-browser to ${BIN}"
echo "Desktop entry: ${DESKTOP_DST}"
