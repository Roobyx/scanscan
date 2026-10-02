#!/usr/bin/env sh
# Read-only Docker disk audit: container/image/volume sizes and mount mapping.
# Usage: docker_audit.sh
set -eu

echo "== Containers, sizes, mounts =="
scanscan docker --json

echo
echo "== Live stats (single sample) =="
scanscan docker --stats --json
