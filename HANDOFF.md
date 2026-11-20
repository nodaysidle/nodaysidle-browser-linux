# HANDOFF: nodaysidle-browser-linux

Date: 2026-11-17
Author: Audit-Hawk
Scope: read-only code + UI/UX audit of `master` @ `2d14e25` (in sync with `origin/master`). Audience: coding agents. Analyzer output: `/tmp/audit-nodaysidle-browser-linux-2026-11-17/` (ephemeral).

## Implementation pass (2026-11-17, later the same day)
The audit below was acted on: see **"Work done — 2026-11-17"** at the end of this file for
what landed, the commit for each finding, the proofs, and what is still open. Branch
`audit-fixes-2026-11-17`, 21 commits on top of `2d14e25`; merged into `master` by fast-forward on 2026-11-21, not pushed. The findings table
below is the original read-only audit and still describes the pre-fix state.

## Summary
- Stack: C++17 + Qt 6 Quick on QtWebEngine 6.11.2 (Chromium), qmake6, pure Wayland, Omarchy theme + Hyprland rounding; ~4.0k LOC, no tests.
- Health: builds clean out-of-tree (0 warnings at -Wall -Wextra); the H1/M1/M2/M4/M5 fixes from the last audit hold; security, a11y and resilience gaps remain. 0 P0, 5 P1, 18 P2, 14 P3.
- Top risks: F01 saved passwords auto-filled into the page with no user gesture, http:// included; F02 dead Anthropic key or used-up credits show only a generic Qt "server replied: Bad Request", because the API's own message is dropped; F03 no control has an accessible name or role, and the toolbar can't be reached with the keyboard.

## Stack Detected
- Language/UI: C++17 (`nodaysidle-browser.pro:1-23`), QML (Qt Quick Controls Basic style, `src/main.cpp:44`), QtWebEngineQuick. JS: `src/login.js` (ApplicationWorld login agent), `src/chrome_app.js` (MainWorld shim). Start page: `src/home.html`.
- Qt on the dev machine: qt6-base 6.12.0, qt6-declarative 6.12.0, qt6-wayland 6.12.0, **qt6-webengine 6.11.2** (version skew, see Out of Scope).
- Entry point: `src/main.cpp:23` → `qrc:/Main.qml` (`src/main.cpp:126-133`). Context properties instead of QML modules: `theme`, `backend`, `appProfile`, `vault`, `importer` (`src/main.cpp:120-124`).
- Build: qmake6 (`nodaysidle-browser.pro`), `bin/build` builds **in-tree** into `build/` (`bin/build:5-6`). Links `-lcrypto` (OpenSSL), Qt Sql (importer), Qt Network (assistant).
- LOC: C++ 1,754 (cpp 1,444 + h 310; `Main.qml` aside, the biggest file is `backend.cpp` at 500), QML 2,007 (`Main.qml` 1,608 in one file), JS 161, HTML 83.
- Tests: none. `dev/harness.py` is an LLM build/evaluate loop, not a test suite. `scripts/validate.sh` = in-tree build + qmllint.
- Packaging: `pkgbuild/PKGBUILD` builds from `$startdir/..` (local only, no `source=`/checksums), `desktop/com.nodaysidle.Browser.desktop` (WebBrowser category, http/https/html MIME, validates clean), fish completions, `scripts/install-desktop.sh` (per-user install).
- Profile: `~/.local/share/nodaysidle-browser/` (dir 0700; `history.json` 0600, `passwords.vault` 0600, `bookmarks.json` 0644). Leftover `webkit-cache/`, `webkit-data/` dirs from the pre-v0.3.0 WebKitGTK app are still there.
- Runtime: Hyprland 0.56.2 with Lua config (`~/.config/hypr/*.lua`), Omarchy theme at `~/.local/state/omarchy/current/theme/colors.toml` (`~/.config/omarchy/current` symlinks to it). Bundled themes: `/usr/share/omarchy/themes/*` (23). Current theme: `prisma-cube`. `QT_QPA_PLATFORMTHEME=gtk3`.

