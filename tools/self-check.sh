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
#   6b. `cargo xtask check-codegen`       — the generated JTD wire types
#                                          (vibe-wire + the specmap engine
#                                          crate) byte-match their schemas;
#                                          needs the project-local
#                                          jtd-codegen binary, installed per
#                                          tool:org.vibevm.ai-native/jtd-codegen.
#   7. the core-ai-native package gate    — fmt + test + clippy on the
#                                          AUTHORED neutral engine crates,
#                                          which ship in their own excluded
#                                          Cargo workspace (PROP-024).
#   8. the language-stack package gates  — fmt + test + clippy on the rust,
#                                          typescript and go frontends/CLIs
#                                          + the vendored engine copies they
#                                          build against.
#   9. the packages' traceability self-trace — `rust-ai-native-specmap --gate` over
#                                          every gated slot carrying a
#                                          specmap.toml, so no discipline
#                                          code drifts untagged (PROP-014).
#  10. the mcp package gates            — the three MCP servers, likewise.
#
# Before any of it, step 0b asserts the floor's own DENOMINATOR: every LIVE
# package workspace under packages/org.vibevm.ai-native/ (the newest version
# slot of each package — superseded slots are frozen history) is one this
# file builds, and every slot it builds is still live. Four of seven
# packages were gated for months and "all green" was true the whole time
# (F-086); a count with no denominator cannot be wrong. The sync gate's
# half of the same lesson lives in `cargo xtask sync-engines --check`.
#
# Step 0c asserts the three agent instruction files (CLAUDE.md / AGENTS.md /
# GEMINI.md) are byte-identical — they are hand-copied triplets whose only
# verified part used to be the generated <vibevm> block, so a hand-edit
# that missed a sibling had nothing to catch it (F-217).
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

# The package workspaces this floor gates. Each is its OWN excluded Cargo
# workspace (PROP-024 §2.4) that the root steps 1-5 never build, so every
# slot path lives here once and the steps below only reference it. The
# denominator guard (step 0b) asserts this list IS the live set.
FAMILY_ROOT="packages/org.vibevm.ai-native"
CORE_SLOT="$FAMILY_ROOT/core-ai-native/v0.8.0"
PKG_DIR="$FAMILY_ROOT/rust-ai-native-lang/v0.7.0"
TSPKG_DIR="$FAMILY_ROOT/typescript-ai-native-lang/v0.6.0"
GOPKG_DIR="$FAMILY_ROOT/go-ai-native-lang/v0.1.0"
MCPR_DIR="$FAMILY_ROOT/rust-ai-native-mcp/v0.7.0"
MCPT_DIR="$FAMILY_ROOT/typescript-ai-native-mcp/v0.6.0"
MCPG_DIR="$FAMILY_ROOT/go-ai-native-mcp/v0.1.0"
GATED_SLOTS="$CORE_SLOT $PKG_DIR $TSPKG_DIR $GOPKG_DIR $MCPR_DIR $MCPT_DIR $MCPG_DIR"
CORE_MANIFEST="$CORE_SLOT/Cargo.toml"
PKG_MANIFEST="$PKG_DIR/Cargo.toml"
TSPKG_MANIFEST="$TSPKG_DIR/Cargo.toml"
GOPKG_MANIFEST="$GOPKG_DIR/Cargo.toml"
MCPR_MANIFEST="$MCPR_DIR/Cargo.toml"
MCPT_MANIFEST="$MCPT_DIR/Cargo.toml"
MCPG_MANIFEST="$MCPG_DIR/Cargo.toml"

