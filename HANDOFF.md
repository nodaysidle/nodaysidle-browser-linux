# Handoff

Current feature: F01
Workspace / branch: f01-google-login (not pushed)
Current code revision: 6cc57b4 plus uncommitted repair 1
Working-tree changes: repair 1 in src/tabs.rs and README.md, not committed; diff in artifacts/20261105_001255_6cc57b4-repair1/working-tree.diff. Commit it before evaluating so the verdict has a revision to record.
Feature status: repair 1 ready for re-evaluation; FEATURES.json untouched (F01 still "todo")
Evidence directory: artifacts/20261105_001255_6cc57b4-repair1/ (repair 1), artifacts/20261105_000447_6cc57b4/ (evaluator), artifacts/20261104_235023_5380982/ (builder)
Attempts used / cap: 1 / 2 repair rounds
Runtime permission and spend settings: Standard local runner

## Code changes

- src/profile.rs: new CHROME_LINUX_USER_AGENT const (the PRODUCT.md string); set_accept_policy(CookieAcceptPolicy::Always) in persistent_web_context.
- src/tabs.rs: configure_view sets the User-Agent. Both WebView creation paths use it (tabs, window.open views). gtk_tab_manager_and_web_context now asserts the policy read back from WebKit and the User-Agent on a tab and a window.open view.
- README.md: privacy model discloses third-party cookies and the Chrome User-Agent.

## Check results on the final working tree

| Check | Command | Result | Log |
|---|---|---|---|
| C01 | code inspection, plus User-Agent assertions in the GTK test and on the wire | applied; header received by server | src/profile.rs, src/tabs.rs, e2e_restart.json |
| C02 | code inspection, plus policy read back in the GTK test | Always | src/profile.rs, c03_test.log |
| C03 | NODAYSIDLE_REQUIRE_DISPLAY=1 cargo test -- --test-threads=1 | 84 passed, 0 failed; GTK test ran, not skipped | c03_test.log |
| C04 | cargo clippy -- -D warnings; cargo fmt --check | exit 0 | c04_clippy.log, c04_fmt.log |
| validate | bash scripts/validate.sh | exit 0, 84 passed (release) | validate.log |
| Behavior 2, 3 | python3 artifacts/20261104_235023_5380982/e2e_restart.py target/debug/nodaysidle-browser <out.json> | exit 0 | e2e_restart.log, e2e_restart.json |
| Control, policy line disabled | same script | exit 1: third-party cookie not stored | e2e_control_no_policy.log |

e2e_restart.py launches the real binary twice on a throwaway XDG_DATA_HOME with a private D-Bus session, against a local HTTP server. Seen: Chrome User-Agent on the wire, first-party cookie sent on launch 2, third-party cookie row in cookies.sqlite, database mode 0600.

The e2e_*.attempt* files are earlier runs, kept on purpose. The script first required the third-party cookie to be SENT on launch 2, and that failed: it was stored but defaulted to SameSite=Lax, so a cross-site iframe does not send it. SameSite=None over plain HTTP was then rejected outright. The script's check was changed to stored-in-sqlite, which is what the contract states and which the control run tells apart.

## Unverified

- A real Google sign-in. No account was used, so whether Google accepts this User-Agent on a WebKit engine is unknown. Behavior 1's actual goal needs a manual run.
- A SameSite=None; Secure cookie being sent cross-site after restart (needs HTTPS with a trusted certificate).
- Persistence of cookies across restart when the process is killed by anything other than SIGTERM.

## Risks for the reviewer

- Third-party cookies are accepted everywhere, not only during OAuth. Cross-site tracking cookies are now stored too; the contract requires this.
- The User-Agent is global and pinned at Chrome 131, so sites see Chrome on all pages and the version will age.

## Evaluator failure (2026-11-05, 6cc57b4)

C01-C04 pass as worded, but C01 is inspection only. Behavior 1 fails at runtime: `accounts.google.com` does not receive the Chrome User-Agent.

Reproduce:

