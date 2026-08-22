#!/usr/bin/env bash
set -euo pipefail

XMLMEASURE_ROOT=${XMLMEASURE_ROOT:-/c/Users/olegc/git/v/vibevm-xmlmeasure}
RUNS=$XMLMEASURE_ROOT/runs
OUTPUT=$RUNS/tiers.txt
MODELS=(gpt-5.6-sol gpt-5.5 gpt-5.5-codex gpt-5.1-codex-mini codex-mini-latest)

mkdir -p "$RUNS"
: > "$OUTPUT"

for model in "${MODELS[@]}"; do
  safe_model=${model//[^A-Za-z0-9._-]/_}
  probe_log=$RUNS/.tier-$safe_model.log
  set +e
  timeout 120s env \
    CODEXRUNNER_MODEL="$model" \
    CODEXRUNNER_EFFORT=low \
    "$HOME/opt/bin/codexrunner" exec "Ответь одним словом: ок" </dev/null >"$probe_log" 2>&1
  rc=$?
  set -e

  brief=$(tr '\r\n\t' '   ' < "$probe_log" | sed 's/[[:space:]][[:space:]]*/ /g' | cut -c1-180)
  [[ -n "$brief" ]] || brief="no output; rc=$rc"
  if (( rc == 0 )); then
    status=served
  else
    status=error
    brief="rc=$rc $brief"
  fi
  printf '%s %s %s\n' "$model" "$status" "$brief" >> "$OUTPUT"
  rm -f "$probe_log"
done

cat "$OUTPUT"