# The LIVE package workspaces, derived — never spelled. A package
# directory may hold several version slots; only the newest is a living
# contract, the older ones are superseded history (`progress.toml` drops
# them from the observed corpus for exactly that reason). Deriving
# "newest" from the slot ordering rather than naming a frozen version is
# the whole point: a version literal here rots at the next release and
# goes on passing, which is the class of defect this guard exists to
# catch. When v0.9.0 lands, v0.8.0 leaves this set and v0.9.0 enters it,
# and the guard goes red until GATED_SLOTS is repointed — F-081 found by
# a checker instead of by a session happening to notice.
live_slots() {
  local pkg newest
  for pkg in "$FAMILY_ROOT"/*/; do
    [ -d "$pkg" ] || continue
    newest="$(ls -1 "$pkg" 2>/dev/null | sort -V | tail -n 1)"
    [ -n "$newest" ] || continue
    # A live slot with no Cargo.toml is a prose-only package: nothing to
    # build, so it is not a floor case. (Holding a vendored engine is a
    # SYNC case — a different set, gated by `sync-engines --check`.)
    [ -f "$pkg$newest/Cargo.toml" ] || continue
    printf '%s\n' "$pkg$newest"
  done
}

# 0b. The floor's denominator. Reported both ways: a live workspace this
# file does not build, and a slot it builds that is no longer live. The
# old failure mode was neither — "all green" over four of seven packages
# was true, and said nothing (F-086).
check_floor_denominator() {
  local slot missing="" stale="" live count=0 rc=0
  live="$(live_slots | tr '\n' ' ')"
  for slot in $live; do
    count=$((count + 1))
    case " $GATED_SLOTS " in
      *" $slot "*) ;;
      *) missing="$missing $slot" ;;
    esac
  done
  for slot in $GATED_SLOTS; do
    case " $live " in
      *" $slot "*) ;;
      *) stale="$stale $slot" ;;
    esac
  done
  for slot in $missing; do
    echo "self-check: \`$slot\` is a live package workspace the floor does not build." >&2
    rc=1
  done
  for slot in $stale; do
    echo "self-check: \`$slot\` is gated but is not the live slot of its package." >&2
    rc=1
  done
  if [ "$rc" -ne 0 ]; then
    echo "self-check: fix GATED_SLOTS in this file (and the steps that use it);" >&2
    echo "self-check: a floor that counts only what it was told about cannot be wrong." >&2
    return 1
  fi
  [ "$QUIET" -ne 0 ] ||
    echo "self-check: the floor builds all $count live package workspace(s) under $FAMILY_ROOT/." >&2
  return 0
}
run_step "the floor builds every live package workspace" \
  check_floor_denominator || OVERALL=$?

# 0c. The three agent instruction files are hand-copied triplets. The boot
# contract says «kept identical»; the generated <vibevm> block is written
# identically into all three by `vibe install` and checked per-file by
# `vibe check`, but everything OUTSIDE the markers — the four rules, the
# delegation directive, the operating-facts ledger, the session commands —
# had no reconciler until 2026-08-02 (campaign finding F-217). Byte-compare
# is the whole check, deliberately: any divergence anywhere in the files is
# a hand-edit that missed a sibling. Algorithmic by owner ruling — never an
# LLM judgement.
check_instruction_triple() {
  local rc=0 f
  for f in AGENTS.md GEMINI.md; do
    if ! cmp -s CLAUDE.md "$f"; then
      echo "self-check: CLAUDE.md and $f differ — the instruction files are" >&2
      echo "self-check: kept identical; reconcile the hand-edit into all three." >&2
      rc=1
    fi
  done
  return "$rc"
}
run_step "instruction files identical (CLAUDE.md = AGENTS.md = GEMINI.md)" \
  check_instruction_triple || OVERALL=$?

# Machine obligations the stacks declare. The go stack's live oracle
# needs gopls (TCG-ORACLE-GO §1) and the TS stack's structural gate
# parses with the project's own tsc (`npm install` under tools/
# ts-extract); both FAIL WITH A RECIPE rather than skip, by design, so
# the floor cannot run them on a box that lacks the tool. It names what
# it drops and why, every run, and a provisioned box runs the suites
# with no filter at all. Never widen these to hide a real failure.
GO_ORACLE_FILTER=""
if ! command -v gopls >/dev/null 2>&1; then
  echo "self-check: NOTE — gopls absent; the go live-oracle test is not run." >&2
  echo "self-check: NOTE — \`go install golang.org/x/tools/gopls@latest\` restores it." >&2
  GO_ORACLE_FILTER="seeded_error_surfaces_through_an_overlay"
fi
# The six TS tests that need a resolvable `typescript`, and nothing else
# in that workspace does — enumerated with `cargo test --workspace
# --no-fail-fast`, which is also how to re-derive the list if it changes.
# Do NOT extend it by watching one failure at a time: cargo stops at the
# first failing TARGET, so an iterated list is always a lower bound (this
# one read as three until the sweep found six).
TS_SKIPS=(
  --skip init_then_gates_catch_violations_and_the_tagged_tree_passes
  --skip clean_fixture_passes_with_zero_findings
  --skip dirty_fixture_yields_the_five_findings_then_freeze_ratchets_them
  --skip clean_fixture_check_is_byte_stable_and_gate_green
  --skip dirty_fixture_index_is_stable_but_the_orphan_gate_blocks
  --skip the_real_chain_validates_enriches_scopes_and_completes
)
TS_NODE_FILTER=0
if [ ! -d "$TSPKG_DIR/tools/ts-extract/node_modules" ] ||
   [ ! -d "$TSPKG_DIR/tools/ts-oracle/node_modules" ]; then
  echo "self-check: NOTE — a tools/*/node_modules under $TSPKG_DIR is absent;" >&2
  echo "self-check: NOTE — the 6 tsc-dependent TS tests are not run;" >&2
  echo "self-check: NOTE — \`npm install\` in tools/ts-extract and tools/ts-oracle restores them." >&2
  TS_NODE_FILTER=1
fi

go_workspace_test() {
  if [ -n "$GO_ORACLE_FILTER" ]; then
    cargo test --manifest-path "$1" --workspace --quiet -- --skip "$GO_ORACLE_FILTER"
  else
    cargo test --manifest-path "$1" --workspace --quiet
  fi
}

ts_workspace_test() {
  if [ "$TS_NODE_FILTER" -eq 1 ]; then
    cargo test --manifest-path "$TSPKG_MANIFEST" --workspace --quiet -- "${TS_SKIPS[@]}"
  else
    cargo test --manifest-path "$TSPKG_MANIFEST" --workspace --quiet
  fi
}

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

# 6b. Codegen drift gate. The generated JTD wire types — vibe-wire's and the
# specmap engine crate's — must byte-match their schemas (the two schema
# homes: schemas/ at the root, and the core-ai-native package's schemas/).
# Regeneration is `cargo xtask codegen`; the step fails actionably when the
# project-local jtd-codegen binary is missing (install per the
# tool:org.vibevm.ai-native/jtd-codegen package README). Added at the
# phase-D exit-gate audit: the byte-compare existed and no panel step named
# it, so schema-vs-generated drift had nothing to catch it.
run_step "cargo xtask check-codegen" cargo xtask check-codegen || OVERALL=$?

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

# 8. The language stacks — frontends + CLI drivers + their vendored engine
# copies — are their own excluded workspaces too (PROP-024). Same lesson as
# step 7, and the typescript and go stacks are here because F-086 measured
# the denominator: typescript-ai-native-lang is a source_root two sync sets
# copy FROM, so its code shipped into other packages while its own tests had
# never run in this floor.
run_step "cargo fmt --all --check (rust-ai-native-lang pkg)" \
  cargo fmt --manifest-path "$PKG_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (rust-ai-native-lang pkg)" \
  cargo test --manifest-path "$PKG_MANIFEST" --workspace --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (rust-ai-native-lang pkg)" \
  cargo clippy --manifest-path "$PKG_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
run_step "cargo fmt --all --check (typescript-ai-native-lang pkg)" \
  cargo fmt --manifest-path "$TSPKG_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (typescript-ai-native-lang pkg)" \
  ts_workspace_test || OVERALL=$?
run_step "cargo clippy --all-targets (typescript-ai-native-lang pkg)" \
  cargo clippy --manifest-path "$TSPKG_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
run_step "cargo fmt --all --check (go-ai-native-lang pkg)" \
  cargo fmt --manifest-path "$GOPKG_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (go-ai-native-lang pkg)" \
  go_workspace_test "$GOPKG_MANIFEST" || OVERALL=$?
run_step "cargo clippy --all-targets (go-ai-native-lang pkg)" \
  cargo clippy --manifest-path "$GOPKG_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 9. The packages' own traceability self-traces (Traceability Relocation Plan
# Phase 4; the authored-engine half moved with the consolidation). Every gated
# package crate's public surface must carry a scope!/#[spec] tag, so no
# discipline code drifts untagged. Orphan-coverage gate only (`--gate`) — the
# scope! targets are cross-package spec units, so a full index would be all
# cross-repo "dangling"; coverage is what matters. The conform step-5 lesson
# (a gate not in self-check drifts silently) applied to the packages' traces.
#
# Only the slots carrying a `specmap.toml` get a self-trace: without one the
# gate reads no config, inventories nothing, and exits 0 — a step that cannot
# fail, which is the same disease as a count with no denominator. That leaves
# typescript-ai-native-lang and go-ai-native-mcp untraced; they need a
# specmap.toml authored before a trace step here would mean anything.
run_step "rust-ai-native-specmap --gate (core-ai-native pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$CORE_SLOT" || OVERALL=$?
run_step "rust-ai-native-specmap --gate (rust-ai-native-lang pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$PKG_DIR" || OVERALL=$?
run_step "rust-ai-native-specmap --gate (go-ai-native-lang pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$GOPKG_DIR" || OVERALL=$?

# 10. The mcp packages (PROP-027; MCP-SOVEREIGNTY Wave 3+) — each is its
# own excluded workspace authoring ONE server crate over a vendored
# closure (sync-engines holds the copies byte-identical to their
# authored homes, step 6). Same lesson as steps 7-8: nothing else runs
# their authored tests; gate them here, self-trace included.
run_step "cargo fmt --all --check (rust-ai-native-mcp pkg)" \
  cargo fmt --manifest-path "$MCPR_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test -p rust-ai-native-mcp (rust-ai-native-mcp pkg)" \
  cargo test --manifest-path "$MCPR_MANIFEST" -p rust-ai-native-mcp --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (rust-ai-native-mcp pkg)" \
  cargo clippy --manifest-path "$MCPR_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
run_step "rust-ai-native-specmap --gate (rust-ai-native-mcp pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$MCPR_DIR" || OVERALL=$?
run_step "cargo fmt --all --check (typescript-ai-native-mcp pkg)" \
  cargo fmt --manifest-path "$MCPT_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test -p typescript-ai-native-mcp (typescript-ai-native-mcp pkg)" \
  cargo test --manifest-path "$MCPT_MANIFEST" -p typescript-ai-native-mcp --quiet || OVERALL=$?
run_step "cargo clippy --all-targets (typescript-ai-native-mcp pkg)" \
  cargo clippy --manifest-path "$MCPT_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?
run_step "rust-ai-native-specmap --gate (typescript-ai-native-mcp pkg self-trace)" \
  cargo run --quiet --manifest-path "$PKG_MANIFEST" -p rust-ai-native-specmap --bin rust-ai-native-specmap -- --gate --path "$MCPT_DIR" || OVERALL=$?
run_step "cargo fmt --all --check (go-ai-native-mcp pkg)" \
  cargo fmt --manifest-path "$MCPG_MANIFEST" --all --check || OVERALL=$?
run_step "cargo test --workspace (go-ai-native-mcp pkg)" \
  go_workspace_test "$MCPG_MANIFEST" || OVERALL=$?
run_step "cargo clippy --all-targets (go-ai-native-mcp pkg)" \
  cargo clippy --manifest-path "$MCPG_MANIFEST" --workspace --all-targets --quiet -- -D warnings || OVERALL=$?

# 11. The vibeterm / vibeframe terminal products moved to a separate repo
# (`vibevm-term`); their pure-logic tests (`node --test` for the shared
# arg/keymap helpers, `vitest` for the vibeterm engine cells) live there now.
# The host's self-check no longer runs them — the vibevm-term repo carries
# its own floor.

# 12. The tripwire again, over the whole run. Steps 7-10 run seven more test
# suites (the authored engines, the three stacks, the three mcp packages)
# against the same baseline from step 0, and each of them is a `cargo test`
# that could reach the real settings home just as easily.
run_step "user-home tripwire (whole run)" check_user_home || OVERALL=$?

if [ "$QUIET" -eq 0 ]; then
  if [ "$OVERALL" -eq 0 ]; then
    printf '\nself-check: all green\n' >&2
  else
    printf '\nself-check: failures above (exit %d)\n' "$OVERALL" >&2
  fi
fi

exit "$OVERALL"
