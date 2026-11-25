---
title: The page assistant uses DeepSeek Flash with thinking off and shows answers as plain text
author: Nodaysidle
date: 2026-11-25
branch: deepseek-page-assistant
cost: { minutes: 32, tokens: 230666 }
id: l-44e45aab
scope: repo
status: active
src: agent
touches:
  - dev/harness.py
  - src/Main.qml
  - src/ThemedMenu.qml
  - src/adblock.h
  - src/backend.cpp
  - src/backend.h
  - src/home.html
  - src/main.cpp
  - src/theme.cpp
  - src/theme.h
---

## Decided
- The "Ask this page" panel talks to DeepSeek (model deepseek-flash) next to the TypeSafe scam check, which stays as it is. The owner asked for DeepSeek in place of the Anthropic Haiku assistant removed in 170c609.
- Thinking is turned off in the request: deepseek-flash thinks by default, which slows the first word and streams reasoning text the panel has no use for.
- Answers are shown as plain text, never rich text or Markdown, because rich text in a Qt text item can fetch remote images, which would leak to third parties from a page the user only asked about.
- Only the first 50,000 characters of the page's visible text are sent, together with the conversation; nothing is sent until the user asks.
- The conversation resets when the active tab or page changes, so answers never mix two pages.

## Watch out
- The deepseek-chat and deepseek-reasoner names were retired in July 2026; use deepseek-flash.
- Cancelling (Stop, New chat, or a new question) ends with askDone, not askError; only a stall past the 60 s transfer timeout reports an error.

In the code: Backend::askPage / cancelAsk / deepseekError in src/backend.cpp, DEEPSEEK_API_KEY in loadKeys(); panel, askAboutPage and onAsk* handlers in src/Main.qml; shortcut Ctrl+Shift+E.
