#!/usr/bin/env bash
# Packages a fully bundled nodaysidle-browser AppImage (x86_64) that ships GTK 3,
# WebKitGTK 4.1 and their dependencies, so the target system does not need them.
# Build host: Debian/Ubuntu with libgtk-3-dev, libwebkit2gtk-4.1-dev, patchelf,
# librsvg2-common, gsettings-desktop-schemas. The slim, host-dependent build is
# still produced by scripts/package-appimage.sh.
#
# Uses linuxdeploy + linuxdeploy-plugin-gtk. Set LINUXDEPLOY / LINUXDEPLOY_PLUGIN_GTK
# to local copies to avoid downloading the "continuous" builds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

MULTIARCH="$(gcc -print-multiarch 2>/dev/null || echo x86_64-linux-gnu)"
SYSLIB="/usr/lib/${MULTIARCH}"
WK_SYS="${SYSLIB}/webkit2gtk-4.1"
APP_DIR="${ROOT}/target/AppDir-bundled"
OUT_DIR="${ROOT}/dist"
OUT="${OUT_DIR}/nodaysidle-browser-bundled-x86_64.AppImage"
TOOL_DIR="${ROOT}/target/linuxdeploy-cache"
LINUXDEPLOY="${LINUXDEPLOY:-${TOOL_DIR}/linuxdeploy-x86_64.AppImage}"
LINUXDEPLOY_PLUGIN_GTK="${LINUXDEPLOY_PLUGIN_GTK:-${TOOL_DIR}/linuxdeploy-plugin-gtk.sh}"

[[ -x "${WK_SYS}/WebKitWebProcess" ]] || { echo "WebKitGTK 4.1 helpers not found in ${WK_SYS}" >&2; exit 1; }

mkdir -p "${TOOL_DIR}" "${OUT_DIR}"
[[ -x "${LINUXDEPLOY}" ]] || { curl -L -s -f -o "${LINUXDEPLOY}" \
  https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage; chmod +x "${LINUXDEPLOY}"; }
[[ -x "${LINUXDEPLOY_PLUGIN_GTK}" ]] || { curl -L -s -f -o "${LINUXDEPLOY_PLUGIN_GTK}" \
  https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh; chmod +x "${LINUXDEPLOY_PLUGIN_GTK}"; }
export PATH="$(dirname "${LINUXDEPLOY_PLUGIN_GTK}"):${PATH}"

echo "Building release binary (locked dependencies)..."
cargo build --release --locked

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/usr/lib"

# WebKit helper processes and injected bundle. libwebkit2gtk looks them up at a
# compiled-in absolute path; below that path is rewritten (same length) to a
# relative one that AppRun resolves by running from ${APPDIR}/usr.
cp -a "${WK_SYS}" "${APP_DIR}/usr/lib/webkit2gtk-4.1"
rm -f "${APP_DIR}/usr/lib/webkit2gtk-4.1/MiniBrowser"

# GIO modules (TLS via glib-networking) and schemas.
mkdir -p "${APP_DIR}/usr/lib/gio/modules"
cp -a "${SYSLIB}"/gio/modules/libgio*.so "${APP_DIR}/usr/lib/gio/modules/" 2>/dev/null || true
mkdir -p "${APP_DIR}/usr/share/glib-2.0/schemas"
cp -a /usr/share/glib-2.0/schemas/*.xml "${APP_DIR}/usr/share/glib-2.0/schemas/"
glib-compile-schemas "${APP_DIR}/usr/share/glib-2.0/schemas"

deploy_args=()
for f in "${APP_DIR}"/usr/lib/webkit2gtk-4.1/WebKit*Process \
         "${APP_DIR}"/usr/lib/webkit2gtk-4.1/injected-bundle/*.so \
         "${APP_DIR}"/usr/lib/gio/modules/*.so; do
  [[ -e "$f" ]] && deploy_args+=(--deploy-deps-only "$f")
done

DEPLOY_GTK_VERSION=3 NO_STRIP=1 ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "${LINUXDEPLOY}" \
  --appdir "${APP_DIR}" \
  --executable "${ROOT}/target/release/nodaysidle-browser" \
  --desktop-file "${ROOT}/desktop/com.nodaysidle.Browser.desktop" \
  --icon-file "${ROOT}/assets/icon.svg" --icon-filename nodaysidle-browser \
  --library "${SYSLIB}/libwebkit2gtk-4.1.so.0" \
  "${deploy_args[@]}" \
  --plugin gtk

# Rewrite the WebKit helper/injected-bundle path inside the bundled library.
WKLIB="$(ls "${APP_DIR}"/usr/lib/libwebkit2gtk-4.1.so.0* | head -n1)"
python3 - "${WKLIB}" "${WK_SYS}" << 'PY'
import sys
path, old = sys.argv[1], sys.argv[2].encode()
tail = b"lib/webkit2gtk-4.1"
pad = len(old) - len(tail)
assert pad > 0 and pad % 2 == 0, "unexpected WebKit path length"
new = b"./" * (pad // 2) + tail
data = open(path, "rb").read()
n = data.count(old)
assert n > 0, "WebKit path not found in library"
open(path, "wb").write(data.replace(old, new))
print(f"patched {n} occurrence(s) of {old.decode()} -> {new.decode()}")
PY

# AppRun: environment for the bundled toolkit, then run from ${APPDIR}/usr so the
# relative WebKit helper path resolves. Existing file arguments are made absolute first.
rm -f "${APP_DIR}/AppRun"
cat << 'RUN' > "${APP_DIR}/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export APPDIR="${APPDIR:-${HERE}}"
for hook in "${HERE}"/apprun-hooks/*.sh; do [ -f "${hook}" ] && . "${hook}"; done
export LD_LIBRARY_PATH="${HERE}/usr/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
export GIO_MODULE_DIR="${HERE}/usr/lib/gio/modules"
export GSETTINGS_SCHEMA_DIR="${HERE}/usr/share/glib-2.0/schemas"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
n=$#
while [ "${n}" -gt 0 ]; do
  a="$1"; shift
  case "${a}" in
    /*|-*) ;;
    *) [ -e "${a}" ] && a="$(readlink -f "${a}")" ;;
  esac
  set -- "$@" "${a}"; n=$((n - 1))
done
cd "${HERE}/usr" || exit 1
exec "${HERE}/usr/bin/nodaysidle-browser" "$@"
RUN
chmod +x "${APP_DIR}/AppRun"

APPIMAGETOOL="$(dirname "${LINUXDEPLOY}")/appimagetool-x86_64.AppImage"
[[ -x "${APPIMAGETOOL}" ]] || { curl -L -s -f -o "${APPIMAGETOOL}" \
  https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage; chmod +x "${APPIMAGETOOL}"; }
ARCH=x86_64 "${APPIMAGETOOL}" --appimage-extract-and-run "${APP_DIR}" "${OUT}"
echo "Bundled AppImage created: ${OUT}"
