#!/usr/bin/env bash
# Packages nodaysidle-browser into a standalone .AppImage for Linux x86_64.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

echo "Building release binary..."
cargo build --release

APP_DIR="${ROOT}/target/AppDir"
OUT_DIR="${ROOT}/dist"
APPIMAGE="${OUT_DIR}/nodaysidle-browser-x86_64.AppImage"

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/usr/bin"
mkdir -p "${APP_DIR}/usr/share/applications"
mkdir -p "${APP_DIR}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${OUT_DIR}"

# Binary and desktop metadata
install -m 0755 "${ROOT}/target/release/nodaysidle-browser" "${APP_DIR}/usr/bin/nodaysidle-browser"
install -m 0644 "${ROOT}/desktop/com.nodaysidle.Browser.desktop" "${APP_DIR}/usr/share/applications/com.nodaysidle.Browser.desktop"
install -m 0644 "${ROOT}/assets/icon.svg" "${APP_DIR}/usr/share/icons/hicolor/scalable/apps/nodaysidle-browser.svg"

# AppImage root metadata
install -m 0644 "${ROOT}/desktop/com.nodaysidle.Browser.desktop" "${APP_DIR}/com.nodaysidle.Browser.desktop"
install -m 0644 "${ROOT}/assets/icon.svg" "${APP_DIR}/nodaysidle-browser.svg"
ln -sf nodaysidle-browser.svg "${APP_DIR}/.DirIcon"

# AppRun entrypoint
cat << 'RUN' > "${APP_DIR}/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "${HERE}/usr/bin/nodaysidle-browser" "$@"
RUN
chmod +x "${APP_DIR}/AppRun"

# Find or download appimagetool
TOOL="${ROOT}/target/appimagetool"
if [[ ! -x "${TOOL}" ]]; then
  if command -v appimagetool >/dev/null 2>&1; then
    TOOL="$(command -v appimagetool)"
  elif [[ -x "/tmp/appimagetool" ]]; then
    TOOL="/tmp/appimagetool"
  else
    echo "Downloading appimagetool..."
    curl -L -s -f -o "${TOOL}" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
    chmod +x "${TOOL}"
  fi
fi

echo "Generating ${APPIMAGE}..."
ARCH=x86_64 "${TOOL}" --appimage-extract-and-run "${APP_DIR}" "${APPIMAGE}"
echo "AppImage created successfully: ${APPIMAGE}"
