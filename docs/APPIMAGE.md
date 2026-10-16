# AppImage packaging contract

The `.AppImage` produced by `scripts/package-appimage.sh` bundles the
`nodaysidle-browser` executable, launcher metadata, and icon. It does **not**
bundle GTK 3, WebKitGTK 4.1, JavaScriptCore, libsoup3, or WebKit helper
processes.

## Supported baseline

Treat the image as **host-dependent**: the target system must provide a
compatible WebKitGTK **4.1** stack (same major/minor family as the build host).
On Arch Linux and Omarchy this is typically:

```bash
sudo pacman -S --needed gtk3 webkit2gtk-4.1
```

Other distributions need equivalent packages (`webkit2gtk-4.1` / `libsoup-3.0`).
Building an AppImage on a developer machine is not a portability test; validate
on a clean system that only installs the documented dependencies.

## Security updates

WebKit security fixes reach users through their distribution (or manual
upgrades of system WebKitGTK packages), not through rebuilding this AppImage
alone.

## Build integrity

Release packaging uses `cargo build --release --locked`. The optional
`appimagetool` download is pinned by `scripts/appimagetool.sha256` and stored
under `target/appimagetool-cache/` (never trusted from `/tmp`).

An executable cached tool takes precedence and is verified on every run. An
invalid cache entry is removed and replaced only with a verified download;
this recovery does not fall back to PATH. Downloads are hashed at their actual
`.partial` path before becoming executable or being promoted into the cache.
Failed downloads and catchable interruptions remove the partial file. Any stale
partial left by an uncatchable interruption is discarded before retrying.

When there is no executable cached tool, an `appimagetool` found on PATH is
trusted as a locally managed tool and used **without checksum verification**.
The script announces this choice. Ensure PATH contains only tools you trust;
to use the pinned download, make sure no `appimagetool` is on PATH. The download
URL uses the mutable `continuous` release, so changed upstream bytes fail closed
against the fixed digest rather than silently updating the pin.

Run the isolated packaging regression tests without network access or a Rust
build:

```bash
python3 -B scripts/test-package-appimage.py
```

## Bundled AppImage (experimental)

`scripts/package-appimage-bundled.sh` builds `dist/nodaysidle-browser-bundled-x86_64.AppImage`
with linuxdeploy + linuxdeploy-plugin-gtk. It bundles GTK 3, WebKitGTK 4.1 and their
libraries, the WebKit helper processes (`WebKitNetworkProcess`, `WebKitWebProcess`,
`WebKitGPUProcess`), the injected bundle, gdk-pixbuf loaders, GIO modules and GSettings
schemas. The compiled-in WebKit helper path in the bundled `libwebkit2gtk-4.1.so.0` is
rewritten to a relative path, so `AppRun` starts the browser from `$APPDIR/usr` (existing
file arguments are converted to absolute paths first). WebKit's sandbox still needs `bwrap`
on the host. The slim build (`scripts/package-appimage.sh`) is unchanged. The
`appimage-bundled.yml` workflow builds it, test-runs it in `debian:bookworm-slim`, and
uploads it as a CI artifact. It is not a release.
