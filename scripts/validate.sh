#!/usr/bin/env bash
# Build and lint gates for release validation.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

./bin/build
/usr/lib/qt6/bin/qmllint src/*.qml 2>&1 | { ! grep -E "^Error"; }
MANIFEST=launcher/Cargo.toml
cargo build --release --locked --manifest-path "${MANIFEST}"
cargo fmt --check --manifest-path "${MANIFEST}"
cargo clippy --locked --manifest-path "${MANIFEST}" -- -D warnings
