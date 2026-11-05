# CLAUDE.md — nodaysidle-browser-linux

## 1. Execution Posture (Opus 5.5 Directives)
- Prioritize direct code execution, compiler checks, and tool use over conversational narration.
- Do not provide mid-task status summaries or narrate routine file reads.
- Continue autonomously through multi-step Rust implementations until full compilation and test pass; only pause if blocked by a critical ambiguity.
- The smallest change that fully satisfies the task (YAGNI). Keep browser chrome quiet and minimal; do not introduce unnecessary crates or complex abstractions.

## 2. Command Matrix
*Execute these exact commands:*

| Command | Purpose |
|---------|---------|
| `./bin/build` | qmake6 + make into `build/nodaysidle-browser` |
| `./bin/run [URL]` | Launch the built binary |
| `cargo build --release` | Build the Rust launcher wrapper |
| `bash scripts/validate.sh` | Build, qmllint, cargo build, fmt, clippy |
| `bash scripts/install-desktop.sh` | Install binary, launcher entry and icon for the current user |

## 3. Architecture & Threading Rules
- **Product**: Quiet, native Linux browser: Qt 6 Quick on QtWebEngine, pure Wayland, minimal chrome, local-first privacy.
- **Stack**: C++17, Qt 6.12 (Quick, QuickControls2, WebEngineQuick), qmake6. `src/main.rs` is only a cargo wrapper that execs the Qt binary.
- **Key files (`src/`)**:
  - `main.cpp` - Wayland env, persistent profile, Chrome UA, downloads, single instance
  - `backend.{h,cpp}` - bookmarks, history, URL resolution, clipboard, Anthropic and Jev calls
  - `theme.{h,cpp}` - Omarchy `colors.toml` watcher and Hyprland rounding
  - `Main.qml` - window, tabs, toolbar, find bar, permission bar, assistant panel
  - `vault.{h,cpp}` - encrypted saved logins, AES-256-GCM, key in the Secret Service keyring
  - `importer.{h,cpp}` - imports from Chromium and Firefox family profiles and passwords CSV
  - `login.js` - login capture and fill, isolated world, console lines carry a per-run nonce
  - `adblock.h` - hardcoded ad/tracker request blocker
  - `chrome_app.js` - `window.chrome.app` shim, required for Google sign-in
- **Profile**: always the named persistent profile; the default QtWebEngine profile is off-the-record.
- **Zero Telemetry**: Never add tracking, analytics beacons, or remote logging.

## 4. Negative Constraints (Anti-Defaults)
- Never bypass Rust borrow-checker safety with unjustified `unsafe` blocks.
- Never introduce new external crates to `Cargo.toml` without explicit user permission.
- Never write placeholder functions or mock implementations.

## 5. Modular Imports
@AGENTS.md
@README.md

## 6. Product Harness & Verification Discipline
- Read PRODUCT.md for high-level goals, scope boundaries, and verified commands.
- Read CONTRACT.md for the active feature's strict checks and evidence criteria.
- Read FEATURES.json and HANDOFF.md when starting or resuming work.
- Build one agreed feature at a time; never break existing tests or touch unrelated code.
- Save full verification logs under artifacts/<run-id>/.
- Mark a feature PASS in FEATURES.json ONLY after the independent EVALUATOR verifies evidence against the recorded git commit.
- Repair rounds are capped at 2 before stopping for human review.
