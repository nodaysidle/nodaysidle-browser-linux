---
title: Drive the browser in a nested Hyprland on a temporary headless output so tests never touch the desktop
author: Nodaysidle
date: 2026-11-05
branch: f01-google-login
cost: { minutes: 1, tokens: 1267 }
id: l-7cfb3eb8
scope: repo
status: active
src: agent
touches:
  - src/tabs.rs
  - src/find.rs
---

## Decided
Run UI checks of the app (`src/tabs.rs`, `src/find.rs`) in a nested Hyprland hosted on a temporary headless output of the live session. Input then goes only to the nested compositor, so a misdirected key cannot reach the owner's windows.

## Tried and ruled out
A nested window on a hidden workspace gets no frames, so grim hangs. HYPRLAND_HEADLESS_ONLY is gone in 0.56 and starting Hyprland without a parent crashes in backend creation. `mouse:272` through send_key_state or send_shortcut does not click.

## Watch out
wtype cannot send Return or End; send_shortcut with a window argument can. Pointer hover and clicks need a wlr-virtual-pointer client. Do not run `pkill -f` with a pattern that appears in your own command: it kills the shell. Kill the nested compositor by pid and remove the output afterwards.

In the code: `hyprctl -i 0 output create headless EVALOUT`, then move class aquamarine to its workspace; the helper is artifacts/20261105_004957_e9e1e1d/vptr.c.
