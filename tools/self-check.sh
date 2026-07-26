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
#                                          source in core-ai-native, so a
#                                          vendored copy can never diverge
#                                          silently (DEFERRALS-CLOSEOUT D1).
#   7. the core-ai-native package gate    — fmt + test + clippy on the
#                                          AUTHORED neutral engine crates,
#                                          which ship in their own excluded
#                                          Cargo workspace (PROP-024).
#   8. the rust-ai-native-lang package gate     — fmt + test + clippy on the Rust
#                                          frontends/CLIs + the vendored
#                                          engine copies they build against.
#   9. the packages' traceability self-trace — `rust-ai-native-specmap --gate` over
#                                          core-ai-native (the authored
#                                          engines) and rust-ai-native-lang (the
#                                          frontends), so no discipline code
#                                          drifts untagged (PROP-014).
#
# Wrapped around all of it: the user-home tripwire. The real per-user
# settings home (`~/.vibe`, or `$VIBE_SETTINGS`) is hashed path-by-path
# before the first step, and compared twice — right after the workspace
# tests, and again after the last package suite. Any change fails the
# floor and names the paths. That home carries the operator's publish
# tokens and API keys; a test that touches it is a bug in the test
# (DRIFT-020), and three findings in a row were caught by accident
# rather than by a gate. Hash-and-path only — never contents.
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

# 0. Baseline the operator's real per-user settings home, before anything
# builds or runs. Taken unconditionally and cheaply (a hash per path); the
# comparisons happen after the test steps below. A home that cannot be
# resolved or read makes the tripwire a no-op, never a false red — the gate
# must not become the thing that blocks work.
TRIPWIRE="tools/user-home-tripwire.sh"
USER_HOME_SNAPSHOT="$(mktemp 2>/dev/null || echo '')"
if [ -n "$USER_HOME_SNAPSHOT" ]; then
  trap 'rm -f "$USER_HOME_SNAPSHOT"' EXIT
  bash "$TRIPWIRE" snapshot "$USER_HOME_SNAPSHOT" || true
else
  echo "self-check: WARNING — no temp file for the user-home tripwire; skipping it" >&2
fi

check_user_home() {
  [ -n "$USER_HOME_SNAPSHOT" ] || return 0
  bash "$TRIPWIRE" compare "$USER_HOME_SNAPSHOT"
}

# 1. Formatting. The cheapest invariant — no compilation — so it runs
# first and fails fast, before the multi-minute test / clippy steps.
run_step "cargo fmt --all --check" cargo fmt --all --check || OVERALL=$?

# 2. Tests.
run_step "cargo test --workspace" cargo test --workspace --quiet || OVERALL=$?

# 2b. The tripwire, immediately after the suite that is most likely to trip
# it. Separate from the later sweep so a failure points at the workspace
# tests specifically rather than at "something in this run".
run_step "user-home tripwire (after cargo test --workspace)" check_user_home || OVERALL=$?

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
# authored ONCE in flow:org.vibevm.ai-native/core-ai-native; each stack ships a
# byte-identical vendored copy under crates/vendor/. This asserts the copies
# match the authored source, so "fixing" a vendored file — the wrong surface —
# cannot land.
run_step "cargo xtask sync-engines --check" cargo xtask sync-engines --check || OVERALL=$?

# 7. The AUTHORED neutral engines — conform-core, specmap-core, specmark,
# specmark-grammar — ship in flow:org.vibevm.ai-native/core-ai-native as its OWN Cargo
# workspace (PROP-024), excluded from the vibevm root. Steps 1-5 build the
# VENDORED copies as dependencies but never run the authored tests/doctests,
# and root fmt+clippy never touch them. Gate the authored source here.
#
# WHICH slot is the authored source is not this file's to decide: it is
# whatever `sync-engines.toml` vendors FROM. That pointer moved to v0.8.0 in
# `0aa4ba01` and this file was not moved with it, so for the whole interval
# steps 7 and 9 gated the frozen v0.7.0 slot — green, faithfully, on a tree
# nothing resolves to, while the authored engines went untested (F-081). The
# guard below makes the coupling checkable instead of remembered: a rule with
# no checker is a WISH, and this one had been a WISH for exactly as long as it
# took to break.
CORE_SLOT="packages/org.vibevm.ai-native/core-ai-native/v0.8.0"
CORE_MANIFEST="$CORE_SLOT/Cargo.toml"
check_core_slot_is_authored() {
  if grep -qF "source_root = \"$CORE_SLOT/crates\"" sync-engines.toml; then
    return 0
  fi
  echo "self-check: \`$CORE_SLOT\` is not a sync-engines.toml source_root." >&2
  echo "self-check: the floor would gate a slot nothing is vendored from." >&2
  echo "self-check: repoint CORE_SLOT at the authored core-ai-native slot:" >&2
  grep -n 'core-ai-native/v[0-9.]*/crates' sync-engines.toml >&2
  return 1
}
run_step "core-ai-native gated slot is the authored one" \
  check_core_slot_is_authored || OVERALL=$?
