# CLAUDE.md — nodaysidle-browser-linux

## 1. Execution Posture (Opus 5.5 Directives)
- Prioritize direct code execution, compiler checks, and tool use over conversational narration.
- Do not provide mid-task status summaries or narrate routine file reads.
- Continue autonomously through multi-step C++/QML work until `scripts/validate.sh` passes; only pause if blocked by a critical ambiguity.
- The smallest change that fully satisfies the task (YAGNI). Keep browser chrome quiet and minimal; do not introduce unnecessary dependencies or complex abstractions.

## 2. Command Matrix
*Execute these exact commands:*

| Command | Purpose |
|---------|---------|
| `./bin/build` | qmake6 + make into `build/nodaysidle-browser` |
| `./bin/run [URL]` | Launch the built binary |
| `bash scripts/validate.sh` | Build, then qmllint every QML file |
| `bash scripts/install-desktop.sh` | Install binary, launcher entry and icon for the current user |
| `./bin/install` | Build the Arch package with `makepkg` and install it (Omarchy launcher entry) |

## 3. Architecture & Threading Rules
- **Product**: Quiet, native Linux browser: Qt 6 Quick on QtWebEngine, pure Wayland, minimal chrome, local-first privacy.
- **Stack**: C++17, Qt 6 (Quick, QuickControls2, WebEngineQuick; 6.11 on the dev machine), qmake6. There is no Rust in this repo any more: the old Rust + GTK app ended at v0.2.2 and the cargo launcher wrapper was removed.
- **Key files (`src/`)**:
  - `main.cpp` - Wayland env, persistent profile, Chrome UA, downloads, single instance
  - `backend.{h,cpp}` - bookmarks, history, URL resolution, clipboard, TypeSafe Jev call
  - `theme.{h,cpp}` - Omarchy `colors.toml` watcher and Hyprland rounding
  - `Main.qml` - window, tabs, toolbar, find bar, permission bar, scam-check panel
  - `vault.{h,cpp}` - encrypted saved logins, AES-256-GCM, key in the Secret Service keyring
  - `importer.{h,cpp}` - imports from Chromium and Firefox family profiles and passwords CSV
  - `login.js` - login capture and fill, isolated world, console lines carry a per-run nonce
  - `adblock.h` - hardcoded ad/tracker request blocker
  - `chrome_app.js` - `window.chrome.app` shim, required for Google sign-in
- **Profile**: always the named persistent profile; the default QtWebEngine profile is off-the-record.
- **Zero Telemetry**: Never add tracking, analytics beacons, or remote logging.

## 4. Negative Constraints (Anti-Defaults)
- Never add a library dependency (`QT +=` or `LIBS` in `nodaysidle-browser.pro`, `depends` in `pkgbuild/PKGBUILD`) without explicit user permission.
- Never pass `--no-sandbox` to QtWebEngine; the renderer sandbox stays on.
- Never write placeholder functions or mock implementations.

## 5. Modular Imports
@dev/AGENTS.md

## 6. Product Harness & Verification Discipline
- Read dev/PRODUCT.md for high-level goals, scope boundaries, and verified commands.
- Read dev/CONTRACT.md for the active feature's strict checks and evidence criteria.
- Read dev/FEATURES.json and HANDOFF.md when starting or resuming work.
- Build one agreed feature at a time; never break existing tests or touch unrelated code.
- Save full verification logs under artifacts/<run-id>/.
- Mark a feature PASS in dev/FEATURES.json ONLY after the independent EVALUATOR verifies evidence against the recorded git commit.
- Repair rounds are capped at 2 before stopping for human review.
