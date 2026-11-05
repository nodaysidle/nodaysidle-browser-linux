# Feature Contract: F02 — Deterministic UI Design & Navigation Polish

Feature: F02 — Deterministic UI Design & Navigation Polish
State: Complete; all checks passed independent evaluation at 8514c0f
Code revision under evaluation: 8514c0f

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
| C01 | Inspect address bar styles and security icon styling | Address bar uses `@surface`, `@text`, `@border`, and HTTPS/HTTP states map to theme colors | PASS at 8514c0f: `artifacts/20261105_012650_8514c0f/eval_evidence.txt`, `screens/f1_https`, `f2_http`, `f3_ctrlL`, `f4_altD` |
| C02 | Test tab strip pill layout and selection indicators | Selected tab has distinct `@accent` highlight; tab switching updates immediately | PASS at 8514c0f: `artifacts/20261105_012650_8514c0f/eval_evidence.txt`, `screens/g1`-`g7`, `g_montage` |
| C03 | Check Home page and Error page styling | Background and text adapt to theme tokens with no missing assets or placeholder layout shifts | PASS at 8514c0f: live change on load-failed, TLS, crashed, background, pop-up and two-window error pages; Back; Menu → Theme; Home. `artifacts/20261105_012650_8514c0f/eval_evidence.txt`, `screens/a1`-`e2`, `f0` |
| C04 | Run full Rust test suite (`cargo test -- --test-threads=1`) | All unit and GTK tests pass (84+ passing, 0 failing) | PASS at 8514c0f: 85 passed, 0 failed. `artifacts/20261105_012650_8514c0f/cargo_test.log`, `validate.log` |
| C05 | Run linter and formatting (`cargo clippy -- -D warnings && cargo fmt --check`) | Zero warnings and clean formatting | PASS at 8514c0f: `artifacts/20261105_012650_8514c0f/cargo_clippy.log`, `cargo_fmt.log` |

## Evaluation Report Fields

Record `feature_id`, `code_revision`, `check_id`, `observed_result`, `verdict` (pass/fail/unverified), `evidence_path`, and `reproduction_steps`.

## Completion

All agreed checks pass with evidence against the current code revision.

Status at 8514c0f: complete. All five checks pass with evidence in `artifacts/20261105_012650_8514c0f/`. C03 failed at e9e1e1d and was fixed in repair round 1.

## Repair Policy

At most two repair rounds for this feature before stopping for human review.
