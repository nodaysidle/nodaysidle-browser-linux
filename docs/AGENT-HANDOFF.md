# Agent handoff — nodaysidle-browser-linux

Last updated: 2026-10-02 (user session on Omarchy / Hyprland)

This document captures **current state**, **known bugs**, and **what was already tried** so the next agent can debug without re-discovering context.

## Project

| Item | Path / note |
|------|-------------|
| Linux port | `/home/arch/dev/nodaysidle/nodaysidle-browser-linux` |
| macOS reference | `/home/arch/dev/nodaysidle/nodaysidle-browser` (SwiftUI + WKWebView) |
| Binary | `~/.local/bin/nodaysidle-browser` |
| Desktop entry | `~/.local/share/applications/nodaysidle-browser.desktop` (`Exec=` full path) |
| Profile data | `~/.local/share/nodaysidle-browser/` (`webkit-data/cookies.sqlite`, `webkit-cache`, `history.json`) |

**Stack:** Rust, GTK 3 (`gtk` 0.18), WebKitGTK (`webkit2gtk` 2.0 → system `webkit2gtk-4.1`).

**Not** GTK 4 Rust bindings yet — `webkit2gtk` crate still targets GTK 3 widgets.

## Intended UX (match macOS)

1. **Tab bar** (top): horizontal pills + **+** for new tab.
2. **Toolbar**: icon buttons + URL field + history.
3. **New tab / first launch**: centered **nodaysidle** + **rounded search pill** (no website loaded until user searches). Search uses **DuckDuckGo** via `navigation::resolve` (same rules as Mac).
4. **Persistent logins**: shared `WebContext` + `WebsiteDataManager` under `~/.local/share/nodaysidle-browser/`.

## Architecture (current code)

```
ApplicationWindow
└── VBox
    ├── tab bar (ScrolledWindow → tab_stripHBox)
    ├── separator
    ├── toolbar (home, back, forward, reload, url_entry, history)
    ├── separator
    └── GtkStack (one child per tab, named by tab id)
        └── per tab: GtkStack page_stack
            ├── "home"  → build_home_surface() (nodaysidle + Entry)
            └── "web"   → WebView (lazy: created on first navigation only)
```

Key modules:

- `src/tabs.rs` — tab lifecycle, lazy `ensure_webview`, navigation, close/select.
- `src/home.rs` — home surface UI.
- `src/navigation.rs` — URL/search resolution (DDG default).
- `src/theme.rs` — CSS (must run on `Application::connect_startup`, **not** before `gtk::init`).

## Bugs fixed earlier in this session

| Issue | Cause | Fix |
|-------|--------|-----|
| App won’t open from Super+Space | `theme::install()` before GTK init → panic | `connect_startup` → `theme::install()` |
| Instant crash on first tab | `RefCell` reborrow: `add_tab` held borrow while `Notebook`/`switch-page` fired | Restructured borrows; later replaced Notebook with custom tab strip |
| Close button useless | Close **inside** parent `GtkButton` pill | Pill = `HBox` + `EventBox` (title) + separate close `Button` |
| New tab loaded a website | Eager `WebView` + `load_uri(START_PAGE)` | `TabOpen::Home`, lazy webview |
| GTK init / desktop | — | Full path in `.desktop`; `install-desktop.sh` regenerates it |

## Active bug — search does not leave home UI (P0)

### Symptoms (user report + screenshot ~2026-10-02)

1. User types e.g. `google` in the **home pill** (or toolbar) and submits.
2. **Toolbar URL** updates correctly (e.g. `https://duckduckgo.com/?q=google`).
3. **Tab title** updates (e.g. `google at DuckDuckGo`).
4. **Main content still shows the home surface** (nodaysidle + pill with query text) — **no visible navigation**.

So **WebView likely loads** (title/URL hooks fire) but **home layer remains visible**.

### Reproduction

```bash
nodaysidle-browser
# 1. On home, type "google" in center pill, press Enter.
# 2. Observe URL bar and tab title vs main viewport.
```

### Suspected cause (not fully verified)

- `page_stack.set_visible_child_name("web")` in `navigate_tab` may not be taking effect on GTK 3 `GtkStack` in this layout, **or**
- Home widget remains visible/stacked incorrectly while webview loads behind it, **or**
- `select_tab_id` / `is_selected_on_home` logic resets visible child to `"home"` after navigation, **or**
- Missing explicit `set_visible_child(&webview)`, `show_all()`, or `home_page.hide()`.

Relevant code: `TabManager::navigate_tab`, `ensure_webview`, `select_tab_id`, `show_home_for_selected` in `src/tabs.rs`.

### Suggested next steps for debugger

1. Add temporary logging in `navigate_tab` after `set_visible_child_name("web")`: log `page_stack.visible_child_name()`.
2. Store `home_page: GtkBox` in `TabEntry`; on navigate call explicit `reveal_web()` / `reveal_home()` (hide/show + `set_visible_child`).
3. Confirm `home_search.connect_activate` and `url_entry.connect_activate` both call `navigate_tab` for the **correct** `tab_id`.
4. Rule out `select_tab_id` running after navigate and forcing home (grep call sites).
5. Try `page_stack.set_transition_type(StackTransitionType::None)` to rule out transition glitches.
6. Run under `GTK_DEBUG=interactive` or `RUST_BACKTRACE=1` if crashes reappear.

## Other rough edges (lower priority)

