# Handoff

Current feature: none active. F02 — Deterministic UI Design and Navigation Polish is done.
Workspace / branch: f01-google-login
Current code revision: 8514c0f (CONTRACT.md, FEATURES.json and HANDOFF.md are uncommitted; artifacts/ is git-ignored, so the evidence stays local)
Evaluation: PASS at 8514c0f. C01–C05 all pass, recorded by the independent Evaluator.
Evidence directory: artifacts/20261105_012650_8514c0f/ (`eval_evidence.txt`, `*.log`, `screens/`, `helpers/`)
Attempts used / cap: 1 / 2 repair rounds
FEATURES.json: F02 `done`, code_revision 8514c0f0d7e3a973844a3541aadce3186d172912.

## Open items for the owner

Observations O1–O9 in `eval_evidence.txt` are not contract failures. Worth deciding:

- O3: the selected tab always shows its close button, while the contract says it appears on hover.
- O7: a page titled exactly like an error page with a style element using `nodaysidle-error-style` has it replaced on a theme change and can read the palette.
- O8: a virtual-pointer click on a Theme submenu item did not activate it; Return did. Harness or product is undetermined, so test a real mouse click.
- O9: with two windows, Ctrl+Q closed only the focused window. Not checked against intended behaviour.

Final release judgment stays with the human.

## How to drive the app without touching the desktop

Nested Hyprland hosted on a temporary headless output, because a nested window on a hidden workspace gets no frames:

- `hyprctl -i 0 output create headless EVALOUT`, start `Hyprland -c <config>` with `WAYLAND_DISPLAY=wayland-1`, then `hyprctl -i 0 dispatch "hl.dsp.window.move({workspace='<id>', follow=false, window='class:aquamarine'})"`.
- `wtype` cannot send Return or End, or arrows into GTK menus; `hl.dsp.send_shortcut({mods='', key='Return', window='class:com.nodaysidle.Browser'})` can.
- Real pointer: `helpers/vptr.c` in the evidence directory. Hyprland `mouse:272` injection does not click.
- A launched `setsid nohup` job's `$!` is not the Hyprland pid. Take the pid from `hyprctl instances`.
- Stop the app with Ctrl+Q inside the nested session, not by process name, which could hit the owner's own browser.
- Remove the output afterwards.

## Next action

Owner review of F02, then commit the harness files. F03 (Tab Session State Restore on Restart) is next and has no contract yet.
