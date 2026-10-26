---
title: "Browser AI panel, themes, and driving the app in tests"
author: Nodaysidle
date: 2026-10-26
branch: review-fixes
cost: { minutes: 30, tokens: 272037 }
touches:
  - ".loop-engineer/current/prompts/00-000-researcher.md"
  - ".loop-engineer/current/prompts/00-001-planner.md"
  - ".loop-engineer/current/prompts/00-002-reviewer.md"
  - ".loop-engineer/current/prompts/00-003-packaging-engineer.md"
  - ".loop-engineer/current/prompts/00-004-browser-engineer.md"
  - ".loop-engineer/current/prompts/00-005-browser-engineer.md"
  - ".loop-engineer/current/prompts/00-006-verifier.md"
  - ".loop-engineer/current/report.md"
  - ".loop-engineer/current/state.json"
  - ".loop-engineer/current/team/browser-engineer.md"
---

## Decided
- The fullscreen notice from the earlier learning was removed at the owner's request (Hyprland user, no fullscreen); fullscreen behaviour is back to WebKit's default.
- Browser AI = TypeSafe Jev (TYPESAFE_API_KEY) for the scam check plus the Anthropic Messages API (ANTHROPIC_API_KEY, claude-haiku-5-5, effort low) for summarize/ask. Owner explicitly rejected OpenRouter and Opus for the browser. Keys only from env; nothing sent until a click.
- Themes: one tokenised stylesheet, DARK_PALETTE / PAPER_PALETTE swapped at runtime; Paper also switches the GTK theme to its light variant (Adwaita-dark -> Adwaita) because WebKit reads dark mode from the theme name.

## Tried and ruled out
- wtype cannot reliably deliver Return, Tab, Alt+mnemonics or keys into GTK menu grabs on this Hyprland. Press buttons through AT-SPI instead (gi Atspi, role name "button", get_action_iface().do_action(0)).

## Watch out
- After `cargo build`, /proc/PID/exe of a running dev instance reads "... (deleted)"; match the exe path by prefix or the old instance keeps receiving forwarded URLs.
- `cargo test`/`clippy` do not refresh target/debug/nodaysidle-browser; run `cargo build` before checking in the app.
- ANTHROPIC_API_KEY is not set in the owner's shell by default (config.fish only unsets it in a wrapper).
