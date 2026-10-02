#!/usr/bin/env sh
# Read-only disk-space report: scan a root, then summarize top-N and extensions.
# Usage: space_report.sh [root]   (default: /)
set -eu

ROOT="${1:-/}"

echo "Scanning ${ROOT} (read-only)…"
scanscan scan "${ROOT}" --json > /tmp/scanscan-scan.json

echo
echo "== Top 25 by allocated size =="
scanscan top "${ROOT}" -n 25 --metric alloc --json

echo
echo "== Extension breakdown =="
scanscan ext "${ROOT}" --json
