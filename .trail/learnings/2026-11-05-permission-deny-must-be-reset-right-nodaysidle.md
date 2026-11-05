---
title: "Permission Deny must be reset right away, because Qt stores it and the site is then stuck"
author: Nodaysidle
date: 2026-11-05
branch: native-qt6-wayland
cost: { minutes: 6, tokens: 51412 }
id: l-391a7195
scope: repo
status: active
src: agent
touches:
  - CLAUDE.md
  - HANDOFF.md
  - README.md
  - nodaysidle-browser.pro
  - src/Main.qml
  - src/adblock.h
  - src/backend.cpp
  - src/backend.h
  - src/main.cpp
---

## Decided
- After a Deny, reset the permission so the site can ask again; keep Allow in memory only. Location requests are refused without a prompt because QtWebEngine has no location service key.
- The ad blocker only drops sub-resources, and skips requests where the page itself is on the blocked host.

In the code:
- src/Main.qml (answerPermission, onPermissionRequested), src/main.cpp (StoreInMemory policy), src/adblock.h.

## Tried and ruled out
- Leaving the default permission policy: a Deny was kept and the page got "denied" forever with no way to undo it.

## Watch out
- PermissionType is an enum class, so QML needs WebEnginePermission.PermissionType.Geolocation, not WebEnginePermission.Geolocation.
- The request interceptor runs off the UI thread; keep its host list read-only and its counter atomic.
