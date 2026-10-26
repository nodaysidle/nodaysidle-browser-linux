<div align="center">
  <img src="assets/icon.svg" alt="nodaysidle app logo" width="132" height="132">

  # nodaysidle (Linux)

  ![NODAYSIDLE Browser for Linux](docs/browser-linux.gif)

  **A quiet, native Linux browser for focused browsing.**

  Minimal chrome. Native WebKit pages. Local-first privacy.

  <p>
    <a href="https://github.com/nodaysidle/nodaysidle-browser-linux/releases"><img src="https://img.shields.io/badge/version-0.2.1-8e8e96?style=flat-square" alt="Version 0.2.1"></a>
    <a href="https://www.kernel.org/"><img src="https://img.shields.io/badge/Linux-x86__64-151518?style=flat-square&logo=linux&logoColor=white" alt="Linux x86_64"></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021"></a>
    <a href="https://webkitgtk.org/"><img src="https://img.shields.io/badge/WebKitGTK-4.1-1f5b82?style=flat-square&logo=webkit&logoColor=white" alt="WebKitGTK 4.1"></a>
    <img src="https://img.shields.io/badge/telemetry-none-4c8c6b?style=flat-square" alt="No telemetry">
  </p>
</div>

## What is nodaysidle?

`nodaysidle` is a Rust + GTK 3 browser shell that renders pages with WebKitGTK 4.1. The v0.2.1 AppImage (~92 MB) bundles GTK 3, WebKitGTK 4.1 and their libraries. The older slim v0.2.0 AppImage (~1.5 MB) and builds from source use the GTK 3 and WebKitGTK 4.1 installed on the system. It keeps the browser controls close at hand and lets websites own the page experience, without an Electron runtime, a heavy JavaScript chrome layer, start-page clutter, or application telemetry.

Two themes share one quiet look: **Dark** (warm charcoal and silver) and **Paper** (a warm off-white that is never pure white). By default the browser follows your desktop's light or dark setting.

## Highlights

- Native GTK 3 browser chrome backed by WebKitGTK 4.1 (`libsoup3`)
- Dark and Paper themes, switched live from the menu; dialogs and web pages follow the choice
- Built-in search-first Home screen on new tabs with DuckDuckGo resolution
- Multi-tab browsing with auto-scrolling active pills, reopen closed tab, middle-click close, and arrow-key tab navigation
- Address bar with instant search / URL resolution, real-time HTTPS indicator, load progress, and spoof-resistant URL display
- `Ctrl+F` in-page find bar with match counts, find next / previous, and narrow tiling support
- Optional page assistant (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd>): a scam check with TypeSafe Jev and page summaries with Claude Haiku 5.5, using your own API keys and sending nothing until you click
- Pop-up protection: links open as unrelated tabs (no `window.opener`), and scripts get one new window per second
- Downloads with folder chooser, progress dialog, cancel protection, and close confirmation
- Local-first persistence: bounded history (`0600`) saved in the background and on exit/signals, and persistent cookies (`0600` SQLite)
- Keyboard-first: zoom, print, history, shortcuts window (<kbd>Ctrl</kbd>+<kbd>?</kbd>), main menu on <kbd>F10</kbd>
- Desktop integration with Wayland `app_id` and X11 `WM_CLASS` (`com.nodaysidle.Browser`)
- Fully bundled `.AppImage` (v0.2.1, ~92 MB; ships GTK 3 and WebKitGTK 4.1, no system web engine needed). The slim host-dependent `.AppImage` (v0.2.0, ~1.5 MB) requires system GTK 3 and WebKitGTK 4.1; see [docs/APPIMAGE.md](docs/APPIMAGE.md).
- No application telemetry

## What's new (unreleased, on `master`)

These changes are in the source but not yet in a release; the v0.2.1 AppImage does not have them.

