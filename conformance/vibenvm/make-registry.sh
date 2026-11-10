#!/usr/bin/env bash
# make-registry.sh — publish the two packages sandbox B needs into a local
# directory registry (PROP-075 `#VIBENVM-SUITE`).
#
# The suite compares vibe against nvm, and the vibe side needs two packages
# that live in their own repositories, not in this one:
#
#   org.vibevm.node/node      the Node.js runtime template, with its npm
#                             ecosystem adapter and its version files
#   org.vibevm.node/vibenvm   the nvm-shaped shim `vibenvm.sh`
#
# This writes them into one `localdir` registry — the plain directory layout
# `<group>/<name>/v<version>/` (PROP-002 `#registry-type`) — which is what
# `run.sh --vibenvm-registry` / `$VIBENVM_REGISTRY` points at. It copies
# working trees, so a package being edited is published as it stands; the
# version comes from each manifest's own `[package] version`.
#
# usage: make-registry.sh [REGISTRY_DIR] [NODE_PKG_DIR] [VIBENVM_PKG_DIR]
#   REGISTRY_DIR      default: $VIBENVM_REGISTRY, else ./mirror/registry
#                     (git-ignored, beside the mirror)
#   NODE_PKG_DIR      default: $VIBENVM_NODE_PKG, else /fast/git/vibe-pkg-node
#   VIBENVM_PKG_DIR   default: $VIBENVM_SHIM_PKG, else /fast/git/vibe-pkg-vibenvm
#
# It reaches no network and reads nothing but those two directories.
# exit: 0 published; 2 a package directory or its version is missing.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

REGISTRY="${1:-${VIBENVM_REGISTRY:-$HERE/mirror/registry}}"
NODE_PKG="${2:-${VIBENVM_NODE_PKG:-/fast/git/vibe-pkg-node}}"
SHIM_PKG="${3:-${VIBENVM_SHIM_PKG:-/fast/git/vibe-pkg-vibenvm}}"

publish() { # publish <package-dir> <group> <name>
    local src="$1" group="$2" name="$3" version dest
    if [ ! -f "$src/vibe.toml" ]; then
        echo "make-registry.sh: no package at $src (no vibe.toml)" >&2
        exit 2
    fi
    version="$(sed -nE 's/^version = "([^"]+)"$/\1/p' "$src/vibe.toml" | head -1)"
    if [ -z "$version" ]; then
        echo "make-registry.sh: $src/vibe.toml declares no [package] version" >&2
        exit 2
    fi
    dest="$REGISTRY/$group/$name/v$version"
    rm -rf "$dest"
    mkdir -p "$dest"
    # The working tree as it stands, minus its history.
    (cd "$src" && tar --exclude=.git -cf - .) | tar -xf - -C "$dest"
    echo "published $group/$name@$version"
}

mkdir -p "$REGISTRY"
publish "$NODE_PKG" org.vibevm.node node
publish "$SHIM_PKG" org.vibevm.node vibenvm
echo "registry:   $REGISTRY"
