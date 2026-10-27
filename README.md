<p align="center">
  <img src="assets/icon.svg" width="148" height="148" alt="nodaysidle logo">
</p>

<h1 align="center">nodaysidle for Linux</h1>

<p align="center">
  <img src="docs/browser-linux.gif" alt="nodaysidle browser for Linux">
</p>

<p align="center">
  <strong>A quiet, native Linux browser for focused browsing.</strong><br>
  Minimal chrome. Native WebKit pages. Local-first privacy.
</p>

<p align="center">
  <a href="https://github.com/nodaysidle/nodaysidle-browser-linux/releases"><img src="https://img.shields.io/badge/version-0.2.2-8e8e96?style=flat-square" alt="Version 0.2.2"></a>
  <a href="https://www.kernel.org/"><img src="https://img.shields.io/badge/Linux-x86__64-151518?style=flat-square&logo=linux&logoColor=white" alt="Linux x86_64"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021"></a>
  <a href="https://webkitgtk.org/"><img src="https://img.shields.io/badge/WebKitGTK-4.1-1f5b82?style=flat-square&logo=webkit&logoColor=white" alt="WebKitGTK 4.1"></a>
  <img src="https://img.shields.io/badge/telemetry-none-4c8c6b?style=flat-square" alt="No telemetry">
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License MIT">
</p>

<p align="center">
  <a href="https://github.com/nodaysidle/nodaysidle-browser-linux/releases/latest/download/nodaysidle-browser-bundled-x86_64.AppImage"><strong>Download Linux AppImage (v0.2.2, ~92 MB)</strong></a>
  ·
  <a href="https://github.com/nodaysidle/nodaysidle-browser-linux/releases">All Releases</a>
  ·
  <a href="https://github.com/nodaysidle/nodaysidle-browser">macOS Version</a>
</p>

<p align="center">
  <a href="#what-is-nodaysidle">What is nodaysidle</a> ·
  <a href="#highlights">Highlights</a> ·
  <a href="#install-and-run">Install</a> ·
  <a href="#keyboard-shortcuts">Shortcuts</a> ·
  <a href="#themes">Themes</a> ·
  <a href="#ask-about-this-page">Page Assistant</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#privacy-model">Privacy</a>
</p>

---

## What is nodaysidle?

`nodaysidle` is a native Linux browser shell written in Rust and GTK 3, rendering web pages through WebKitGTK 4.1 (`libsoup3`). It keeps browser controls close at hand and gives the website ownership of the viewport, without an Electron runtime, heavy JavaScript UI layers, start-page clutter, or background telemetry.

The visual design offers two quiet palettes: Dark (warm charcoal and silver) and Paper (a warm off-white for daytime reading). By default, the browser respects your desktop environment appearance.

## Highlights

- Native GTK 3 browser chrome backed by WebKitGTK 4.1 (`libsoup3`)
- Dark and Paper themes switched live from the menu; dialogs and web pages match the choice
- Search-first Home screen on new tabs with DuckDuckGo query resolution
- Multi-tab browsing with auto-scrolling pills, middle-click close, reopen closed tab, and keyboard navigation
- Multi-window browsing with shared cookies, site storage, and history (<kbd>Ctrl</kbd>+<kbd>N</kbd> or `--new-window`)
- Address bar with instant URL resolution, real-time HTTPS indicator, load progress, and spoof-resistant Unicode display
- In-page find bar (<kbd>Ctrl</kbd>+<kbd>F</kbd>) with match counts, match cycling, and narrow window tiling support
- Optional page assistant (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd>): scam check via TypeSafe Jev and page summaries via Claude Haiku 5.5, sending data only on click
- Pop-up isolation: links opening new windows become normal tabs with no `window.opener`, and page scripts get rate-limited pop-up windows
- Downloads with folder chooser, progress reporting, cancel protection, and close confirmation
- Local-first persistence: bounded history (`0600`) flushed on exit or signals, and persistent cookies (`0600` SQLite)
- Desktop integration with Wayland `app_id` and X11 `WM_CLASS` (`com.nodaysidle.Browser`), plus Hyprland corner radius matching
- Zero application telemetry

## Install and run

### Download AppImage

The standalone bundled AppImage bundles GTK 3, WebKitGTK 4.1, and their dependencies, running without system library setup:

```bash
# Make executable and run
chmod +x nodaysidle-browser-bundled-x86_64.AppImage
./nodaysidle-browser-bundled-x86_64.AppImage
```

