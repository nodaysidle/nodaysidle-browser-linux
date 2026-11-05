---
title: "Quirks off makes the Chrome User-Agent reach Google, but Google acceptance is still unproven"
author: Nodaysidle
date: 2026-11-04
branch: f01-google-login
cost: { minutes: 2, tokens: 86224 }
id: l-00b7f2b6
scope: repo
status: active
src: agent
touches:
  - src/tabs.rs
---

## Decided
Turned WebKit site-specific quirks off for every view, because the contract requires the Chrome string at accounts.google.com and there is no per-host switch.

## Watch out
This removes WebKit's own quirk for Google sign-in, so forcing Chrome may be what triggers Google's block. No real sign-in has been tried. Pages can still tell it is not Chrome: navigator.vendor is Apple Computer, Inc., and window.chrome and navigator.userAgentData are absent. If quirks-on signs in and quirks-off is blocked, revert this and amend the contract.

In the code: `configure_view` in src/tabs.rs.
