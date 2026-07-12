#!/usr/bin/env bash
# Preserve paid trial-run evidence durably (owner directive 2026-07-12:
# «новые тесты тоже сохраняй»). Copies the small evidential subset from the
# GITIGNORED target/trial-results/ into the COMMITTED reports/trial-results/,
# gzipping transcripts (jsonl ~10:1) and excluding the reproducible
# proj-final/ repo copies. Idempotent — re-run any time; overwrites its own
# prior copy of each run. Optional arg: a run-dir glob to limit the scope
# (default: every *-run-* directory, i.e. arm-a/b/g/g2 and advise-*).
#
#   bash save-results.sh              # preserve all runs
#   bash save-results.sh 'arm-g2-*'   # just the g2 re-run
#
# Then commit reports/trial-results/. A paid run whose evidence lives only in
# target/ is one `cargo clean` from wasted.
set -euo pipefail

TRIAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$(cd "$TRIAL_DIR/../../.." && pwd)"                     # fractality/v0.1.0
SRC="$WS/target/trial-results"
DST="$(cd "$WS/../.." && pwd)/reports/trial-results"        # workspace reports/
GLOB="${1:-*-run-*}"

[ -d "$SRC" ] || { echo "save-results: no $SRC — nothing to save" >&2; exit 0; }
mkdir -p "$DST"

saved=0
for d in "$SRC"/$GLOB/; do
  [ -d "$d" ] || continue
  name=$(basename "$d")
  mkdir -p "$DST/$name"
  for f in "$d"*; do
    b=$(basename "$f")
    case "$b" in
      proj-final) continue ;;                              # huge + reproducible
      boss-transcript.jsonl) gzip -c "$f" > "$DST/$name/$b.gz" ;;
      *) cp "$f" "$DST/$name/" 2>/dev/null || true ;;
    esac
  done
  saved=$((saved + 1))
  echo "saved: $name"
done
echo "save-results: $saved run(s) preserved into $DST"
