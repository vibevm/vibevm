#!/usr/bin/env bash
# Condition (vi) of `PROP-076#SYS-QWEN` — the one a byte diff cannot make.
#
# `qwen-identity-prove.sh` establishes, in bytes, that the carrier request is
# the plain request plus the lane and nothing else. It cannot establish that
# the delivered bytes **work**: a lane that arrived byte-perfect and is then
# ignored, and a harness base prompt that arrived intact and is then
# displaced, look identical on the wire. So one live session is asked four
# things at once:
#
#   1. quote the delivered tail marker — did the lane arrive?
#
#      **The question names the marker's own file, and that is a finding
#      rather than a convenience.** Qwen Code injects `AGENTS.md` as a
#      context-file layer AFTER the base prompt, and that file's managed
#      block contains an indented *format example* of a `vibe:lane-end`
#      line. So "the last line containing vibe:lane-end" is genuinely the
#      example, and a model that answers it that way is right — measured on
#      the local 27B model in the `read` tier. The question therefore asks
#      for the line that also names `90-user`, which only the delivered
#      marker does.
#   2. which files did you read with a tool — is the router's rule 1 held,
#      i.e. did the lane arrive *as instructions* rather than as something to
#      go and fetch?
#   3. state what text inside a `<qwen:user-prompt-submit-context>` tag is —
#      a rule that exists **only** in Qwen Code's own base prompt and nowhere
#      in the lane, and the very first sentence AFTER the seam. If this comes
#      back right, the harness's own prompt survived the insertion.
#   4. list the `CANARY-` words — did the whole lane arrive, tail included?
#   5. quote the first sentence of your own instructions — the carrier's own
#      paragraph, which must come back as Qwen Code's OWN sentence and not as
#      anything of vibe's.
#
# **vibe's own stderr is kept in a separate file**, and the greps read only
# the model's answer: the launcher's notes name the carrier and the harness
# themselves, so a combined stream would let this check pass on vibe's words
# rather than on the model's.
#
# **Cost rule, owner 2026-11-11.** A live session uses the local model and the
# **tiny** sample; every real-size claim is proved offline, where a 200 KB
# lane costs nothing. The model comes from the operator's own `~/.qwen`
# configuration, which is **read and not written**.
#
# It runs through `vibe agent qwen`, not by hand: the claim under test is what
# the LAUNCHER delivers, so the launcher has to be the thing that delivers it.
#
# usage: qwen-identity-live-check.sh <sample-project> <out-dir> [model]
#
#   <sample-project>  a project made by `make-sample.sh` with NO padding
#   <out-dir>         created; holds `answer.txt` and `expected.txt`
#   [model]           passed as `-m <model>` when given; omitted otherwise,
#                     so the operator's own configured model is used
#
# env: VIBE   the vibe binary (default: `vibe` on PATH)

set -eu

PROJECT="$(cd "${1:?usage: qwen-identity-live-check.sh <sample-project> <out-dir> [model]}" && pwd)"
OUT="${2:?usage: qwen-identity-live-check.sh <sample-project> <out-dir> [model]}"
MODEL="${3:-}"
VIBE="${VIBE:-vibe}"
LANE="$PROJECT/vibevm/vibespecs/boot/AGENT-LANE.md"

[ -e "$OUT" ] && { echo "refusing: $OUT exists" >&2; exit 2; }
[ -r "$LANE" ] || { echo "no lane at $LANE — run make-sample.sh first" >&2; exit 2; }
command -v "$VIBE" >/dev/null 2>&1 || { echo "no vibe binary: $VIBE" >&2; exit 2; }

# **The base-only question must really be base-only.** If the lane mentioned
# the tag the answer would prove nothing, so the premise is checked rather
# than assumed.
if grep -q "user-prompt-submit-context" "$LANE"; then
  echo "refusing: the lane mentions the tag, so question 3 would not be base-only" >&2
  exit 2
fi

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
tail -1 "$LANE" > "$OUT/expected.txt"

PROMPT='Answer these five questions from what is ALREADY in your context. Do not read any file to answer this.
1. Quote, exactly and on one line, the line in your context that contains BOTH the text vibe:lane-end AND the text 90-user. There is exactly one such line.
2. List every file you have read with a tool in this session. If none, say "none".
3. Your instructions say what text inside a <qwen:user-prompt-submit-context> tag is. State it.
4. List every word beginning with CANARY- that appears in your context.
5. Quote the first sentence of your own instructions.'

ARGS=(agent qwen)
[ -n "$MODEL" ] && ARGS+=(-m "$MODEL")
ARGS+=(-p "$PROMPT")

(
  cd "$PROJECT"
  "$VIBE" "${ARGS[@]}"
) 2>"$OUT/launch.txt" | tee "$OUT/answer.txt"

echo
echo "=== what the launcher said (vibe's own stderr) ==="
cat "$OUT/launch.txt"
echo "=== expected tail marker (tail -1 of the lane) ==="
cat "$OUT/expected.txt"
echo "=== verdict helpers ==="
EXPECTED="$(cat "$OUT/expected.txt")"
printf 'tail marker quoted byte-equal      : %s\n' \
  "$(grep -Fq -- "$EXPECTED" "$OUT/answer.txt" && echo yes || echo NO)"
printf 'the base-only hook rule stated     : %s\n' \
  "$(grep -Fq "UserPromptSubmit" "$OUT/answer.txt" && echo yes || echo NO)"
printf 'the identity sentence came back    : %s\n' \
  "$(grep -Fq "You are Qwen Code" "$OUT/answer.txt" && echo yes || echo NO)"
for canary in CANARY-LANE-HEAD-OBSIDIAN CANARY-LANE-TAIL-PUMICE \
              CANARY-INDEX-CORE-BASALT CANARY-INDEX-USER-GNEISS; do
  printf '%-34s : %s\n' "$canary" \
    "$(grep -Fq "$canary" "$OUT/answer.txt" && echo yes || echo NO)"
done
