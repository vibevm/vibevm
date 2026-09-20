#!/usr/bin/env bash
# vibevm self-check — the small integration/release gate.
#
# This gate intentionally covers the host product only. Independently shipped
# package workspaces, documentation/browser suites, self-traces and historical
# policy ratchets are affected-area checks, not release blockers. Run those
# commands directly when their inputs change.

set -u

KEEP_GOING=0
QUIET=0
for arg in "$@"; do
  case "$arg" in
    --keep-going) KEEP_GOING=1 ;;
    --quiet) QUIET=1 ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^#\s\?//'
      exit 0
      ;;
    *)
      echo "self-check: unknown flag \`$arg\`" >&2
      exit 2
      ;;
  esac
done

cd "$(dirname "$0")/.." || exit 2

STEP_TOTAL=7
STEP_NO=0
OVERALL=0

run_step() {
  local label="$1"; shift
  STEP_NO=$((STEP_NO + 1))
  if [ "$QUIET" -eq 0 ]; then
    printf '\n=== [%d/%d %s] %s ===\n' \
      "$STEP_NO" "$STEP_TOTAL" "$(date +%H:%M:%S)" "$label" >&2
  fi

  local started=$SECONDS ticker="" rc=0
  if [ "$QUIET" -eq 0 ]; then
    ( while sleep 30; do
        printf 'self-check: … still in [%d/%d] %s, +%ss\n' \
          "$STEP_NO" "$STEP_TOTAL" "$label" "$((SECONDS - started))" >&2
      done ) &
    ticker=$!
  fi

  "$@" || rc=$?
  if [ -n "$ticker" ]; then
    kill "$ticker" 2>/dev/null
    wait "$ticker" 2>/dev/null
  fi

  if [ "$rc" -eq 0 ]; then
    if [ "$QUIET" -eq 0 ]; then
      printf 'self-check: ✓ [%d/%d] %s (%ss)\n' \
        "$STEP_NO" "$STEP_TOTAL" "$label" "$((SECONDS - started))" >&2
    fi
    return 0
  fi

  echo "self-check: \`$label\` failed (exit $rc, step $STEP_NO/$STEP_TOTAL, $((SECONDS - started))s)" >&2
  if [ "$KEEP_GOING" -eq 0 ]; then
    exit "$rc"
  fi
  return "$rc"
}

run_step "cargo fmt --all --check" \
  cargo fmt --all --check || OVERALL=$?
run_step "cargo test --workspace" \
  cargo test --workspace --quiet || OVERALL=$?
run_step "cargo clippy --workspace --all-targets -- -D warnings" \
  cargo clippy --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
run_step "vibe check --path . --quiet" \
  cargo run --quiet -p vibe-cli -- check --path . --quiet || OVERALL=$?
run_step "cargo xtask conform check" \
  cargo xtask conform check || OVERALL=$?
run_step "cargo xtask check-codegen" \
  cargo xtask check-codegen || OVERALL=$?
run_step "cargo xtask wire-diff" \
  cargo xtask wire-diff || OVERALL=$?

if [ "$QUIET" -eq 0 ]; then
  if [ "$OVERALL" -eq 0 ]; then
    printf '\nself-check: all green\n' >&2
  else
    printf '\nself-check: failures above (exit %d)\n' "$OVERALL" >&2
  fi
fi
exit "$OVERALL"
