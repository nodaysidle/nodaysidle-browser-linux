#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_SRC="${ROOT}/assets/icon.svg"
ICON_DIR="${HOME}/.local/share/icons/hicolor"

echo "Building release binary..."
cargo build --release --manifest-path "${ROOT}/Cargo.toml"

install -d "${BIN_DIR}"
install -m 0755 "${ROOT}/target/release/nodaysidle-browser" "${BIN_DIR}/nodaysidle-browser"

install -d "${APP_DIR}"
cat > "${APP_DIR}/nodaysidle-browser.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=nodaysidle
GenericName=Web Browser
Comment=Quiet WebKit browser for focused browsing
Exec=${BIN_DIR}/nodaysidle-browser %u
Icon=nodaysidle-browser
Terminal=false
Categories=Network;WebBrowser;
MimeType=text/html;text/xml;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;
StartupWMClass=nodaysidle-browser
Keywords=browser;web;nodaysidle;
EOF
chmod 0644 "${APP_DIR}/nodaysidle-browser.desktop"

if command -v rsvg-convert >/dev/null 2>&1; then
  for size in 48 128 256; do
    install -d "${ICON_DIR}/${size}x${size}/apps"
    rsvg-convert -w "${size}" -h "${size}" "${ICON_SRC}" \
      -o "${ICON_DIR}/${size}x${size}/apps/nodaysidle-browser.png"
  done
else
  echo "Tip: install librsvg (rsvg-convert) for PNG launcher icons."
  install -d "${HOME}/.local/share/pixmaps"
  install -m 0644 "${ICON_SRC}" "${HOME}/.local/share/pixmaps/nodaysidle-browser.svg"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${HOME}/.local/share/applications" || true
fi

echo "Installed nodaysidle-browser to ${BIN_DIR}"
echo "Open Super+Space → Apps and search for nodaysidle."
