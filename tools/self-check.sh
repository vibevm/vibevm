#!/usr/bin/env bash
# vibevm self-check — runs the floor invariants every commit on `main`
# is supposed to satisfy. Designed to be cheap to invoke locally and
# trivial to wire into a CI matrix later. See `DEV-GUIDE.md` §6.
#
# Invariants checked, in order:
#   1. `cargo fmt --all --check`         — every file is rustfmt-clean.
#   2. `cargo test --workspace`          — all tests green.
#   3. `cargo clippy --workspace ...`     — zero warnings under `-D warnings`.
#   4. `vibe check --path . --quiet`      — spec linter clean against the
#                                          repo's own bootstrap manifest.
#   5. `cargo xtask conform check`        — the discipline gate (Class-F/G
#                                          doctests + REQ-citing errors,
#                                          the file-length budget, the
#                                          unwrap ban) clean vs. the
#                                          baseline, so it cannot drift
#                                          silently between commits.
#   6. the rust-ai-native package gate     — fmt + test + clippy on the
#                                          relocated conform engine, which
#                                          ships in its own excluded Cargo
#                                          workspace (PROP-024) that steps
#                                          1-5 build but cannot otherwise
#                                          reach (its tests + doctests).
#   7. the package traceability self-trace  — `specmap-rust --gate` over the
#                                          package, so the discipline code it
#                                          ships cannot drift untagged itself
#                                          (PROP-014; Phase 4 of the move).
#
# Each step prints a short header. On the first failure the script exits
# non-zero; later steps are skipped (no "fix the next thing while broken"
# slog). Pass `--keep-going` to run all steps even if earlier ones fail.

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

# 1. Formatting. The cheapest invariant — no compilation — so it runs
# first and fails fast, before the multi-minute test / clippy steps.
run_step "cargo fmt --all --check" cargo fmt --all --check || OVERALL=$?

# 2. Tests.
run_step "cargo test --workspace" cargo test --workspace --quiet || OVERALL=$?

# 3. Clippy as errors.
run_step "cargo clippy --workspace --all-targets -- -D warnings" \
  cargo clippy --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 4. Spec linter on the bootstrap manifest. Always go through
# `cargo run` so the binary used is guaranteed to match the source
# tree — a stale `target/release/vibe.exe` from a previous workspace
# state was a real footgun (e.g. binaries built before a subcommand
# existed reject it as `unrecognized subcommand`). The compile is a
# no-op once `cargo test` / `cargo clippy` above have populated the
# build cache.
run_step "cargo run -p vibe-cli -- check --path . --quiet" \
  cargo run --quiet -p vibe-cli -- check --path . --quiet || OVERALL=$?

# 5. The AI-Native discipline gate (conform). Runs last: it reuses the
# build cache the steps above populated, and its content-addressed fact
# store re-extracts only changed files. Wiring it here is what keeps the
# Class-F/G + file-length + unwrap invariants from drifting unnoticed the
# way they did across the bridge-packages sessions (the gate was green in
# the RAID, then silently red until a sweep re-ran it).
run_step "cargo xtask conform check" cargo xtask conform check || OVERALL=$?

# 6. The AI-Native discipline toolchain — the conform engine itself — now ships
# in stack:org.vibevm/rust-ai-native as its OWN Cargo workspace (PROP-024
# code-bearing packages), excluded from the vibevm root. Steps 1-5 build it as a
# dependency but never run its unit tests / doctests, and root fmt+clippy never
# touch it. Run its fmt + test + clippy here against the package manifest so the
# engine that powers the gate cannot itself drift outside the discipline unseen
# (the same "a gate not in self-check drifts silently" lesson that wired conform
# in as step 5).
PKG_MANIFEST="packages/org.vibevm/rust-ai-native/v0.2.0/Cargo.toml"
run_step "cargo fmt --all --check (rust-ai-native pkg)" \
  cargo fmt --manifest-path "$PKG_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (rust-ai-native pkg)" \
  cargo test --manifest-path "$PKG_MANIFEST" --workspace --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (rust-ai-native pkg)" \
  cargo clippy --manifest-path "$PKG_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 7. The package's own traceability self-trace (Traceability Relocation Plan,
# Phase 4). The package ships the specmap engine and now traces ITSELF: every
# gated package crate's public surface must carry a scope!/#[spec] tag, so no
# discipline code drifts untagged. Orphan-coverage gate only (`--gate`) — the
# scope! targets are vibevm-hosted spec units, so a full index would be all
# cross-repo "dangling"; coverage is what matters. The conform step-5 lesson
# (a gate not in self-check drifts silently) applied to the package's own trace.
PKG_DIR="packages/org.vibevm/rust-ai-native/v0.2.0"
run_step "specmap-rust --gate (rust-ai-native pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p specmap-cli --bin specmap-rust -- --gate --path "$PKG_DIR" || OVERALL=$?

if [ "$QUIET" -eq 0 ]; then
  if [ "$OVERALL" -eq 0 ]; then
    printf '\nself-check: all green\n' >&2
  else
    printf '\nself-check: failures above (exit %d)\n' "$OVERALL" >&2
  fi
fi

exit "$OVERALL"
