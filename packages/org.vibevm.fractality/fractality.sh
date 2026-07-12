#!/usr/bin/env bash
# fractality — thin launcher (Bash), no global install.
#
# Runs the working-tree build of the CLI straight from the project tree,
# so there is nothing to install on PATH. State lives in the default
# global home (~/.fractality) — where a mission-control daemon and
# profiles.toml already live — unless FRACTALITY_HOME or the --home flag
# says otherwise.
#
# Usage:  ./fractality.sh mc status
#         ./fractality.sh ps
#         ./fractality.sh run --packet fractality/v0.1.0/spec/examples/hello-glm.toml
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bin="$here/fractality/v0.1.0/target/debug/fractality.exe"
if [[ ! -x "$bin" ]]; then
  echo "fractality: binary not built at $bin — build it: cargo build -p fractality-cli (from fractality/v0.1.0)" >&2
  exit 2
fi

exec "$bin" "$@"
