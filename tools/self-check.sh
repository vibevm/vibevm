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
#   6d. `cargo xtask wire-diff`           — the epoch verdict over the golden
#                                          corpora: corpora re-proven against
#                                          their journals, byte shift vs the
#                                          commit judged by the EPOCHS.toml
#                                          regime, so an unannounced wire
#                                          break cannot land green.
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
# Step 0d is the wire-derive ratchet: the per-crate count of handwritten
# Serialize/Deserialize derive files outside **/generated/** is frozen in
# wire-derive-baseline.json, and any growth — or a drop the baseline
# never followed — fails the floor. PROP-044 §2 law 5 made a ratchet, not
# a ban, because almost all of today's handwritten derive is lawful; the
# law comment at the step says which and why.
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
#
# What that costs the reader, said here because it has been paid for twice:
# a run that went red says NOTHING about the steps after the red one — they
# did not execute. So "the panel failed at step X" is not a report on the
# panel, and a fixed X does not license a claim about X+1. After any repair
# the panel is re-run END TO END, and the green tail is the only evidence
# that every step ran. Measured 2026-08-14, when a new gate sat behind a
# step that failed first and went untested in a run everyone read as
# covering it; and again 2026-08-17, when a red run stopped at step 9 of 53.

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
CORE_SLOT="$FAMILY_ROOT/core-ai-native/v1.0.0"
PKG_DIR="$FAMILY_ROOT/rust-ai-native-lang/v1.0.0"
TSPKG_DIR="$FAMILY_ROOT/typescript-ai-native-lang/v1.0.0"
GOPKG_DIR="$FAMILY_ROOT/go-ai-native-lang/v1.0.0"
MCPR_DIR="$FAMILY_ROOT/rust-ai-native-mcp/v1.0.0"
MCPT_DIR="$FAMILY_ROOT/typescript-ai-native-mcp/v1.0.0"
MCPG_DIR="$FAMILY_ROOT/go-ai-native-mcp/v1.0.0"
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

# 0d. The wire-derive ratchet (PROP-044 §2, law 5 — FORBID-HANDWRITTEN-WIRE).
# The law bans a handwritten parser or writer of our own wire formats not as
# an end in itself but because that is the mechanism by which the other four
# bans break unnoticed. A flat ban is impossible today: handwritten
# Serialize/Deserialize derives stand in 139 files across eleven crate keys
# (measured 2026-08-17, frozen below), and almost all of it is lawful —
# configs, CLI-local types, internal structs, foreign formats. A
# named-exception list of that length rots within a week, so the form is a
# RATCHET: today's count is frozen per crate in wire-derive-baseline.json
# and any GROWTH goes red. New handwritten wire stops appearing silently;
# not one lawful line is declared a violation today.
#
# Why PER-CRATE and not one number: a single sum hides a transfer — a crate
# that shed ten files and a crate that gained ten net to the same total, and
# the ratchet stays silent exactly where it must speak. Why a DROP is also
# red, with a different recipe: a baseline the tree has moved past is not a
# measurement but a CEILING — wire can grow back under it unnoticed, the
# same reason the map is rebuilt in the same landing that moves the code.
#
# The unit is the FILE, not the occurrence: a file either carries
# handwritten wire or it does not; counting lines is a precision the rule
# does not use.
#
# CODE only, same line-shape filter as the clock gate below: a hit whose
# first non-blank content is `//` is prose — doc-comments legally show
# `#[derive(Serialize)]` while explaining what is banned (the codegen
# postproc prose does exactly that). An inline trailing comment does NOT
# exempt a line: only lines whose first non-blank content is the comment
# pass.
#
# Why the baseline file explains nothing: JSON has no comments. The
# neighbour gates explain themselves in their baseline FILES — conform.toml
# carries its exemption reasons as TOML comments — and this gate cannot, so
# its explanation lives here, next to the recipe.
#
# Lawful handwritten wire INSIDE the baseline, do not "fix" it: the
# cli-package-tree format is registered against a JSON Schema 2020-12 file,
# which the codegen generator deliberately does not touch (codegen routing
# keys on the `*.jtd.json` suffix) — its handwritten emitter is lawful and
# is counted inside vibe-cli's baseline number.
WIRE_DERIVE_BASELINE="wire-derive-baseline.json"
WIRE_DERIVE_PATTERN='^[[:space:]]*#\[derive\([^)]*(Serialize|Deserialize)'

