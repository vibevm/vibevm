#!/usr/bin/env bash
# The whole-request proof of `PROP-076#SYS-SUPPLIED-BASE`, conditions (0) and
# (i)–(v), run offline against a loopback recorder.
#
# This is the one thing that may earn `verified = true` for a prompt-base set.
# It does not compare a base to a base: it compares the WHOLE request a
# carrier launch sends against the whole request a plain run of the same shape
# sends, and then says in bytes what the difference is. The owner's rule the
# day the mechanism was specified: «нужно чтобы не только базовый промт
# совпал, но и чтобы их базовый промт с нашим префиксом собрались правильно».
#
# No model is ever called and no real provider is ever reached: the only
# endpoint in the configuration is `127.0.0.1`. The recorder drops the
# `authorization` header before writing, so a captured file can never carry a
# credential. The owner's own OpenCode configuration and background service
# are untouched — an isolated `HOME`/XDG tree and `--standalone` throughout.
#
# usage: opencode-prove.sh <sample-project> <out-dir> <carrier.json> [opencode]
#        opencode-prove.sh <sample-project> <out-dir> --from-vibe   [opencode]
#
#   <sample-project>  a project made by `make-sample.sh`
#   <out-dir>         created; holds one directory per run and `report.txt`
#   <carrier.json>    {"env": …, "value": …, "flags": [ … ]} — the carrier the
#                     launch would make. The sync tool writes this file from
#                     the set under test, because a set that has not been
#                     proved yet cannot be asked of `vibe prompt flags`: that
#                     command refuses an unverified set, which is the gate
#                     this proof exists to open.
#   --from-vibe       ask `vibe prompt flags --harness opencode --json`
#                     instead. Used once a set IS verified, to close the loop:
#                     the bytes this proof measured are the bytes vibe emits.
#   [opencode]        the OpenCode binary (default: `opencode` on PATH)
#
# env: PORT   loopback port (default 8741)
#      VIBE   the vibe binary for `--from-vibe` (default: `vibe` on PATH)
#      VIBE_PROMPT_BASES_DIR  passed through to `--from-vibe`
#
# Five runs, because five of the six conditions are comparisons and a
# comparison needs both sides:
#
#   plain/      the control: same flags, same prompt, no carrier
#   carrier/    the launch under test
#   carrier2/   the same launch again, for the two-launch prefix comparison
#   child-with/     a real subagent session, carrier set
#   child-plain/    a real subagent session, carrier unset — the child's own
#                   control, which is what condition (v) compares against
#
# Condition (vi) is NOT here and cannot be: it asks whether a live model still
# obeys an instruction that exists only in the harness's own base, which no
# byte diff can establish. It runs on the local model with the small sample —
# see `RUNBOOK.md` and `opencode-live-check.sh`.

set -eu

PROJECT="$(cd "${1:?usage: opencode-prove.sh <sample-project> <out-dir> <carrier.json|--from-vibe> [opencode]}" && pwd)"
OUT="${2:?usage: opencode-prove.sh <sample-project> <out-dir> <carrier.json|--from-vibe> [opencode]}"
CARRIER="${3:?usage: opencode-prove.sh <sample-project> <out-dir> <carrier.json|--from-vibe> [opencode]}"
OPENCODE="${4:-opencode}"
VIBE="${VIBE:-vibe}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LANE="$PROJECT/vibevm/vibespecs/boot/AGENT-LANE.md"

[ -e "$OUT" ] && { echo "refusing: $OUT exists" >&2; exit 2; }
[ -r "$LANE" ] || { echo "no lane at $LANE — run make-sample.sh first" >&2; exit 2; }
command -v "$OPENCODE" >/dev/null 2>&1 || { echo "no opencode: $OPENCODE" >&2; exit 2; }

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
for leg in plain carrier carrier2 child-with child-plain; do mkdir -p "$OUT/$leg"; done
HOMEDIR="$OUT/home"
mkdir -p "$HOMEDIR/.config/opencode" "$HOMEDIR/.local/share"

PORT="${PORT:-8741}"
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

