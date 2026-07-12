#!/usr/bin/env bash
# Preserve paid/important run evidence durably (owner directive 2026-07-12).
# Related runs go under ONE dated GROUP directory, and each run dir is ALSO
# dated — the год-число-месяц-HH-MM (reversedate-forwardtime-description)
# convention the reports use, e.g. 2026-12-07-06-42-arm-g-run-3. Copies the
# small evidential subset, gzips transcripts (jsonl ~10:1), excludes the
# reproducible proj-final/. Idempotent per run name.
#
#   bash save-results.sh c3-mt-c3-03-gated-rerun             # all runs -> one dated group
#   bash save-results.sh c3-mt-c3-03-gated-rerun 'arm-g2-*'  # just the g2 runs
#
# Then commit reports/trial-results/. A paid run whose evidence lives only in
# the gitignored target/ is one `cargo clean` from wasted. See the workspace
# CLAUDE.md §"Preserve valuable test/run evidence" — this also applies, by
# judgment, to any important/long non-harness run (pass a group name + copy
# the artifacts under the dated group dir yourself).
set -euo pipefail

GROUP_DESC="${1:?usage: save-results.sh <group-description> [run-glob]}"
GLOB="${2:-*-run-*}"

TRIAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$(cd "$TRIAL_DIR/../../.." && pwd)"                     # fractality/v0.1.0
SRC="$WS/target/trial-results"
ROOT="$(cd "$WS/../.." && pwd)/reports/trial-results"       # workspace reports/

[ -d "$SRC" ] || { echo "save-results: no $SRC — nothing to save" >&2; exit 0; }

# Group dir: dated at save time (год-число-месяц-HH-MM) + the description.
GROUP="$(date +%Y-%d-%m-%H-%M)-${GROUP_DESC}"
DST="$ROOT/$GROUP"
mkdir -p "$DST"

# Scaffold the group README (owner rule 2026-07-12): each results directory
# carries a README describing what the test was + its summary results,
# amended when analysis lands later. Created once, never clobbered.
if [ ! -f "$DST/README.md" ]; then
  {
    echo "# ${GROUP_DESC} — trial run group"
    echo ""
    echo "_Saved $(date +%Y-%d-%m) from \`target/trial-results\`. Raw evidence"
    echo "(bus facts + gzipped transcripts) sits per-run beside this file._"
    echo ""
    echo "## What this was"
    echo ""
    echo "TODO: the test/trial, its pre-registration (MT id), the arms, what it measured."
    echo ""
    echo "## Summary results"
    echo ""
    echo "TODO: the scored verdicts / metrics once analyzed. Amend this section when"
    echo "analysis lands — results written later supplement this README (owner rule)."
  } > "$DST/README.md"
  echo "scaffolded: $GROUP/README.md (fill in What/Summary)"
fi

saved=0
for d in "$SRC"/$GLOB/; do
  [ -d "$d" ] || continue
  base=$(basename "$d")
  # Run dir: dated by the run's OWN fire time (run-info.txt mtime), so the
  # per-run timestamp reflects when it ran, not when it was saved.
  stamp="$(date -r "$d/run-info.txt" +%Y-%d-%m-%H-%M 2>/dev/null || date +%Y-%d-%m-%H-%M)"
  rundst="$DST/${stamp}-${base}"
  mkdir -p "$rundst"
  for f in "$d"*; do
    b=$(basename "$f")
    case "$b" in
      proj-final) continue ;;                              # huge + reproducible
      boss-transcript.jsonl) gzip -c "$f" > "$rundst/$b.gz" ;;
      *) cp "$f" "$rundst/" 2>/dev/null || true ;;
    esac
  done
  saved=$((saved + 1))
  echo "saved: $GROUP/${stamp}-${base}"
done
echo "save-results: $saved run(s) preserved into $DST"
