#!/usr/bin/env bash
# Reproducible quality gates for release validation.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
NODAYSIDLE_REQUIRE_DISPLAY=1 cargo test --locked --offline --release
