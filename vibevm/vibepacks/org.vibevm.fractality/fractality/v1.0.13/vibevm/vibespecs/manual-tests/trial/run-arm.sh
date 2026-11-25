#!/usr/bin/env bash
# Ф6 trial runner (C2: MT-C2-01/-04; C3: MT-C3-01): one cold GLM-served
# boss session over the staged mini_logfmt repo. Thin launcher by the
# language law; the experiment design lives in the MT documents.
#
#   run-arm.sh a|b|g <run-number>
#
# Arm a: snippet-in-CLAUDE.md only (the C2 Ф6 baseline arm).
# Arm b: same + `fractality harness install` (hooks + statusline).
# Arm g: C3 gated arm (MT-C3-01) — the menu is prefixed with the RLM
#        preamble (`preamble-g.md`); the decision journal + escalations
#        are collected too.
#
# Never echoes secret values (set +x is load-bearing).
set -euo pipefail
set +x

ARM="${1:?arm a|b}"
N="${2:?run number}"
TRIAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$(cd "$TRIAL_DIR/../../.." && pwd)"
EXE="$WS/target/debug/fractality.exe"
OUT="$WS/target/trial-results/arm-$ARM-run-$N"
SCRATCH="$(mktemp -d)/trial-$ARM-$N"
HOME_DIR="$SCRATCH/home"
PROJ="$SCRATCH/proj"

[ -x "$EXE" ] || { echo "build first: cargo build --workspace" >&2; exit 2; }
mkdir -p "$OUT" "$HOME_DIR" "$SCRATCH/cc-config" "$SCRATCH/userhome"

# --- staging repo (worktree-mode packets need git + main + clean tree)
cp -r "$TRIAL_DIR/staging" "$PROJ"
# Arm g (MT-C3-01) prefixes the RLM preamble so the boss is told to reach
# for the need-gate verbs (C2 F23: a -p boss must be instructed to use
# them); other arms get the bare menu.
if [ "$ARM" = "g" ]; then
  cat "$TRIAL_DIR/preamble-g.md" "$TRIAL_DIR/menu.md" > "$SCRATCH/menu.md"
elif [ "$ARM" = "g2" ]; then
  cat "$TRIAL_DIR/preamble-g.md" "$TRIAL_DIR/menu.md" "$TRIAL_DIR/menu-g2-extra.md" > "$SCRATCH/menu.md"
else
  cp "$TRIAL_DIR/menu.md" "$SCRATCH/menu.md"
fi
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

# --- the fabric
export FRACTALITY_HOME="$HOME_DIR"
"$EXE" mc start >/dev/null

if [ "$ARM" = "b" ]; then
  "$EXE" harness install claude-code --target "$PROJ"
  "$EXE" harness status claude-code --target "$PROJ" | tee "$OUT/harness-status.txt"
fi

# --- Rust toolchain passthrough (DEF-C2-2a, F24; verified 2026-07-10).
# Without these, env -i breaks the boss's AND the workers' cargo twice
# over: the rustup shim cannot resolve a toolchain under the scratch
# USERPROFILE, and rustc's MSVC auto-detect (vswhere lives under
# ProgramFiles(x86)) silently falls back to Git's GNU link.exe, which
# cannot link test binaries. The trial measured both bites; values are
# paths only, never secrets. Other boxes may need more — extend here.
RUSTUP_HOME_W="$(cygpath -w "$HOME/.rustup")"
CARGO_HOME_W="$(cygpath -w "$HOME/.cargo")"
PF86="$(printenv 'ProgramFiles(x86)' || echo 'C:\Program Files (x86)')"

# --- the cold boss: worker-shaped clean env (I1 style), menu on stdin
echo "arm=$ARM run=$N boss=$BIG_ID proj=$PROJ home=$HOME_DIR" | tee "$OUT/run-info.txt"
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
    ANTHROPIC_DEFAULT_OPUS_MODEL="$BIG_ID" \
    ANTHROPIC_DEFAULT_SONNET_MODEL="$BIG_ID" \
    ANTHROPIC_DEFAULT_HAIKU_MODEL="$SMALL_ID" \
    FRACTALITY_HOME="$HOME_DIR" \
    timeout 1500 claude --print \
      --output-format stream-json --verbose \
      --model "$BIG_ID" \
      --permission-mode acceptEdits \
      --max-turns 100 \
      --allowed-tools Bash Edit Write Read Glob Grep \
      < "$SCRATCH/menu.md" \
      > "$OUT/boss-transcript.jsonl" 2> "$OUT/boss-stderr.log")
BOSS_EXIT=$?
set -e
echo "boss_exit=$BOSS_EXIT wall_secs=$(( $(date +%s) - START_TS ))" | tee -a "$OUT/run-info.txt"

# --- collect the bus facts, then stop the daemon
"$EXE" ps --json      > "$OUT/runs.json"        || true
"$EXE" session ls     > "$OUT/sessions.txt"     || true
"$EXE" stats --json   > "$OUT/stats.json"       || true
"$EXE" scoreboard     > "$OUT/scoreboard.txt"   || true
"$EXE" escalations --json > "$OUT/escalations.json" || true
"$EXE" tree --json    > "$OUT/forest.json"      || true
"$EXE" decisions --json > "$OUT/decisions.json" || true
"$EXE" mc stop >/dev/null || true

RUNS=$(python -c "import json,sys;print(len(json.load(open(sys.argv[1],encoding='utf-8'))))" "$OUT/runs.json" 2>/dev/null || echo "?")
cp -r "$PROJ" "$OUT/proj-final" 2>/dev/null || true
echo "RESULT arm=$ARM run=$N boss_exit=$BOSS_EXIT mc_runs=$RUNS out=$OUT"
