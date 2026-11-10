#!/usr/bin/env bash
# The discipline tools, built in the slot the install just materialised.
#
# `B-188`: an install used to copy the source and stop, leaving the reader to
# discover `vibe bin build` from the tutorial. It is now a declared
# contribution, so it appears in `vibe install --plan-only` with its title and
# its prerequisites before anything runs (PROP-065 `#ordinary-consumer`).
set -euo pipefail
exec "${VIBE_EXECUTABLE:-vibe}" bin build
