---
title: "Saved logins live in a keyring-keyed vault, and tabs must never re-load their own URL"
author: Nodaysidle
date: 2026-11-05
branch: native-qt6-wayland
cost: { minutes: 23, tokens: 300092 }
id: l-7b81da55
scope: repo
status: active
src: agent
touches:
  - CLAUDE.md
  - README.md
  - nodaysidle-browser.pro
  - pkgbuild/PKGBUILD
  - src/IconButton.qml
  - src/Main.qml
  - src/PasswordsPanel.qml
  - src/backend.cpp
  - src/backend.h
  - src/importer.cpp
---

## Decided
Saved logins are AES-256-GCM encrypted; the key sits in the Secret Service keyring, so the app stores nothing readable and says so when no keyring exists. The login agent runs in an isolated page world and talks to the browser through console lines that start with a per-run nonce.

## Tried and ruled out
Binding a tab's url to its model url: every URL change re-navigated, so POST result pages (a wrong-password page) were fetched again as GET. Load the first address once instead. Passing the fill arguments as one JSON array to the fill function put "user,pass" into the username field; apply them as separate arguments.

## Watch out
Virtual-pointer tests on the real session must check the active workspace before every click: the user switched away mid-run and clicks hit their own windows. Use the nested session for clicks. The file dialog of a nested app opens on the real desktop through the portal, so test CSV import with a headless harness.

In the code:
- src/vault.cpp, src/login.js, src/importer.cpp
- src/Main.qml: Component.onCompleted: url = model.url in the tab view
