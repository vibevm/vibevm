#!/usr/bin/env bash
# tcg-bench — the automated agent battery (AGENTIC-TCG-TS-PLAN v0.1, D7).
#
# Drives the opencode CLI agent headlessly over the task set in tasks/,
# one fresh throwaway copy of research/ts-demo per task, then verifies
# the result MECHANICALLY (tsc / node --test / conform-typescript) and
# appends one JSON line per task to the results file. The agent under
# test is the WEAK-model population the tcg line targets (DR1-015).
#
# Arms:
#   control     — tools withheld (the pre-oracle baseline; runnable today)
#   with-tools  — the tcg_* surface named in the prompt (requires the
#                 Phase-3 tcg-typescript artifact; refuses until built)
#
# Usage:
#   bash run-battery.sh [--arm control|with-tools] [--model <id>]
#                       [--tasks "<glob>"] [--timeout <s>] [--keep-work]
#
# Requirements: Git Bash, node >= 22.6, the opencode CLI on PATH (or at
# the fallback path below) with OpenRouter auth configured, and
# research/ts-demo/node_modules installed (npm install once).
#
# Machine quirks honoured: mklink needs verbatim-free absolute Windows
# paths (cygpath -w); node --test needs explicit file lists, never bare
# dirs; real exit codes are captured per step, never piped away.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEMO="$REPO_ROOT/research/ts-demo"
TASKS_DIR="$SCRIPT_DIR/tasks"
WORK_ROOT="$SCRIPT_DIR/work"
REPORTS_DIR="$SCRIPT_DIR/reports"

ARM="control"
MODEL="openrouter/openai/gpt-oss-20b:free"
TASK_GLOB="*.md"
TIMEOUT_S=300
KEEP_WORK=0
OPENCODE_FALLBACK='C:\opt\nvm\v24.18.0\node_modules\opencode-ai\bin\opencode.exe'

while [ $# -gt 0 ]; do
  case "$1" in
    --arm) ARM="$2"; shift 2 ;;
    --model) MODEL="$2"; shift 2 ;;
    --tasks) TASK_GLOB="$2"; shift 2 ;;
    --timeout) TIMEOUT_S="$2"; shift 2 ;;
    --keep-work) KEEP_WORK=1; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

# --- toolchain probes (hard failures with recipes, never silent skips) ---
OPENCODE="opencode"
if ! command -v opencode >/dev/null 2>&1; then
  if [ -x "$(cygpath -u "$OPENCODE_FALLBACK" 2>/dev/null)" ]; then
    OPENCODE="$(cygpath -u "$OPENCODE_FALLBACK")"
  else
    echo "FATAL: opencode not on PATH and fallback missing ($OPENCODE_FALLBACK)" >&2
    exit 3
  fi
fi
command -v node >/dev/null 2>&1 || { echo "FATAL: node not on PATH" >&2; exit 3; }
[ -d "$DEMO/node_modules" ] || {
  echo "FATAL: $DEMO/node_modules missing - run: cd research/ts-demo && npm install" >&2
  exit 3
}

CONFORM_BIN="$REPO_ROOT/vibedeps/stack-typescript-ai-native/0.3.0/target/release/conform-typescript.exe"
if [ ! -x "$CONFORM_BIN" ]; then
  echo "note: conform artifact missing; building via vibe bin build (org.vibevm, consented)" >&2
  (cd "$REPO_ROOT" && cargo run -q -p vibe-cli -- bin build conform-typescript) || {
    echo "FATAL: could not build conform-typescript" >&2
    exit 3
  }
fi