- **Paper theme:** a warm off-white light theme next to Dark. **Menu → Theme** picks Match System (default), Dark or Paper. It switches live, is remembered, and dialogs, menus and web pages follow it. Every text colour pair meets WCAG AA contrast.
- **Ask about this page** (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd>): an optional side panel with a **Scam check** (TypeSafe Jev) and **Summarize** / follow-up questions (Claude Haiku 5.5). See [Ask about this page](#ask-about-this-page).
- **Safer pop-ups and links:** a link that opens a new window becomes an ordinary tab with no `window.opener`; page scripts get one new window per second, and the address bar shows when pop-ups were blocked.
- **Clearer permission prompts:** the prompt explains that embedded sites can only ask when the page allows them; pressing Esc counts as Deny; the browser can quit while a prompt is open.
- **Certificate errors** get their own "Connection not secure" page, with no "continue anyway".
- **Spoofing fixes:** the address never shows `user:password@`, and page titles and download names drop invisible and right-to-left override characters.
- **Clear Browsing Data** also removes Cache API storage and old history backups, and "Cached files" is ticked by default. The temporary profile used when no private profile folder is available is deleted on quit.
- **Keyboard:** <kbd>Ctrl</kbd>+<kbd>Q</kbd>, reopen closed tab, zoom, find next / previous, hard reload, print, history, <kbd>F10</kbd> main menu, and a shortcuts window. <kbd>Esc</kbd> leaves the address bar, and Reload turns into Stop while a page loads.
- **Small things:** the window title follows the page, a hint under the Home search field, search in the history popover, shorter error titles.
- **Speed:** history is written off the UI thread; the history list, load progress, find-as-you-type and download progress no longer redo work.

## What's new in v0.2.1

- **Fully bundled AppImage:** `nodaysidle-browser-bundled-x86_64.AppImage` ships GTK 3, WebKitGTK 4.1 and their libraries, so it runs on systems without them. CI test-runs it in a clean `debian:bookworm-slim` image with no GTK or WebKitGTK installed.

## What's new in v0.2.0

- **Multiple windows:** `nodaysidle-browser --new-window` (`-n`), <kbd>Ctrl</kbd>+<kbd>N</kbd>, or **New Window** in the ☰ menu. Windows share one profile (cookies, site storage, history), and only closing the last window asks to confirm active downloads.
- **Hyprland corners:** the window frame insets itself to match Hyprland's `decoration:rounding`, read once at startup. The inset is 0 when the window is tiled, maximized or fullscreen. Set `NODAYSIDLE_CORNER_INSET=<px>` to override it.
- **History popover:** sized to fit narrow tiles, with an empty state. The find bar hides on Home and keeps its query.
- **Accessibility:** tabs are exposed as page tabs with names and selected state, close buttons are labelled "Close <title>", and the close icon has higher contrast (about 6:1).
- **Fish completion and desktop action:** the installer adds a fish completion and a **New Window** desktop action.
- **Hardening:** one shared URL display, with lookalike (mixed-script) hostnames kept in punycode and bidi or zero-width characters escaped. Clear Data now reports failures in a dialog.
- **AppImage packaging:** the pinned `appimagetool` download is checksum-verified before use, with offline regression tests (`scripts/test-package-appimage.py`).

## Install and run

### Download `.AppImage`

Download the [v0.2.1 bundled AppImage](https://github.com/nodaysidle/nodaysidle-browser-linux/releases/download/v0.2.1/nodaysidle-browser-bundled-x86_64.AppImage) ([see all releases](https://github.com/nodaysidle/nodaysidle-browser-linux/releases)). It bundles GTK 3, WebKitGTK 4.1 and their libraries, so they do not need to be installed (see [docs/APPIMAGE.md](docs/APPIMAGE.md)):

```bash
chmod +x nodaysidle-browser-bundled-x86_64.AppImage
./nodaysidle-browser-bundled-x86_64.AppImage
```

SHA-256 (v0.2.1 AppImage): `4fa962df7cc6cd405de863289b17436dfae57a8321491a580939f204db31c7a6` ([.sha256](https://github.com/nodaysidle/nodaysidle-browser-linux/releases/download/v0.2.1/nodaysidle-browser-bundled-x86_64.AppImage.sha256)). Verify with `sha256sum -c nodaysidle-browser-bundled-x86_64.AppImage.sha256` in the download folder.

Size: 91,916,792 bytes (~92 MB).

#### Older slim AppImage (v0.2.0, needs system GTK 3 and WebKitGTK 4.1)

The [v0.2.0 AppImage](https://github.com/nodaysidle/nodaysidle-browser-linux/releases/download/v0.2.0/nodaysidle-browser-x86_64.AppImage) (1,530,360 bytes, ~1.5 MB) contains only the browser binary, desktop file and icon. SHA-256: `02d7a85c3d99f5e395425722c57c46d12fabceccd063554b7f01c2b43229ad93` ([SHA256SUMS](https://github.com/nodaysidle/nodaysidle-browser-linux/releases/download/v0.2.0/SHA256SUMS)). Runtime dependencies that must come from the system:

- WebKitGTK 4.1 (`libwebkit2gtk-4.1`, with `libjavascriptcoregtk-4.1` and `libsoup-3.0`)
- GTK 3 (`libgtk-3`, with GLib/GIO, Pango, Cairo, GDK-Pixbuf)

The bundled AppImage is built by `./scripts/package-appimage-bundled.sh`; the slim one by `./scripts/package-appimage.sh`.

### Build `.AppImage` locally

```bash
./scripts/package-appimage.sh
# Outputs: dist/nodaysidle-browser-x86_64.AppImage
```

### Requirements (Arch / Omarchy)

```bash
sudo pacman -S --needed gtk3 webkit2gtk-4.1 base-devel
```

Requires Rust **1.88** or newer (`rust-version` in `Cargo.toml`).

### Run from source

```bash
git clone https://github.com/nodaysidle/nodaysidle-browser-linux.git
cd nodaysidle-browser-linux
cargo run --release
cargo test --release   # GTK tests run when a display is available
```

### Install launcher + icon

```bash
./scripts/install-desktop.sh
```

This builds a release binary and installs:
- `~/.local/bin/nodaysidle-browser`
- `${XDG_DATA_HOME:-~/.local/share}/applications/com.nodaysidle.Browser.desktop` (quoted only when path characters require it)
- `${XDG_DATA_HOME:-~/.local/share}/icons/hicolor/scalable/apps/nodaysidle-browser.svg` (plus 48/128/256 px PNGs when `rsvg-convert` is available)
- `${XDG_CONFIG_HOME:-~/.config}/fish/completions/nodaysidle-browser.fish`

The launcher name matches the application ID `com.nodaysidle.Browser`, so Hyprland window rules match `class:^(com\.nodaysidle\.Browser)$`.

#### Upgrading from `nodaysidle-browser.desktop`

Versions before the rename installed `nodaysidle-browser.desktop`. Running `./scripts/install-desktop.sh`:
- Migrates `nodaysidle-browser.desktop` entries in `mimeapps.list` files to `com.nodaysidle.Browser.desktop`.
- Updates `xdg-settings set default-web-browser com.nodaysidle.Browser.desktop` if the old name was previously default.
- Removes the old generated launcher.

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| New tab | <kbd>Ctrl</kbd>+<kbd>T</kbd> |
| New window | <kbd>Ctrl</kbd>+<kbd>N</kbd> |
| Close tab | <kbd>Ctrl</kbd>+<kbd>W</kbd>, <kbd>Ctrl</kbd>+<kbd>F4</kbd> |
| Next tab | <kbd>Ctrl</kbd>+<kbd>Tab</kbd>, <kbd>Ctrl</kbd>+<kbd>Page Down</kbd>, <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Page Down</kbd> |
| Previous tab | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Tab</kbd>, <kbd>Ctrl</kbd>+<kbd>Page Up</kbd> |
| Select tab | <kbd>Ctrl</kbd>+<kbd>1</kbd>–<kbd>Ctrl</kbd>+<kbd>8</kbd>, <kbd>Ctrl</kbd>+<kbd>9</kbd> |
| Focus address bar | <kbd>Ctrl</kbd>+<kbd>L</kbd>, <kbd>Alt</kbd>+<kbd>D</kbd>, <kbd>F6</kbd> |
| Reload / stop | <kbd>Ctrl</kbd>+<kbd>R</kbd>, <kbd>F5</kbd> |
| Find in page | <kbd>Ctrl</kbd>+<kbd>F</kbd> |
| Navigate back / forward | <kbd>Alt</kbd>+<kbd>Left</kbd> / <kbd>Alt</kbd>+<kbd>Right</kbd> |
| Full screen | <kbd>F11</kbd> |
| Reopen closed tab | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> |
| Reload, ignoring cache | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd>, <kbd>Ctrl</kbd>+<kbd>F5</kbd> |
| Find next / previous | <kbd>Ctrl</kbd>+<kbd>G</kbd>, <kbd>F3</kbd> / <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>, <kbd>Shift</kbd>+<kbd>F3</kbd> |
| Zoom in / out / reset | <kbd>Ctrl</kbd>+<kbd>+</kbd> / <kbd>Ctrl</kbd>+<kbd>-</kbd> / <kbd>Ctrl</kbd>+<kbd>0</kbd> |
| History | <kbd>Ctrl</kbd>+<kbd>H</kbd> |
| Print | <kbd>Ctrl</kbd>+<kbd>P</kbd> |
| Ask about this page | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> |
| Main menu | <kbd>F10</kbd> |
| All shortcuts | <kbd>Ctrl</kbd>+<kbd>?</kbd> |
| Quit | <kbd>Ctrl</kbd>+<kbd>Q</kbd> |

In the main window, shortcuts work wherever the focus is, including inside web pages. The selected tab is one Tab stop; <kbd>Left</kbd> and <kbd>Right</kbd> move between tabs, and middle-click closes one. <kbd>Esc</kbd> in the address bar drops your edit and returns to the page.

## Themes

**Menu → Theme** picks:

- **Match System** (default): follows the desktop's `color-scheme` setting (`org.gnome.desktop.interface`). Without a light preference it stays Dark.
- **Dark:** warm charcoal and silver.
- **Paper:** a warm off-white (`#f3efe6`), never pure white, for daylight.

The choice applies at once to every window and is saved in the profile's `settings.json`. Dialogs use the matching light or dark GTK theme, and web pages see the matching `prefers-color-scheme`.

## Ask about this page

<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> (or **Menu → Ask About This Page**) opens an optional side panel. It does nothing without your own API keys. Nothing is sent until you click, and only for http(s) pages; local files are never sent:

- **Scam check** asks TypeSafe Jev whether the page impersonates a brand, asks
  for passwords or card details, or pressures you to act, and shows a verdict
  with the three estimates. They are estimates and can be wrong. Needs
  `TYPESAFE_API_KEY`.
- **Summarize** and typed follow-up questions use Claude Haiku 5.5 through the
  Anthropic API, streamed into the panel, with **Stop**. Needs
  `ANTHROPIC_API_KEY` (override the model with `NODAYSIDLE_AI_MODEL`).

The panel sends the selection, or the page's main text (up to 12,000
characters), its title, and its address without the query string. Keys are
read from the environment and never stored. In fish:

```fish
set -Ux ANTHROPIC_API_KEY 'sk-ant-...'
set -Ux TYPESAFE_API_KEY '...'
# for launches from the desktop launcher rather than a terminal:
systemctl --user set-environment ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY TYPESAFE_API_KEY=$TYPESAFE_API_KEY
```

## Pop-up windows

A page script that calls `window.open` with a size (for example an OAuth sign-in) gets a pop-up window; without a size it gets a new tab. A page can open one new window per second; extra requests are blocked and the address bar shows a blocked-pop-up icon. Links that open a new window always become ordinary tabs with no `window.opener`, so the new page cannot redirect the one that opened it.

A pop-up window shows one page and a read-only address bar. It has:
- <kbd>Ctrl</kbd>+<kbd>W</kbd> or <kbd>Ctrl</kbd>+<kbd>F4</kbd> to close it, <kbd>Ctrl</kbd>+<kbd>R</kbd> or <kbd>F5</kbd> to reload, and <kbd>Esc</kbd> to close it unless the page handles the key itself.
- <kbd>Ctrl</kbd>+<kbd>L</kbd>, <kbd>Alt</kbd>+<kbd>D</kbd>, or <kbd>F6</kbd> to select the read-only address for copying with <kbd>Ctrl</kbd>+<kbd>C</kbd>; Tab can also focus it.
- Built-in error pages (in the active theme) for failed loads, certificate errors, and web process recovery.
- Pop-ups lack find, back/forward history, and editable address bar, and close with the parent window.

## Clearing browsing data

The app menu lets you clear history, cookies and site storage (including Cache API
storage), cached files, and session permission denials independently. History,
site storage, cached files and denials are ticked by default; clearing history also
removes old history backups. A failure dialog reports unsuccessful
selected operations, including website-data failures reported asynchronously by
WebKit. Successfully cleared categories remain cleared when another category fails.

## Address bar input

URL presentation is shared by the address bar, popup addresses, tab URL tooltips,
fallback host titles, history rows, and error pages. Approved accented Latin labels (for example,
`bücher.de` and `café.fr`) and Han-only labels (`例子.测试`) display as Unicode.
Other international hosts remain in punycode, including Cyrillic and Greek names
that may resemble Latin names. Paths may display readable Unicode, while encoded
delimiters, bidi controls, zero-width and tag characters remain escaped, and
`user:password@` credentials are never shown (navigation keeps them). Stored history
URLs and navigation destinations stay canonical; submitting an unchanged displayed
address reuses the current URI.

- `http://`, `https://`, `file://` and `about:` URLs load as typed.
- `/absolute/path` and `~/path` open local files.
- `localhost`, `*.localhost`, a single-label `host:port`, and local addresses use `http://`: loopback, private IPv4 (`10/8`, `172.16/12`, `192.168/16`), link-local IPv4 (`169.254/16`), IPv6 loopback, unique local (`fc00::/7`) and link-local (`fe80::/10`) addresses. IPv6 literals such as `::1` are bracketed.
- Other input that looks like a domain (`example.com`, `en.wikipedia.org/wiki/Rust`) gets `https://`.
- Everything else is searched with DuckDuckGo, including text with spaces, file names such as `node.js` or `notes.txt`, numbers such as `3.14`, and `javascript:` / `data:` URLs.

## Command line

```bash
nodaysidle-browser [URL-or-file …]
nodaysidle-browser --new-window [URL-or-file …]   # -n
```

An argument naming an existing file (or starting with `./` or `../`) opens that file; anything else is resolved like address-bar input (e.g. `nodaysidle-browser wikipedia.org` opens `https://wikipedia.org`). Running the command again reuses the existing window unless `--new-window` (`-n`) is given, which opens a new window on the current workspace sharing the same profile (cookies, site storage and history). <kbd>Ctrl</kbd>+<kbd>N</kbd> and the ☰ menu's **New Window** do the same in-app.

## Privacy model

- No analytics, telemetry, or application-owned browsing backend.
- The browser itself contacts no server except the pages you open, and, only when you click a button in the optional assistant panel, `api.typesafe.ai` (Scam check) and `api.anthropic.com` (Summarize, questions). API keys are read from the environment and never written to disk.
- Website cookies persist locally in `~/.local/share/nodaysidle-browser/webkit-data/cookies.sqlite` (`0600`).
- Local browsing history is stored in `~/.local/share/nodaysidle-browser/history.json` (`0600`), saved in the background and flushed atomically on exit or signals (`SIGTERM`, `SIGINT`, `SIGHUP`). The theme choice is in `settings.json` next to it. Use **Menu → Clear Browsing Data** to erase history, site storage, cache, or remembered permission denials.
- Web content runs in WebKitGTK's own sandboxed web process; the app does not disable it.
- Site requests for location, camera, microphone, notifications, and pointer lock require explicit user permission. A Deny (or closing the prompt) is remembered for that site until you clear it or quit.
- Invalid certificates stop the load with a warning page; there is no way to continue anyway.

## Related

- macOS app: [nodaysidle-browser](https://github.com/nodaysidle/nodaysidle-browser) (SwiftUI + WKWebView)
