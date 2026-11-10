#!/usr/bin/env bash
# Register the discipline's MCP server with the agents configured on this
# machine.
#
# `B-188`: the server needed `vibe mcp install` and the install named it
# nowhere. It is declared `optional = true` because it writes into the user's
# own agent configuration: an unmet need or a failure is recorded and the
# installation carries on (PROP-065 `#ordinary-consumer`, `#steps`).
set -euo pipefail
exec "${VIBE_EXECUTABLE:-vibe}" mcp install
