# nodaysidle for Linux

A quiet, native Linux browser for Omarchy: Qt 6 Quick on QtWebEngine (Chromium), pure Wayland, minimal chrome, no telemetry.

## Build and run

Requires Qt 6.12 (`qt6-base`, `qt6-declarative`, `qt6-webengine`) and Rust for the optional cargo launcher.

```bash
./bin/build           # qmake6 + make -> build/nodaysidle-browser
./bin/run [URL]       # launch
cargo run --release   # same binary, started through the Rust wrapper
./scripts/install-desktop.sh   # install for your user: Omarchy app menu entry, icon, ~/.local/bin
```

Launching again while it runs hands the URLs to the open window, so links from other apps open as tabs and the profile is never opened twice. Set it as the default browser with `xdg-settings set default-web-browser com.nodaysidle.Browser.desktop`.

The app forces `QT_QPA_PLATFORM=wayland` and `QT_WAYLAND_DISABLE_WINDOWDECORATION=1`. GPU flags are `--ignore-gpu-blocklist --enable-gpu-rasterization --enable-zero-copy --no-sandbox`. Do not add `--use-gl=egl`: it forces GLES2 and crashes the in-process GPU thread (SIGTRAP) on Wayland.

## Features

- Tabs: pills with an accent line under the active one, close button, middle-click close, `Ctrl+T`, `Ctrl+W`, `Ctrl+Tab` / `Ctrl+Shift+Tab`
- Omnibox: URL or DuckDuckGo search, local hosts over http, `Ctrl+L` / `Alt+D`, https indicator, star bookmark (`Ctrl+D`)
- Bookmarks bar (`Ctrl+B`) stored in `~/.local/share/nodaysidle-browser/bookmarks.json`; right-click for Open in New Tab / Delete
- Page context menu: open link in new tab, copy link address, back, forward, reload
- Back / forward (`Alt+Left`, `Alt+Right`), reload / stop (`Ctrl+R`, `F5`), reload ignoring cache (`Ctrl+Shift+R`), Home
- Find in page (`Ctrl+F`, `Enter` / `Shift+Enter` or `Ctrl+G` to step, match count), zoom (`Ctrl++`, `Ctrl+-`, `Ctrl+0`), reopen closed tab (`Ctrl+Shift+T`), full screen (`F11`)
- Camera, microphone, location, notifications and screen requests ask first, through a bar under the toolbar; the answer is remembered per site
- Right-click menu also offers Cut, Copy and Paste
- Popups from `window.open` (OAuth) open in their own window and close themselves; `target=_blank` opens a tab
- Page fullscreen hides the chrome, `Esc` leaves it
- Downloads go to the XDG Downloads folder without overwriting, with a short confirmation
- Theme follows `~/.local/state/omarchy/current/theme/colors.toml` live, including the start page; corner radius follows Hyprland's `decoration:rounding`
- Cookies, cache and site data persist in `~/.local/share/nodaysidle-browser/` (`webengine/`, `cache/`), so sign-ins survive a restart
- Page assistant (`Ctrl+Shift+A`): Summarize and free-form questions through Claude Haiku 5.5, Scam check through TypeSafe Jev. Data is sent only when you click or press Enter, and a Stop button cancels a stream

## Google sign-in

Google rejects QtWebEngine with "This browser or app may not be secure" (`rrk=46`). Two things are needed, both in the app:

- a plain Chrome user agent that matches the engine version (no `QtWebEngine` token), set in `src/main.cpp`
- `window.chrome.app`, which QtWebEngine never defines, injected at document creation by `src/chrome_app.js`

Sign in once; the session is kept in the persistent profile.

## API keys

Read from the environment, then `~/.config/fish/secrets.fish`, then `~/.config/fish/fish_variables`: `ANTHROPIC_API_KEY` or `HAIKU_API_KEY`, and `TYPESAFE_API_KEY`. Keys are never written to disk by the app.

## Layout

```text
nodaysidle-browser.pro   qmake6 project
bin/build, bin/run       build and launch scripts
src/main.cpp             Wayland setup, persistent profile, downloads, single instance, QML engine
src/backend.{h,cpp}      bookmarks, URL resolution, clipboard, Anthropic and Jev calls
src/theme.{h,cpp}        Omarchy colors.toml watcher and Hyprland rounding
src/Main.qml             window, tabs, toolbar, find and permission bars, assistant panel
src/ThemedMenu*.qml      themed context menus
src/chrome_app.js        window.chrome.app shim
src/home.html            start page
src/main.rs              cargo wrapper that starts the Qt binary
```

## Not included

History, print, extensions, sync, and reading saved passwords from a password manager. Geolocation prompts appear but position lookups fail, because QtWebEngine ships without a Google location API key.

MIT licensed.
