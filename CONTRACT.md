# Feature Contract: F01 — Persistent Google Login & Auth Session Support

Feature: F01 — Persistent Google Login & Auth Session Support
State: Proposed; agree before implementation
Code revision under evaluation: 6cc57b4 plus uncommitted repair 1 (see HANDOFF.md)

## Behavior

1. When navigating to Google Accounts or OAuth login endpoints (`accounts.google.com`), WebKit sends a standard Chrome/Linux desktop User-Agent string rather than the default WebKit identifier, bypassing Google's "This browser or app may not be secure" block.
2. The WebKit CookieManager accept policy is set to `CookieAcceptPolicy::Always` so third-party OAuth redirect cookies and session tokens are preserved in `cookies.sqlite`.
3. Cookies persist between browser launches: logging in or setting a test cookie in one session persists when restarting the application.

## Checks

| ID | Action | Expected observation | Evidence |
|---|---|---|---|
| C01 | Inspect WebKit settings user-agent in `profile.rs` / `main.rs` | Standard Chrome/Linux User-Agent string is applied to WebKit settings | PASS as worded: Chrome/Linux string applied via `set_user_agent` in `src/tabs.rs:1378` on both WebView paths, const in `src/profile.rs` (not set in `main.rs`). Inspection only, so it does not cover Behavior 1, which FAILED. See Evaluation Result. |
| C02 | Check CookieManager configuration in `profile.rs` | `cookie_manager.set_accept_policy(CookieAcceptPolicy::Always)` is configured | PASS: `src/profile.rs:58`; read back as Always by the GTK test; third-party rows `tp1`-`tp4|localhost` in cookies.sqlite. `artifacts/20261105_000447_6cc57b4/eval_persistence.log` |
| C03 | Run Rust test suite (`cargo test -- --test-threads=1`) | All unit and GTK tests pass (84+ passing, 0 failing) | PASS: 84 passed, 0 failed, with and without NODAYSIDLE_REQUIRE_DISPLAY=1. `artifacts/20261105_000447_6cc57b4/c03_test_contract_cmd.log`, `artifacts/20261105_000447_6cc57b4/c03_test_require_display.log` |
| C04 | Run clippy (`cargo clippy -- -D warnings`) and formatter (`cargo fmt --check`) | Zero warnings or formatting issues | PASS: clippy exit 0, fmt exit 0, empty output. `artifacts/20261105_000447_6cc57b4/c04_clippy.log`, `artifacts/20261105_000447_6cc57b4/c04_fmt.log` |
| C05 | Run `eval_ua_by_host.py` against the real binary for `example.com` and `accounts.google.com` | Both hosts receive the exact Chrome/Linux string from PRODUCT.md, on the wire and in `navigator.userAgent` | Unevaluated. Builder run: `artifacts/20261105_001255_6cc57b4-repair1/ua_by_host.log` |

## Evaluation Report Fields

Record `feature_id`, `code_revision`, `check_id`, `observed_result`, `verdict` (pass/fail/unverified), `evidence_path`, and `reproduction_steps`. State whether checks were verified via automated unit/integration tests or manual runtime inspection.

## Completion

All agreed checks pass with evidence against the current code revision. If code changes during evaluation, rerun affected checks.

## Evaluation Result

Evaluated revision: 6cc57b4 (the `Code revision under evaluation: 6cc57b4 plus uncommitted repair 1 (see HANDOFF.md)` line above is stale). Date: 2026-11-05. Method: automated tests plus manual runtime inspection of the real binary. Full log: `artifacts/20261105_000447_6cc57b4/eval_evidence.txt`.

**Verdict: NOT COMPLETE. Behavior 1 FAILED.** All four table checks pass, but C01 only inspects the setting and cannot see what goes on the wire.

| Behavior | Verdict | Observed |
|---|---|---|
| 1. Chrome/Linux User-Agent at `accounts.google.com` | FAIL | `accounts.google.com` receives `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15`, on the wire and in `navigator.userAgent` |
| 1. Google's block is bypassed | UNVERIFIED | No real sign-in attempted |
| 2. Accept policy Always, third-party cookies stored | PASS | See C02. Storing is proven; sending a `SameSite=None; Secure` cookie cross-site over HTTPS is unverified |
| 3. Cookies persist across launches | PASS | Persistent cookies and localStorage survived SIGTERM and SIGKILL restarts; session cookies did not, as expected. Closing the window was not tested |

Reproduce: `python3 artifacts/20261105_000447_6cc57b4/eval_ua_by_host.py target/debug/nodaysidle-browser out.json example.com accounts.google.com`

Cause: WebKit's site-specific quirks replace the custom User-Agent per host. `configure_view` leaves them on. A throwaway copy with `set_enable_site_specific_quirks(false)` sent the Chrome string (`artifacts/20261105_000447_6cc57b4/diag_quirks_off.log`).

## Repair Policy

At most two repair rounds for this feature before a human reviews the blocker.
