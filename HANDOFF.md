# Handoff

Current feature: F02 — Deterministic UI Design and Navigation Polish
Workspace / branch: f01-google-login
Current code revision: 55eace2 (F02 changes below are UNCOMMITTED on top of it; no commit was requested)
Working-tree changes: src/error_page.rs, src/find.rs, src/home.rs, src/tabs.rs, src/theme.rs, HANDOFF.md
Last passing checks (Builder's own run on the working tree, not an evaluation):
- `cargo check`, `cargo build`: exit 0
- `cargo test -- --test-threads=1`: 85 passed, 0 failed
- `cargo clippy -- -D warnings`, `cargo fmt --check`: exit 0
- `bash scripts/validate.sh`: exit 0 (85 passed in release mode)
Failed / blocked checks: none
Evidence directory: artifacts/20261105_003907_55eace2/ (`*.log`, `f02.diff`, `git_revision.txt`, `screens/`)
Attempts used / cap: 0 / 2 repair rounds
Runtime permission and spend settings: Standard local runner

## Code changes

- `src/theme.rs`: tab pills use a 4px top radius and a flush 2px bottom border (`@accent` when selected, transparent otherwise, so selection cannot shift layout). Close button is hidden with opacity, so its slot stays reserved. Added the `placeholder_text_color` token to all palettes, a neutral `.url-bar image.left` colour, `.find-entry.find-found`, and a focus ring and font size for the find entry. New `palette_colour()` reads tokens from the active palette.
- `src/error_page.rs`: hardcoded hex removed; colours come from `palette_colour()`, so Omarchy themes apply.
- `src/tabs.rs`: address bar always has a primary icon via `show_security` (lock, warning, or neutral search), so the text does not shift; insecure icon is now `dialog-warning-symbolic`. Placeholder removed (GTK 3 drops it on focus) and replaced by an accessible name. Pill labels have a fixed width. A `hover` class is set on pills from enter/leave events. Esc in the address bar restores the page's security icon.
- `src/home.rs`: placeholder removed; the permanent hint stays and the entry has an accessible name.
- `src/find.rs`: find bar sits in a `gtk::Revealer` (SlideDown, 120 ms); `FindBar::is_open()` replaces `bar.is_visible()`; adds the `find-found` state.
- Tests added: `palette_tokens_are_read_by_their_exact_name`, plus `verify_chrome_styling` and `verify_address_bar_status` inside the GTK test.

## Check status (Builder observations; verdicts belong to the Evaluator)

| ID | Evidence |
|---|---|
| C01 | `screens/04_https_secure_tab.png`, `05_http_insecure_tab.png`, `11_montage.png`; `verify_address_bar_status` |
| C02 | `screens/09_montage.png` (no hover / hover / left, no layout shift), `04`, `05`; `verify_chrome_styling` |
| C03 | `screens/01_error_page_third_tab.png` and `07_home_page.png` under Omarchy theme nodaysidle-miasma |
| C04 | `cargo_test.log`, `validate.log` |
| C05 | `cargo_clippy.log`, `cargo_fmt.log` |

## Not verified

- Paper and built-in Dark were not checked visually. All screenshots use the live Omarchy theme; GTK tests assert Dark and Paper computed colours.
- The slide animation was not observed frame by frame. `06a_find_opening_mid.png` was taken about 50 ms in, is not a proven mid-transition frame, and predates the find-entry restyle (see `10_find_refined.png`).
- Clicking a real close button with the pointer was not done: the GTK test uses `button.clicked()`. Non-activation rests on the button being a sibling of the row that handles selection.
- Shortcut responsiveness: Ctrl+L, Ctrl+1/2, Ctrl+F, Ctrl+T and Ctrl+Q were driven live; Alt+D, Ctrl+R and Alt+Left/Right were not exercised in this run.

## Known limits

- WebKit paints the in-page match highlight (yellow) and it cannot be themed from GTK. "Distinct highlight styling" is that highlight plus the entry state colours (`find-found` / `find-none`).
- A lone Home tab has no close button, so its pill gains about 20 px after the first page load. Left as is.

## Open questions for the owner

- CONTRACT.md still records revision 5ef8424, and evaluation needs a commit. F02 must be committed before the Evaluator can record a revision.
- 55eace2 switched the User-Agent to Firefox, but PRODUCT.md and README.md still say Chrome and the constant is still `CHROME_LINUX_USER_AGENT`. Outside F02; not touched.

## Next action

Commit the F02 working tree, then run the EVALUATOR against CONTRACT.md with the evidence above. FEATURES.json is unchanged (F02 stays `in_progress`).

The app is a desktop binary with no URL. To run it without touching the real profile: `XDG_DATA_HOME=<scratch dir> cargo run`.
