#!/usr/bin/env bash
# The discipline tools, built in the slot the install just materialised.
#
# `B-188`: an install used to copy the source and stop, leaving the reader to
# discover `vibe bin build` from the tutorial. It is now a declared
# contribution, so it appears in `vibe install --plan-only` with its title and
# its prerequisites before anything runs (PROP-065 `#ordinary-consumer`).
set -euo pipefail
# The lifecycle runs in the target slot, but `bin build` takes the owning
# project's lock, so run from the project root when the environment names one.
if [ -n "${VIBE_PROJECT_ROOT:-}" ]; then
  cd -- "$VIBE_PROJECT_ROOT"
fi
# Pass --assume-yes: the install that materialised this slot already authorised
# dependency installation, so the narrated build contribution needs no second
# prompt (PROP-054 `#INSTALL-IS-CONSENT`).
exec "${VIBE_EXECUTABLE:-vibe}" bin build --assume-yes
