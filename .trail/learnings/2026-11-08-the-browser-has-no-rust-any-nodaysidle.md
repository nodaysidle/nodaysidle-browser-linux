---
title: "The browser has no Rust any more, and every doc that names the stack must say so in the same change"
author: Nodaysidle
date: 2026-11-08
branch: remove-rust-launcher
cost: { minutes: 10, tokens: 73859 }
id: l-704f0854
scope: repo
status: active
src: agent
touches:
  - CLAUDE.md
  - HANDOFF.md
  - dev/CONTRACT.md
  - dev/PRODUCT.md
  - dev/harness.py
  - README.md
  - scripts/validate.sh
---

## Decided
The cargo launcher wrapper was removed because the app is C++17 + Qt 6 Quick + QtWebEngine and the wrapper only exec'd the Qt binary; bin/run, bin/install and the Arch package already start the binary directly. The validate script is now build plus qmllint, because there is nothing left for cargo to check. When the stack or the commands change, the owner wants CLAUDE.md, HANDOFF.md, the dev harness docs and the vault pages fixed in the same task without asking, because stale stack notes misled the agent in this session and would mislead the next one.

## Tried and ruled out
Listing the stale docs as follow-ups for the owner: rejected by the owner, since it leaves the next agent reading Rust instructions for a C++ repo.

## Watch out
The F01 and F02 PASS records were evaluated on the old Rust + GTK app (F02 at 8514c0f), so they say nothing about the Qt code; write F03's contract against the Qt app. Release notes under docs/releases describe their own era and stay as they are. Anything mentioning cargo, GTK, WebKitGTK or launcher/ describes the pre-v0.3.0 app.

In the code: launcher/ deleted in 7b84561; scripts/validate.sh = ./bin/build + qmllint; docs aligned in 7ead365 (CLAUDE.md, HANDOFF.md, dev/PRODUCT.md, dev/CONTRACT.md note, dev/harness.py evaluator prompt).