count_wire_derive_files() {
  grep -rlE --include='*.rs' "$WIRE_DERIVE_PATTERN" "$1" 2>/dev/null \
    | grep -v '/generated/' | wc -l | tr -d '[:space:]'
}

check_wire_derive_ratchet() {
  local rc=0 sum=0 dir name measured frozen key
  if [ ! -f "$WIRE_DERIVE_BASELINE" ]; then
    printf 'self-check: `%s` is missing — the wire-derive ratchet has nothing\n' "$WIRE_DERIVE_BASELINE" >&2
    printf 'self-check: to measure against. fix: re-freeze it from today'"'"'s tree\n' >&2
    printf 'self-check: (schema 1, per-crate file counts outside **/generated/**).\n' >&2
    return 1
  fi
  if ! jq -e '(.schema == 1) and (.crates | type == "object")
              and ([.crates[] | type == "number"] | all)' \
      "$WIRE_DERIVE_BASELINE" >/dev/null 2>&1; then
    printf 'self-check: `%s` is not a schema-1 baseline of per-crate numbers\n' "$WIRE_DERIVE_BASELINE" >&2
    printf 'self-check: ({"schema":1,"crates":{"<crate>":N,...}}). fix: re-freeze it\n' >&2
    printf 'self-check: from today'"'"'s tree.\n' >&2
    return 1
  fi
  for dir in crates/*/ xtask/; do
    [ -d "$dir" ] || continue
    name="${dir%/}"; name="${name#crates/}"
    measured="$(count_wire_derive_files "$dir")"
    # `tr -d '\r'`: a Windows-native jq writes CRLF even on a pipe, and a
    # trailing CR makes `[ -gt ]`/`[ -d ]` misjudge silently.
    frozen="$(jq -r --arg k "$name" '.crates[$k] // ""' "$WIRE_DERIVE_BASELINE" | tr -d '\r')"
    if [ -z "$frozen" ]; then
      if [ "$measured" -gt 0 ]; then
        printf 'self-check: `%s` carries %d handwritten Serialize/Deserialize derive\n' "$name" "$measured" >&2
        printf 'self-check: file(s) and is absent from %s. the rule — a new crate with\n' "$WIRE_DERIVE_BASELINE" >&2
        printf 'self-check: handwritten wire must be NAMED, not silent (PROP-044 §2 law 5:\n' >&2
        printf 'self-check: a handwritten parser or writer of our own format is the mechanism\n' >&2
        printf 'self-check: by which the other four bans break unnoticed). fix: add `%s` to\n' "$name" >&2
        printf 'self-check: %s in the same commit and say in the commit body what the type is.\n' "$WIRE_DERIVE_BASELINE" >&2
        rc=1
      fi
      continue
    fi
    if [ "$measured" -gt "$frozen" ]; then
      printf 'self-check: `%s` carries %d handwritten Serialize/Deserialize derive file(s);\n' "$name" "$measured" >&2
      printf 'self-check: %s froze %d. the rule — handwritten wire grows only through a\n' "$WIRE_DERIVE_BASELINE" "$frozen" >&2
      printf 'self-check: named decision, never silently (PROP-044 §2 law 5: a handwritten\n' >&2
      printf 'self-check: parser or writer of our own format is the mechanism by which the\n' >&2
      printf 'self-check: other four bans break unnoticed). fix: describe the format as a schema\n' >&2
      printf 'self-check: and run `cargo xtask codegen`; if the new type is NOT our wire (a\n' >&2
      printf 'self-check: config, a CLI-local struct, a foreign format), raise the `%s` count\n' "$name" >&2
      printf 'self-check: in %s in the same commit and say in the commit body what the type is.\n' "$WIRE_DERIVE_BASELINE" >&2
      rc=1
    elif [ "$measured" -lt "$frozen" ]; then
      printf 'self-check: `%s` carries %d handwritten Serialize/Deserialize derive file(s);\n' "$name" "$measured" >&2
      printf 'self-check: %s froze %d — the rule has tightened past its baseline. the rule —\n' "$WIRE_DERIVE_BASELINE" "$frozen" >&2
      printf 'self-check: a stale baseline is not a measurement but a CEILING: wire can grow\n' >&2
      printf 'self-check: back under it unnoticed (PROP-044 §2 law 5). fix: lower the `%s`\n' "$name" >&2
      printf 'self-check: count in %s in the same commit.\n' "$WIRE_DERIVE_BASELINE" >&2
      rc=1
    else
      sum=$((sum + measured))
    fi
  done
  for key in $(jq -r '.crates | keys[]' "$WIRE_DERIVE_BASELINE" | tr -d '\r'); do
    case "$key" in
      xtask) [ -d xtask ] && continue ;;
      *) [ -d "crates/$key" ] && continue ;;
    esac
    printf 'self-check: %s names `%s`, which the tree no longer has — the baseline\n' "$WIRE_DERIVE_BASELINE" "$key" >&2
    printf 'self-check: describes a tree that no longer exists. the rule — the ratchet\n' >&2
    printf 'self-check: measures only while it measures the real tree (PROP-044 §2 law 5).\n' >&2
    printf 'self-check: fix: drop the `%s` key from %s in the same commit.\n' "$key" "$WIRE_DERIVE_BASELINE" >&2
    rc=1
  done
  if [ "$rc" -eq 0 ]; then
    [ "$QUIET" -ne 0 ] ||
      printf 'self-check: wire-derive ratchet holds: %d handwritten derive file(s) match %s.\n' \
        "$sum" "$WIRE_DERIVE_BASELINE" >&2
  fi
  return "$rc"
}
run_step "wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json)" \
  check_wire_derive_ratchet || OVERALL=$?

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

# 6c. Host traceability-index gate (B-014). `specmap.json` is a committed
# derived artifact and nothing compared it against the tree: measured
# 2026-08-05, 599 of 5266 units' recorded line no longer landed on their
# anchor, and the drift had accumulated unseen because the panel's specmap
# steps below are the PACKAGES' own self-traces, never the host's. The same
# run also carries the ratchet: a public item in a gated crate with no spec
# tag is an orphan and fails here rather than in a reader's confusion.
# Regeneration is `cargo xtask specmap`.
run_step "cargo xtask specmap --check" cargo xtask specmap --check || OVERALL=$?

# 6d. The wire-diff verdict (PROP-044 §4.7 ##M-BREAK-WINDOW). The LAW this
# step enforces: the gate does not forbid breaks — it makes an UNANNOUNCED
# break impossible. Breaking in this project is lawful and cheap; breaking
# unnoticed is not. The verb re-proves every registered golden corpus
# against its journal (the rebuild projector, reused), asks git whether the
# corpus bytes shifted vs the commit, and reads the regime out of
# formats/EPOCHS.toml: pre-publication (public = false) a shift is green
# but REPORTED — the step names what moved, so a green line is never
# misread as "nothing changed"; a closed window rejects wire changes
# outright; an open public window demands a break note added in the same
# change (formats/breaks/NNN.md — git must see it as new, so an old note
# never counts). Without this step the corpus and the flags were facts with
# no verdict joining them: an unannounced break would land green.
run_step "cargo xtask wire-diff" cargo xtask wire-diff || OVERALL=$?

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

# 10b. The discipline engine, pointed at its own sources (B-057). Steps 7-10
# run fmt/test/clippy and the specmap self-traces over every live package
# workspace — but `cargo xtask conform check` (step 5) is HOST-only, so the
# Class-F/G rules, the file-length budget and the unsafe gate never ran over
# the code that IMPLEMENTS them. The panel said nothing false about it (no
# step claimed the coverage), which is why this was uncovered ground rather
# than a lying gate — but a projection where the discipline does not apply is
# still a projection.
#
# One binary, built once from the rust stack, run per slot: the policy stays
# with the consumer (PROP-024), so each slot carries its own `conform.toml`
# and its own ratchet baseline. The loop is driven by GATED_SLOTS, so step
# 0b's live-set denominator already covers this step too — a new package
# cannot appear without its conform run appearing with it.
#
# What the first run measured, so a later reader knows what these numbers
# mean: 134 findings over the four authored slots, 102 of them one rule
# (seam-has-doctest). None of that was frozen. The policies gate the crates
# that are already clean and name every other crate `exempt` WITH its finding
# count — the same expand-as-you-conform posture the host's own conform.toml
# takes. The debt stays visible in the config instead of buried in a ratchet.
for slot in $GATED_SLOTS; do
  run_step "rust-ai-native-conform check ($slot)" \
    cargo run --quiet --manifest-path "$PKG_MANIFEST" \
      -p rust-ai-native-conform --bin rust-ai-native-conform \
      -- --path "$slot" check || OVERALL=$?
done

# 10c. The mcp packages' authored-crate denominator. Their conform policy
# names the scan perimeter LITERALLY (`roots = ["crates/<authored>"]`) rather
# than by glob, because the glob would scan the vendored copies too and the
# engine's `exclude_substrings` cannot exclude a crate directory — it matches
# the CRATE-relative path (`src/lib.rs`), never the repo-relative one the
# findings print (B-059). A literal perimeter is correct but loses what a glob
# gave for free: the gated-or-exempt invariant that forces a NEW crate to be
# classified. This restores it, derived rather than spelled — the authored set
# is whatever `crates/` holds minus `vendor/` minus what sync-engines.toml
# declares vendored INTO that slot. A version bump or a new copy changes the
# manifest and this guard follows; a new AUTHORED crate makes it red.
mcp_vendored_into() {
  awk -v want="\"$1\"" 'BEGIN { RS = "" }
    index($0, want) == 0 { next }
    {
      s = index($0, "crates = [")
      if (s == 0) next
      rest = substr($0, s + 10)
      e = index(rest, "]")
      if (e == 0) next
      n = split(substr(rest, 1, e - 1), parts, "\"")
      for (i = 2; i <= n; i += 2) if (parts[i] != "") print parts[i]
    }
  ' sync-engines.toml
}
check_mcp_authored_denominator() {
  local slot rc=0 authored declared
  for slot in "$MCPR_DIR" "$MCPT_DIR" "$MCPG_DIR"; do
    authored="$(comm -23 \
      <(ls -1 "$slot/crates" | grep -v '^vendor$' | sort) \
      <(mcp_vendored_into "$slot/crates" | sort -u) | tr '\n' ' ')"
    declared="$(sed -n 's/^roots = \["crates\/\([^"]*\)"\]$/\1 /p' "$slot/conform.toml")"
    if [ "$authored" != "$declared" ]; then
      echo "self-check: \`$slot\` authored crates are {${authored% }}," >&2
      echo "self-check: but conform.toml scans {${declared% }}." >&2
      echo "self-check: classify the newcomer — an AUTHORED crate belongs in" >&2
      echo "self-check: this slot's conform.toml (roots + gated); a VENDORED" >&2
      echo "self-check: copy belongs in sync-engines.toml. Neither is optional." >&2
      rc=1
    fi
  done
  return "$rc"
}
run_step "mcp packages' authored crates are the conform perimeter" \
  check_mcp_authored_denominator || OVERALL=$?

# 10d. The index clock gate (F2-1). Time enters at the EDGE — the CLI
# command moment or the server mutation event — and never inside the
# writer modules: a writer that calls `now()` itself makes "rebuild and
# compare" measure nothing, because two writes of one state stamp two
# different instants (determinism is the measuring instrument,
# PROP-044 §4.3). The perimeter is named by module directory, not by a
# repo-wide mask: `crates/vibe-index/src/index`,
# `crates/vibe-index/src/types` and `crates/vibe-index/src/journal`,
# recursively — a new file under any of them is covered the day it
# lands.
#
# CODE only: a hit whose line opens with a comment marker is prose —
# doc-comments legally show `Utc::now()` in examples (e.g. the scanner
# doctest) — so lines starting with `//` are filtered by line shape
# before the verdict. An inline trailing comment does NOT exempt a
# line: only lines whose first non-blank content is the comment pass.
check_index_clock_gate() {
  local hits
  hits=$(grep -rnE 'Utc::now\(|SystemTime::now\(' \
      crates/vibe-index/src/index \
      crates/vibe-index/src/types \
      crates/vibe-index/src/journal \
      2>/dev/null \
    | grep -vE ':[0-9]+:[[:space:]]*//')
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    printf 'self-check: the index writer modules call the clock directly.\n' >&2
    printf 'self-check: the rule — time enters at the edge (CLI command or\n' >&2
    printf 'self-check: server mutation event) and never inside index/, types/ or journal/:\n' >&2
    printf 'self-check: one state must produce one byte sequence, or "rebuild and\n' >&2
    printf 'self-check: compare" measures nothing (PROP-044 §4.3, F2-1).\n' >&2
    printf 'self-check: fix: pass the time as an argument — a WriteCtx for\n' >&2
    printf 'self-check: write_to, an `at` for Index::new / VersionEntry::minimal.\n' >&2
    return 1
  fi
  return 0
}
run_step "index clock gate (no Utc::now/SystemTime::now in index/, types/ or journal/)" \
  check_index_clock_gate || OVERALL=$?

