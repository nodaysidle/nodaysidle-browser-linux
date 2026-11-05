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
  - Set WebKit `CookieAcceptPolicy::Always` on persistent CookieManager SQLite storage.
  - Verify cookies and site data persist across restarts.
- **F02**: **Deterministic UI Design & Navigation Polish**
  - Eliminate layout jumps and placeholder text styling issues in the tab strip, address bar, and find overlay.
  - Ensure dark/light theme contrast and keyboard shortcuts behave deterministically.
- **F03**: **Tab Session State Persistence**
  - Serialize active tab URLs to disk on exit; restore tabs on launch.

## Decisions Settled

- **Stack**: Rust 2021, GTK 3 (`gtk = "0.18"`), WebKitGTK 4.1 (`webkit2gtk = "2.0"`), Soup3, JavaScriptCore.
- **User-Agent**: Modern Chrome on Linux (`Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36`).
- **Cookie Storage**: SQLite in `~/.local/share/nodaysidle-browser/webkit-data/cookies.sqlite` with strict user-only permissions.
- **Negative Constraints**: Zero telemetry, no unjustified unsafe Rust blocks, no external crates without explicit permission.

## Actual Project Commands

Setup: `cargo build`
Start: `cargo run`
Unit/Integration Checks: `cargo test -- --test-threads=1`
Linter / Static Analysis: `cargo clippy -- -D warnings && cargo fmt --check`
Release Validation: `bash scripts/validate.sh`
Local Verification: `./harness status`

## Scope Boundary

Focus on native Linux WebKit client. No account sync clouds, no telemetry, no bloated extensions system.
