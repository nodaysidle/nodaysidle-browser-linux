---
title: The Chromium renderer sandbox stays on; the no-sandbox flag was removed
author: Nodaysidle
date: 2026-11-08
branch: audit-fixes
cost: { minutes: 1, tokens: 1351 }
id: l-b1747105
scope: repo
status: active
src: agent
touches:
  - src/main.cpp
---

## Decided
The browser no longer passes --no-sandbox to QtWebEngine, because with it every web page's renderer ran with the user's full rights and could run secret-tool and read the vault key. The flag is not needed here because Arch/Omarchy has unprivileged user namespaces on, so the namespace sandbox starts without setup.

## Watch out
When checking the sandbox at runtime, confirm the renderer is a child of this browser, because other QtWebEngine apps on the machine (here, children of claude) run unsandboxed renderers that are easy to mistake for ours. Ours show their own user namespace and Seccomp: 2 in /proc/<pid>/status.
If a machine lacks user namespaces, QTWEBENGINE_DISABLE_SANDBOX=1 is the user's escape hatch; keep the flag out of the code, otherwise every user loses the sandbox for one machine's problem.

In the code: src/main.cpp QTWEBENGINE_CHROMIUM_FLAGS (audit-fixes commit 8c86426).
