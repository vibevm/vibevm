#!/usr/bin/env sh
set -eu

cargo_bin="$(command -v cargo || true)"
if [ -z "$cargo_bin" ]; then
  cargo_bin="${HOME}/.cargo/bin/cargo"
fi
"$cargo_bin" test --quiet
