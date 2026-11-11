#!/usr/bin/env bash
# Condition (vi) of `PROP-076#SYS-SUPPLIED-BASE` — the one a byte diff cannot
# make.
#
# `opencode-prove.sh` establishes, in bytes, that the carrier request is the
# plain request plus the lane and nothing else. It cannot establish that the
# harness's own instructions **still work**: a base that arrived byte-perfect
# and is then ignored, or displaced by position, looks identical on the wire.
# So one live session is asked three things at once:
#
#   1. quote the delivered tail marker — did the lane arrive?
#
#      **The question has to name the marker's own file, and that is a
#      finding rather than a convenience.** In `launch` delivery the harness
#      injects `AGENTS.md` AFTER the agent's `system`, and that file's managed
#      block contains an indented *format example* of a `vibe:lane-end` line.
#      So "the last line containing vibe:lane-end" is genuinely the example,
#      not the delivered marker, and a model that answers it that way is
#      right. Measured on the local 27B model, which did exactly that. The
#      question therefore asks for the line that also names `90-user`, which
#      only the delivered marker does.
#   2. which files did you read with a tool — is the router's rule 1 held, i.e.
#      did the lane arrive *as instructions* rather than as something to go
#      and fetch?
#   3. state the rule your instructions give about the edit tool's `oldString`
#      — an instruction that exists **only** in OpenCode's own base text and
#      nowhere in the lane. If this comes back right, the base was SUPPLIED
#      and not displaced.
#
# **Cost rule, owner 2026-11-11.** A live session uses the local model and the
# **tiny** sample; every real-size claim is proved offline, where a 200 KB lane
# costs nothing. This script therefore defaults to the unpadded sample and
# takes the model as an argument so it can never be run against a metered one
# by accident.
#
# It runs through `vibe agent`, not by hand: the claim under test is what the
# LAUNCHER delivers, so the launcher has to be the thing that delivers it. The
# model named on the command line is also what vibe's own model discovery
# reads, so the base it supplies is the one for that model.
#
# usage: opencode-live-check.sh <sample-project> <out-dir> [model]
#
#   <sample-project>  a project made by `make-sample.sh` with NO padding
#   <out-dir>         created; holds `answer.txt` and `expected.txt`
#   [model]           provider/model (default: homelocal/qwen3.8-27b)
#
# env: VIBE                   the vibe binary (default: `vibe` on PATH)
#      VIBE_PROMPT_BASES_DIR  the prompt-base tree, when OpenCode was not
#                             installed through vibe
#
# The owner's own OpenCode configuration is **read and not written**, and the
# background service is untouched: the carrier's own `--standalone` is what
# makes the launch private.

set -eu

PROJECT="$(cd "${1:?usage: opencode-live-check.sh <sample-project> <out-dir> [model]}" && pwd)"
OUT="${2:?usage: opencode-live-check.sh <sample-project> <out-dir> [model]}"
MODEL="${3:-homelocal/qwen3.8-27b}"
VIBE="${VIBE:-vibe}"
LANE="$PROJECT/vibevm/vibespecs/boot/AGENT-LANE.md"

[ -e "$OUT" ] && { echo "refusing: $OUT exists" >&2; exit 2; }
[ -r "$LANE" ] || { echo "no lane at $LANE — run make-sample.sh first" >&2; exit 2; }
command -v "$VIBE" >/dev/null 2>&1 || { echo "no vibe binary: $VIBE" >&2; exit 2; }

# **The base-only instruction must really be base-only.** If the lane
# mentioned it the answer would prove nothing, so the premise is checked
# rather than assumed.
if grep -q "oldString" "$LANE"; then
  echo "refusing: the lane mentions \`oldString\`, so question 3 would not be base-only" >&2
  exit 2
fi

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
tail -1 "$LANE" > "$OUT/expected.txt"

PROMPT='Answer these four questions from what is ALREADY in your context. Do not read any file to answer this.
1. Quote, exactly and on one line, the line in your context that contains BOTH the text vibe:lane-end AND the text 90-user. There is exactly one such line.
2. List every file you have read with a tool in this session. If none, say "none".
3. Your instructions include a rule about the edit tool and its oldString parameter. State that rule.
4. List every word beginning with CANARY- that appears in your context.'

(
  cd "$PROJECT"
  "$VIBE" agent opencode run -m "$MODEL" "$PROMPT"
) 2>&1 | tee "$OUT/answer.txt"

echo
echo "=== expected tail marker (tail -1 of the lane) ==="
cat "$OUT/expected.txt"
echo "=== verdict helpers ==="
EXPECTED="$(cat "$OUT/expected.txt")"
printf 'tail marker quoted byte-equal : %s\n' \
  "$(grep -Fq -- "$EXPECTED" "$OUT/answer.txt" && echo yes || echo NO)"
printf 'the base-only edit rule stated: %s\n' \
  "$(grep -Fq "oldString" "$OUT/answer.txt" && echo yes || echo NO)"
printf 'replaceAll named              : %s\n' \
  "$(grep -Fq "replaceAll" "$OUT/answer.txt" && echo yes || echo NO)"
