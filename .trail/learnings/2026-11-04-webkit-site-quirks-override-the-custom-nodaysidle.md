---
title: "WebKit site quirks override the custom User-Agent on some hosts, so setting it does not prove what is sent"
author: Nodaysidle
date: 2026-11-04
branch: f01-google-login
cost: { minutes: 1, tokens: 0 }
id: l-3045fb4b
scope: repo
status: active
src: agent
touches:
  - src/tabs.rs
---

## Decided
Check the User-Agent per host at runtime, on the wire, because reading it back from WebKit settings proves nothing about what a given host receives.

## Tried and ruled out
Asserting `settings.user_agent()` in the GTK test, because it passes while accounts.google.com gets a different string.

## Watch out
WebKitGTK 2.54 site-specific quirks are on by default and replace a custom User-Agent per host: accounts.google.com got a Safari-style `Version/60.5` string, docs.google.com got `Chrome/300.0.0.0`, and other hosts got the custom string. Turning quirks off needs a real sign-in test, because the quirk may be what makes Google sign-in work.

In the code: `configure_view` in src/tabs.rs. Turning quirks off is `set_enable_site_specific_quirks(false)`, which is global. A local forward proxy over plain http via `http_proxy` lets you probe named hosts without reaching them.
