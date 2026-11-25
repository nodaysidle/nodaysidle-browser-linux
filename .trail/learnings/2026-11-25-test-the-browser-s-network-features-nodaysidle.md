---
title: "Test the browser's network features with a small program built on the backend, not by typing into the live desktop"
author: Nodaysidle
date: 2026-11-25
branch: master
cost: { minutes: 13, tokens: 54176 }
id: l-6baed130
scope: repo
status: active
src: agent
touches:
  - src/backend.cpp
  - src/Main.qml
  - scripts/validate.sh
  - README.md
  - docs/releases/v0.4.0.md
  - pkgbuild/PKGBUILD
  - src/main.cpp
---

## Decided
- To check a backend feature that talks to a web service (`Backend::askPage`, `Backend::checkScam` in `src/backend.cpp`), build a throwaway program in a scratch folder that links the freshly built backend object and its moc object, calls the method directly and prints the signals. It runs offscreen with scratch data folders and exercises the real request, streaming and error paths in seconds.
- New learnings must be added with `git add -f`: a user-level git ignore hides the `.trail/` folder, although the learnings already in the repo are tracked.
- Pushing master or a tag is blocked by a pre-push hook until the owner has said yes in the session; the hook names its own override, used only after that yes.
- A release is built from the PKGBUILD with makepkg (no install), and the tarball is made from the files inside that package plus LICENSE, so both downloads carry the same binary.
- The version lives in two places, `pkgver` in `pkgbuild/PKGBUILD` and `setApplicationVersion` in `src/main.cpp`; bump both.
- Release notes go in `docs/releases/vX.Y.Z.md` in the New / Changed / Fixed shape and are the GitHub release body. The README badge, download links, install commands and size rows point at the new version in the same commit.
- Tags from v0.4.0 on are annotated.

## Tried and ruled out
- Driving the panel in `src/Main.qml` with simulated key presses on the owner's live Hyprland session: a floating System Monitor terminal took focus each time the workspace changed, so the focus guard aborted every run before any key was sent, and the test windows landed next to the owner's terminal. Launching through a Hyprland exec rule with env variables did not start the browser at all.

## Watch out
- Always check the focused window's pid before any simulated key press; without that guard the keys would have gone into the owner's terminal.
- `pkill -f` with the binary's path kills the shell running the command, since that command line contains the same path.

In the code: link `<build>/backend.o` and `moc_backend.o` (from `bin/build` or `scripts/validate.sh`) with `pkg-config --cflags --libs Qt6Gui Qt6Network Qt6Core`; run with QT_QPA_PLATFORM=offscreen and scratch XDG_DATA_HOME / XDG_CONFIG_HOME; force-add `.trail/learnings/*.md`.
- Start time, CPU, memory and frame rate in the README were measured on v0.3.0 and not since; say so rather than presenting them as current.
- Pushing master or a tag needs the owner's yes (pre-push hook). After publishing, download the assets back and run `sha256sum -c SHA256SUMS`.

In the code: `cd pkgbuild && makepkg -f --noconfirm`; tarball folder `nodaysidle-browser-X.Y.Z-linux-x86_64/` with `nodaysidle-browser`, `com.nodaysidle.Browser.desktop`, `nodaysidle-browser.svg`, `LICENSE`, `nodaysidle-browser.fish`; artifacts kept in `dist/vX.Y.Z/` (gitignored); `gh release create vX.Y.Z <pkg> <tar.gz> SHA256SUMS --notes-file docs/releases/vX.Y.Z.md --verify-tag --latest`.
