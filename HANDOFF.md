# Handoff — nodaysidle-browser-linux, 2026-11-09

Facts below were checked on 2026-11-09 unless marked otherwise.

## What it is
A quiet, native Linux browser for Omarchy: C++17, Qt 6 Quick on QtWebEngine (Chromium), pure Wayland, qmake6 build (`nodaysidle-browser.pro`). There is no Rust in the repo: the old Rust + GTK 3 + WebKitGTK app ended at `v0.2.2`, and the cargo launcher wrapper (`launcher/`) was removed on branch `remove-rust-launcher`. See `README.md` for features and `dev/PRODUCT.md` for scope.

## State
- `master` at `cc5a65c` (2026-11-05), in sync with `origin/master`. Two local branches wait for the owner's review; neither is pushed or merged. They touch different files and should merge cleanly in either order.
  - `audit-fixes` (7 commits on master): the 2026-11-08 audit. H1 renderer sandbox back on (`--no-sandbox` removed), M1 stop on `RAND_bytes` failure, M2 atomic history and bookmarks writes, M4 read-only address line in popups, M5 popup blocker (popups only from a click or key press), plus two trail learnings. Each fix was checked in the running app; logs under `artifacts/audit-fixes/` (gitignored).
  - `remove-rust-launcher` (on master): deletes `launcher/`, drops the cargo steps from `scripts/validate.sh`, and brings `CLAUDE.md`, this file, `dev/PRODUCT.md`, `dev/CONTRACT.md` and `dev/harness.py` up to date with the Qt stack.
- `scripts/validate.sh` now means: `./bin/build`, then `qmllint` on every QML file. It passed on both branches on 2026-11-08/09.
- Latest release `v0.3.0` (tag on `2487b54`, published 2026-11-05): the Qt 6 rewrite. Assets: `nodaysidle-browser-0.3.0-1-x86_64.pkg.tar.zst` (Arch package for the Omarchy launcher), `nodaysidle-browser-0.3.0-linux-x86_64.tar.gz`, `SHA256SUMS`. Qt and QtWebEngine are not bundled. Notes: `docs/releases/v0.3.0.md`.
- Older tags: `v0.2.2` (2026-10-27), `v0.2.1` (2026-10-17), `v0.1.0` (2026-10-07), all the Rust + GTK app. The old AppImage in `dist/` is from that era.
- Other branches: `f01-google-login` (0 commits ahead of master), `review-fixes` (merged into master), `review-fixes-old-base` (8 commits not on master; unreviewed).
- Harness (`dev/FEATURES.json`): F01 and F02 are marked done, but both were evaluated on the old Rust app (F02 at `8514c0f`; `dev/CONTRACT.md` carries a note). **F03 tab session restore is todo** and has no contract yet.
- Qt on the dev machine: `qt6-webengine` 6.11.2.

## Open decisions (audit findings left for the owner)
- M3: autofill fills saved passwords into the page on load, without a click, also on `http://` origins (`src/vault.cpp:214`, `src/login.js:472`). Chrome exposes autofilled values to page script only after user interaction.
- Low follow-ups: assistant API error text is lost and mid-stream errors end as success (`src/backend.cpp:409-431`); the single-instance `listen()` result is unchecked and URL lines can split across reads (`src/main.cpp:70-79`); fish `\xNN` escapes are not decoded when reading keys (`src/backend.cpp:218`); the `__nd_user` marker sits in the page's own `sessionStorage` (`src/login.js:455`); popups closed by the window manager are never destroyed (`src/Main.qml`, popup `onWindowCloseRequested`); downloads are accepted without a prompt (`src/main.cpp:102`); `bookmarks.json` is not 0600; `Backend::readResource` takes any path (QML-only).

## Next
1. Owner reviews and merges `audit-fixes` and `remove-rust-launcher`.
2. F03 (tab session restore): write `dev/CONTRACT.md` for it against the Qt app, build one feature at a time, mark PASS only after the independent EVALUATOR checks evidence against the recorded commit (`CLAUDE.md` section 6).
3. Decide what to do with `review-fixes-old-base` (delete or review).

## Key files
`src/main.cpp`, `src/backend.{h,cpp}`, `src/vault.{h,cpp}` (AES-256-GCM, key in the Secret Service keyring), `src/importer.{h,cpp}`, `src/login.js`, `src/adblock.h`, `src/Main.qml`, `scripts/validate.sh`, `bin/build`, `bin/install`, `pkgbuild/PKGBUILD`, `dev/PRODUCT.md`, `dev/CONTRACT.md`, `dev/FEATURES.json`.

## Watch out
- Zero telemetry: never add tracking, analytics beacons or remote logging.
- No new library dependencies (`.pro` `QT +=`/`LIBS`, PKGBUILD `depends`) without the owner's permission. Keep the renderer sandbox on.
- Run checks with a scratch `HOME` and `XDG_DATA_HOME` so the real profile is untouched. To send keys to a page, open and close the find bar first (Ctrl+F, Escape), and confirm the active window is the browser before every `wtype`. Serve test pages over `http://127.0.0.1`, not `file://`. See `.trail/learnings`.
- Save verification logs under `artifacts/<run-id>/` (gitignored).
- `./bin/install` runs `makepkg -fsi` (installs a system package); `./bin/build` only builds into `build/`.
- Anything that says this repo is Rust, GTK or WebKitGTK, or mentions `launcher/` or `cargo`, describes the pre-v0.3.0 app and is stale.
