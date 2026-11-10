#!/usr/bin/env bash
# Measure, offline, what OpenCode actually puts in a request — and whether
# the `instructions` carrier is what delivered it.
#
# It starts a loopback OpenAI-compatible provider that records every request
# body (`opencode-fake-provider.py`), configures OpenCode to use **only**
# that provider in an isolated `HOME`/XDG tree, and runs the same prompt
# twice against the sample project: once with `OPENCODE_CONFIG_CONTENT`
# naming the composed lane, once without. The second run is the control —
# without it, "the lane arrived" says nothing about *what* delivered it.
#
# No model is ever called and no real provider is ever reached: the only
# endpoint in the configuration is `127.0.0.1`. The recorder drops the
# `authorization` header before writing, so a captured file can never carry
# a credential.
#
# usage: opencode-capture.sh <sample-project> <out-dir> [opencode]
#
#   <sample-project>  a project made by `make-sample.sh`
#   <out-dir>         created; holds `with/`, `without/` and `report.txt`
#   [opencode]        the OpenCode binary (default: `opencode` on PATH)
#
# `--standalone` is **required** and is itself a finding: OpenCode's default
# mode serves sessions from one long-lived background service that never
# sees this launch's environment, so without it the carrier does not arrive
# at all (PROP-076 `#SYS-OPENCODE`).

set -eu

PROJECT="$(cd "${1:?usage: opencode-capture.sh <sample-project> <out-dir> [opencode]}" && pwd)"
OUT="${2:?usage: opencode-capture.sh <sample-project> <out-dir> [opencode]}"
OPENCODE="${3:-opencode}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LANE="$PROJECT/vibevm/vibespecs/boot/AGENT-LANE.md"

[ -e "$OUT" ] && { echo "refusing: $OUT exists" >&2; exit 2; }
[ -r "$LANE" ] && : || { echo "no lane at $LANE — run make-sample.sh first" >&2; exit 2; }
command -v "$OPENCODE" >/dev/null 2>&1 || { echo "no opencode: $OPENCODE" >&2; exit 2; }

mkdir -p "$OUT/with" "$OUT/without"
OUT="$(cd "$OUT" && pwd)"
HOMEDIR="$OUT/home"
mkdir -p "$HOMEDIR/.config/opencode" "$HOMEDIR/.local/share"

PORT="${PORT:-8731}"
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

run() {           # run <capture-dir> [carrier]
  CAPTURE_DIR="$1" python3 -I "$HERE/opencode-fake-provider.py" "$PORT" &
  local server=$!
  sleep 1
  (
    cd "$PROJECT"
    # The fake provider answers without a `finish_reason`, so OpenCode
    # retries and finally fails. That is expected: the claim under test is
    # the REQUEST, and the request is already on disk.
    env -u OPENCODE_CONFIG_CONTENT \
      HOME="$HOMEDIR" \
      XDG_CONFIG_HOME="$HOMEDIR/.config" XDG_DATA_HOME="$HOMEDIR/.local/share" \
      XDG_CACHE_HOME="$HOMEDIR/.cache" XDG_STATE_HOME="$HOMEDIR/.local/state" \
      ${2:+OPENCODE_CONFIG_CONTENT=$2} \
      timeout 300 "$OPENCODE" --standalone run -m fake/fake-1 "say ok" >/dev/null 2>&1 || true
  )
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
}

run "$OUT/with" "{\"instructions\":[\"$LANE\"]}"
run "$OUT/without"

python3 -I - "$LANE" "$OUT" <<'PY' | tee "$OUT/report.txt"
import json, sys, glob, os

lane = open(sys.argv[1], encoding="utf-8").read()
out = sys.argv[2]


def tool_bytes(directory):
    files = [f for f in sorted(glob.glob(os.path.join(directory, "req-*.json")))
             if not f.endswith(".headers.json")]
    if not files:
        return 0
    body = json.load(open(files[-1], encoding="utf-8"))
    return len(json.dumps(body.get("tools", [])).encode())


def system_blob(directory):
    # The second request is the one that carries the whole prompt; the first
    # is OpenCode's own title call.
    files = sorted(glob.glob(os.path.join(directory, "req-*.json")))
    files = [f for f in files if not f.endswith(".headers.json")]
    if not files:
        return None
    body = json.load(open(files[-1], encoding="utf-8"))
    parts = []
    for message in body["messages"]:
        if message["role"] != "system":
            continue
        content = message["content"]
        for piece in content if isinstance(content, list) else [content]:
            parts.append(piece["text"] if isinstance(piece, dict) else piece)
    return "\n".join(parts)


CANARIES = [
    "CANARY-LANE-HEAD-OBSIDIAN",
    "CANARY-LANE-TAIL-PUMICE",
    "CANARY-INDEX-CORE-BASALT",
    "CANARY-INDEX-USER-GNEISS",
    "CANARY-ENTRYPOINT-QUARTZ",
    "CANARY-TEMPLATE-SLATE",
]

def at(blob, needle):
    """A BYTE offset. `str.find` answers in characters, and a record that
    mixes the two cannot be compared with `wc -c`."""
    index = blob.find(needle)
    return -1 if index < 0 else len(blob[:index].encode())


with_carrier = system_blob(os.path.join(out, "with"))
without = system_blob(os.path.join(out, "without"))
print(f"lane file                : {len(lane.encode())} bytes")
print(f"tool definitions         : {tool_bytes(os.path.join(out, 'with'))} bytes")
for name, blob in (("with carrier", with_carrier), ("control, no carrier", without)):
    if blob is None:
        print(f"{name:25}: NO REQUEST CAPTURED")
        continue
    print(f"{name:25}: system prompt {len(blob.encode())} bytes, lane x{blob.count(lane)}")
    for canary in CANARIES:
        print(f"    {canary:26} {blob.count(canary)}")
    print(f"    byte offsets: instructions {at(blob, 'Instructions from:')}, "
          f"lane {at(blob, lane[:200])}, <env> {at(blob, '<env>')}, "
          f"date {at(blob, 'Today' + chr(39) + 's date')}")
PY
