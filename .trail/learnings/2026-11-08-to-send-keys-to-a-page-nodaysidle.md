---
title: "To send keys to a page in the Qt browser, open and close the find bar first"
author: Nodaysidle
date: 2026-11-08
branch: audit-fixes
cost: { minutes: 3, tokens: 31210 }
id: l-cae3967a
scope: repo
status: active
src: agent
touches:
  - src/Main.qml
---

## Decided
In UI checks, send page key presses with wtype only after Ctrl+F then Escape, because closing the find bar calls forceActiveFocus on the web view; otherwise keys go to the QML toolbar and the page never sees them. A key press sent this way counts as a user gesture, so it can test things like the popup blocker.

## Tried and ruled out
Plain wtype keys right after launch: the page's keydown handler never fired (its title did not change). A popup opened from a file:// page fails with ERR_NETWORK_ACCESS_DENIED, so serve test pages with python -m http.server on 127.0.0.1 instead.

## Watch out
Check that the active window's pid is the browser before every wtype call, otherwise the keys land in the owner's windows.

In the code: src/Main.qml closeFind (v.forceActiveFocus), settings.javascriptCanOpenWindows.
