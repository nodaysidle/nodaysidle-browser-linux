---
title: Opening the review-fixes PR from a fresh clone
author: Nodaysidle
date: 2026-10-26
branch: review-fixes
cost: { minutes: 4, tokens: 67783 }
touches:
  - AGENTS.md
---

## Decided
- Clone into ~/.local/state/nodaysidle/work/<name> from the local checkout, then repoint origin to https://github.com/nodaysidle/nodaysidle-browser-linux.git so fetch/ahead-behind/push hit GitHub, not the local path.
- Push with `git push origin review-fixes:refs/heads/nds/<name>`; PR #11 opened from that branch (head fae6771).

## Tried and ruled out
- `trail check` with no args in the clone: it compared against the stale clone-time `origin/review-fixes` ("nothing to check"). Use `trail check --base origin/master .`.
- Its 17 "conflicts" on this branch are keyword overlaps (wtype, deepseek, GTK theme warning) with testing/health-check learnings, not real regressions; say so in the PR body.

## Watch out
- Fish is the login shell but the Bash tool runs bash: use ${PIPESTATUS[0]}, not $pipestatus.
- When the task says "end the body with X", drop the default Claude Code PR footer.
