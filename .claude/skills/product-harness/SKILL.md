---
name: product-harness
description: Use when planning, building, evaluating or marking done a feature in nodaysidle-browser-linux through the dev/ product harness (PRODUCT.md, CONTRACT.md, FEATURES.json).
---

# Product harness discipline

- Read `dev/PRODUCT.md` for goals, scope boundaries and verified commands, and `dev/CONTRACT.md` for the active feature's checks and evidence criteria. Read `dev/FEATURES.json` when starting or resuming work.
- Build one agreed feature at a time; never break existing tests or touch unrelated code.
- Save full verification logs under `artifacts/<run-id>/`.
- Mark a feature PASS in `dev/FEATURES.json` only after the independent EVALUATOR verifies the evidence against the recorded git commit.
- Repair rounds are capped at 2 before stopping for human review.
- Status: `cd dev && ./harness status`.