# 11. The vibeterm / vibeframe terminal products moved to a separate repo
# (`vibevm-term`); their pure-logic tests (`node --test` for the shared
# arg/keymap helpers, `vitest` for the vibeterm engine cells) live there now.
# The host's self-check no longer runs them — the vibevm-term repo carries
# its own floor.

# 11b. B-011's lane-citation lint (PROP-035 §11
# ##COMPILED-LANE-IS-NOT-A-CITATION-TARGET): authored text never TARGETS a
# compiled STATIC lane — no `@spec://…/boot/STATIC#…` in-place use and no
# directive whose address lands there. The directive scanner already rejects
# these at compile time; this is the tree-level half over the authored trees.
# Prose that merely mentions such an address (the rule's own statement, the
# design doc) is legal — only the sigil/directive forms are the violation,
# which is why the grep matches those forms and not the bare address.
check_lane_citations() {
  local hits
  hits=$(grep -rEn --include='*.md' --include='*.xml' \
      -e '@spec://[^[:space:]`]*/boot/STATIC#' \
      -e '^[[:space:]]*#(use|embed|source)[[:space:]]+[^ ]*spec://[^[:space:]]*/boot/STATIC' \
      spec/ packages/ crates/ 2>/dev/null \
    | grep -vE 'spec/boot/STATIC\.(md|xml)' \
    | grep -v 'legacy-spec/')
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    printf 'authored text targets a compiled STATIC lane (PROP-035 §11)\n' >&2
    return 1
  fi
  return 0
}
run_step "lane-citation lint (B-011)" check_lane_citations || OVERALL=$?