# The isolated environment every leg runs in. `HOME` and the four XDG
# variables point into this run's own tree, so nothing reads or writes the
# operator's own OpenCode configuration, state or session database.
isolated() {
  env -u OPENCODE_CONFIG_CONTENT \
    HOME="$HOMEDIR" \
    XDG_CONFIG_HOME="$HOMEDIR/.config" XDG_DATA_HOME="$HOMEDIR/.local/share" \
    XDG_CACHE_HOME="$HOMEDIR/.cache" XDG_STATE_HOME="$HOMEDIR/.local/state" \
    "$@"
}

# **The carrier under test.** Either the sync tool handed it to us, or — for
# a set that is already verified — vibe is asked for it, which is how the
# proof's own bytes and the launcher's own bytes are shown to be the same
# bytes.
if [ "$CARRIER" = "--from-vibe" ]; then
  command -v "$VIBE" >/dev/null 2>&1 || { echo "no vibe binary: $VIBE" >&2; exit 2; }
  isolated ${VIBE_PROMPT_BASES_DIR:+VIBE_PROMPT_BASES_DIR=$VIBE_PROMPT_BASES_DIR} \
    "$VIBE" prompt flags --harness opencode --json --path "$PROJECT" \
    > "$OUT/vibe-flags.json" || {
      echo "vibe prompt flags refused — see $OUT/vibe-flags.json" >&2; exit 2; }
  python3 -I - "$OUT/vibe-flags.json" "$OUT/carrier.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1], encoding="utf-8"))
assert len(report["env"]) == 1, report["env"]
json.dump({"env": report["env"][0]["name"], "value": report["env"][0]["value"],
           "flags": report["flags"]}, open(sys.argv[2], "w", encoding="utf-8"))
PY
else
  cp "$CARRIER" "$OUT/carrier.json"
fi

ENV_NAME="$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["env"])' "$OUT/carrier.json")"
ENV_VALUE="$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["value"], end="")' "$OUT/carrier.json")"
# One flag per line, not a space-joined string: a carrier whose flag ever
# carries a space would otherwise be split into two arguments silently.
mapfile -t FLAGS < <(python3 -I -c 'import json,sys
for flag in json.load(open(sys.argv[1]))["flags"]: print(flag)' "$OUT/carrier.json")

# One leg. `provider` is the recorder script; `carrier` is empty for a
# control run and `set` for a run under test.
leg() {                 # leg <dir> <provider.py> <carrier|plain> <prompt>
  local dir="$1" provider="$2" mode="$3" prompt="$4"
  CAPTURE_DIR="$OUT/$dir" setsid nohup python3 -I "$HERE/$provider" "$PORT" \
    >"$OUT/$dir/recorder.log" 2>&1 &
  local server=$!
  sleep 2
  (
    cd "$PROJECT"
    if [ "$mode" = "carrier" ]; then
      isolated "$ENV_NAME=$ENV_VALUE" \
        timeout 300 "$OPENCODE" "${FLAGS[@]}" run -m fake/fake-1 "$prompt" \
        >"$OUT/$dir/run.log" 2>&1 || true
    else
      isolated \
        timeout 300 "$OPENCODE" "${FLAGS[@]}" run -m fake/fake-1 "$prompt" \
        >"$OUT/$dir/run.log" 2>&1 || true
    fi
  )
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
}

# The fake provider answers once and stops, so a plain capture never spawns
# anything; the subagent provider answers the first request with a tool call
# to OpenCode's own spawning tool, so the harness creates a real child.
leg plain       opencode-fake-provider.py     plain   "say ok"
leg carrier     opencode-fake-provider.py     carrier "say ok"
leg carrier2    opencode-fake-provider.py     carrier "say ok"
leg child-with  fake-provider-subagent.py     carrier "Spawn one subagent to check the canaries."
leg child-plain fake-provider-subagent.py     plain   "Spawn one subagent to check the canaries."

# **The judge's exit status is this script's own.** A `| tee` would make it
# `tee`'s instead, and a proof whose FAIL exits 0 is worse than no proof: the
# sync tool reads this status to decide whether `verified = true` may be
# written at all.
set +e
python3 -I "$HERE/opencode-prove.py" "$LANE" "$OUT" > "$OUT/report.txt" 2>&1
VERDICT=$?
set -e
cat "$OUT/report.txt"
exit "$VERDICT"
