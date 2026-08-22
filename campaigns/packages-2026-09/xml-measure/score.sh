#!/usr/bin/env bash
set -euo pipefail

XMLMEASURE_ROOT=${XMLMEASURE_ROOT:-/c/Users/olegc/git/v/vibevm-xmlmeasure}
RUNS=$XMLMEASURE_ROOT/runs
CSV=$RUNS/scores.csv
SUMMARY=$RUNS/summary.txt

[[ -d "$RUNS" ]] || { echo "runs directory not found: $RUNS" >&2; exit 1; }
printf 'stamp,lane,model,effort,variant,q1,q2,q3neg\n' > "$CSV"

meta_value() {
  local key=$1
  local file=$2
  sed -n "s/^${key}=//p" "$file" | head -n 1
}

for run_dir in "$RUNS"/*; do
  [[ -d "$run_dir" && -f "$run_dir/meta.txt" && -f "$run_dir/transcript.log" ]] || continue

  answers=$(tr -d '\r' < "$run_dir/transcript.log" | grep -E '^[[:space:]]*[123]:' || true)
  line1=$(printf '%s\n' "$answers" | grep -E '^[[:space:]]*1:' | tail -n 1 || true)
  line2=$(printf '%s\n' "$answers" | grep -E '^[[:space:]]*2:' | tail -n 1 || true)

  q1=FAIL
  q2=FAIL
  q3neg=FAIL
  [[ "$line1" == *ROUTER-W1-e5b19c* && "$line1" == *router-win* ]] && q1=PASS
  [[ "$line2" == *ROUTER-A2-93d07f* && "$line2" == *router-aux* ]] && q2=PASS
  [[ "$answers" != *ROUTER-L3-48c2ab* ]] && q3neg=PASS

  meta=$run_dir/meta.txt
  stamp=$(meta_value stamp "$meta")
  lane=$(meta_value lane "$meta")
  model=$(meta_value model "$meta")
  effort=$(meta_value effort "$meta")
  variant=$(meta_value variant "$meta")
  printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$stamp" "$lane" "$model" "$effort" "$variant" "$q1" "$q2" "$q3neg" >> "$CSV"
done

{
  printf '%-10s %-24s %-10s %8s %8s %8s\n' lane model variant q1 q2 q3neg
  awk -F, '
    NR == 1 { next }
    {
      key = $2 SUBSEP $3 SUBSEP $5
      total[key]++
      if ($6 == "PASS") q1[key]++
      if ($7 == "PASS") q2[key]++
      if ($8 == "PASS") q3[key]++
      lane[key] = $2
      model[key] = $3
      variant[key] = $5
    }
    END {
      for (key in total) {
        printf "%-10s %-24s %-10s %3d/%-4d %3d/%-4d %3d/%-4d\n", \
          lane[key], model[key], variant[key], \
          q1[key] + 0, total[key], q2[key] + 0, total[key], q3[key] + 0, total[key]
      }
    }
  ' "$CSV" | sort
} > "$SUMMARY"

cat "$SUMMARY"
printf 'csv=%s\n' "$CSV"
