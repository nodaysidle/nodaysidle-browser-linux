---
title: "Tab pill hover needs enter and leave handlers, and a compositor cursor warp inside the window does not test hover"
author: Nodaysidle
date: 2026-11-04
branch: f01-google-login
cost: { minutes: 9, tokens: 198259 }
id: l-786fbc4e
scope: repo
status: active
src: agent
touches:
  - HANDOFF.md
  - src/tabs.rs
  - src/theme.rs
---

## Decided
Pills get a `hover` style class from enter/leave handlers on their two windowed children (the title ListBox and the close button), because CSS `:hover` on a GTK 3 Box never matched in the running app.

## Tried and ruled out
`.tab-pill:hover .tab-close` in CSS: entering the window from outside over a pill showed the tooltip but no close button.

## Watch out
Hyprland cursor moves between two points inside the same window deliver no pointer events to GTK, so hover looks broken even when the code is right. Move the cursor outside the window first, then onto the target.
A file-level `opacity` on `.tab-close` is not reported by `WidgetExt::opacity`; read it with `style_property_for_state("opacity", ...)`. `border-top-left-radius` cannot be read that way, but `border-radius` can.

In the code: `open_tab` in src/tabs.rs, `verify_chrome_styling` in src/theme.rs. Cursor move: `hyprctl dispatch "hl.dsp.cursor.move({x=..,y=..})"`.
