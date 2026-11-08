# Product / nodaysidle-browser-linux

Status: Active Development Brief

## Outcome

A quiet, native Linux browser built with Qt 6 Quick and QtWebEngine (Chromium), pure Wayland, following the Omarchy theme.
It focuses on:
1. **Persistent Authentication & Google Login**: Reliable cookie and session persistence across browser restarts, with standard modern User-Agent handling to prevent Google OAuth security blocks ("This browser or app may not be secure").
2. **Deterministic UI Design**: Clean, tactile interface with zero placeholder or layout shift glitches. High-contrast, polished tab strip and URL navigation.
3. **Session Continuity**: Optional automatic restoration of open tabs upon restart.

## Core User Journeys

1. **Persistent Login Flow**: User visits `https://accounts.google.com` (or any auth provider) -> Logs in successfully -> Closes browser -> Reopens browser -> Session/cookies remain valid.
2. **Navigation Flow**: Clean URL entry -> Clear autocomplete and tab pills -> Back/Forward/Reload with immediate visual feedback -> No broken or blank placeholder layouts.
3. **Download & History Flow**: Downloads save predictably to user folder -> History accurately records visits without corrupting files.

## Initial Features

- **F01**: **Persistent Google Login & Secure Auth Session**
  - Configure custom desktop User-Agent string (Chrome/Linux) to avoid Google login blocks.
  - Use the named persistent `QQuickWebEngineProfile` with `ForcePersistentCookies` (the default QtWebEngine profile is off-the-record).
  - Verify cookies and site data persist across restarts.
- **F02**: **Deterministic UI Design & Navigation Polish**
  - Eliminate layout jumps and placeholder text styling issues in the tab strip, address bar, and find overlay.
  - Ensure dark/light theme contrast and keyboard shortcuts behave deterministically.
- **F03**: **Tab Session State Persistence**
  - Serialize active tab URLs to disk on exit; restore tabs on launch.

## Decisions Settled

- **Stack**: C++17, Qt 6 Quick + QtWebEngine (Chromium), pure Wayland, qmake6 (`nodaysidle-browser.pro`). The Rust + GTK 3 + WebKitGTK app ended at v0.2.2; v0.3.0 is the Qt rewrite. There is no Rust left in the repo.
- **User-Agent**: Plain Chrome on Linux whose major version matches the engine's Chromium (`Chrome/<major>.0.0.0`, set in `src/main.cpp`), plus the `window.chrome.app` shim; both are needed for Google sign-in.
- **Profile Storage**: `~/.local/share/nodaysidle-browser/webengine` (cookies, site data) and `.../cache`; history, bookmarks and the encrypted `passwords.vault` sit in `~/.local/share/nodaysidle-browser/`.
- **Negative Constraints**: Zero telemetry, no new library dependencies without explicit permission, renderer sandbox stays on (no `--no-sandbox`).

## Actual Project Commands

Setup: `./bin/build` (qmake6 + make into `build/nodaysidle-browser`)
Start: `./bin/run [URL]`
Unit/Integration Checks: none automated in the Qt app; verify in the running app with a scratch `HOME`/`XDG_DATA_HOME` (see `.trail/learnings`)
Release Validation: `bash scripts/validate.sh` (build, then qmllint)
Local Verification: `cd dev && ./harness status`

## Scope Boundary

Focus on a native Linux QtWebEngine client. No account sync clouds, no telemetry, no bloated extensions system.
