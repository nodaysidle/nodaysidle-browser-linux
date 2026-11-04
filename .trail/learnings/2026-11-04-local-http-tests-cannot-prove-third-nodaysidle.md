---
title: Local HTTP tests cannot prove third-party cookies are sent cross-site in this browser
author: Nodaysidle
date: 2026-11-04
branch: master
cost: { minutes: 4, tokens: 126686 }
id: l-f54fd76a
scope: repo
status: active
src: agent
---

## Decided
Check third-party cookie acceptance by looking for the row in cookies.sqlite, and compare against a run with the accept policy disabled. That comparison does discriminate.

## Tried and ruled out
Asserting a third-party iframe cookie is sent on the next launch over plain HTTP. Without a SameSite attribute the cookie was stored but not sent cross-site. With SameSite=None and no Secure it was not stored at all.

## Watch out
A real cross-site send needs SameSite=None; Secure over HTTPS with a trusted certificate, and the browser has no bypass for invalid certificates.
In the code: persistent_web_context in the profile module sets CookieAcceptPolicy::Always.
