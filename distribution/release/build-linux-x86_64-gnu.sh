#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
VIBEVM_LINUX_ABI=gnu
export VIBEVM_LINUX_ABI
exec sh "$script_dir/build-linux-x86_64.sh" "$@"
