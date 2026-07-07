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
#   6. `cargo xtask sync-engines --check`  — every vendored engine crate under
#                                          the stacks' crates/vendor/ is
#                                          byte-identical to the authored
#                                          source in discipline-core, so a
#                                          vendored copy can never diverge
#                                          silently (DEFERRALS-CLOSEOUT D1).
#   7. the discipline-core package gate    — fmt + test + clippy on the
#                                          AUTHORED neutral engine crates,
#                                          which ship in their own excluded
#                                          Cargo workspace (PROP-024).
#   8. the rust-ai-native package gate     — fmt + test + clippy on the Rust
#                                          frontends/CLIs + the vendored
#                                          engine copies they build against.
#   9. the packages' traceability self-trace — `specmap-rust --gate` over
#                                          discipline-core (the authored
#                                          engines) and rust-ai-native (the
#                                          frontends), so no discipline code
#                                          drifts untagged (PROP-014).
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

# 6. Vendor-sync gate (DEFERRALS-CLOSEOUT D1). The neutral engine crates are
# authored ONCE in flow:org.vibevm/discipline-core; each stack ships a
# byte-identical vendored copy under crates/vendor/. This asserts the copies
# match the authored source, so "fixing" a vendored file — the wrong surface —
# cannot land.
run_step "cargo xtask sync-engines --check" cargo xtask sync-engines --check || OVERALL=$?

# 7. The AUTHORED neutral engines — conform-core, specmap-core, specmark,
# specmark-grammar — ship in flow:org.vibevm/discipline-core as its OWN Cargo
# workspace (PROP-024), excluded from the vibevm root. Steps 1-5 build the
# VENDORED copies as dependencies but never run the authored tests/doctests,
# and root fmt+clippy never touch them. Gate the authored source here.
CORE_MANIFEST="packages/org.vibevm/discipline-core/v0.6.0/Cargo.toml"
run_step "cargo fmt --all --check (discipline-core pkg)" \
  cargo fmt --manifest-path "$CORE_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (discipline-core pkg)" \
  cargo test --manifest-path "$CORE_MANIFEST" --workspace --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (discipline-core pkg)" \
  cargo clippy --manifest-path "$CORE_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 8. The Rust stack — frontends + CLI drivers + its vendored engine copies —
# is its own excluded workspace too (PROP-024). Same lesson as step 7.
PKG_MANIFEST="packages/org.vibevm/rust-ai-native/v0.5.0/Cargo.toml"
run_step "cargo fmt --all --check (rust-ai-native pkg)" \
  cargo fmt --manifest-path "$PKG_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (rust-ai-native pkg)" \
  cargo test --manifest-path "$PKG_MANIFEST" --workspace --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (rust-ai-native pkg)" \
  cargo clippy --manifest-path "$PKG_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 9. The packages' own traceability self-traces (Traceability Relocation Plan
# Phase 4; the authored-engine half moved with the consolidation). Every gated
# package crate's public surface must carry a scope!/#[spec] tag, so no
# discipline code drifts untagged. Orphan-coverage gate only (`--gate`) — the
# scope! targets are cross-package spec units, so a full index would be all
# cross-repo "dangling"; coverage is what matters. The conform step-5 lesson
# (a gate not in self-check drifts silently) applied to the packages' traces.
CORE_DIR="packages/org.vibevm/discipline-core/v0.6.0"
run_step "specmap-rust --gate (discipline-core pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p specmap-cli-rust --bin specmap-rust -- --gate --path "$CORE_DIR" || OVERALL=$?
PKG_DIR="packages/org.vibevm/rust-ai-native/v0.5.0"
run_step "specmap-rust --gate (rust-ai-native pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p specmap-cli-rust --bin specmap-rust -- --gate --path "$PKG_DIR" || OVERALL=$?

# 10. The mcp packages (PROP-027; MCP-SOVEREIGNTY Wave 3+) — each is its
# own excluded workspace authoring ONE server crate over a vendored
# closure (sync-engines holds the copies byte-identical to their
# authored homes, step 6). Same lesson as steps 7-8: nothing else runs
# their authored tests; gate them here, self-trace included.
MCPR_MANIFEST="packages/org.vibevm/discipline-rust/v0.5.0/Cargo.toml"
run_step "cargo fmt --all --check (discipline-rust pkg)" \
  cargo fmt --manifest-path "$MCPR_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test -p discipline-mcp-rust (discipline-rust pkg)" \
  cargo test --manifest-path "$MCPR_MANIFEST" -p discipline-mcp-rust --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (discipline-rust pkg)" \
  cargo clippy --manifest-path "$MCPR_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
MCPR_DIR="packages/org.vibevm/discipline-rust/v0.5.0"
run_step "specmap-rust --gate (discipline-rust pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p specmap-cli-rust --bin specmap-rust -- --gate --path "$MCPR_DIR" || OVERALL=$?

if [ "$QUIET" -eq 0 ]; then
  if [ "$OVERALL" -eq 0 ]; then
    printf '\nself-check: all green\n' >&2
  else
    printf '\nself-check: failures above (exit %d)\n' "$OVERALL" >&2
  fi
fi

exit "$OVERALL"
