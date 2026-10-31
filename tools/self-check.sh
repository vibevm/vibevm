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

STEP_TOTAL=8
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
    (
      # The heartbeat must never pin an owned temporary source volume.
      cd / || exit 2
      sleep_pid=""
      cleanup_ticker() {
        trap - EXIT TERM INT
        if [ -n "$sleep_pid" ]; then
          kill "$sleep_pid" 2>/dev/null || :
          wait "$sleep_pid" 2>/dev/null || :
        fi
      }
      trap cleanup_ticker EXIT
      trap 'exit 0' TERM INT
      while :; do
        sleep 30 &
        sleep_pid=$!
        wait "$sleep_pid" || break
        sleep_pid=""
        printf 'self-check: … still in [%d/%d] %s, +%ss\n' \
          "$STEP_NO" "$STEP_TOTAL" "$label" "$((SECONDS - started))" >&2
      done
    ) &
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

case "${OSTYPE:-}" in
  msys*|cygwin*|win32*)
    run_step "cargo fmt --all --check" \
      python tools/self-check-format-windows.py || OVERALL=$?
    ;;
  *)
    run_step "cargo fmt --all --check" \
      cargo fmt --all --check || OVERALL=$?
    ;;
esac
TEST_OPTIONS=(test --workspace --quiet)
if [ "$KEEP_GOING" -eq 1 ]; then
  TEST_OPTIONS+=(--no-fail-fast)
fi
run_step "cargo test --workspace" \
  cargo "${TEST_OPTIONS[@]}" || OVERALL=$?
run_step "cargo clippy --workspace --all-targets -- -D warnings" \
  cargo clippy --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
run_step "vibe check --path . --quiet" \
  cargo run --quiet -p vibe-cli -- check --path . --quiet || OVERALL=$?
# A Windows main thread owns 1 MiB of stack and a Linux one 8 MiB, so a debug
# build that grows past the smaller limit at startup (clap's derived builders
# did on 2026-10-31) passes every step above and dies on `vibe --version`
# over there. Start the binary under the Windows limit here, on a subshell
# whose ulimit does not leak into the steps that follow.
run_step "vibe --version under a 1 MiB stack" \
  bash -c 'ulimit -s 1024 && exec target/debug/vibe --version' || OVERALL=$?
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