run_step "cargo fmt --all --check (core-ai-native pkg)" \
  cargo fmt --manifest-path "$CORE_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (core-ai-native pkg)" \
  cargo test --manifest-path "$CORE_MANIFEST" --workspace --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (core-ai-native pkg)" \
  cargo clippy --manifest-path "$CORE_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 8. The Rust stack — frontends + CLI drivers + its vendored engine copies —
# is its own excluded workspace too (PROP-024). Same lesson as step 7.
PKG_MANIFEST="packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/Cargo.toml"
run_step "cargo fmt --all --check (rust-ai-native-lang pkg)" \
  cargo fmt --manifest-path "$PKG_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (rust-ai-native-lang pkg)" \
  cargo test --manifest-path "$PKG_MANIFEST" --workspace --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (rust-ai-native-lang pkg)" \
  cargo clippy --manifest-path "$PKG_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 9. The packages' own traceability self-traces (Traceability Relocation Plan
# Phase 4; the authored-engine half moved with the consolidation). Every gated
# package crate's public surface must carry a scope!/#[spec] tag, so no
# discipline code drifts untagged. Orphan-coverage gate only (`--gate`) — the
# scope! targets are cross-package spec units, so a full index would be all
# cross-repo "dangling"; coverage is what matters. The conform step-5 lesson
# (a gate not in self-check drifts silently) applied to the packages' traces.
CORE_DIR="$CORE_SLOT"
run_step "rust-ai-native-specmap --gate (core-ai-native pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$CORE_DIR" || OVERALL=$?
PKG_DIR="packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0"
run_step "rust-ai-native-specmap --gate (rust-ai-native-lang pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$PKG_DIR" || OVERALL=$?

# 10. The mcp packages (PROP-027; MCP-SOVEREIGNTY Wave 3+) — each is its
# own excluded workspace authoring ONE server crate over a vendored
# closure (sync-engines holds the copies byte-identical to their
# authored homes, step 6). Same lesson as steps 7-8: nothing else runs
# their authored tests; gate them here, self-trace included.
MCPR_MANIFEST="packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/Cargo.toml"
run_step "cargo fmt --all --check (rust-ai-native-mcp pkg)" \
  cargo fmt --manifest-path "$MCPR_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test -p rust-ai-native-mcp (rust-ai-native-mcp pkg)" \
  cargo test --manifest-path "$MCPR_MANIFEST" -p rust-ai-native-mcp --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (rust-ai-native-mcp pkg)" \
  cargo clippy --manifest-path "$MCPR_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
MCPR_DIR="packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0"
run_step "rust-ai-native-specmap --gate (rust-ai-native-mcp pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$MCPR_DIR" || OVERALL=$?
MCPT_MANIFEST="packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/Cargo.toml"
run_step "cargo fmt --all --check (typescript-ai-native-mcp pkg)" \
  cargo fmt --manifest-path "$MCPT_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test -p typescript-ai-native-mcp (typescript-ai-native-mcp pkg)" \
  cargo test --manifest-path "$MCPT_MANIFEST" -p typescript-ai-native-mcp --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (typescript-ai-native-mcp pkg)" \
  cargo clippy --manifest-path "$MCPT_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
MCPT_DIR="packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0"
run_step "rust-ai-native-specmap --gate (typescript-ai-native-mcp pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$MCPT_DIR" || OVERALL=$?

# 11. The vibeterm / vibeframe terminal products moved to a separate repo
# (`vibevm-term`); their pure-logic tests (`node --test` for the shared
# arg/keymap helpers, `vitest` for the vibeterm engine cells) live there now.
# The host's self-check no longer runs them — the vibevm-term repo carries
# its own floor.

# 12. The tripwire again, over the whole run. Steps 7-10 run four more test
# suites (the authored engines, the Rust stack, both mcp packages) against
# the same baseline from step 0, and each of them is a `cargo test` that
# could reach the real settings home just as easily.
run_step "user-home tripwire (whole run)" check_user_home || OVERALL=$?

if [ "$QUIET" -eq 0 ]; then
  if [ "$OVERALL" -eq 0 ]; then
    printf '\nself-check: all green\n' >&2
  else
    printf '\nself-check: failures above (exit %d)\n' "$OVERALL" >&2
  fi
fi

exit "$OVERALL"
