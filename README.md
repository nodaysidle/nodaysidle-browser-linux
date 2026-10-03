# nodaysidle-browser (Linux)

GTK 3 + WebKitGTK port of [nodaysidle-browser](https://github.com/nodaysidle/nodaysidle-browser) for Omarchy / Linux.

## Features (v0.1)

- Multi-tab browsing (WebKitGTK 4.1) with a custom tab strip; tab tooltips show the full title and URL
- Address bar with search / URL rules aligned with the macOS app (see [Address bar input](#address-bar-input)),
  a lock / "Not secure" indicator, a load-progress bar, and Reload that turns into Stop while loading
- Built-in Home page on every new tab; the Home button returns to it (Back takes you to the page you left)
- **Persistent profile** under `~/.local/share/nodaysidle-browser/` (directories `0700`):
  cookies in `webkit-data/cookies.sqlite` (`0600`), other site data in `webkit-data/`, cache in `webkit-cache/`
- **Local history** in `~/.local/share/nodaysidle-browser/history.json` (`0600`), written atomically in
  batches (about two seconds after the first unsaved visit, retried with backoff if a write fails) and on exit;
  an unreadable file is kept as `history.json.corrupt-<time>`
- Opens URLs and local files passed on the command line or by other applications (see [Command line](#command-line))
- `window.open` pop-ups that ask for a size (e.g. sign-in windows) open in their own small window with a read-only
  address bar; links with `target=_blank` and plain `window.open` open a new tab
- Find in page (Ctrl+F) as a bar under the toolbar
- Downloads ask where to save and show progress; Cancel stops the download, and closing the progress window
  while a download runs asks before cancelling it
- Site requests for location, camera, microphone, notifications and pointer lock need an explicit Allow; the
  answer is remembered per site until the browser exits
- Failed loads and crashed pages show a built-in error page with a Try again / Reload button
- Video and other element fullscreen; F11 window fullscreen
- Menu (☰) with New Tab, Find in Page, Full Screen and About
- Desktop entry and icon for the Omarchy app launcher (Super+Space → Apps)

### Keyboard shortcuts

| Keys | Action |
|------|--------|
| Ctrl+T | New tab |
| Ctrl+W, Ctrl+F4 | Close tab |
| Ctrl+Tab, Ctrl+Page Down | Next tab |
| Ctrl+Shift+Tab, Ctrl+Page Up | Previous tab |
| Ctrl+1 … Ctrl+8, Ctrl+9 | Go to tab 1–8, last tab |
| Ctrl+L, Alt+D, F6 | Focus the address bar |
| Ctrl+F | Find in page (Enter / Shift+Enter: next / previous, Esc: close) |
| Ctrl+R, F5 | Reload |
| Alt+Left, Alt+Right | Back, Forward |
| F11 | Full screen (also leaves video fullscreen) |

Shortcuts work wherever the focus is, including inside web pages. Tab titles can be reached with Tab and
activated with Enter or Space.

### Tabs

Closing the last tab replaces it with a fresh Home tab, so the window always has one tab; a lone Home tab that
has not loaded anything has no close button. Close the window to quit.

### Address bar input

- `http://`, `https://`, `file://` and `about:` URLs load as typed.
- `/absolute/path` and `~/path` open local files.
- `localhost`, `*.localhost`, a single-label `host:port`, and local addresses use `http://`: loopback,
  private IPv4 (10/8, 172.16/12, 192.168/16), link-local IPv4 (169.254/16), IPv6 loopback, unique local
  (fc00::/7) and link-local (fe80::/10) addresses. IPv6 literals such as `::1` are bracketed.
- Other input that looks like a domain (`example.com`, `en.wikipedia.org/wiki/Rust`) gets `https://`.
- Everything else is searched with DuckDuckGo, including text with spaces, file names such as `node.js` or
  `notes.txt`, numbers such as `3.14`, and `javascript:` / `data:` URLs.

### Command line

```bash
nodaysidle-browser [URL-or-file …]
```

An argument naming an existing file (or starting with `./` or `../`) opens that file; anything else is
resolved like address-bar input, so `nodaysidle-browser wikipedia.org` opens `https://wikipedia.org`.
Running the command again while the browser is open raises the existing window and opens the arguments there.

## Requirements (Arch / Omarchy)

```bash
sudo pacman -S --needed gtk3 webkit2gtk-4.1 base-devel
```

Rust **1.88** or newer (the locked dependencies need it; `rust-version` in `Cargo.toml`).

## Run from source

```bash
cd ~/dev/nodaysidle/nodaysidle-browser-linux
cargo run --release
cargo test --release   # GTK tests run only when a display is available
```

## Install launcher + icon

```bash
./scripts/install-desktop.sh
```

This builds a release binary and installs:

- `~/.local/bin/nodaysidle-browser`
- `~/.local/share/applications/com.nodaysidle.Browser.desktop`: the tracked
  `desktop/com.nodaysidle.Browser.desktop` with only `Exec=` pointing at the installed binary (quoted)
- `~/.local/share/icons/hicolor/scalable/apps/nodaysidle-browser.svg`, plus 48/128/256 px PNGs when
  `rsvg-convert` is installed

The desktop file is named after the application ID `com.nodaysidle.Browser`, which the browser also uses as
its Wayland app_id and X11 window class (`StartupWMClass`). Hyprland window rules therefore match
`class:^(com\.nodaysidle\.Browser)$` (older builds used `nodaysidle-browser`). The script removes the
`nodaysidle-browser.desktop` that older versions installed, if it is the generated one. Run
`update-desktop-database ~/.local/share/applications` if the app does not appear immediately.

## Notes

- The UI uses **GTK 3** with **webkit2gtk-4.1** through the `webkit2gtk` 2.0 crate (same engine family as the
  Mac app's WebKit). GTK 4 bindings for WebKitGTK 6.0 exist (`webkit6` crate), but moving to them means
  rewriting the UI for GTK 4; not planned for now. The `webkit2gtk` crate pins gtk-rs 0.18, so the gtk/glib
  crates cannot be upgraded past 0.18 without that migration (glib advisory RUSTSEC-2024-0429 does not affect
  the code paths used here).
- Google and other sites may still challenge uncommon browsers; persistent login depends on the on-disk
  profile not being cleared.
- There is no CI: the repository has no remote yet.

## Related

- macOS app: `~/dev/nodaysidle/nodaysidle-browser` (Swift)
