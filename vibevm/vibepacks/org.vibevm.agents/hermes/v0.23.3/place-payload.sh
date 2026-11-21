#!/bin/sh
# Placement only: the source owner already authenticated and extracted the archive.
set -eu
: "${VIBE_SOURCES:?missing selected sources}"
: "${VIBE_OUT:?missing owned output}"
source="$VIBE_SOURCES/payload/hermes"
[ -f "$source/VIBEVM-BUILD.json" ]
[ -x "$source/bin/hermes" ]
[ -f "$source/lib/launch-hermes.py" ]
mkdir -p "$VIBE_OUT"
cp -a "$source/." "$VIBE_OUT/"
[ -x "$VIBE_OUT/bin/hermes" ]