# 11c. A plan is temporary and a spec is permanent: once a plan is closed,
# spec prose must not send a reader to that file for content. Active-plan
# links remain lawful, and spec/WAL.md is excluded because it is a volatile
# checkpoint rather than a contract. Dated provenance survives by plan name
# and date, without a file path, so this deliberately dumb path gate does not
# see that third genre.
CLOSED_PLANS_REGISTRY="campaigns/CLOSED-PLANS.toml"

check_closed_plan_links() {
  local plan_path basename hits rc=0
  if [ ! -f "$CLOSED_PLANS_REGISTRY" ]; then
    printf 'self-check: `%s` is missing — closed plans have no gate input.\n' \
      "$CLOSED_PLANS_REGISTRY" >&2
    printf 'self-check: fix: restore the schema-1 closed-plan registry.\n' >&2
    return 1
  fi

  # This is deliberately not a TOML parser: schema 1 gives every entry one
  # literal `path = "…"` line, so grep plus quote stripping is sufficient.
  while IFS= read -r plan_path; do
    [ -n "$plan_path" ] || continue
    basename="${plan_path##*/}"
    # A closed plan may have converted serialisations (PROP-051): match the
    # recorded spelling AND its md/xml twin, over both source forms.
    twin_path=""; twin_base=""
    case "$plan_path" in
      *.md)  twin_path="${plan_path%.md}.xml" ;;
      *.xml) twin_path="${plan_path%.xml}.md" ;;
    esac
    [ -n "$twin_path" ] && twin_base="${twin_path##*/}"
    hits=$(grep -rFn --include='*.md' --include='*.xml' \
        -e "$plan_path" -e "$basename" \
        ${twin_path:+-e "$twin_path"} ${twin_base:+-e "$twin_base"} \
        spec/ 2>/dev/null \
      | grep -vE '^spec/WAL\.(md|xml):')
    if [ -n "$hits" ]; then
      printf '%s\n' "$hits" \
        | sed -E 's/^([^:]+:[0-9]+):.*$/\1/' \
        | while IFS= read -r location; do
            printf '%s · закрытый план %s\n' "$location" "$basename" >&2
          done
      rc=1
    fi
  done < <(grep -E '^[[:space:]]*path[[:space:]]*=[[:space:]]*"[^"]+"[[:space:]]*$' \
      "$CLOSED_PLANS_REGISTRY" | sed -E 's/^[^"]*"([^"]+)"[[:space:]]*$/\1/')

  if [ "$rc" -ne 0 ]; then
    printf 'self-check: fix: замени ссылку датированным provenance-упоминанием по имени\n' >&2
    printf 'self-check: (жанр 3) или удали.\n' >&2
  else
    [ "$QUIET" -ne 0 ] ||
      printf 'self-check: specs do not link to a closed plan.\n' >&2
  fi
  return "$rc"
}
run_step "spec does not link to a closed plan" \
  check_closed_plan_links || OVERALL=$?