- The tab strip caps title labels and scrolls the selected pill into view (X-4); live visual verification with many tabs remains outstanding.
- Home layout uses `set_valign`/`set_halign` on inner box — verify centering on all resolutions.
- `TabOpen::Url` variant unused; history from toolbar vs pill should behave identically.
- Warnings: dead `tab_scroll` field, unused `TabOpen::Url`.
- macOS parity not done: bookmarks, find-in-page, zoom, gear menu, Secure Sync, tab drag-reorder.

## Build / install

```bash
cd ~/dev/nodaysidle/nodaysidle-browser-linux
cargo build --release
install -m 0755 target/release/nodaysidle-browser ~/.local/bin/nodaysidle-browser
./scripts/install-desktop.sh
```

Dependencies (Arch): `gtk3`, `webkit2gtk-4.1`, `base-devel`.

## Git / repo hygiene

- Linux port is a **sibling** repo under `~/dev/nodaysidle/`, not inside `~/Projects` umbrella repo.
- User is migrating to `~/dev/nodaysidle/<github-repo-name>` layout; legacy tree still at `~/Projects` (do not use as Cursor root).

## User goal reminder

Quiet, native-feeling **nodaysidle** browser on Linux: home page on every **+** tab, DDG search, persistent sessions, Super+Space launcher with icon — visually close to macOS `Theme.swift` / `NewTabView.swift`.

---

## Resolved (2026-10-02)

### P0 Bug Fixed: Search does not leave home UI
- **Root Cause**: In GTK 3, newly constructed widgets (including `WebView::with_context`) default to `is_visible == false`. Because `window.show_all()` was only invoked once at initial startup before any WebView existed, newly created WebViews remained invisible. When `page_stack.set_visible_child_name("web")` was called, GTK 3's `gtk_stack_set_visible_child` checked `if (!gtk_widget_get_visible(child)) return;` and silently dropped the transition, keeping `"home"` displayed while WebKitGTK loaded in the background.
- **Fix applied in `src/tabs.rs`**:
  1. In `ensure_webview`: Added `webview.show_all()` immediately after `tab.page_stack.add_named(&webview, "web")`.
  2. In `navigate_tab`: Added `webview.show()` and `webview.grab_focus()`.
  3. In `open_tab`: Added `page_stack.show_all()` after `stack.add_named(&page_stack, &name)` so dynamically added tabs (via `+`) have visible page stacks.
  4. In `show_home_for_selected`: Reset `title_label` back to `"New Tab"` when navigating back to home.
  5. Silenced dead code warnings for `tab_scroll` and `TabOpen::Url`.
- **Status**: Verified build (`cargo check` and `cargo build --release` with 0 warnings) and re-installed binary via `./scripts/install-desktop.sh`.

### GTK callback borrow safety
- A panic in a GTK callback can cross an FFI boundary and abort the process; the earlier blanket claim that no panic was reachable from a signal handler was incorrect.
- Closing a background tab now copies the selected tab ID out of the `RefCell` borrow before calling `select_tab_id`. Closing a missing tab returns without panicking.
- Continue to keep `RefCell` borrows out of calls that can re-enter `TabManager` or emit GTK signals, and avoid panics in GTK callbacks.

### WebKit-created views (X-3)
- WebKit `create` requests open related views as selected tabs in the existing tab strip.
- `ready-to-show` shows the view, and `close` closes its tab.

### External URI handling (X-5)
- The GApplication `open` handler accepts HTTP, HTTPS, and file URIs and opens each in a tab. Unsupported schemes are ignored.
- When the selected tab is still on its home page, the first external URI reuses that tab.

### Web process sandbox and app reuse (X-6 / X-10)
- The shared WebContext enables WebKit's process sandbox.
- GApplication remains unique by application ID, and repeated activation/open callbacks reuse one TabManager, window, WebContext, and HistoryStore.

### Downloads and site permissions (X-7 / X-13)
- Downloads ask for a destination, default to the XDG Downloads directory when available, and show progress with cancel support.
- Location, camera, microphone, notification, and pointer-lock requests show the requesting origin and require an explicit Allow response. Unknown permission request types are denied.

### Empty page titles (X-16)
- Pages without a nonempty document title use a URL-derived tab and history title.

### Navigation state and fullscreen (X-8 / X-11 / X-12 / X-18)
- URI and title property changes keep the selected tab's URL bar, title, and Back/Forward buttons current, including same-document SPA navigation.
- The Home button navigates the current WebView to `START_PAGE`, preserving browser history; new tabs continue to use the built-in home surface.
- WebKit fullscreen requests hide the tab bar and toolbar and fullscreen the window. Leaving fullscreen restores the chrome.
- URI notifications do not replace text while the URL bar has focus.

### Keyboard access (X-9)
- Ctrl+T opens a tab, Ctrl+W closes the selected tab, Ctrl+L focuses and selects the URL bar, Ctrl+Tab / Ctrl+Shift+Tab cycle tabs, Ctrl+F opens find-in-page, and F11 toggles window fullscreen.
- Tab titles can be focused and activated with Enter or Space. The tab scroller is skipped in the Tab order, and focused controls have a high-contrast highlight.

## Pending

- **Git remote not configured — push deferred.** `master` has one local commit (`ae08c40`); nothing pushed. When the user says to, add `origin` and push, e.g. `git remote add origin <url> && git push -u origin master`.
