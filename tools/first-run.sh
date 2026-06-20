#!/usr/bin/env bash
# vibevm first-run — bootstrap the very first installation from a source
# checkout. Builds the current tree, installs it as your first VVM version,
# and puts `vibe` on PATH so a new shell can run it. See README.md
# "First run" and `spec/common/PROP-019-version-manager.md`.
#
# What it does, in order:
#   1. vibe self install        — build this checkout, publish it as
#                                 instance 1, make it the active version.
#   2. vibe self doctor --fix   — write the shims into ~/opt/bin and put
#                                 ~/opt/bin on PATH (durable; new shells).
#   3. vibe self ls             — show what is installed.
#
# This edits your durable PATH (the registry on Windows, the shell rc on
# POSIX). To try VVM WITHOUT touching ~/opt or PATH, skip this script and run:
#   VIBEVM_INSTALL_ROOT="$(mktemp -d)" cargo run -p vibe-cli -- self install
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
cd "$repo_root"

if [ ! -f Cargo.toml ] || [ ! -d crates/vibe-cli ]; then
  echo "first-run: run this from a vibevm source tree (Cargo.toml + crates/vibe-cli not found)" >&2
  exit 1
fi

run() {
  echo "==> vibe $*"
  cargo run -q -p vibe-cli -- "$@"
}

echo "first-run: building this checkout and installing it as your first version…"
run self install

echo
echo "first-run: writing shims and putting ~/opt/bin on PATH…"
run self doctor --fix --yes

echo
run self ls || true

cat <<'EOF'

first-run: done. Open a NEW terminal, then:

    vibe self ls

From now on the loop is fast: `vibe self install` rebuilds, flips the active
version, and the next `vibe` in the same shell uses it — no console reload.
EOF
