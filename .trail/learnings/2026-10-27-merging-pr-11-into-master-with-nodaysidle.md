---
title: "Merging PR #11 into master with a head-locked merge commit"
author: Nodaysidle
date: 2026-10-27
branch: review-fixes
cost: { minutes: 1, tokens: 45438 }
touches:
  - AGENTS.md
---

## Decided
PR #11 merged to master as a merge commit (9d9eae3) with `gh pr merge 11 --merge --match-head-commit <sha>`, so the merge fails if the head moved after verification. Approval quotes went verbatim into --body.

## Tried and ruled out
`gh api .../commits/<short-sha>` returns an array, not an object; pass the full 40-char sha to get the single commit.

## Watch out
Local master was not pulled after the merge; run `git fetch` before branching from master.