## Analyzers Run / Missing
| Tool | Ran | Result (in-repo only) | Log |
|---|---|---|---|
| out-of-tree `qmake6` + `bear -- make` (-Wall -Wextra) | yes | 0 warnings, links | `build.log`, `compile_commands.json` |
| `/usr/lib/qt6/bin/qmllint -I /usr/lib/qt6/qml src/*.qml` | yes | 2 real: layout-managed `height` (`src/PasswordsPanel.qml:211`, `:269`). 291 `unqualified` = context-property / parent-member noise | `qmllint.log` |
| clang-tidy `bugprone-*,clang-analyzer-*,performance-*,concurrency-*` | yes | 27: qsizetype→int narrowing, swappable params, branch-clone `src/backend.cpp:431`, no-automatic-move `:342`, throwing static init `src/vault.cpp:18`. No analyzer path bugs | `clang-tidy.log` |
| clazy-standalone level1 | yes | 35: detaching-temporary, range-loop-detach, non-POD static (see F34) | `clazy.log` |
| cppcheck warning,performance,portability | yes | no usable result (only `unknownMacro` on `Q_PROPERTY`, needs Qt config) | `cppcheck.log` |
| g++ -fsyntax-only -Wall -Wextra -Wshadow -Wconversion | yes | 14 `-Wconversion` (qsizetype→int), 0 `-Wshadow` | `gxx-warnings.log` |
| shellcheck `bin/* scripts/*.sh` | yes | SC1007 ×3 (`bin/build:4`, `bin/run:4`, `bin/install:4`), SC2251 (`scripts/validate.sh:9`) | `shellcheck.log` |
| namcap `pkgbuild/PKGBUILD` | yes | E: "File referenced in $startdir" ×2 | `namcap.log` |
| desktop-file-validate | yes | clean | `desktop-file-validate.log` |
| WCAG contrast script (24 themes: 23 bundled + current) | yes | see F10 | `contrast.py`, `contrast.txt` |
| eslint, tsc | **missing** (not installed) | JS is 161 lines, read by hand | - |
| cargo clippy | n/a | no Rust in the repo | - |
| Orca / accerciser, live HiDPI, running the app | not run (audit rule: don't run the app) | a11y and HiDPI findings are static | - |

## Prior Handoff Status
Old file: `HANDOFF.2026-11-17.prev.md` (dated 2026-11-09). It had no numbered IDs for its low follow-ups, so they are labeled L1-L8 here in the order it listed them.

| Old ID | Item | Status | Evidence |
|---|---|---|---|
| H1 | Renderer sandbox back on | fixed | no `--no-sandbox`: `src/main.cpp:29-34` |
| M1 | Stop on `RAND_bytes` failure | fixed | `src/vault.cpp:25-26` |
| M2 | Atomic history/bookmark writes | fixed | `QSaveFile`: `src/backend.cpp:47`, `:259` |
| M3 | Autofill without click, also on http:// | **open** → F01 | `src/vault.cpp:216-245`, `src/login.js:73-79`, `src/Main.qml:183-186`. Old cite `login.js:472` is stale (the file has 144 lines) |
| M4 | Read-only address line in popups | fixed | `src/Main.qml:313-327` |
| M5 | Popup blocker | fixed | `settings.javascriptCanOpenWindows: false` at `src/Main.qml:335`, `:1052` |
| L1 | Assistant API error text lost; mid-stream errors end as success | **open** → F02 | `src/backend.cpp:411-438` |
| L2 | Single-instance `listen()` unchecked; URL lines split across reads | **open** → F25 | `src/main.cpp:69`, `:73-79` |
| L3 | fish `\xNN` escapes not decoded | **open** → F28 | `src/backend.cpp:220-228` (old cite `:218`) |
| L4 | `__nd_user` lives in the page's sessionStorage | **open** → F27 | `src/login.js:6`, `:43`, `:49` |
| L5 | Popups closed by the WM never destroyed | **open** → F21 | `src/Main.qml:303-348` (no `onClosing`) |
| L6 | Downloads accepted without a prompt | **open** → F07 | `src/main.cpp:101-104` |
| L7 | `bookmarks.json` not 0600 | **open** → F29 | `src/backend.cpp:256-267`; on disk it is 0644 |
| L8 | `Backend::readResource` takes any path | **open** → F30 | `src/backend.cpp:313-316` |
| FEAT-F03 (harness) | Tab session restore todo | **open** → F04 | `dev/FEATURES.json:25-27` (`"status": "todo"`); no session code in `src/` |
| FEAT-F01/F02 (harness) | Marked done but evaluated on the old Rust app | **open** (needs re-evaluation on Qt) | `dev/FEATURES.json:5-17` |
| Next-3 | Decide `review-fixes-old-base` | **open** | branch still at `99593c3`, not on master |
| State: "Qt on the dev machine: qt6-webengine 6.11.2" | Dev-machine Qt note | obsolete | qt6-base is now 6.12.0 while webengine is still 6.11.2 (`pacman -Q`) |

Counts: open 14, fixed 5, obsolete 1.

## Findings
Severity: P0 = data loss, crash in the main flow, or security; P1 = frequent bug, UI freeze, or broken core a11y; P2 = noticeable perf/UX/theme/scale issue; P3 = hygiene/polish. "Manual:" checks run in a scratch `HOME`/`XDG_DATA_HOME` per `.trail/learnings`.

| ID | Sev | Area | file:line | Problem | Fix | Acceptance check |
|---|---|---|---|---|---|---|
| F01 | P1 | security | `src/vault.cpp:216-245`, `src/login.js:73-79`, `src/Main.qml:183-186` | On page load `scan()` sends `need`, QML calls `vault.fillScript` and the saved password is written into the DOM with no user gesture. Page scripts (third-party or XSS) can read it, and on `http://` origins so can a network attacker. Prior M3. | Fill only after a trusted click/keydown on the username or password field (handled in login.js's isolated world). Refuse password fill when `originOf()` scheme is `http` unless the host is loopback. Keep username-only prefill optional. | `grep -n '"http"' src/vault.cpp` shows a scheme gate in `fillScript`. Manual: saved login for `http://127.0.0.1:8000`; after load, `document.querySelector('input[type=password]').value === ''` until the field is clicked. |
| F02 | P1 | ux | `src/backend.cpp:411-438` | On HTTP errors the `readyRead` lambda drains the body into `pending` (`:412`), so `reply->readAll()` at `:434` is empty and the user sees Qt's generic `errorString()` ("server replied: Bad Request/Unauthorized/Payment Required"). No mapping for 401 (bad key), 400/402 (credit or billing; Anthropic's docs say a spend limit returns 400 and billing errors 402), 429 (rate or spend cap), 529 (overloaded). An SSE `event: error` mid-stream is ignored and ends as `assistantDone`. Credit exhaustion is exactly what Alan hit. | Read `QNetworkRequest::HttpStatusCodeAttribute` in `readyRead`. If ≥400, keep the raw body and skip SSE parsing. In `finished`, parse `error.type`/`error.message` from it and map to actionable copy: "API key rejected (401): check ANTHROPIC_API_KEY / HAIKU_API_KEY", "Out of credits or billing problem (400/402): add credits or switch provider", "Rate or spend limit (429)", "Provider overloaded (529), retry". Emit `assistantError` on SSE `"type":"error"`. | `grep -n 'HttpStatusCodeAttribute' src/backend.cpp` non-empty. Manual: `ANTHROPIC_API_KEY=invalid ./build/nodaysidle-browser` (scratch HOME), Summarize, and the panel shows "API key rejected (401)" plus the API message. |
| F03 | P1 | a11y | `src/IconButton.qml:4-66`, `src/Main.qml:414-503`, `:507-526`, `:700-746`, `:758-779`, `:884-905`, `:941-962` | `grep -c Accessible src/*.qml` = 0. Every control is a `Rectangle`+`MouseArea` with no role or name and isn't focusable (`activeFocusOnTab` is never set), so Tab skips the toolbar, tabs, star, AI button, and the permission and login bars. Screen readers get unnamed items. | Make `IconButton` the single button primitive: `Accessible.role: Accessible.Button`, `Accessible.name` (Back/Forward/Reload/Stop/Home/Passwords), `activeFocusOnTab: true`, `Keys.onReturnPressed/onSpacePressed: clicked()`, and a visible focus ring (`border.color: theme.accent` when `activeFocus`). Reuse it for tab close, new tab, star, AI, and the bar buttons. Tab pills get `Accessible.role: Accessible.PageTab`. | `grep -c 'Accessible.name' src/*.qml` ≥ 15. Manual: from the URL bar, Shift+Tab reaches Home/Reload/Forward/Back and Enter activates them. |
| F04 | P1 | ux | `src/Main.qml:380`, `:74`, `:104-121`; `dev/FEATURES.json:25-27` | Ctrl+Q (next to Ctrl+W) calls `Qt.quit()` at once and there's no session restore, so every open tab is lost. Closing the window does the same. `closedTabs` lives only in memory. | Persist `[{url}]` + current index to `AppDataLocation/session.json` via `QSaveFile` on tab add/remove/url change (debounced) and on `aboutToQuit`; restore in `Component.onCompleted` (`:77-82`) when no URLs were passed. Optionally confirm Ctrl+Q when there are more than 3 tabs. | Manual: open 3 sites, Ctrl+Q, relaunch, and the same 3 tabs come back with the same tab active. `test -f ~/.local/share/nodaysidle-browser/session.json` (scratch HOME). |
| F05 | P1 | perf | `src/importer.cpp:98-130`, `:167-173`, `:247-332`; `src/PasswordsPanel.qml:262` | Import runs on the GUI thread: it copies History/places.sqlite (can be hundreds of MB), runs SQL, and calls `secret-tool` with `waitForFinished(30000)`. The window freezes for seconds, and up to 30 s on a keyring prompt. | Move `importSource`/`importPasswordsCsv` into `QtConcurrent::run` with a `QFutureWatcher`. Apply the results to `Backend`/`Vault` on the GUI thread. Add a "Importing…" state and disable the buttons. | `grep -n 'QtConcurrent' src/importer.cpp` non-empty. Manual: during a Chrome import, hovering tabs still repaints and the cursor isn't stuck. |
| F06 | P2 | perf | `src/vault.cpp:76-96`, `:100-105`; `src/main.cpp:116` | `Vault`'s constructor runs `secret-tool lookup` (and `store` on first run) synchronously before `engine.load`. With a locked keyring the window doesn't appear for up to 30 s (60 s on first run). | Construct the vault unopened. Open the key with async `QProcess` signals after the window is shown, and expose `available` with `NOTIFY` (it already has `changed`). | Manual: with a `secret-tool` stub on PATH that sleeps 30 s, the window maps in under 2 s and the Passwords panel shows "Unlocking keyring…". |
| F07 | P2 | security | `src/main.cpp:101-112` | Every download is auto-accepted into `~/Downloads`: no prompt, no origin shown, no progress, no cancel. Any page can drop files (drive-by). Prior L6. | Forward `downloadRequested` to QML. Show a bar with filename, origin, size, and Save/Cancel; accept only on Save. Show progress and a "Show in folder" action (`xdg-open` on the dir). | Manual: a page that auto-triggers a download (`a.download=…; a.click()` from a timer) shows a prompt, and `ls ~/Downloads` (scratch HOME) is unchanged until Save. |
| F08 | P2 | security | `src/Main.qml:1103-1115`, `:861-908`, `:75` | There's one global `pendingPermission`. A background tab's camera/mic request shows on whatever tab is visible. A new request silently denies the old one. If the requesting tab closes, the bar keeps a stale object. The bar has no keyboard path. | Keep a pending permission per view (property on the WebEngineView delegate) and show the bar only when its tab is current. Deny and reset on tab close. Add Enter = Allow and Escape = Deny while the bar has focus, and move focus to it. | Manual: a background tab requests the mic, so no bar on the current tab. Switch to it and the bar appears. Close it and no bar remains. |
| F09 | P2 | ui | `src/theme.cpp:92-105`, `src/theme.h:52-62` | Only `lighter_background`/`dark_background`/`light_foreground` are read. Alan's current theme (`prisma-cube`) uses `lighter_bg`/`dark_bg`/`light_fg`, so surface, chrome and text fall back to hardcoded slate (`#14171A`/`#050607`/`#94A3B8`). `mode` is ignored, and the default chrome differs between `theme.h:55` (`#0D0F12`) and `theme.cpp:98` (`#050607`). Values must be quoted 6-digit hex, otherwise they're silently ignored. | Accept alias keys (`lighter_bg`, `dark_bg`, `light_fg`, `fg`, `bg`). When a key is missing, derive it from `background`/`foreground` (mix) instead of the fixed slate. Use one fallback table. Read `mode`. | `grep -n 'lighter_bg' src/theme.cpp` non-empty. Manual: with prisma-cube active, the toolbar color equals `dark_bg` `#0A0A12`. |
| F10 | P2 | a11y | `src/Main.qml:492`, `:612`, `:680`, `:1014`, `:1246`, `:1310`, `:1384`; `src/PasswordsPanel.qml:72`, `:95`, `:161` | WCAG AA (computed in `contrast.txt`): `muted` text on `surface` fails 4.5:1 in **24/24** themes and on `chrome` in 20/24. It's used for placeholders, URL lines and every "×" close glyph. `accent` link text on `surface` ("Clear all", "Stop", "Import", "Copy") fails in catppuccin-latte, miasma, nord, rose-pine, white (min 2.80). `line` borders are under 3:1 in 23/24, so unfocused inputs have no visible edge. | Add derived tokens in `Theme` (like `accentForeground`, `src/theme.cpp:69-74`): `mutedText` = `muted` moved toward `foreground` until ≥4.5:1 on its panel background, `linkText` likewise, `inputBorder` ≥3:1. Use them for text and controls. | Re-run `contrast.py` with the derived tokens and get 0 text-pair failures across `/usr/share/omarchy/themes/*`. |
| F11 | P2 | perf | `src/theme.cpp:39-42`, `:53-57` | Spawns `hyprctl -j getoption decoration:rounding` every 1 s for the app's lifetime (~86k processes/day) just to track the corner radius. | Read it once at start and in `reload()` (the theme watcher already fires on switches), or subscribe to Hyprland `socket2` `configreloaded`. Drop the 1 s timer. | `grep -n 'm_roundingPoll' src/theme.cpp` returns nothing. Manual: `strace -f -e trace=execve -p $(pidof nodaysidle-browser)` shows no `hyprctl` over 10 s idle. |
| F12 | P2 | perf | `src/Main.qml:1038-1055` | Every tab is a live `WebEngineView`. Hidden tabs are never frozen or discarded (`grep lifecycleState` = 0), so memory and CPU grow with tab count. | Bind `lifecycleState: visible ? WebEngineView.LifecycleState.Active : recommendedState` (QtWebEngine ≥5.14 API) or freeze after N minutes hidden. Exclude tabs playing audio. | `grep -n 'lifecycleState' src/Main.qml` non-empty. Manual: 20 tabs, idle 5 min, renderer RSS sum (`ps -C QtWebEngineProcess -o rss=`) drops. |
| F13 | P2 | ux | `src/Main.qml:1042-1203` | No `onRenderProcessTerminated` handler, so a crashed or OOM-killed renderer leaves a blank tab with no explanation or reload button. | Handle `renderProcessTerminated(status, exitCode)`: mark the tab crashed and show a themed overlay with "This page stopped (status)" and Reload (Enter). | Manual: `kill -SEGV` a `QtWebEngineProcess` renderer PID and the tab shows the overlay. Reload works. |
| F14 | P2 | ux | `src/Main.qml:621-636`, `:687-692` | After Enter/Escape in the URL bar (and after a suggestion click), focus is set to `false` but never handed to the page, so Space/PageDown/arrows do nothing until you click. That breaks the keyboard-first flow. | After accept or escape, call `currentView().forceActiveFocus()` (as `closeFind()` does at `:268`). | Manual (wtype per `.trail/learnings`): Ctrl+L, type URL, Enter, then Space scrolls the page. |
| F15 | P2 | ux | `src/home.html:78-79`; `src/Main.qml:79`, `:90-102`, `:352` | The new-tab page autofocuses its own input, which always submits to DuckDuckGo, even for "github.com" (its placeholder says "or type a URL"). Ctrl+T doesn't focus the browser URL bar. | In `newTab()` focus `urlInput` when the URL is home (`selectAll(); forceActiveFocus()`). Drop `autofocus` from `home.html` or change its copy to "Search DuckDuckGo". | Manual: Ctrl+T, type `github.com`, Enter, and it navigates to `https://github.com`. |
| F16 | P2 | ux | `src/Main.qml:1401`; `src/backend.cpp:385-405`, `:452-460` | The assistant badge says "[Private]", but Summarize/Ask send title, URL and up to 12,000 chars of page text to `api.anthropic.com`, and Scam check sends them to `api.typesafe.ai`. The copy is misleading. | Replace it with "Sends page text to Anthropic (model …)" / "…to TypeSafe", built from config (F20). | `grep -n 'Private' src/Main.qml` returns nothing. |
| F17 | P2 | ux | `src/Main.qml:405-416`, `:43` | Tab pills have a 120 px minimum. The strip never scrolls to the current tab (no `positionViewAtIndex`), so in a narrow tiled column with many tabs the active tab is off-screen and there's no overflow cue. | On `currentTabIndexChanged`, call `tabsList.positionViewAtIndex(currentTabIndex, ListView.Contain)`. Allow icon-only pills below ~80 px. Add edge fades when `contentWidth > width`. | Manual: 600 px-wide tile, 12 tabs. Ctrl+Tab cycling keeps the active pill fully visible. |
| F18 | P2 | ux | `src/Main.qml:1230-1240`, `:850-853`; `src/PasswordsPanel.qml:200-201` | "Clear all" history, "Delete Bookmark" and password "Delete" act at once, with no confirm or undo. | Add an undo toast (keep the deleted items for 6 s), or a two-step "Click again to clear". For passwords, require the second step. | Manual: Clear all shows "Undo". Undo restores the same number of rows. |
| F19 | P2 | a11y | `src/Main.qml:481-484`, `:801`, `:1011-1019`, `:1243-1251`, `:1381-1389`; `src/PasswordsPanel.qml:69-78`, `:172` | Targets under 24×24: tab close 18×18, bookmark chips 22 px tall, text-only "×" closes. Password row actions show only on mouse hover (`visible: rowHover.hovered`), so the keyboard can't reach them. | Minimum 24×24 hit areas (MouseArea margins or the `IconButton` from F03). Show row actions when hovered or when the row has `activeFocus`. Make ListViews keyboard-navigable. | `grep -nE 'width: 18$\|height: 18$' src/Main.qml` returns nothing. Manual: Tab into the Passwords list, then arrow plus Enter reaches Copy. |
| F20 | P2 | code | `src/backend.cpp:187-199`, `:377-441`; `src/backend.h:15`, `:25`; `src/Main.qml:1337`, `:1401`, `:1516` | The assistant provider is hardcoded: endpoint `https://api.anthropic.com/v1/messages` (`:385`), `x-api-key` and `anthropic-version: 2023-06-01` (`:387-388`), model `"claude-haiku-5-5"` (`:391`), `max_tokens` 4000, Anthropic-specific SSE parse (`:420-421`) and error schema (`:434`). The key is read once at startup (the `CONSTANT` property), so a new key needs a restart. `hasAnthropicKey` is never used in QML. | Add a small provider interface (`AssistantProvider::buildRequest`, `parseChunk`, `parseError`) with the Anthropic implementation. Read endpoint, model and key-env name from a config file (proposed: `~/.config/nodaysidle-browser/assistant.json`). Re-read the key per request. Derive UI labels from config. | `grep -c 'claude-haiku-5-5' src/*.cpp src/*.qml` = 1 (the default in the config loader only). Manual: change the model in config and the request body uses it without a rebuild. |
| F21 | P2 | code | `src/Main.qml:303-348` | A popup window closed by the WM (Super+W) is hidden, not destroyed: no `onClosing`, so its WebEngineView keeps running (audio, timers, memory). Prior L5. | Add `onClosing: destroy()` to `popupWin`. Keep the `onWindowCloseRequested` path. | `grep -n 'onClosing' src/Main.qml` non-empty. Manual: open a popup playing audio, close it with Super+W, and the audio stops. |
| F22 | P2 | ux | `src/Main.qml:1279-1323`, `:1255-1269`, `:1208`, `:1330`, `:1340` | The history list is mouse-only. Enter in history search does nothing and Up/Down don't move a selection. Side panels have a fixed 360 px width, which takes over half of a 600-700 px tiled column. | Add `currentIndex`, `Keys.onUp/Down/Return` to the history ListView with Enter = open (Ctrl+Enter = new tab), and route Down from the search field into the list. Clamp the panel width to `Math.min(360, parent.width * 0.4)`. | Manual: Ctrl+H, type, Down, Enter opens the entry. In a 640 px tile the panel is 256 px wide. |
| F23 | P2 | ux | `src/Main.qml:1123-1137`, `:101` | For tab destinations, `newTab(request.requestedUrl)` re-navigates instead of `request.openIn(view)`. That loses `window.opener`/postMessage (OAuth and payment flows) and turns a POST `target=_blank` into a GET. Middle-click/background-tab requests steal focus because `newTab` always switches tabs. | Create the tab, then `request.openIn(newView)` once the delegate exists (in `onItemAdded`). For `InNewBackgroundTab`, don't change `currentTabIndex`. | Manual: middle-click a link and the current tab stays active. `window.opener` is non-null in the opened tab for a `target=_blank` without `noopener`. |
| F24 | P3 | ux | `src/Main.qml:1432-1440`, `:1465-1471`, `:1521-1533`, `:1590-1598` | `assistantBusy` is set only inside the `runJavaScript` callback. The buttons stay enabled while busy, a second click silently cancels the first (stray `assistantDone` newline), and all tabs share one transcript. There's no "no key" empty state. | Set busy before `runJavaScript` and disable the actions while busy. Keep transcripts per tab. Show "No API key: set ANTHROPIC_API_KEY or HAIKU_API_KEY" up front when `!backend.hasAnthropicKey`. | Manual: double-click Summarize and get one request and one answer. |
| F25 | P3 | code | `src/main.cpp:66-80` | `server.listen()` result is unchecked, and `readAll().split('\n')` per `readyRead` can split a URL across reads. Prior L2. | Check `listen()` and log or fall back. Buffer per socket and emit only complete lines. | `grep -n 'if (!server.listen' src/main.cpp` non-empty. |
| F26 | P3 | security | `src/Main.qml:1052-1055`; `src/backend.cpp:340-349` | `localContentCanAccessFileUrls` is left at its QtWebEngine default (enabled per Qt docs, UNVERIFIED on 6.11), and path input becomes `file://`. A downloaded HTML file opened locally could read other local files. | Set `settings.localContentCanAccessFileUrls: false` on tab and popup views. | `grep -n 'localContentCanAccessFileUrls: false' src/Main.qml` gives 2 hits. |
| F27 | P3 | security | `src/login.js:6`, `:43-49` | The typed username is cached in the page's own `sessionStorage['__nd_user']`, where page scripts can read and write it. Prior L4. | Keep it in an isolated-world variable plus a QML-side map keyed by view and origin. | `grep -n 'sessionStorage' src/login.js` returns nothing. |
| F28 | P3 | code | `src/backend.cpp:201-234` | Key lookup only parses `set -gx NAME value` (`:208`). `fish_variables` values aren't unescaped (`\x20`, `\x3a`…) (`:224-228`). Prior L3. | Decode fish `\xNN` escapes. Accept `set -x`/`-Ux`. Better: rely on env plus config (F20). | Unit-style: a `fish_variables` line `SETUVAR --export X:a\x3ab` yields `a:b`. |
| F29 | P3 | security | `src/backend.cpp:256-267` | `bookmarks.json` is written 0644 (it's 0644 on disk now; the parent dir 0700 mitigates). Prior L7. | `QFile::setPermissions(..., ReadOwner\|WriteOwner)` after `commit()`, like `flushHistory` (`:54-55`). | `stat -c %a ~/.local/share/nodaysidle-browser/bookmarks.json` = 600 after a toggle. |
| F30 | P3 | security | `src/backend.cpp:313-316` | `readResource` opens any path. Only used for `:/chrome_app.js` and `:/login.js` (`src/Main.qml:30`, `:38`). Prior L8. | Reject paths that don't start with `:/`. | `grep -n 'startsWith(":/")' src/backend.cpp` non-empty. |
| F31 | P3 | ux | `src/Main.qml:591-606` | The "secure" badge only checks for an `https://` prefix. There's no `certificateError` handler (grep = 0), so cert failures fall to QtWebEngine's default reject with no chrome-level explanation, and the badge says nothing about it. | Handle `onCertificateError`: keep rejecting by default and show a themed bar naming the error and host. Drive the badge from load and cert state. | `grep -n 'onCertificateError' src/Main.qml` non-empty. Manual: `https://expired.badssl.com` shows the bar. |
| F32 | P3 | perf | `src/Main.qml:1491-1499`, `:1586-1588` | Each streamed chunk does `aiOutput.text += chunk`, re-laying out the whole document (O(n²)), and `onTextChanged` forces the cursor to the end. | Use `aiOutput.insert(aiOutput.length, chunk)` or a ListModel of messages. | `grep -n 'aiOutput.text +=' src/Main.qml` returns nothing. |
| F33 | P3 | build | `pkgbuild/PKGBUILD:35-54`; `scripts/validate.sh:8-9`; `bin/build:4-6`, `bin/run:4`, `bin/install:4-7` | namcap: "File referenced in $startdir" ×2. No `source=`/checksums (local-only, not AUR-ready). `validate.sh` ignores qmllint's exit code and greps `^Error` only (SC2251), and builds in-tree. SC1007 in `bin/*`. | Add `BUILD_DIR="${BUILD_DIR:-$ROOT/build}"` to `bin/build`, use `CDPATH=''`, and gate `validate.sh` on `qmllint --max-warnings 0` with the 2 real warnings fixed. | `shellcheck bin/build bin/run bin/install scripts/*.sh` is clean. `namcap pkgbuild/PKGBUILD` is clean, or the `$startdir` use is documented. |
| F34 | P3 | code | `src/backend.cpp:40`, `:51`, `:87`, `:106`, `:118-119`, `:170`, `:241`, `:262`, `:272-273`, `:285`, `:304`, `:421`, `:434`, `:482-485`; `src/importer.cpp:64`, `:80`, `:106`, `:137`, `:264`, `:268`, `:282`; `src/vault.cpp:18`, `:39-61`, `:168-183`; `src/PasswordsPanel.qml:211`, `:269` | 35 clazy warnings (detaching-temporary, range-loop-detach, range-loop-reference, non-POD static). 14 `-Wconversion` qsizetype→int in the OpenSSL calls. 2 qmllint layout warnings. | `std::as_const`/`qAsConst` loops, `.value()` instead of `operator[]` on temporaries, `static_cast<int>` with size checks before OpenSSL calls, `QByteArrayLiteral` for `magic`, `Layout.preferredHeight` in PasswordsPanel. | Re-run the clazy/g++/qmllint commands in Verification and get 0 in-repo warnings for these checks. |
| F35 | P3 | code | `src/main.cpp:25-27` | `qputenv("QT_QPA_PLATFORM","wayland")` overrides any user or test value (offscreen, xcb), and `NO_AT_BRIDGE=1` documents intent to disable AT-SPI (commit "disable AT-SPI hanging bus" on `f01-google-login`). The string isn't present in `libQt6Gui`/`libQt6WebEngineCore`, so it likely does nothing for Qt, but it misleads. | Set the platform only if it's unset (`qEnvironmentVariableIsEmpty`). Drop `NO_AT_BRIDGE` or replace it with a comment explaining the a11y stance. | `grep -n 'NO_AT_BRIDGE' src/main.cpp` returns nothing. `QT_QPA_PLATFORM=offscreen` is respected. |
| F36 | P3 | ui | `src/theme.cpp:15`; `src/home.html:8-11`; `src/Main.qml:140-153`, `:1081-1082` | The theme dir is hardcoded to `~/.local/state/omarchy/current`, with no fallback to `~/.config/omarchy/current`. The start page renders hardcoded dark colors until `LoadSucceeded` applies the theme, so it flashes on light themes (white, flexoki-light, catppuccin-latte, lupine, rose-pine). Page `prefers-color-scheme` doesn't follow the Omarchy `mode`. | Try both theme paths. Inject theme CSS variables at `DocumentCreation` for `qrc:/home.html` (a userScript) instead of after load. Map `mode` to the web color scheme (UNVERIFIED API: Qt 6.x `QStyleHints::setColorScheme` / WebEngine `forceDarkMode`). | Manual: with the `white` theme active, opening a new tab shows no dark flash. |
| F37 | P3 | perf | `src/Main.qml:52-69`, `:711`, `:832-842` | The bookmark bar is a JS-array model rebuilt on every change. Each imported folder instantiates a Menu with a Repeater over all its children (hundreds after a Chrome import). The star calls O(n) `backend.isBookmarked` on every binding re-evaluation. | Expose a C++ `QAbstractListModel` for bookmarks. Use `Instantiator` or lazy menus. Cache `isBookmarked` in a `QSet`. | Manual: after importing 1,000 bookmarks, startup and Ctrl+D don't stall (QML profiler `qmlprofiler` shows <16 ms per binding pass). |

### AI / LLM integration map (for F02, F16, F20, F24)
- Key sources, in order: env `ANTHROPIC_API_KEY` → env `HAIKU_API_KEY` → `~/.config/fish/secrets.fish` `set -gx HAIKU_API_KEY` → same file `ANTHROPIC_API_KEY` → `~/.config/fish/fish_variables` (`src/backend.cpp:187-194`, `:201-234`). Scam check: `TYPESAFE_API_KEY` (`:196-198`). Keys are read once in the constructor (`:23`). No key values were read or recorded in this audit.
- Request: `POST https://api.anthropic.com/v1/messages`, headers `x-api-key`, `anthropic-version: 2023-06-01`, body `model: "claude-haiku-5-5"`, `max_tokens: 4000`, `stream: true`, page text in `system` (`src/backend.cpp:385-405`).
- Model id `claude-haiku-5-5`: listed as the Claude API ID for Claude Haiku 5.5 in Anthropic's models overview (docs.anthropic.com, checked 2026-11-17). So the model string isn't the failure; a dead key or exhausted credits is.
- Missing-key path: `"Set ANTHROPIC_API_KEY or HAIKU_API_KEY in your shell."` (`:380-382`). OK, but only shown after a click.
- Failure path: credits, billing, auth and rate errors surface as generic Qt text (F02). Mid-stream errors look like success.
- UI copy naming the provider: `src/Main.qml:1337`, `:1401`, `:1516`; docs `README.md:72`, `:159`.
- Swapping providers touches: `backend.cpp:187-199` (key names), `:385-392` (endpoint, headers, model), `:411-426` (stream parser), `:428-438` (error parser), `backend.h:15`, `:25`, `:75` (key property), `Main.qml:1401`, `:1516` (labels). Pick the vendor with the owner; this audit doesn't recommend one.

## UI/UX Ideas
Ideas, not defects. Severity = idea (P3). Each lands on existing code.

| ID | Sev | Idea | Lands at |
|---|---|---|---|
| I01 | idea | Ctrl+K command palette: one fuzzy list over open tabs, bookmarks, history and actions (new tab, clear history, toggle bar). Enter switches or opens. Fully keyboard-driven, Omarchy-launcher feel. | Shortcuts `src/Main.qml:352-385`; data from `backend.suggest` `src/backend.cpp:155-175` + `tabsModel` `src/Main.qml:19-21`; popup modeled on the suggestion Popup `src/Main.qml:638-697` |
| I02 | idea | Link hints (press `f`, type 2 letters, follow a link; `F` opens in background) for mouse-free browsing in a tiling WM. | New ApplicationWorld script next to `loginScripts` `src/Main.qml:33-40`; message path like `loginMessage` `src/Main.qml:176-201` |
| I03 | idea | Compact chrome for narrow tiles: below ~700 px width, merge the tab strip and toolbar into one 32 px row (favicon tabs + URL field) and hide the bookmark bar. | Tab strip `src/Main.qml:392-528`, toolbar `:531-781`, bookmark bar `:784-858`; drive from `appWindow.width` |
| I04 | idea | Ctrl+1…9 jump to tab N (9 = last), Ctrl+Shift+PgUp/PgDn to move the current tab. | `cycleTab` `src/Main.qml:296-299`, Shortcuts `:378-379`; `tabsModel.move` |
| I05 | idea | `?` / F1 shortcut sheet overlay generated from the Shortcut list, styled like Omarchy's Super+K keybindings menu, so the many shortcuts (Ctrl+Shift+K, Ctrl+Shift+A, F6…) are discoverable. | `src/Main.qml:352-385`; overlay beside the toast `:1544-1568` |
| I06 | idea | Download shelf: one toast-like bar per download with progress, Open, Show in folder (`xdg-open`), and Cancel. Pairs with F07. | `src/main.cpp:101-112` → QML; toast `src/Main.qml:1543-1568` |
| I07 | idea | "Open in new window" in the link context menu, so Hyprland tiles it next to the current window (native split view). Single-instance handoff already exists. | Context menu `src/Main.qml:1172-1181`; `openRequested` `:1574-1578`; a second `ApplicationWindow` |
| I08 | idea | Assistant header shows provider, model and key state ("Haiku 5.5, no key" / "out of credits"), plus Copy answer and per-tab transcripts. Disable the actions when there's no key. | `src/Main.qml:1356-1405`; `backend.hasAnthropicKey` `src/backend.h:15` |
| I09 | idea | Recently-closed list (Ctrl+Shift+T opens a picker on a second press) and reopening keeps back/forward history. | `closedTabs` `src/Main.qml:74`, `reopenTab` `:231-236`; history panel `:1207-1326` |
| I10 | idea | After a CSV password import, offer to securely delete the plaintext CSV ("Delete export file" button), with an undo window. | `importPasswordsCsv` `src/importer.cpp:334-363`; result text `src/PasswordsPanel.qml:288-295` |

## Agent Task List
Do them in order, one commit each. Run every proof command from the repo root with `B=/tmp/ndb-build` as the out-of-tree build dir.

1. F01: gate password fill on a trusted user gesture and https/loopback. Proof: `grep -n '"http"' src/vault.cpp && grep -n 'isTrusted' src/login.js`, plus the manual http://127.0.0.1 check in F01.
2. F02: status-aware error capture and mapping in `askAssistant`. SSE `error` events → `assistantError`. Proof: `grep -n 'HttpStatusCodeAttribute' src/backend.cpp && grep -n '"error"' src/backend.cpp`, plus the manual `ANTHROPIC_API_KEY=invalid` run.
3. F20 + F16 + F24: provider config (endpoint, model, key-env), key re-read per request, honest panel copy, busy state, no-key empty state. Proof: `test "$(grep -c 'claude-haiku-5-5' src/*.cpp src/*.qml | awk -F: '{s+=$2} END{print s}')" -le 1 && ! grep -n 'Private' src/Main.qml`.
4. F03 + F19: `IconButton` with Accessible role/name, focus ring, keys, ≥24 px; replace ad-hoc Rectangle buttons. Proof: `test $(grep -ho 'Accessible.name' src/*.qml | wc -l) -ge 15 && ! grep -nE 'width: 18$' src/Main.qml`.
5. F14 + F15: return focus to the page after URL accept/escape; Ctrl+T focuses the URL bar; fix the home page input. Proof: `grep -n 'forceActiveFocus' src/Main.qml | wc -l` grows by ≥2, `! grep -n 'autofocus' src/home.html`.
6. F04: session save/restore. Proof: `grep -n 'session.json' src/*.cpp src/*.qml`, plus the manual 3-tab relaunch.
7. F05 + F06: importer on QtConcurrent, async vault key open. Proof: `grep -n 'QtConcurrent' src/importer.cpp && grep -n 'waitForFinished' src/vault.cpp src/importer.cpp` returns nothing.
8. F07 + I06: download prompt and shelf. Proof: `! grep -n 'download->accept();' src/main.cpp` (accept moves behind a user action).
9. F08: per-tab permission prompts with keyboard answers. Proof: `! grep -n 'property var pendingPermission' src/Main.qml`.
10. F09 + F10 + F36: alias keys, derived accessible tokens, theme path fallback, no start-page flash. Proof: `grep -n 'lighter_bg' src/theme.cpp`, and `contrast.py` shows 0 text-pair failures.
11. F11: remove the 1 s hyprctl poll. Proof: `! grep -n 'm_roundingPoll' src/theme.cpp src/theme.h`.
12. F12 + F13: tab lifecycle and crashed-renderer overlay. Proof: `grep -n 'lifecycleState' src/Main.qml && grep -n 'onRenderProcessTerminated' src/Main.qml`.
13. F17 + F22 + F23: tab strip scroll-into-view, keyboard history list, clamped panel width, `openIn` for new tabs, background tabs. Proof: `grep -n 'positionViewAtIndex' src/Main.qml && grep -n 'openIn' src/Main.qml | wc -l` ≥ 3.
14. F18: undo for Clear all / Delete. Proof: `grep -n -i 'undo' src/Main.qml src/PasswordsPanel.qml`.
15. F21 + F25 + F26 + F27 + F29 + F30 + F31: small hardening batch. Proof: `grep -n 'onClosing' src/Main.qml && grep -n 'if (!server.listen' src/main.cpp && grep -c 'localContentCanAccessFileUrls: false' src/Main.qml && ! grep -n sessionStorage src/login.js && grep -n 'startsWith(":/")' src/backend.cpp && grep -n 'onCertificateError' src/Main.qml`.
16. F28 + F35: fish unescape (or drop the fish parsing in favor of F20 config); env handling in `main.cpp`. Proof: `! grep -n 'NO_AT_BRIDGE' src/main.cpp`.
17. F32 + F37: streaming insert, C++ bookmark model. Proof: `! grep -n 'aiOutput.text +=' src/Main.qml`.
18. F33 + F34: script/PKGBUILD hygiene, clazy/conversion/qmllint cleanup. Proof: the Verification lint block shows 0 in-repo warnings, and `shellcheck bin/* scripts/*.sh` exits 0.
19. Harness: re-evaluate F01/F02 in `dev/FEATURES.json` on the Qt app (prior-handoff item). Proof: `git log -1 -- dev/FEATURES.json` newer than this handoff, with evidence under `artifacts/<run-id>/`.

## Verification
Out-of-tree only. Don't use `bin/build` (it builds into `<repo>/build`).
```sh
R=$PWD; B=/tmp/ndb-build; rm -rf "$B"; mkdir -p "$B" && cd "$B"
qmake6 "$R/nodaysidle-browser.pro" && bear --output "$B/compile_commands.json" -- make -j"$(nproc)" 2>&1 | tee build.log
grep -c 'warning:' build.log                                   # expect 0
/usr/lib/qt6/bin/qmllint -I /usr/lib/qt6/qml "$R"/src/*.qml 2>&1 | grep -v '\[unqualified\]' | grep -E '^(Warning|Error)'   # expect none
for f in main backend theme vault importer; do clang-tidy -p "$B" --checks='-*,bugprone-*,clang-analyzer-*,performance-*,concurrency-*' "$R/src/$f.cpp"; done 2>&1 | grep -E "^$R/src/.*warning" | sort -u
for f in main backend theme vault importer; do clazy-standalone -p "$B" -checks=level1 "$R/src/$f.cpp"; done 2>&1 | grep -cE "^$R/src/.*warning"   # target 0
shellcheck "$R"/bin/build "$R"/bin/run "$R"/bin/install "$R"/scripts/*.sh
namcap "$R/pkgbuild/PKGBUILD"
desktop-file-validate "$R/desktop/com.nodaysidle.Browser.desktop"
python3 /tmp/audit-nodaysidle-browser-linux-2026-11-17/contrast.py   # copy it out first; /tmp is ephemeral
```
Manual runs: `HOME=$(mktemp -d) XDG_DATA_HOME=$HOME/.local/share "$B/nodaysidle-browser" http://127.0.0.1:8000/`. Serve test pages over http://127.0.0.1, and drive keys per `.trail/learnings/2026-11-08-to-send-keys-to-a-page-nodaysidle.md`. There's no automated test suite. Adding one (Qt Quick Test for `normalizeUrl`, `Theme::readColor`, the `Vault` seal/unseal round-trip, SSE error parsing) would make F02/F09/F28 provable without a GUI.

## Out of Scope
- Repo agent rules that conflict with this read-only audit, noted and not obeyed: `CLAUDE.md:6` ("Continue autonomously through multi-step C++/QML work until `scripts/validate.sh` passes"), `dev/AGENTS.md:46` ("writes the code and applies fixes"), untracked `AGENTS.md:17-18` ("edit straight from `--source`", use `trail` first). `scripts/validate.sh` and `bin/build` build in-tree, and `bin/install` runs `makepkg -fsi --noconfirm` (a system install). None were run.
- Not done: code edits, git writes, installs, running the app, Orca/AT-SPI checks, live HiDPI at 1.25/1.5/1.75 (static review only: font sizes are `pixelSize` logical px, and glyphs are 16 px `Canvas` drawings, UNVERIFIED sharpness at fractional scale; SVG would be safer).
- Anthropic account: the dead key and exhausted credits are an account matter. No key values were read, printed or written. Topping up or choosing another vendor is the owner's call (integration points above).
- Environment: qt6-base/declarative/wayland 6.12.0 alongside qt6-webengine 6.11.2 on this machine (partial upgrade). Whether the binary still starts is UNVERIFIED. Re-sync with `pacman -Syu` before manual checks.
- Hyprland bind conflicts: none found between the app's Ctrl shortcuts and `~/.config/hypr/*.lua` (`CTRL + SHIFT + Escape`, `CTRL + SPACE`, `CTRL + SHIFT + semicolon`, `CTRL + ALT + …` don't overlap). Default browser is `omarchy-browser.desktop` (`xdg-settings`). Registering nodaysidle is the owner's choice.
- Tiling: no forced min size (`src/Main.qml:7-11`), no CSD (`QT_WAYLAND_DISABLE_WINDOWDECORATION=1`, `src/main.cpp:26`). Fine on Hyprland.
- Pre-existing untracked files left as found: `.optim-jev/`, `AGENTS.md`, `opencode.json`, in-tree `build/`. Leftover profile dirs `~/.local/share/nodaysidle-browser/webkit-{cache,data}` from the WebKitGTK era weren't touched.
- Branches `audit-fixes`, `remove-rust-launcher`, `f01-google-login`, `review-fixes`, `review-fixes-old-base` still exist locally. Cleanup is the owner's decision.

---

# Work done — 2026-11-17

**Status: closed out on 2026-11-17.** Everything below is committed, built, package-installed
and verified on the running app. The only open item is the clazy/`-Wconversion` half of F34.

Branch `audit-fixes-2026-11-17`, based on `master` @ `2d14e25`, **21 commits**; merged into `master` by fast-forward on 2026-11-21 (branch deleted), **not pushed**.
Owner instruction that overrode the audit's own recommendations: **remove the Anthropic
integration completely** (no Anthropic usage on this machine); TypeSafe is the only AI/key
integration left, read from `TYPESAFE_API_KEY`.

Build gate, run from the repo root with `B=/tmp/nds-browser-build`:
`rm -rf $B && mkdir -p $B && cd $B && qmake6 <repo>/nodaysidle-browser.pro && make -j$(nproc)`
-> **0 warnings, 0 errors**. `qmllint -I /usr/lib/qt6/qml src/*.qml` (unqualified filtered)
-> no output. `shellcheck bin/* scripts/*.sh` -> exit 0.

## Anthropic removal (priority change) — `170c609`
Everything Anthropic is gone from `src/`: both key lookups and their fish fallbacks, the
`api.anthropic.com/v1/messages` request, `x-api-key` / `anthropic-version`, the
`claude-haiku-5-5` model id, the SSE stream parser and the Anthropic error schema. The "AI"
toolbar button and the Ask/Summarize panel went with it; no stub, no disabled button, no
placeholder. The panel that remains is the TypeSafe scam check, relabelled honestly.
Proof: `rg -n -i 'anthropic|haiku|claude|x-api-key' src/` -> no match (exit 1).

## Finding status

| ID | Status | Commit | Proof / note |
|---|---|---|---|
| F01 | done | `77c7788` | password fill only on a trusted gesture (`isTrusted` in login.js); `passwordFillAllowed()` refuses non-loopback http. 9/9 origin cases pass against the compiled function |
| F02, F20 | superseded | `170c609` | "superseded: Anthropic removed". What carried over is in `checkScam`: `HttpStatusCodeAttribute`, body kept, 401/402/403/429/5xx mapped, error object on 2xx = failure |
| F03 | done | `8181fa4` | 19 `Accessible.name` hits (was 0); IconButton is the one button primitive. Live: Ctrl+L then Shift+Tab walks URL -> Home -> Reload -> Forward with the ring; Enter on Home loads the start page |
| F04 | done | `42a170d` | `session.json` (0600) with urls+index, restored at start, written on change and close; Ctrl+Q asks above 3 tabs. Live: 3 tabs restored with the third active; Escape cancels the prompt |
| F05 | done | `d50c2d9` | `QtConcurrent::run` for both imports, results applied on the GUI thread, `importFinished` + `importing`. One `waitForFinished` remains in `keyringPassword` **on purpose** — it is on the worker thread now |
| F06 | done | `d50c2d9` | `waitForFinished` -> 0 in vault.cpp. Live with a 30 s `secret-tool` stub: window mapped in 638 ms while the stub still ran |
| F07 | done | `341aa2f` | downloadRequested handled in QML; nothing written until Save. Live: shelf showed origin + size, `~/Downloads` absent before any answer |
| F08 | done | `82e3abd` | per-view `permissionRequest`; bar follows the current tab; close denies; Allow/Deny keyboard-reachable; Escape = No |
| F09, F10, F36 | done | `4d9d153` | alias keys (`lighter_bg`, `dark_bg`, `light_fg`, `fg`, `bg`), derived WCAG ink, theme-dir fallback, DocumentCreation home theme, `mode` -> QStyleHints colour scheme. Ran the shipped Theme against all 24 themes: **0 text-pair failures** |
| F11 | done | `7925183` | `m_roundingPoll` gone; live with a logging hyprctl stub: 2 calls in 8 s, both at startup |
| F12, F13 | done | `c55c4e1` | `lifecycleState` frozen when hidden (audio-aware via `recentlyAudible`), crash overlay with status/exit code + Reload. Live SIGKILL -> "status 2, exit code 9" overlay |
| F14, F15 | done | `1d0a796` | focus returns to the page after URL accept/escape; Ctrl+T focuses the URL bar; start page no longer autofocuses |
| F16, F24 | done | `170c609` | honest "Sends this page's text to TypeSafe (api.typesafe.ai, jev-latest)" badge, no-key state, busy state + 30 s watchdog |
| F17, F22, F23 | done | `3f81542` | `positionViewAtIndex`, compact pills, overflow fade; history list keyboard + clamped panels; `request.openIn(view)` for new tabs and background tabs |
| F18 | done | `0c9b35c` | undo for Clear all / Delete Bookmark via the toast; password delete needs a second click |
| F19 | done | `b69a220` | no 18 px or 22 px targets left; password rows keyboard-reachable, actions show on current row |
| F21, F25, F26, F27, F29, F30, F31 | done | `77c6954` | popup `onClosing: destroy()`; listen() checked + line buffering; `localContentCanAccessFileUrls: false` x2; no sessionStorage; bookmarks 0600 (verified: mode 600 from a probe); readResource only `:/`; certificate bar |
| F28, F35 | done | `48c5c52`, `7925183` | fish `\xNN` decoded (5/5 cases + end-to-end pickup), `set -x`/`-Ux` accepted; no `NO_AT_BRIDGE`, platform only set when empty |
| F32 | satisfied | `170c609` | the streaming `text +=` path no longer exists |
| F33 | done | `3d46c22`, `076e4f7` | shellcheck exit 0; `BUILD_DIR` honoured by bin/build, bin/run, install-desktop.sh and validate.sh; validate.sh gates on real qmllint findings. namcap's two `$startdir` errors are documented in the PKGBUILD (accepted: local-only package) |
| F34 | **partial** | `b69a220` | qmllint is clean (both PasswordsPanel warnings fixed). **clazy level1 + `-Wconversion` are NOT done** (~35 + 14 findings, mechanical, needs a `bear` build to re-measure) |
| F37 | done | `b69a220` | `isBookmarked()` is O(1) via a `QSet` rebuilt on load/save. The bookmark-bar model is unchanged (no C++ list model) |
| I06 | done | `341aa2f` | download shelf (item 8 of the task list authorises it) |
| Harness | done | `d8515b2` | `dev/FEATURES.json` re-evaluated on the Qt app; F01 live (cookie on disk with `is_persistent=1`), F02 re-checked, F03 closed. Evidence in `artifacts/2026-11-17-audit-fixes/` (gitignored on purpose) |

Not started: none of the audit's findings. The only open item is the clazy/conversion half
of F34.

## Install state (final)
The owner ran `bin/install`; the package was rebuilt from this branch and reinstalled:
`nodaysidle-browser 0.3.0-1`, **Build/Install Date 2026-11-17 02:36**, owning
`/usr/bin/nodaysidle-browser`, `/usr/share/applications/com.nodaysidle.Browser.desktop`,
`/usr/share/icons/hicolor/scalable/apps/nodaysidle-browser.svg` and the fish completion
under `/usr/share/fish/vendor_completions.d/`. The packaged binary was smoke-tested on its
own: it loads a page, Ctrl+T opens a tab, Ctrl+Q quits, and `session.json` is written.

The earlier per-user install (`~/.local/bin` + `~/.local/share/applications`) was
**retired afterwards**, because it shadowed `/usr/bin` in `PATH` and the same desktop id, so
a future `pacman -Syu` would never have been picked up. It was moved (not deleted) to
`~/.hermes/cache/scratch/nds-local-install-backup-20261117/`; restore it by copying
`{bin,applications,icons,fish}/*` back into `~/`. After the move:
`command -v nodaysidle-browser` -> `/usr/bin/nodaysidle-browser`, and
`gtk-launch com.nodaysidle.Browser` started `/usr/bin/nodaysidle-browser` with the real
profile, rendering normally.

Checked before the removal so nothing could break: the package owns every file the user copy
provided; no `~/.config` file (hypr, omarchy, fish, autostart, systemd) referenced the
`~/.local/bin` path; and the default browser is `helium`, not nodaysidle. `bin/install`
leaves no `pkgbuild/src`, `pkg` or `.pkg.tar.zst` behind.

## Pitfalls found the hard way (for the next agent)
- `WebEngineView` in Qt 6.12 has **no `audible`** property. Only `recentlyAudible`. Check
  `/usr/lib/qt6/qml/QtWebEngine/plugins.qmltypes` before binding a WebEngine property.
- `SIGSEGV` on a QtWebEngine renderer is **swallowed by Chromium's crash handler** — the
  process stays alive and no `renderProcessTerminated` fires. Use `SIGKILL` to test F13.
- `QColor::fromRgbF` keeps 16-bit channels, so a contrast check can measure a different
  colour than the `#rrggbb` it exports (4.5126 vs 4.4956). Mix on 8-bit channels.
- `colorsPath()` must handle both `~/.local/state/omarchy/current/theme/colors.toml` and
  `/usr/share/omarchy/themes/<name>/colors.toml` (no `theme/` segment). Missing the second
  one silently renders every package theme with fallback colours.
- QML `console.log` did not reach this app's stderr in these probes; do not trust a silent
  log as "the handler never ran". Verify by making the effect visible instead.
- `pkill -f nodaysidle-browser` kills the agent's own shell (the wrapper command line
  contains the repo path). Anchor the pattern, e.g. `[.]local/bin/nodaysidle-browser`.
