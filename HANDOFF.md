# Handoff

Branch native-qt6-wayland. The app is the Qt 6 / QtWebEngine browser described in README.md (the older GTK/WebKit Rust sources in src/*.rs are unused leftovers; only src/main.rs is built by cargo).

## Verified this session (nested Hyprland on a headless output, scratch HOME and XDG_DATA_HOME)

Tabs (+, middle click, hover close, Ctrl+T/W/Tab), omnibox (URL, search, local http), back/forward, bookmarks (star, bar, menu, file path), page context menu (copy link, open in tab), popups (open and window.close), target=_blank, cookies persisting across restart, live Omarchy theme and Hyprland rounding, assistant (Summarize stream, Scam check, Q&A, Stop), fullscreen, downloads.

## Fixed

Profile was off-the-record (now a named persistent profile). Bookmarks wrote to a doubled directory. Toolbar bindings were not reactive. Copy Link was a stub. normalizeUrl forced https on 127.0.0.1:port. SSE stream was never read. `.hovered` on MouseAreas. Esc did not leave fullscreen.

## Google sign-in

Firefox UA interceptor from the old spec was removed: it did not work. Cause was missing window.chrome.app; fixed with a Chrome UA plus src/chrome_app.js. Verified only up to Google's reCAPTCHA step; a real account sign-in has not been done.

## Not done

Permission prompts (camera, mic, location), find in page, history UI, print. QtWebEngine prints a deprecation warning for QQuickWebEngineProfile (use WebEngineProfilePrototype later).
