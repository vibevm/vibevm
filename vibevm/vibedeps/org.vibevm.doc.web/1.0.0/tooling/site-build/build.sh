#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export VIBE_NODE_EXECUTABLE="$(command -v node)"
exec "$VIBE_NODE_EXECUTABLE" "$root/build.mjs"
