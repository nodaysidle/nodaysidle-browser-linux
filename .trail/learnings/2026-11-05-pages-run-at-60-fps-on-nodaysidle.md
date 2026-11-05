---
title: "Pages run at 60 fps on a 165 Hz screen and there is no flag to fix it, so leave it"
author: Nodaysidle
date: 2026-11-05
branch: native-qt6-wayland
cost: { minutes: 6, tokens: 142870 }
id: l-cbb0fd78
scope: repo
status: active
src: agent
touches:
  - LICENSE
  - README.md
  - bin/install
  - pkgbuild/PKGBUILD
  - src/Main.qml
  - src/main.cpp
  - CLAUDE.md
  - HANDOFF.md
  - nodaysidle-browser.pro
  - src/adblock.h
---

## Decided
Accept the 60 fps page cap. The package is the install path for the Omarchy launcher, built from the working tree.

## Tried and ruled out
Disabling the frame-rate limit removes the cap but runs requestAnimationFrame at about 1900 fps with no vsync, so it is not a fix. Disabling GPU vsync as well made no difference. No Chromium switch in the shipped engine sets the display refresh rate.

## Watch out
Test scripts that pkill or pgrep by binary path match their own shell command line and kill the shell (exit 144). Start the app from a script file and kill by pid.

In the code:
- pkgbuild/PKGBUILD builds from the checkout via $startdir/.., bin/install runs makepkg -fsi (needs a sudo terminal)
- src/main.cpp appends QTWEBENGINE_CHROMIUM_FLAGS from the environment to the default GPU flags
