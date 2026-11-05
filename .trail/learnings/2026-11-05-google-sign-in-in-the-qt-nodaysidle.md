---
title: "Google sign-in in the Qt browser needs a Chrome user agent plus a chrome.app shim, and the default web profile must not be used"
author: Nodaysidle
date: 2026-11-05
branch: native-qt6-wayland
cost: { minutes: 45, tokens: 508780 }
id: l-5081b608
scope: repo
status: active
src: agent
touches:
  - CLAUDE.md
  - Cargo.lock
  - Cargo.toml
  - HANDOFF.md
  - PRODUCT.md
  - README.md
  - completions/nodaysidle-browser.fish
  - desktop/com.nodaysidle.Browser.desktop
---

## Decided
- Use a plain Chrome user agent that matches the engine's Chromium major version, with no QtWebEngine token, and define window.chrome.app in the main world at document creation. With both, Google's sign-in moves past the "may not be secure" page to a normal captcha.
- Use a named, non-off-the-record profile with explicit storage and cache paths. The default QtWebEngine profile is off-the-record, so nothing was ever written to disk and logins vanished on restart.
- One running instance owns the profile; later launches pass their URLs over a local socket and exit.
- The assistant uses the model id claude-haiku-5-5. The old claude-3-5-haiku-latest alias returns not_found.

In the code:
- src/main.cpp (profile, user agent, single instance), src/chrome_app.js, src/Main.qml (userScripts.collection on every WebEngineView).

## Tried and ruled out
- A Firefox user agent, set on the profile and rewritten per request for Google hosts: Google still answered rrk=46. The engine's client hints still say Chromium, so it never matched.
- Chrome user agent alone: still rrk=46. The missing chrome.app was the trigger.
- WebEngineScript as its own QML component file, or with the type name inline: not creatable in Qt 6. Use an object literal in userScripts.collection.
- QNetworkReply canReadLine() for the streaming response: it never became true, so the stream was never read. Buffer readAll() and split on newlines.
- findText callback for the match count: use the findTextFinished signal; the result properties are numberOfMatches and activeMatch.
- ContextMenuRequest.isContentEditable is false for a textarea. Use editFlags with CanCut and CanPaste.

## Watch out
- QML load failures exit with code 255 and print nothing when Qt logs to the journal. Run with QT_FORCE_STDERR_LOGGING=1 to see them.
- Chrome-flavoured Google checks can be tested without credentials: open accounts.google.com sign-in, submit any address; a rejected browser lands on /v3/signin/rejected with rrk=46.
- Hover state on a MouseArea is containsMouse; .hovered does not exist there. Use a HoverHandler on a parent so a child button keeps the parent hovered.
- Geolocation prompts work but lookups fail, since QtWebEngine has no Google location key; test permission plumbing with notifications instead.
- Pointer and key injection in the nested Hyprland test session is flaky on the first click after a launch; move the pointer, wait, then click.
