#!/usr/bin/env bash
# MT-C3-02 advisor help/hurt trial runner: one cold GLM-served caller session
# over the staged mini_logfmt repo, consulting (advised arm) or not consulting
# (alone arm) a stronger advisor via `fractality advise`. Thin launcher by the
# language law; the experiment design lives in MT-C3-02-advisor-help-hurt.md.
#
#   run-advise.sh alone|advised <run-number>
#
# Arm alone:   a glm-5-turbo caller works the uncertain-task menu with no
#              advisor — the baseline quality the weaker caller reaches alone.
# Arm advised: the SAME glm-5-turbo caller, its preamble instructing it to
#              consult `fractality advise` (routed to the big/glm-5.2 rung by
#              the fabric) before committing each uncertain task.
#
# The caller is the top-level agent, served by the SMALL model. The advisor's
# model is NOT set here — the fabric's pod routes it from the advice packet's
# routing.model=big, so the caller-vs-advisor capability gap is the fabric's
# job, not this launcher's.
#
# Never echoes secret values (set +x is load-bearing).
set -euo pipefail
set +x

ARM="${1:?arm alone|advised}"
N="${2:?run number}"
case "$ARM" in
  alone|advised) ;;
  *) echo "arm must be 'alone' or 'advised', got '$ARM'" >&2; exit 2 ;;
esac
TRIAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$(cd "$TRIAL_DIR/../../.." && pwd)"
EXE="$WS/target/debug/fractality.exe"
OUT="$WS/target/trial-results/advise-$ARM-run-$N"
SCRATCH="$(mktemp -d)/trial-advise-$ARM-$N"
HOME_DIR="$SCRATCH/home"
PROJ="$SCRATCH/proj"

[ -x "$EXE" ] || { echo "build first: cargo build --workspace" >&2; exit 2; }
mkdir -p "$OUT" "$HOME_DIR" "$SCRATCH/cc-config" "$SCRATCH/userhome"

# --- staging repo (worktree-mode packets need git + main + clean tree)
cp -r "$TRIAL_DIR/staging" "$PROJ"
# The menu is the arm's preamble (alone vs advised) followed by the
# uncertain-task menu. The preamble is the ONLY thing that differs between
# arms — the delta isolates the advice effect, not a model difference.
cat "$TRIAL_DIR/preamble-$ARM.md" "$TRIAL_DIR/menu-advise.md" > "$SCRATCH/menu.md"
(cd "$PROJ" && git init -q -b main && git add -A \
  && git -c user.name=trial -c user.email=trial@local commit -qm "staging baseline")

# --- profiles + provider facts (values never printed)
cp ~/.fractality/profiles.toml "$HOME_DIR/profiles.toml"
eval "$(python - "$HOME_DIR/profiles.toml" <<'PY'
import sys, tomllib, pathlib
p = tomllib.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
glm = p["profile"]["glm"]
token_file = pathlib.Path(glm["token_file"].replace("~", str(pathlib.Path.home()), 1))
print(f'BASE_URL="{glm["base_url"]}"')
print(f'BIG_ID="{glm["models"]["big"]}"')
print(f'SMALL_ID="{glm["models"]["small"]}"')
print(f'TOKEN_FILE="{token_file}"')
PY
)"
TOKEN="$(cat "$TOKEN_FILE")"

# --- the fabric (must be up so `fractality advise` works in the advised arm)
export FRACTALITY_HOME="$HOME_DIR"
"$EXE" mc start >/dev/null

# --- Rust toolchain passthrough (DEF-C2-2a, F24; verified 2026-07-10).
# Without these, env -i breaks the caller's AND the workers' cargo twice
# over: the rustup shim cannot resolve a toolchain under the scratch
# USERPROFILE, and rustc's MSVC auto-detect (vswhere lives under
# ProgramFiles(x86)) silently falls back to Git's GNU link.exe, which
# cannot link test binaries. The trial measured both bites; values are
# paths only, never secrets. Other boxes may need more — extend here.
RUSTUP_HOME_W="$(cygpath -w "$HOME/.rustup")"
CARGO_HOME_W="$(cygpath -w "$HOME/.cargo")"
PF86="$(printenv 'ProgramFiles(x86)' || echo 'C:\Program Files (x86)')"

