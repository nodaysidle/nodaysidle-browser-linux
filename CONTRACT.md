# Feature Contract: F02 — Deterministic UI Design & Navigation Polish

Feature: F02 — Deterministic UI Design & Navigation Polish
State: Agreed; ready for implementation
Code revision under evaluation: 5ef8424

## Behavior

1. **Deterministic Navigation Chrome**:
   - The address bar (`url_bar`) provides instant, deterministic focus and editing without layout shifts or placeholder blinking.
   - Address bar security status (lock icon for HTTPS, shield/warning for HTTP) is crisp and color-coded with `@secure` and `@insecure` from the active theme.
   - Clean keyboard shortcuts (`Ctrl+L`, `Alt+D`, `Ctrl+R`, `Alt+Left`, `Alt+Right`) provide immediate responsive state updates.

2. **Crisp Tab Strip Design**:
   - Tab pills use clean 4px border-radius with flush bottom accent border (`@accent`) for the active tab, eliminating macOS-like oversized pill margins.
   - Tab titles truncate gracefully with ellipsis when space is constrained; full title and URL remain accessible via native tooltip.
   - Tab close button (`×`) appears cleanly on hover and does not trigger accidental tab activation.

3. **Polished Internal Pages & States**:
   - The Home page and error pages render deterministically using theme tokens (`@bg`, `@surface`, `@text`, `@accent`) with zero unstyled placeholders.
   - Find-in-page bar slides/reveals smoothly with match counts and distinct highlight styling.

## Checks

| ID | Action | Expected observation | Evidence |
|---|---|---|---|
| C01 | Inspect address bar styles and security icon styling | Address bar uses `@surface`, `@text`, `@border`, and HTTPS/HTTP states map to theme colors | not collected |
| C02 | Test tab strip pill layout and selection indicators | Selected tab has distinct `@accent` highlight; tab switching updates immediately | not collected |
| C03 | Check Home page and Error page styling | Background and text adapt to theme tokens with no missing assets or placeholder layout shifts | not collected |
| C04 | Run full Rust test suite (`cargo test -- --test-threads=1`) | All unit and GTK tests pass (84+ passing, 0 failing) | not collected |
| C05 | Run linter and formatting (`cargo clippy -- -D warnings && cargo fmt --check`) | Zero warnings and clean formatting | not collected |

## Evaluation Report Fields

Record `feature_id`, `code_revision`, `check_id`, `observed_result`, `verdict` (pass/fail/unverified), `evidence_path`, and `reproduction_steps`.

## Completion

All agreed checks pass with evidence against the current code revision.

## Repair Policy

At most two repair rounds for this feature before stopping for human review.
