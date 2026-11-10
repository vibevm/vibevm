#!/usr/bin/env bash
# Read the cache counters a Claude Code session recorded, for P9 steps 2
# and 3. It reads session logs and nothing else: no network, no model, no
# write.
#
# usage: read-usage.sh [session-log-dir] [since-epoch-seconds]
#
#   [session-log-dir]  default `$CLAUDE_CONFIG_DIR/projects`, else
#                      `~/.claude2/projects` — the directory the owner's
#                      `claude2` wrapper points the harness at. Pass it
#                      explicitly when the wrapper sets the variable
#                      internally and `vibe agent list` reports the
#                      directory as unknown.
#   [since-epoch]      only files modified at or after this time, so a run's
#                      numbers are not mixed with an earlier session's. Get
#                      it with `date +%s` *before* starting the session.
#
# It prints one row per assistant message carrying a usage record:
#   <file> <input> <cache_creation> <cache_read> <output>
# then the per-file totals. A cache **read** at or above the lane's token
# count on the second session is what step 2 is looking for; the first
# session should show the cache **creation** instead.

set -eu

DIR="${1:-${CLAUDE_CONFIG_DIR:-$HOME/.claude2}/projects}"
SINCE="${2:-0}"

[ -d "$DIR" ] || { echo "no session log directory: $DIR" >&2; exit 2; }

python3 -I - "$DIR" "$SINCE" <<'PY'
import json, pathlib, sys

root = pathlib.Path(sys.argv[1])
since = float(sys.argv[2])
rows = 0
for log in sorted(root.rglob("*.jsonl")):
    if log.stat().st_mtime < since:
        continue
    totals = [0, 0, 0, 0]
    for line in log.read_text(errors="replace").splitlines():
        try:
            record = json.loads(line)
        except ValueError:
            continue
        usage = (record.get("message") or {}).get("usage") or record.get("usage")
        if not isinstance(usage, dict):
            continue
        got = [
            usage.get("input_tokens", 0) or 0,
            usage.get("cache_creation_input_tokens", 0) or 0,
            usage.get("cache_read_input_tokens", 0) or 0,
            usage.get("output_tokens", 0) or 0,
        ]
        if not any(got):
            continue
        rows += 1
        totals = [a + b for a, b in zip(totals, got)]
        print(f"{log.name} input={got[0]} cache_creation={got[1]} "
              f"cache_read={got[2]} output={got[3]}")
    if any(totals):
        print(f"  TOTAL {log.name} input={totals[0]} cache_creation={totals[1]} "
              f"cache_read={totals[2]} output={totals[3]}")
if rows == 0:
    print("no usage records found — check the directory and the --since time")
    sys.exit(1)
PY
