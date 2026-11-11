#!/usr/bin/env bash
# Ask the one question `opencode-capture.sh` cannot: does a **subagent**
# inherit the lane?
#
# `opencode-capture.sh` proves the `instructions` carrier delivers, and it
# answers one non-streaming body, so the session never spawns anything. This
# script drives a real child session offline: `fake-provider-subagent.py`
# answers the first request with a **tool call** to OpenCode's own
# subagent-spawning tool, so the harness creates the child and sends its
# request, which lands here as a later capture file.
#
# The claim under test is `PROP-076#SYS-WORKER-EXCLUSION`: the lane must be
# in the primary's request **exactly once** and in the child's **zero**
# times. The 2026-11-10 survey read OpenCode's source at 1.19.5 and
# concluded that a child inherits the configuration `instructions`; this
# measures it on the installed build instead.
#
# No model is ever called and no real provider is ever reached: the only
# endpoint in the configuration is `127.0.0.1`. The recorder drops the
# `authorization` header before writing, so a captured file can never carry
# a credential. The owner's own OpenCode configuration and background
# service are untouched — an isolated `HOME`/XDG tree and `--standalone`.
#
# usage: opencode-subagent-capture.sh <sample-project> <out-dir> [opencode]
#
#   <sample-project>  a project made by `make-sample.sh`
#   <out-dir>         created; holds `run/` and `report.txt`
#   [opencode]        the OpenCode binary (default: `opencode` on PATH)
#
# env: PORT        loopback port (default 8761)
#      SPAWN_TOOL  the spawning tool's name, if it is not `subagent`

set -eu

PROJECT="$(cd "${1:?usage: opencode-subagent-capture.sh <sample-project> <out-dir> [opencode]}" && pwd)"
OUT="${2:?usage: opencode-subagent-capture.sh <sample-project> <out-dir> [opencode]}"
OPENCODE="${3:-opencode}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LANE="$PROJECT/vibevm/vibespecs/boot/AGENT-LANE.md"

[ -e "$OUT" ] && { echo "refusing: $OUT exists" >&2; exit 2; }
[ -r "$LANE" ] || { echo "no lane at $LANE — run make-sample.sh first" >&2; exit 2; }
command -v "$OPENCODE" >/dev/null 2>&1 || { echo "no opencode: $OPENCODE" >&2; exit 2; }

mkdir -p "$OUT/run"
OUT="$(cd "$OUT" && pwd)"
HOMEDIR="$OUT/home"
mkdir -p "$HOMEDIR/.config/opencode" "$HOMEDIR/.local/share"

PORT="${PORT:-8761}"
cat > "$HOMEDIR/.config/opencode/opencode.json" <<JSON
{
  "\$schema": "https://opencode.ai/config.json",
  "model": { "providerID": "fake", "model": "fake-1" },
  "providers": {
    "fake": {
      "name": "Fake",
      "package": "aisdk:@ai-sdk/openai-compatible",
      "settings": { "apiKey": "not-a-real-key", "baseURL": "http://127.0.0.1:$PORT/v1" },
      "models": { "fake-1": { "name": "Fake 1", "limit": { "context": 400000, "output": 1024 } } }
    }
  }
}
JSON

CAPTURE_DIR="$OUT/run" python3 -I "$HERE/fake-provider-subagent.py" "$PORT" &
SERVER=$!
sleep 1
(
  cd "$PROJECT"
  # `--standalone` is required and is itself a finding: OpenCode's default
  # mode serves sessions from one long-lived background service that never
  # sees this launch's environment (PROP-076 `#SYS-OPENCODE`).
  env -u OPENCODE_CONFIG_CONTENT \
    HOME="$HOMEDIR" \
    XDG_CONFIG_HOME="$HOMEDIR/.config" XDG_DATA_HOME="$HOMEDIR/.local/share" \
    XDG_CACHE_HOME="$HOMEDIR/.cache" XDG_STATE_HOME="$HOMEDIR/.local/state" \
    OPENCODE_CONFIG_CONTENT="{\"instructions\":[\"$LANE\"]}" \
    timeout 300 "$OPENCODE" --standalone run -m fake/fake-1 \
      "Spawn one subagent to check the canaries." >/dev/null 2>&1 || true
)
kill "$SERVER" 2>/dev/null || true
wait "$SERVER" 2>/dev/null || true

python3 -I - "$LANE" "$OUT" <<'PY' | tee "$OUT/report.txt"
import glob, json, os, sys

lane = open(sys.argv[1], encoding="utf-8").read()
out = sys.argv[2]

CANARIES = [
    "CANARY-LANE-HEAD-OBSIDIAN",
    "CANARY-LANE-TAIL-PUMICE",
    "CANARY-INDEX-CORE-BASALT",
    "CANARY-INDEX-USER-GNEISS",
    "CANARY-ENTRYPOINT-QUARTZ",
    "CANARY-TEMPLATE-SLATE",
]


def texts(message):
    content = message["content"]
    pieces = content if isinstance(content, list) else [content]
    return [piece["text"] if isinstance(piece, dict) else piece for piece in pieces
            if not isinstance(piece, dict) or "text" in piece]


def blob(body, role):
    parts = []
    for message in body["messages"]:
        if message["role"] == role:
            parts.extend(str(part) for part in texts(message))
    return "\n".join(parts)


def at(text, needle):
    """A BYTE offset, so the record can be compared with `wc -c`."""
    index = text.find(needle)
    return -1 if index < 0 else len(text[:index].encode())


files = [f for f in sorted(glob.glob(os.path.join(out, "run", "req-*.json")))
         if not f.endswith(".headers.json")]
print(f"lane file   : {len(lane.encode())} bytes")
print(f"requests    : {len(files)}")
if not files:
    print("NO REQUEST CAPTURED — the harness never reached the provider")
    raise SystemExit(1)

# **Which request is the child?** The one whose user turn is the prompt the
# provider handed to the spawning tool. Identifying it by position would
# break the moment the primary takes an extra turn.
CHILD_MARK = "whether you can see a line containing vibe:lane-begin"
for path in files:
    body = json.load(open(path, encoding="utf-8"))
    system = blob(body, "system")
    user = blob(body, "user")
    tools = sorted(tool.get("function", {}).get("name") for tool in body.get("tools", []))
    kind = "CHILD (subagent)" if CHILD_MARK in user else "primary"
    print(f"\n{os.path.basename(path)} — {kind}")
    print(f"    system {len(system.encode())} bytes, user {len(user.encode())} bytes, "
          f"{len(tools)} tools, lane x{system.count(lane)}")
    for canary in CANARIES:
        print(f"    {canary:26} system {system.count(canary)}  user {user.count(canary)}")
    print(f"    offsets: instructions {at(system, 'Instructions from:')}, "
          f"lane {at(system, lane[:200])}, <env> {at(system, '<env>')}, "
          f"date {at(system, chr(39).join(('Today', 's date')))}")

children = [f for f in files
            if CHILD_MARK in blob(json.load(open(f, encoding="utf-8")), "user")]
primaries = [f for f in files if f not in children]
def total(paths):
    return sum(blob(json.load(open(f, encoding="utf-8")), "system").count(lane)
               for f in paths)
print(f"\nVERDICT: primary requests {len(primaries)} carrying the lane {total(primaries)} "
      f"time(s); child requests {len(children)} carrying it {total(children)} time(s)")
if not children:
    print("  no child session was created — the spawning tool was not called, so "
          "worker exclusion is NOT measured by this run")
PY
