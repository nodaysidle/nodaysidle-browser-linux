#!/usr/bin/env bash
# Packages nodaysidle-browser into a host-dependent .AppImage for Linux x86_64.
# GTK 3 and WebKitGTK 4.1 must be installed on the target system (see docs/APPIMAGE.md).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

echo "Building release binary (locked dependencies)..."
cargo build --release --locked

APP_DIR="${ROOT}/target/AppDir"
OUT_DIR="${ROOT}/dist"
APPIMAGE="${OUT_DIR}/nodaysidle-browser-x86_64.AppImage"
TOOL_DIR="${ROOT}/target/appimagetool-cache"
TOOL="${TOOL_DIR}/appimagetool-x86_64.AppImage"
CHECKSUM_FILE="${ROOT}/scripts/appimagetool.sha256"
APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/usr/bin"
mkdir -p "${APP_DIR}/usr/share/applications"
mkdir -p "${APP_DIR}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${OUT_DIR}"
mkdir -p "${TOOL_DIR}"

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

verify_appimagetool() {
  local expected
  expected="$(awk '{print $1}' "${CHECKSUM_FILE}")"
  local actual
  actual="$(sha256sum "${TOOL}" | awk '{print $1}')"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "appimagetool checksum mismatch (expected ${expected}, got ${actual})" >&2
    rm -f "${TOOL}"
    return 1
  fi
}

download_appimagetool() {
  local tmp="${TOOL}.partial"
  rm -f "${tmp}"
  echo "Downloading pinned appimagetool..."
  curl -L -s -f -o "${tmp}" "${APPIMAGETOOL_URL}"
  chmod 0755 "${tmp}"
  if ! sha256sum -c "${CHECKSUM_FILE}" --status 2>/dev/null; then
    # checksum file names the final artifact; verify the download explicitly
    local expected actual
    expected="$(awk '{print $1}' "${CHECKSUM_FILE}")"
    actual="$(sha256sum "${tmp}" | awk '{print $1}')"
    if [[ "${actual}" != "${expected}" ]]; then
      rm -f "${tmp}"
      echo "Downloaded appimagetool failed checksum verification" >&2
      exit 1
    fi
  fi
  mv "${tmp}" "${TOOL}"
}

if [[ -x "${TOOL}" ]]; then
  verify_appimagetool || download_appimagetool
elif command -v appimagetool >/dev/null 2>&1; then
  TOOL="$(command -v appimagetool)"
  echo "Using system appimagetool at ${TOOL} (not checksum-pinned)."
else
  download_appimagetool
fi

echo "Generating ${APPIMAGE}..."
ARCH=x86_64 "${TOOL}" --appimage-extract-and-run "${APP_DIR}" "${APPIMAGE}"
echo "AppImage created successfully: ${APPIMAGE}"
