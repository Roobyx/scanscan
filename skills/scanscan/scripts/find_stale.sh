#!/usr/bin/env sh
# Read-only: find large, old files as cleanup candidates. Never deletes anything.
# Usage: find_stale.sh [root] [size] [age]
set -eu

ROOT="${1:-/}"
SIZE="${2:-+1G}"
AGE="${3:--180d}"

echo "Stale candidates under ${ROOT}: size ${SIZE}, mtime ${AGE}"
scanscan find "${ROOT}" --size "${SIZE}" --mtime "${AGE}" --json
