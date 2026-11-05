---
title: "Open error pages follow a theme change by swapping their style text in place, not by reloading"
author: Nodaysidle
date: 2026-11-05
branch: f01-google-login
cost: { minutes: 8, tokens: 154482 }
id: l-af759fab
scope: repo
status: active
src: agent
---

## Decided
On every theme change, restyle open error pages by replacing the text of their style element through JavaScript. A reload would retry the failed address, so a theme switch would cause network traffic and could navigate the tab. Web views are found by walking the GTK toplevels, so there is no registry to keep in sync, and background tabs and pop-ups are covered.

## Tried and ruled out
Using only prefers-color-scheme: it cannot tell two Omarchy themes of the same mode apart. An unguessable style id to stop a site titling itself like an error page: a same-origin pop-up that fails to load can already read those colours, so it adds no real protection.

## Watch out
evaluate_javascript runs in the page's global scope, so a top-level `const` fails with a redeclaration error on the second run. Wrap the script in a block. GTK menus ignore wtype arrows here; hyprctl send_shortcut with a window argument reaches them, which is how Menu → Theme was driven.

In the code: `error_page::follow_theme`, called at the end of `theme::apply`.
