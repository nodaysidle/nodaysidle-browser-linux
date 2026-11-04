# Feature Contract: F01 — Persistent Google Login & Auth Session Support

Feature: F01 — Persistent Google Login & Auth Session Support
State: Proposed; agree before implementation
Code revision under evaluation: 5aee40a

## Behavior

1. When navigating to Google Accounts or OAuth login endpoints (`accounts.google.com`), WebKit sends a standard Chrome/Linux desktop User-Agent string rather than the default WebKit identifier, bypassing Google's "This browser or app may not be secure" block.
2. The WebKit CookieManager accept policy is set to `CookieAcceptPolicy::Always` so third-party OAuth redirect cookies and session tokens are preserved in `cookies.sqlite`.
3. Cookies persist between browser launches: logging in or setting a test cookie in one session persists when restarting the application.

## Checks

| ID | Action | Expected observation | Evidence |
|---|---|---|---|
| C01 | Inspect WebKit settings user-agent in `profile.rs` / `main.rs` | Standard Chrome/Linux User-Agent string is applied to WebKit settings | PASS: Chrome/Linux string applied via `set_user_agent` in `src/tabs.rs:1378` on both WebView paths (`src/tabs.rs:525`, `src/tabs.rs:754`), const in `src/profile.rs:10`, not in `main.rs`. Read back by the GTK test; seen on the wire in C05. `artifacts/20261105_001702_5aee40a/eval_evidence.txt` |
| C02 | Check CookieManager configuration in `profile.rs` | `cookie_manager.set_accept_policy(CookieAcceptPolicy::Always)` is configured | PASS: `src/profile.rs:58`; read back as Always by the GTK test; third-party row `third|localhost` in cookies.sqlite. `artifacts/20261105_001702_5aee40a/e2e_restart.log` |
| C03 | Run Rust test suite (`cargo test -- --test-threads=1`) | All unit and GTK tests pass (84+ passing, 0 failing) | PASS: 84 passed, 0 failed, with and without NODAYSIDLE_REQUIRE_DISPLAY=1; GTK test ran. `artifacts/20261105_001702_5aee40a/c03_test_contract_cmd.log`, `artifacts/20261105_001702_5aee40a/c03_test_require_display.log` |
| C04 | Run clippy (`cargo clippy -- -D warnings`) and formatter (`cargo fmt --check`) | Zero warnings or formatting issues | PASS: clippy exit 0, fmt exit 0, no output. `artifacts/20261105_001702_5aee40a/c04_clippy.log`, `artifacts/20261105_001702_5aee40a/c04_fmt.log` |
| C05 | Run `eval_ua_by_host.py` against the real binary for `example.com` and `accounts.google.com` | Both hosts receive the exact Chrome/Linux string from PRODUCT.md, on the wire and in `navigator.userAgent` | PASS: exact string on all 8 hosts probed, including both named hosts, on the wire and in `navigator.userAgent`. Plain http through a local proxy. `artifacts/20261105_001702_5aee40a/eval_ua_by_host.log` |

## Evaluation Report Fields

Record `feature_id`, `code_revision`, `check_id`, `observed_result`, `verdict` (pass/fail/unverified), `evidence_path`, and `reproduction_steps`. State whether checks were verified via automated unit/integration tests or manual runtime inspection.

## Completion

All agreed checks pass with evidence against the current code revision. If code changes during evaluation, rerun affected checks.

## Evaluation Result

Evaluated revision: 5aee40a. Date: 2026-11-05. Method: automated tests plus runtime runs of the real binary. Full log: `artifacts/20261105_001702_5aee40a/eval_evidence.txt`.

**Verdict: all five checks PASS, so F01 meets the Completion rule. One part of Behavior 1 is UNVERIFIED: no check tests whether Google accepts the sign-in.**

| Behavior | Verdict | Observed |
|---|---|---|
| 1. Chrome/Linux User-Agent at `accounts.google.com` | PASS | Exact string on the wire and in `navigator.userAgent`. The 6cc57b4 failure is fixed |
| 1. Google's block is bypassed | UNVERIFIED | No real sign-in attempted. Quirks are off, and pages can still tell this is not Chrome: `navigator.vendor` is `Apple Computer, Inc.`, with no `window.chrome`, `navigator.userAgentData` or `Sec-CH-UA` |
| 2. Accept policy Always, third-party cookies stored | PASS | Third-party row in cookies.sqlite, database mode 0600. Sending a `SameSite=None; Secure` cookie cross-site over HTTPS is unverified |
| 3. Cookies persist across launches | PASS | After a real window close and reopen, the exact random cookie value was resent and the exact localStorage value was read back (`artifacts/20261105_001702_5aee40a/eval_window_close.log`). Also survived SIGTERM and SIGKILL |

Reproduce: `python3 artifacts/20261105_000447_6cc57b4/eval_ua_by_host.py target/debug/nodaysidle-browser out.json example.com accounts.google.com`, and `python3 artifacts/20261105_001702_5aee40a/eval_window_close.py target/debug/nodaysidle-browser out.json` (needs Hyprland).

Prior evaluation of 6cc57b4 (Behavior 1 FAILED): `artifacts/20261105_000447_6cc57b4/eval_evidence.txt`.

Returned to the human: the State line still says Proposed, C05 was added by the builder during repair 1, and `roles/EVALUATOR.md` describes F01 as saving a named revision, which does not match this product.

## Repair Policy

At most two repair rounds for this feature before a human reviews the blocker.