# --- the cold caller: worker-shaped clean env (I1 style), menu on stdin.
# The CALLER is the small/weak tier; OPUS+SONNET both pin to SMALL_ID so an
# internal tier escalation inside the caller stays on the weak model.
echo "arm=$ARM run=$N caller=$SMALL_ID proj=$PROJ home=$HOME_DIR" | tee "$OUT/run-info.txt"
START_TS=$(date +%s)
set +e
(cd "$PROJ" && env -i \
    PATH="$WS/target/debug:$PATH" \
    SYSTEMROOT="${SYSTEMROOT:-C:\\Windows}" \
    COMSPEC="${COMSPEC:-C:\\Windows\\system32\\cmd.exe}" \
    TEMP="$SCRATCH" TMP="$SCRATCH" \
    USERPROFILE="$(cygpath -w "$SCRATCH/userhome")" \
    HOME="$SCRATCH/userhome" \
    RUSTUP_HOME="$RUSTUP_HOME_W" \
    CARGO_HOME="$CARGO_HOME_W" \
    PROGRAMFILES="${PROGRAMFILES:-C:\\Program Files}" \
    "ProgramFiles(x86)=$PF86" \
    PROGRAMDATA="${PROGRAMDATA:-C:\\ProgramData}" \
    SYSTEMDRIVE="${SYSTEMDRIVE:-C:}" \
    PROCESSOR_ARCHITECTURE="${PROCESSOR_ARCHITECTURE:-AMD64}" \
    NUMBER_OF_PROCESSORS="${NUMBER_OF_PROCESSORS:-8}" \
    windir="${WINDIR:-C:\\Windows}" \
    CLAUDE_CONFIG_DIR="$(cygpath -w "$SCRATCH/cc-config")" \
    ANTHROPIC_BASE_URL="$BASE_URL" \
    ANTHROPIC_AUTH_TOKEN="$TOKEN" \
    ANTHROPIC_DEFAULT_OPUS_MODEL="$SMALL_ID" \
    ANTHROPIC_DEFAULT_SONNET_MODEL="$SMALL_ID" \
    ANTHROPIC_DEFAULT_HAIKU_MODEL="$SMALL_ID" \
    FRACTALITY_HOME="$HOME_DIR" \
    timeout 1500 claude --print \
      --output-format stream-json --verbose \
      --model "$SMALL_ID" \
      --permission-mode acceptEdits \
      --max-turns 100 \
      --allowed-tools Bash Edit Write Read Glob Grep \
      < "$SCRATCH/menu.md" \
      > "$OUT/boss-transcript.jsonl" 2> "$OUT/boss-stderr.log")
BOSS_EXIT=$?
set -e
echo "boss_exit=$BOSS_EXIT wall_secs=$(( $(date +%s) - START_TS ))" | tee -a "$OUT/run-info.txt"

# --- collect the bus facts (runs.json carries the advice-marked advisor
# runs the scorer counts for PR-adv-2), then stop the daemon.
"$EXE" ps --json      > "$OUT/runs.json"        || true
"$EXE" session ls     > "$OUT/sessions.txt"     || true
"$EXE" stats --json   > "$OUT/stats.json"       || true
"$EXE" scoreboard     > "$OUT/scoreboard.txt"   || true
"$EXE" escalations --json > "$OUT/escalations.json" || true
"$EXE" tree --json    > "$OUT/forest.json"      || true
"$EXE" decisions --json > "$OUT/decisions.json" || true
"$EXE" mc stop >/dev/null || true

RUNS=$(python -c "import json,sys;print(len(json.load(open(sys.argv[1],encoding='utf-8'))))" "$OUT/runs.json" 2>/dev/null || echo "?")
# proj-final is the caller's task output — the scorer drops the hidden
# acceptance tests into its tests/ and runs cargo test there.
cp -r "$PROJ" "$OUT/proj-final" 2>/dev/null || true
echo "RESULT arm=$ARM run=$N boss_exit=$BOSS_EXIT mc_runs=$RUNS out=$OUT"
