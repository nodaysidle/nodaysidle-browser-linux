---
title: "v0.2.2 release blocked: master already ships the README's unreleased features"
author: Nodaysidle
date: 2026-10-27
branch: review-fixes
cost: { minutes: 3, tokens: 44679 }
touches:
  - AGENTS.md
  - src/theme.rs
  - src/assistant.rs
  - scripts/test-package-appimage.py
  - scripts/package-appimage-bundled.sh
---

## Decided
- Halted the v0.2.2 release before tagging or pushing: master 9d9eae3 already contains the Paper theme, the assistant panel (src/theme.rs, src/assistant.rs), the security fixes and the new shortcuts. An AppImage built from it ships them, so release notes calling them "unreleased" would be false.
- Bumped to 0.2.2 in a separate worktree of master (/tmp/ndi-release, uncommitted). fmt, clippy -D warnings, cargo test --release (84) and scripts/test-package-appimage.py (13) all pass.

## Tried and ruled out
- Building the bundled AppImage on this Arch host: scripts/package-appimage-bundled.sh needs /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1 (Debian/Ubuntu only). Use the appimage-bundled.yml CI workflow.

## Watch out
- Local `master` lags GitHub; fast-forward from origin/master before release work.
- Any release from master needs the owner to decide: list the unreleased features as shipped (and update the README), or release from a commit that leaves them out.