- **Latest bundle:** [nodaysidle-browser-bundled-x86_64.AppImage](https://github.com/nodaysidle/nodaysidle-browser-linux/releases/latest/download/nodaysidle-browser-bundled-x86_64.AppImage) (~92 MB)
- **Checksums:** [.sha256](https://github.com/nodaysidle/nodaysidle-browser-linux/releases/download/v0.2.2/nodaysidle-browser-bundled-x86_64.AppImage.sha256) and [SHA256SUMS](https://github.com/nodaysidle/nodaysidle-browser-linux/releases/download/v0.2.2/SHA256SUMS)
- **Verification:** `sha256sum -c nodaysidle-browser-bundled-x86_64.AppImage.sha256`

An older host-dependent slim AppImage (v0.2.0, ~1.5 MB) is also available. It relies on system-installed GTK 3 and WebKitGTK 4.1 packages. See [docs/APPIMAGE.md](docs/APPIMAGE.md) for packaging contracts and dependencies.

### Build and install from source

Requirements (Arch / Omarchy), plus Rust 1.88 or newer (`rust-version` in `Cargo.toml`):

```bash
sudo pacman -S --needed gtk3 webkit2gtk-4.1 base-devel
```

Build and run:

```bash
git clone https://github.com/nodaysidle/nodaysidle-browser-linux.git
cd nodaysidle-browser-linux
cargo run --release
cargo test --release   # GTK tests run when a display is available
```

Install desktop launcher and icon:

```bash
./scripts/install-desktop.sh
```

This compiles a release binary and installs:
- `~/.local/bin/nodaysidle-browser`
- `${XDG_DATA_HOME:-~/.local/share}/applications/com.nodaysidle.Browser.desktop`
- `${XDG_DATA_HOME:-~/.local/share}/icons/hicolor/scalable/apps/nodaysidle-browser.svg` (plus PNG sizes when `rsvg-convert` is available)
- `${XDG_CONFIG_HOME:-~/.config}/fish/completions/nodaysidle-browser.fish`

Hyprland window rules match `class:^(com\.nodaysidle\.Browser)$`. If an older version installed `nodaysidle-browser.desktop`, the script migrates `mimeapps.list` entries and the default-browser setting to `com.nodaysidle.Browser.desktop` and removes the old launcher.

Package an AppImage locally:

```bash
./scripts/package-appimage.sh           # slim: dist/nodaysidle-browser-x86_64.AppImage
./scripts/package-appimage-bundled.sh   # bundled AppImage (Debian/Ubuntu or GitHub Actions CI)
```

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| New tab | <kbd>Ctrl</kbd>+<kbd>T</kbd> |
| Reopen closed tab | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> |
| New window | <kbd>Ctrl</kbd>+<kbd>N</kbd> |
| Close tab | <kbd>Ctrl</kbd>+<kbd>W</kbd>, <kbd>Ctrl</kbd>+<kbd>F4</kbd> |
| Next tab | <kbd>Ctrl</kbd>+<kbd>Tab</kbd>, <kbd>Ctrl</kbd>+<kbd>Page Down</kbd>, <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Page Down</kbd> |
| Previous tab | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Tab</kbd>, <kbd>Ctrl</kbd>+<kbd>Page Up</kbd> |
| Select tab | <kbd>Ctrl</kbd>+<kbd>1</kbd> to <kbd>Ctrl</kbd>+<kbd>8</kbd>, <kbd>Ctrl</kbd>+<kbd>9</kbd> |
| Focus address bar | <kbd>Ctrl</kbd>+<kbd>L</kbd>, <kbd>Alt</kbd>+<kbd>D</kbd>, <kbd>F6</kbd> |
| Reload / stop | <kbd>Ctrl</kbd>+<kbd>R</kbd>, <kbd>F5</kbd> |
| Reload, ignoring cache | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd>, <kbd>Ctrl</kbd>+<kbd>F5</kbd> |
| Find in page | <kbd>Ctrl</kbd>+<kbd>F</kbd> |
| Find next / previous | <kbd>Ctrl</kbd>+<kbd>G</kbd>, <kbd>F3</kbd> / <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>, <kbd>Shift</kbd>+<kbd>F3</kbd> |
| Zoom in / out / reset | <kbd>Ctrl</kbd>+<kbd>+</kbd> / <kbd>Ctrl</kbd>+<kbd>-</kbd> / <kbd>Ctrl</kbd>+<kbd>0</kbd> |
| Back / forward | <kbd>Alt</kbd>+<kbd>Left</kbd> / <kbd>Alt</kbd>+<kbd>Right</kbd> |
| History | <kbd>Ctrl</kbd>+<kbd>H</kbd> |
| Print | <kbd>Ctrl</kbd>+<kbd>P</kbd> |
| Ask about this page | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> |
| Full screen | <kbd>F11</kbd> |
| Main menu | <kbd>F10</kbd> |
| All shortcuts | <kbd>Ctrl</kbd>+<kbd>?</kbd> |
| Quit | <kbd>Ctrl</kbd>+<kbd>Q</kbd> |

In the main window, shortcuts work wherever the focus is, including inside web pages. <kbd>Left</kbd> and <kbd>Right</kbd> move between tabs, and middle-click closes one. <kbd>Esc</kbd> in the address bar drops your edit and returns focus to the page. Reload turns into Stop while a page loads.

## Themes

**Menu → Theme** selects:

- **Match System** (default): follows the desktop `color-scheme` setting (`org.gnome.desktop.interface`). Without a light preference, it defaults to Dark.
- **Dark:** warm charcoal and silver.
- **Paper:** a warm off-white (`#f3efe6`), never pure white.

The selection applies immediately across every open window and saves to the profile `settings.json`. Dialogs, menus, and error pages follow the choice, and web pages receive the matching `prefers-color-scheme`. Every text color pair meets WCAG AA contrast.

## Ask about this page

<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> (or **Menu → Ask About This Page**) opens an optional side panel. It remains idle without API keys. Data is sent only when you click, and only for http(s) pages; local files are never sent.

- **Scam check:** queries TypeSafe Jev to detect brand impersonation, credential solicitation, or coercion tactics, displaying a verdict with probability scores. Requires `TYPESAFE_API_KEY`.
- **Summarize:** streams page summaries and answers follow-up questions via Claude Haiku 5.5 through the Anthropic API. Includes a **Stop** button. Requires `ANTHROPIC_API_KEY` (override the model with `NODAYSIDLE_AI_MODEL`).

The panel transmits the selection (or the page body text up to 12,000 characters), page title, and address stripped of query parameters. Keys are read from the environment and never written to disk. In fish shell:

```fish
set -Ux ANTHROPIC_API_KEY 'sk-ant-...'
set -Ux TYPESAFE_API_KEY '...'
# For desktop launcher sessions:
systemctl --user set-environment ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY TYPESAFE_API_KEY=$TYPESAFE_API_KEY
```

## Security and browsing controls

### Pop-up windows and links

Page scripts invoking `window.open` with dimensions (such as OAuth login prompts) open a transient pop-up window; invocations without dimensions open an ordinary tab. A pop-up window contains one page and a read-only address bar:

- <kbd>Ctrl</kbd>+<kbd>W</kbd> or <kbd>Ctrl</kbd>+<kbd>F4</kbd> closes the window, <kbd>Ctrl</kbd>+<kbd>R</kbd> or <kbd>F5</kbd> reloads, and <kbd>Esc</kbd> closes it unless intercepted by the page.
- <kbd>Ctrl</kbd>+<kbd>L</kbd>, <kbd>Alt</kbd>+<kbd>D</kbd>, or <kbd>F6</kbd> focuses the address for copying with <kbd>Ctrl</kbd>+<kbd>C</kbd>.
- Built-in error pages for failed loads and web process recovery match the active theme.
- Pop-ups omit find bars, back/forward history, and address editing. They close automatically with the parent window.

Links targeted to open new windows always become tabs without `window.opener`, preventing the target page from navigating the origin tab. Pages are rate-limited to one new window per second; blocked requests display an indicator in the address bar.

### Address bar input

- `http://`, `https://`, `file://`, and `about:` URLs load directly.
- `/absolute/path` and `~/path` open local files.
- `localhost`, `*.localhost`, single-label `host:port`, and local IP addresses resolve over `http://`: loopback, private IPv4 (`10/8`, `172.16/12`, `192.168/16`), link-local IPv4 (`169.254/16`), IPv6 loopback, unique local (`fc00::/7`), and link-local (`fe80::/10`) addresses. IPv6 literals such as `::1` are bracketed automatically.
- Input resembling domain names (`example.com`, `en.wikipedia.org/wiki/Rust`) defaults to `https://`.
- Other inputs resolve to DuckDuckGo searches, including spaced text, filenames like `node.js` or `notes.txt`, numbers, and `javascript:` or `data:` URLs.

URL formatting is shared across the address bar, pop-up headers, tab tooltips, fallback host titles, history entries, and error surfaces. Approved accented Latin labels (`bücher.de`, `café.fr`) and Han labels (`例子.测试`) display as Unicode. Lookalike international hosts remain in punycode, including Cyrillic and Greek names resembling Latin script. Paths display readable Unicode while encoded delimiters, bidirectional controls, and zero-width characters stay escaped. Embedded credentials (`user:password@`) are stripped from display while preserved for navigation.

### Clearing browsing data

**Menu → Clear Browsing Data** clears history, cookies, site storage (including Cache API data), cached files, and session permission denials independently. A failure dialog reports any unsuccessful actions, including asynchronous WebKit errors. Successfully cleared categories stay cleared even if another category encounters an issue.

## Command line

```bash
nodaysidle-browser [URL-or-file ...]
nodaysidle-browser --new-window [URL-or-file ...]   # -n
```

Arguments naming existing files (or prefixed with `./` or `../`) open directly. Other arguments resolve through the address bar logic (for example, `nodaysidle-browser wikipedia.org` loads `https://wikipedia.org`). Running the command while the browser is open activates the existing window unless `--new-window` (`-n`) is specified, which opens a new window sharing the same profile.

## Architecture

The project is structured as a modular Rust binary crate targeting GTK 3 and WebKitGTK 4.1:

```text
src/
├── main.rs          # Application entry, CLI dispatch, single-instance GIO bus, Hyprland frame insets
├── tabs.rs          # Tab strip, webview container, active pill styling, tab lifecycle
├── navigation.rs    # Address bar input resolution, protocol handlers, and navigation policy
├── url_display.rs   # Canonical URL formatting, Unicode IDN handling, and punycode safety
├── theme.rs         # Dark and Paper CSS palettes, runtime switching, system color-scheme listener
├── assistant.rs     # Page assistant panel (TypeSafe Jev scam detection and Claude summaries)
├── history.rs       # Local bounded history persistence (0600 JSON), background writes, atomic flushing
├── history_ui.rs    # History popover with search filter and item activation
├── downloads.rs     # Download manager, folder picker, progress tracking, cancel protection
├── permissions.rs   # WebKit permission requests (camera, mic, location) and session denial storage
├── popup.rs         # Transient pop-up window lifecycle (OAuth login) and window open rate limits
├── privacy.rs       # Clear Browsing Data dialog and WebKit data store removal
├── profile.rs       # Profile directory management (~/.local/share/nodaysidle-browser), cookies SQLite
├── find.rs          # In-page search bar (Ctrl+F), match counter, match cycling
├── error_page.rs    # Custom styled error and crash surfaces matching the active theme
├── home.rs          # Search-first start page with DuckDuckGo resolution
├── shortcuts.rs     # Keyboard shortcuts cheat sheet window (Ctrl+?)
└── icon.rs          # Scalable application icon assets
```

## Privacy model

- No telemetry, analytics, or remote tracking infrastructure. The browser contacts no external servers outside of requested web pages, except for explicit user clicks in the assistant panel (`api.typesafe.ai` and `api.anthropic.com`).
- Website cookies persist locally in `~/.local/share/nodaysidle-browser/webkit-data/cookies.sqlite` with strict file permissions (`0600`).
- Browsing history is stored in `~/.local/share/nodaysidle-browser/history.json` (`0600`), written in the background and flushed atomically on exit or signals (`SIGTERM`, `SIGINT`, `SIGHUP`).
- Web rendering executes inside WebKitGTK sandboxed web processes.
- Device permissions for location, camera, microphone, notifications, and pointer lock require explicit confirmation. Denials persist per-site during the session until cleared or on browser exit.
- Invalid certificates halt page loads with an untrusted connection warning surface, omitting any bypass option.

## Graphics rendering note

To avoid Mesa/GBM driver crashes on Intel graphics with Wayland, the browser sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` at launch if not already defined in the environment. To test or restore DMA-BUF rendering manually:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=0 nodaysidle-browser
```

## Related

- macOS browser: [nodaysidle-browser](https://github.com/nodaysidle/nodaysidle-browser) (SwiftUI + WKWebView)

## License

MIT (declared in `Cargo.toml`).
