#!/usr/bin/env bash
# vibevm self-check — runs the three invariants every commit on `main`
# is supposed to satisfy. Designed to be cheap to invoke locally and
# trivial to wire into a CI matrix later. See `DEV-GUIDE.md` §6.
#
# Invariants checked, in order:
#   1. `cargo test --workspace`          — all tests green.
#   2. `cargo clippy --workspace ...`     — zero warnings under `-D warnings`.
#   3. `vibe check --path . --quiet`      — spec linter clean against the
#                                          repo's own bootstrap manifest.
#
# Each step prints a short header. On the first failure the script exits
# non-zero; later steps are skipped (no "fix the next thing while broken"
# slog). Pass `--keep-going` to run all three even if earlier ones fail.

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

step() {
  if [ "$QUIET" -eq 0 ]; then
    printf '\n=== %s ===\n' "$1" >&2
  fi
}

run_step() {
  local label="$1"; shift
  step "$label"
  if "$@"; then
    return 0
  else
    local rc=$?
    echo "self-check: \`$label\` failed (exit $rc)" >&2
    if [ "$KEEP_GOING" -eq 0 ]; then
      exit "$rc"
    fi
    return "$rc"
  fi
}

OVERALL=0

# 1. Tests.
run_step "cargo test --workspace" cargo test --workspace --quiet || OVERALL=$?

# 2. Clippy as errors.
run_step "cargo clippy --workspace --all-targets -- -D warnings" \
  cargo clippy --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 3. Spec linter on the bootstrap manifest. Always go through
# `cargo run` so the binary used is guaranteed to match the source
# tree — a stale `target/release/vibe.exe` from a previous workspace
# state was a real footgun (e.g. binaries built before a subcommand
# existed reject it as `unrecognized subcommand`). The compile is a
# no-op once `cargo test` / `cargo clippy` above have populated the
# build cache.
run_step "cargo run -p vibe-cli -- check --path . --quiet" \
  cargo run --quiet -p vibe-cli -- check --path . --quiet || OVERALL=$?

if [ "$QUIET" -eq 0 ]; then
  if [ "$OVERALL" -eq 0 ]; then
    printf '\nself-check: all green\n' >&2
  else
    printf '\nself-check: failures above (exit %d)\n' "$OVERALL" >&2
  fi
fi

exit "$OVERALL"