1. `cargo build`
2. `python3 artifacts/20261105_000447_6cc57b4/eval_ua_by_host.py target/debug/nodaysidle-browser out.json example.com accounts.google.com`
3. Expected on both hosts: `... AppleWebKit/537.36 ... Chrome/131.0.0.0 Safari/537.36`
4. Observed on `example.com`: that string. Observed on `accounts.google.com`: `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15`

`docs.google.com` also differs (`Chrome/300.0.0.0` on AppleWebKit/605.1.15). `mail`, `myaccount`, `www.google.com`, `accounts.youtube.com` and `login.microsoftonline.com` get the Chrome string. The probe uses plain http through a local proxy; https was not intercepted.

Cause: WebKit's site-specific quirks override the custom User-Agent per host, and `configure_view` (`src/tabs.rs:1373`) leaves them on. The GTK test reads the setting back, so it cannot see this. A throwaway copy with `settings.set_enable_site_specific_quirks(false)` sent the Chrome string to both hosts (artifacts/20261105_000447_6cc57b4/diag_quirks_off.log).

## Repair 1 (2026-11-05)

Change: `settings.set_enable_site_specific_quirks(false)` in `configure_view` (`src/tabs.rs`), which covers both WebView paths. `assert_chrome_user_agent` in the GTK test now also asserts quirks are off. README privacy model discloses it. CONTRACT.md: stale revision line fixed, C05 added as the wire check; no existing check changed. The contract State line was left as is.

This took the candidate repair without the manual sign-in the section below asked for, because the contract requires the Chrome string at `accounts.google.com`. That sign-in is still open.

| Check | Command | Result | Log |
|---|---|---|---|
| C05, Behavior 1 string | python3 artifacts/20261105_000447_6cc57b4/eval_ua_by_host.py target/debug/nodaysidle-browser <out.json> example.com accounts.google.com docs.google.com mail.google.com myaccount.google.com www.google.com accounts.youtube.com login.microsoftonline.com | all 8 hosts get the Chrome string, on the wire and in navigator.userAgent | artifacts/20261105_001255_6cc57b4-repair1/ua_by_host.log |
| C03 | NODAYSIDLE_REQUIRE_DISPLAY=1 cargo test -- --test-threads=1 | 84 passed, 0 failed | artifacts/20261105_001255_6cc57b4-repair1/c03_test.log |
| C04 | cargo clippy -- -D warnings; cargo fmt --check | exit 0 | artifacts/20261105_001255_6cc57b4-repair1/c04_clippy.log, c04_fmt.log |
| Behavior 2, 3 | python3 artifacts/20261104_235023_5380982/e2e_restart.py target/debug/nodaysidle-browser <out.json> | exit 0 | artifacts/20261105_001255_6cc57b4-repair1/e2e_restart.log |
| validate | bash scripts/validate.sh | exit 0, 84 passed | artifacts/20261105_001255_6cc57b4-repair1/validate.log |

The probe is plain http through a local proxy, so https was not intercepted. All runs predate the README line, which changes no code.

Still unverified, and the main risk: whether Google accepts this. WebKit shipped that quirk for `accounts.google.com`, and it was removed here. The same probe shows the page can still tell this is not Chrome: `navigator.vendor` is `Apple Computer, Inc.`, and `window.chrome` and `navigator.userAgentData` are absent. Other sites that relied on a quirk are also untested.

## Open question for the human

WebKit's quirk may exist so that Google sign-in works, so forcing Chrome could be what triggers the block. Decide with one manual sign-in on the build with quirks on (6cc57b4) and one on repair 1: if quirks-on works, amend Behavior 1 to accept WebKit's quirk User-Agent on that host. Repair 1 already turned quirks off, which disables every quirk on every site, so that build needs this sign-in test. If quirks-on works and quirks-off is blocked, revert repair 1 and amend Behavior 1.

Contract cleanup still needed: State still says Proposed. roles/EVALUATOR.md describes F01 as saving a named revision, which does not match this product.

## Next action

Commit repair 1, then run the evaluator against C01-C05. The human still owes one real Google sign-in; Behavior 1's bypass stays UNVERIFIED until then.
