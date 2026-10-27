---
title: Local checkout drifted 9 commits behind GitHub master
author: Nodaysidle
date: 2026-10-26
branch: review-fixes
cost: { minutes: 6, tokens: 56162 }
touches:
  - AGENTS.md
  - src/main.rs
  - src/assistant.rs
  - README.md
  - Cargo.lock
  - Cargo.toml
  - src/shortcuts.rs
  - src/tabs.rs
  - src/theme.rs
  - src/error_page.rs
---

## Decided
- review-fixes was rebased onto origin/master (050242b, v0.2.1 + PR #10) with `git rebase --onto origin/master <snapshot-commit>`; the old base is kept as local branch review-fixes-old-base.
- The one local-only change (WEBKIT_DISABLE_DMABUF_RENDERER=1 workaround for an Intel/Wayland teardown segfault, src/main.rs) was not on GitHub and was kept as its own commit.

## Tried and ruled out
- Merging review-fixes into origin/master directly: 13 conflicting files, because the old uncommitted work was the same code as PRs #3/#4 under different commits.

## Watch out
- Local `master` here sat at a813e70 (2026-10-07) while GitHub had 9 more commits; uncommitted files looked like new work but were PR #3/#4. Run `git fetch && git rev-list --left-right --count master...origin/master` before starting.
- The vault (nodaysidle-knowledge) records the public state: README snapshot, catalog card, briefs. App facts there beat this checkout.
