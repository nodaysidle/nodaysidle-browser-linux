---
title: Releasing v0.2.2 from master via the bundled AppImage CI
author: Nodaysidle
date: 2026-10-27
branch: review-fixes
cost: { minutes: 13, tokens: 153429 }
touches:
  - AGENTS.md
  - README.md
---

## Decided
- v0.2.2 released from master (commit 7478985, annotated tag v0.2.2) after the owner approved listing the Paper theme, the assistant panel, the security fixes and the new shortcuts as shipped. The README's Unreleased section was merged into the main sections.
- README links to releases/latest/download/nodaysidle-browser-bundled-x86_64.AppImage and to the .sha256 and SHA256SUMS assets instead of an inline hash, because the AppImage is only built after the tag exists.
- The AppImage comes from appimage-bundled.yml, started with `gh workflow run appimage-bundled.yml --ref v0.2.2`. The workflow only runs on workflow_dispatch or pull_request and does not publish a release. Download the artifact with `gh run download`, add SHA256SUMS, then `gh release create --verify-tag --latest`.

## Tried and ruled out
- Building the bundled AppImage locally on Arch: the packaging script needs Debian paths. Use CI.

## Watch out
- `trail check` flags README lines that mention "Tab" keys or "API"/"PATH" as conflicts with unrelated tooling learnings (wtype, deepseek). These are false positives.
- Run `trail check` as its own step before committing, not chained with `;` before the commit.
