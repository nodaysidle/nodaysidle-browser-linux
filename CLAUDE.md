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
| `cargo check` | Fast syntax and type verification |
| `cargo build` | Debug build |
| `cargo build --release` | Optimized release binary |
| `cargo test` | Run test suite |
| `cargo clippy -- -D warnings` | Clippy linter with warnings treated as errors |
| `cargo fmt --check` | Verify standard Rust formatting |
| `bash scripts/validate.sh` | Project release validation script |

## 3. Architecture & Threading Rules
- **Product**: Quiet, native Linux WebKit browser with minimal chrome and local-first privacy.
- **Stack**: Rust 2021 (rust-version 1.88), GTK 3 (`gtk = "0.18"`), WebKitGTK 4.1 (`webkit2gtk = "2.0"`), Soup3, JavaScriptCore.
- **Key Modules (`src/`)**:
  - `main.rs` — Application initialization, GtkApplication setup, CLI flags.
  - `tabs.rs` — Tab management, WebKitWebView creation, lifecycle events.
  - `navigation.rs` — URL bar, back/forward, reload, navigation state.
  - `privacy.rs` & `profile.rs` — Ephemeral profiles, cookie isolation, tracking prevention.
  - `shortcuts.rs` — Keybindings and keyboard navigation.
  - `assistant.rs` — Integrated AI / search assistant integration.
- **GTK Thread Affinity**: All UI mutations and WebKit interactions **must** occur on the main GLib thread. Use `glib::MainContext::default().spawn_local(...)` or `glib::idle_add_local(...)` when bridging async/background operations.
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