# 11d. PROP-000 §3 ##NO-CRATES-IO states that crates in this workspace set
# `license-file = "LICENSE.md"` and `publish = false` — so that none can be
# pushed to crates.io by accident, and so the shipped surface carries one
# licence. Nothing checked it, and one member drifted out of it silently for
# months: `vibe-index` carried full package metadata and no licence key of any
# kind, while the owner-maintained ledger asserted every host crate inherits
# one. An audit walk found it; no gate could have. The member list is read from
# the workspace manifest rather than restated here, so a crate added tomorrow is
# covered without anyone remembering this step exists.
check_member_licence_keys() {
  local members missing=""
  members=$(sed -n '/^members = \[/,/^]/p' Cargo.toml \
    | grep -oE '"[^"]+"' | tr -d '"')
  if [ -z "$members" ]; then
    printf 'could not read `members` from the workspace Cargo.toml\n' >&2
    return 1
  fi
  for m in $members; do
    local manifest="$m/Cargo.toml"
    if [ ! -f "$manifest" ]; then
      missing="$missing\n  $manifest — listed as a member, absent from the tree"
      continue
    fi
    grep -qE '^license(-file)?(\.workspace)? *=' "$manifest" \
      || missing="$missing\n  $manifest — no license/license-file key"
    grep -qE '^publish(\.workspace)? *=' "$manifest" \
      || missing="$missing\n  $manifest — no publish key"
  done
  if [ -n "$missing" ]; then
    printf 'workspace members that do not declare PROP-000 §3 #license:%b\n' "$missing" >&2
    printf 'fix: add `license-file.workspace = true` / `publish.workspace = true`\n' >&2
    return 1
  fi
  return 0
}
run_step "every workspace member declares its licence (PROP-000 §3)" \
  check_member_licence_keys || OVERALL=$?

# 11b. Markup validation. The corpus markup is what the whole campaign rests
# on — a fact with no marker is judged by nobody — and until now NO gate ran
# a progress verb at all, so between phase boundaries a markup error lived
# for exactly as long as it took someone to think of checking by hand. It is
# not theoretical: a design document was committed carrying five unmarked
# facts and nothing said anything — not this panel, not `vibe check`, not
# conform, not the map.
#
# `--exhaustive` is the point of the step. The default form walks the same
# tree and prints `clean` without looking at unmarked facts at all: measured
# here on a probe file, the plain form reported `clean (276 files)` while the
# exhaustive form reported the error and exited 1.
#
# No `--campaign`: the zone comes from the host's own `progress.toml`, which
# is the right answer when the panel runs outside a campaign, and stays right
# inside one.
#
# Safe beside the tripwire, and that was measured rather than assumed before
# this step was added: the settings home was snapshotted by content across
# 169 files, both forms of the verb were run, and nothing moved.
run_step "markup validation (vibe facts check --exhaustive)" \
  cargo run --quiet -p vibe-cli -- facts check --exhaustive || OVERALL=$?

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
