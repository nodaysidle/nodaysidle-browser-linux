---
title: "Driving the browser for manual checks, and review-fix decisions"
author: Nodaysidle
date: 2026-10-26
branch: master
cost: { minutes: 30, tokens: 380318 }
touches:
  - README.md
  - desktop/com.nodaysidle.Browser.desktop
  - docs/APPIMAGE.md
  - scripts/install-desktop.sh
  - scripts/package-appimage.sh
  - src/downloads.rs
  - src/error_page.rs
  - src/history.rs
  - src/home.rs
  - src/main.rs
---

## Decided
- Fullscreen follows Firefox/Epiphany: page fullscreen hides all chrome and shows "<host> is now full screen · Press Esc to exit" for 4 s in a pass-through gtk::Overlay over the tab stack. A generation counter stops an old hide timer cutting a new notice short (CVE-2025-1018), and opening a new view exits fullscreen (CVE-2020-6810).
- History saves on the timer are written off the GTK thread (gio::spawn_blocking). HistoryStore keeps a generation and an Arc<Mutex<u64>> "generation on disk", so a stale background write can never overwrite a clear or newer data; `flush()` takes the same lock.
- URL display strips userinfo (navigation keeps it). Page titles and download names go through url_display::is_unsafe_display_char.
- Jev/TypeSafe was not added: hosted-only API, every URL/page would leave the machine.

## Tried and ruled out
- Forcing punycode for hosts mixing Han and Latin labels (S11): it also punycodes real hosts like 百度.com. Reverted; the shortcut comment stays.
- wtype cannot deliver Return to the app; load pages by running the binary with a URL (forwards to the running instance).

## Watch out
- Run manual checks with XDG_DATA_HOME=<scratch dir> so the real profile is untouched.
- `pkill -f target/debug/nodaysidle-browser` kills your own shell (the pattern is in the command); use Ctrl+Q or pgrep by name.
- This Hyprland uses Lua config: `hyprctl dispatch closewindow address:..` fails with a Lua syntax error.
- GtkShortcutsWindow is modal and swallows later wtype keys until closed.
