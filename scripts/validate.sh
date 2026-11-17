#!/usr/bin/env bash
# Build and lint gates for release validation.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

# Build out of tree so the gate never dirties the repository.
BUILD_DIR="${BUILD_DIR:-/tmp/nds-browser-validate}" ./bin/build

# qmllint's "unqualified" warnings are context-property noise here (theme, backend,
# vault and importer are context properties), so only real findings gate.
lint_output="$(/usr/lib/qt6/bin/qmllint -I /usr/lib/qt6/qml src/*.qml 2>&1 || true)"
findings="$(printf '%s\n' "$lint_output" | grep -v '\[unqualified\]' | grep -E '^(Warning|Error)' || true)"
if [ -n "$findings" ]; then
    printf '%s\n' "$findings" >&2
    echo "qmllint: fix the findings above" >&2
    exit 1
fi
echo "qmllint: clean"
