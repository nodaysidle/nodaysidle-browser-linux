# nodaysidle-browser (Linux)

GTK + WebKitGTK port of [nodaysidle-browser](https://github.com/nodaysidle/nodaysidle-browser) for Omarchy / Linux.

## Features (v0.1 scaffold)

- Multi-tab browsing (WebKitGTK)
- URL bar with search / URL rules aligned with the macOS app
- **Persistent profile** (cookies in `webkit-data/cookies.sqlite` and site data) under `~/.local/share/nodaysidle-browser/`
- **Local history** at `~/.local/share/nodaysidle-browser/history.json`
- Open HTTP(S) URLs and local HTML files passed by other applications or the command line
- Downloads prompt for a save location and show progress with cancel support
- Site requests for location, camera, microphone, notifications, and pointer lock require explicit approval
- Keyboard shortcuts: Ctrl+T/W/L/Tab, Ctrl+F, and F11; tab titles can receive keyboard focus
- Desktop entry for the Omarchy app launcher (Super+Space → Apps)

## Requirements (Arch / Omarchy)

```bash
sudo pacman -S --needed gtk3 webkit2gtk-4.1 base-devel
```

Rust 1.70+ (you already have rustc from rustup or pacman).

## Run from source

```bash
cd ~/dev/nodaysidle/nodaysidle-browser-linux
cargo run --release
```

## Install launcher + icon

```bash
./scripts/install-desktop.sh
```

This installs:

- `~/.local/bin/nodaysidle-browser`
- `~/.local/share/applications/nodaysidle-browser.desktop`
- Icons under `~/.local/share/icons/hicolor/`

Log out of the app launcher or run `update-desktop-database` if the app does not appear immediately.

## Notes

- Rust bindings target **GTK 3** + **webkit2gtk-4.1** (same engine family as the Mac app’s WebKit). Native **GTK 4** Rust WebKit bindings are not mature yet; UI can be migrated later.
- Google and other sites may still challenge uncommon browsers; persistent login depends on the on-disk profile not being cleared.

## Related

- macOS app: `~/dev/nodaysidle/nodaysidle-browser` (Swift)
