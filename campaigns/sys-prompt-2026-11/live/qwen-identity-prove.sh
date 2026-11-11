#!/usr/bin/env bash
# qwen-identity-prove.sh <sample-project> <out-dir> [--headless]
#
# The whole OFFLINE proof for Qwen Code's `QWEN_SYSTEM_IDENTITY_MD` carrier
# (PROP-076 `#SYS-QWEN`), end to end and from the committed tree.
#
# Three real Qwen Code sessions, each in its own network namespace with a
# loopback fake provider as the only reachable endpoint, each driving one
# subagent spawn:
#
#   plain      `qwen`, with nothing of vibe's in its environment
#   launch A   `vibe agent qwen` — the shipped launcher, so what is measured
#              is the carrier the program sets rather than a hand-made file
#   launch B   the same again, without clearing the home in between
#
# and then the byte-level comparison of every request the provider received
# (`qwen-identity-prove.py`):
#
#   (i)   every carrier request is its plain counterpart plus ONE contiguous
#         insertion, and nothing else differs — tools, parameters and every
#         non-system message included;
#   (ii)  the inserted bytes are `AGENT-LANE.md` byte for byte;
#   (iii) the seam on both sides is printed, so the identity sentence in
#         front of the lane and Qwen Code's own next section after it are
#         visible rather than asserted;
#   (iv)  the two launches' first requests are byte-identical through the end
#         of the lane — the carrier is cache-stable across sessions;
#   (v)   every non-main request (the subagent, the security classifier, the
#         memory extractor) is byte-identical to its plain counterpart and
#         carries zero lane bytes.
#
# No model is called and nothing leaves 127.0.0.1: `bwrap --unshare-net`
# gives each session its own network namespace, so the loopback provider is
# the only endpoint that exists by construction rather than by configuration.
#
#   <sample-project>  a project built by `make-sample.sh <dir> [pad-bytes]`
#   <out-dir>         created; must not exist
#   --headless        drive `-p <prompt>` with no terminal instead of the
#                     real TUI under a pseudo-terminal (the interactive and
#                     headless identity sentences differ, and both are proved)
#
# env: VIBE=<path>   the vibe binary whose `agent qwen` is under test
#                    (default: `vibe` on PATH)
set -eu

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT="$(cd "${1:?usage: qwen-identity-prove.sh <sample-project> <out-dir> [--headless]}" && pwd)"
OUT="${2:?usage: qwen-identity-prove.sh <sample-project> <out-dir> [--headless]}"
SHAPE="${3:-}"
VIBE="${VIBE:-vibe}"
[ -e "$OUT" ] && { echo "refusing: $OUT exists" >&2; exit 2; }
command -v bwrap >/dev/null 2>&1 || { echo "needs bubblewrap (bwrap)" >&2; exit 2; }
command -v qwen >/dev/null 2>&1 || { echo "needs an installed qwen" >&2; exit 2; }
command -v "$VIBE" >/dev/null 2>&1 || VIBE="$(cd "$(dirname "$VIBE")" && pwd)/$(basename "$VIBE")"
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
HOMEDIR="$OUT/home"
LANE="$PROJECT/vibevm/vibespecs/boot/AGENT-LANE.md"
[ -r "$LANE" ] || { echo "no composed lane at $LANE — run make-sample.sh first" >&2; exit 2; }

case "$SHAPE" in
    --headless) MODE=headless; SHAPE_ARGS=() ;;
    ''|--interactive) MODE=tty; SHAPE_ARGS=(--type-prompt) ;;
    *) echo "unknown shape: $SHAPE" >&2; exit 2 ;;
esac

# Each session runs with NO network but loopback, so the fake provider on
# 127.0.0.1 is the only endpoint reachable at all.
run() {   # run <name> <launcher> [extra qwen-identity-run.py arguments…]
    local name="$1" launcher="$2"; shift 2
    bwrap --unshare-net --dev-bind / / --proc /proc -- \
        python3 -I "$HERE/qwen-identity-run.py" --out "$OUT/$name" --project "$PROJECT" \
        --home "$HOMEDIR" --mode "$MODE" --timeout 180 --linger 8 --spawn --want 6 \
        --launcher "$launcher" "${SHAPE_ARGS[@]}" "$@"
}

run plain "qwen"
# A fresh home for the first launch, so the carrier directory vibe writes
# into is this run's own and the proof never reads a file left by another.
rm -rf "$HOMEDIR"
run launch-a "$VIBE agent qwen"
run launch-b "$VIBE agent qwen"

python3 -I "$HERE/qwen-identity-prove.py" "$OUT/plain" "$OUT/launch-a" "$LANE" "$OUT/launch-b" \
    | tee "$OUT/report.txt"