TCG_BIN="$REPO_ROOT/vibedeps/stack-typescript-ai-native"/*/target/release/tcg-typescript.exe
if [ "$ARM" = "with-tools" ]; then
  # shellcheck disable=SC2086
  set -- $TCG_BIN
  [ -x "${1:-/nonexistent}" ] || {
    echo "FATAL: --arm with-tools needs the Phase-3 tcg-typescript artifact (vibe bin build tcg-typescript)" >&2
    exit 3
  }
fi

mkdir -p "$WORK_ROOT" "$REPORTS_DIR"
STAMP="$(date +%Y-%m-%d-%H%M)"
RESULTS="$REPORTS_DIR/${ARM}-${STAMP}.jsonl"
: > "$RESULTS"

echo "== tcg-bench battery =="
echo "arm=$ARM model=$MODEL timeout=${TIMEOUT_S}s"
echo "opencode=$("$OPENCODE" --version 2>/dev/null) node=$(node --version)"
echo "results=$RESULTS"
echo

json_escape() { # minimal escaper for values we embed
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr -d '\n\r'
}

# Junctions via PowerShell: mklink under Git Bash loses its /J switch to
# MSYS path conversion. And NEVER rm -rf a tree that still contains the
# junction — unlink the point first ([IO.Directory]::Delete removes the
# link, not the target), or the demo's real node_modules is at risk.
make_junction() { # <link> <target> (POSIX paths)
  powershell.exe -NoProfile -Command \
    "New-Item -ItemType Junction -Path '$(cygpath -w "$1")' -Target '$(cygpath -w "$2")' | Out-Null"
}
remove_work() { # <work dir>
  if [ -d "$1/node_modules" ]; then
    powershell.exe -NoProfile -Command \
      "[System.IO.Directory]::Delete('$(cygpath -w "$1/node_modules")')" 2>/dev/null
  fi
  rm -rf "$1"
}

TOOLS_BLOCK="Tools available for this task: run
  \"$REPO_ROOT/vibedeps/stack-typescript-ai-native/0.4.0/target/release/tcg-typescript.exe\" validate <file> --json
to type-check a file (with discipline findings) BEFORE writing it to disk is final, and
  ... scope <file> / complete <file> --position L:C / type <file> --position L:C
for in-scope symbols, type-valid completions, and expression types. Consult them before and after each edit."

shopt -s nullglob
TASKS=("$TASKS_DIR"/$TASK_GLOB)
[ ${#TASKS[@]} -gt 0 ] || { echo "FATAL: no tasks match $TASK_GLOB" >&2; exit 2; }

pass_n=0
fail_n=0

for task_file in "${TASKS[@]}"; do
  task_id="$(basename "$task_file" .md)"
  work="$WORK_ROOT/$ARM-$task_id"
  remove_work "$work"
  mkdir -p "$work"

  # fresh copy of the demo, sans heavy/derived dirs
  tar -C "$DEMO" \
      --exclude=node_modules --exclude=vibedeps --exclude=.vibe \
      --exclude=target --exclude=discipline/health \
      -cf - . | tar -C "$work" -xf -

  # junction the demo's node_modules (PowerShell; see make_junction note)
  make_junction "$work/node_modules" "$DEMO/node_modules" || {
    echo "[$task_id] FATAL: junction failed" >&2
    continue
  }

  prompt="$(cat "$task_file")"
  if [ "$ARM" = "with-tools" ]; then
    prompt="$prompt

$TOOLS_BLOCK"
  fi

  echo "[$task_id] agent run..."
  t0=$(date +%s)
  (
    cd "$work" && timeout "${TIMEOUT_S}s" \
      "$OPENCODE" run -m "$MODEL" --auto --format json "$prompt" \
      > agent.jsonl 2> agent.err
  )
  agent_exit=$?
  t1=$(date +%s)
  wall=$((t1 - t0))

  steps=$(grep -c '"type":"step_finish"' "$work/agent.jsonl" 2>/dev/null || true)
  tool_calls=$(grep -c '"type":"tool"' "$work/agent.jsonl" 2>/dev/null || true)

  # --- mechanical verification (each exit captured for real; --pretty
  # false / TAP keep the outputs ANSI-free so the counters actually count) ---
  (cd "$work" && ./node_modules/.bin/tsc --noEmit --pretty false > tsc.out 2>&1)
  tsc_exit=$?
  tsc_errors=$(grep -c "error TS" "$work/tsc.out" || true)
  tsc_halluc=$(grep -cE "error TS(2304|2552|2339)" "$work/tsc.out" || true)

  mapfile -t test_files < <(cd "$work" && find src -name "*.test.ts" | sort)
  if [ ${#test_files[@]} -gt 0 ]; then
    (cd "$work" && node --test --test-reporter tap "${test_files[@]}" > tests.out 2>&1)
    tests_exit=$?
    tests_pass=$(grep -oE "^# pass [0-9]+" "$work/tests.out" | grep -oE "[0-9]+" | tail -1)
    tests_fail=$(grep -oE "^# fail [0-9]+" "$work/tests.out" | grep -oE "[0-9]+" | tail -1)
  else
    tests_exit=99; tests_pass=0; tests_fail=0
  fi

  "$CONFORM_BIN" check --path "$work" > "$work/conform.out" 2>&1
  conform_exit=$?
  conform_new=$(grep -ciE "new finding|not in the baseline" "$work/conform.out" || true)

  # a task PASSES mechanically when: agent finished, types are clean,
  # all tests pass, and conform introduced nothing new
  verdict="FAIL"
  if [ "$agent_exit" -eq 0 ] && [ "$tsc_exit" -eq 0 ] && [ "$tests_exit" -eq 0 ] \
     && [ "$conform_exit" -eq 0 ]; then
    verdict="PASS"; pass_n=$((pass_n + 1))
  else
    fail_n=$((fail_n + 1))
  fi

  printf '{"task":"%s","arm":"%s","model":"%s","verdict":"%s","agent_exit":%d,"wall_s":%d,"steps":%s,"tool_calls":%s,"tsc_exit":%d,"tsc_errors":%s,"tsc_hallucination":%s,"tests_exit":%d,"tests_pass":%s,"tests_fail":%s,"conform_exit":%d,"conform_new":%s}\n' \
    "$(json_escape "$task_id")" "$ARM" "$(json_escape "$MODEL")" "$verdict" \
    "$agent_exit" "$wall" "${steps:-0}" "${tool_calls:-0}" \
    "$tsc_exit" "${tsc_errors:-0}" "${tsc_halluc:-0}" \
    "$tests_exit" "${tests_pass:-0}" "${tests_fail:-0}" \
    "$conform_exit" "${conform_new:-0}" >> "$RESULTS"

  echo "[$task_id] $verdict (agent=$agent_exit wall=${wall}s tsc=$tsc_errors errs/${tsc_halluc} halluc tests=${tests_pass:-0}p/${tests_fail:-0}f conform=$conform_exit)"

  [ "$KEEP_WORK" -eq 1 ] || remove_work "$work"
  sleep 3   # be polite to the free tier
done

echo
echo "== summary: $pass_n PASS / $fail_n FAIL of $((pass_n + fail_n)) =="
echo "results: $RESULTS"
