# CONTINUE.md — cold-resume checkpoint (2026-07-14, cont.)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

The active work is the **spec-compiler mission** — the owner's "inline vision" (`refs/inline-vision.md`):
turn vibevm's boot loading into a real **preprocessor + linker for the context budget**, a two-mode
compiler (inline = algorithmic AOT, structural = lazy JIT) over one directive semantics
(`#embed` / `#use` / `#source` + `@spec`). It is specified in **PROP-035**
(`spec/modules/vibe-workspace/PROP-035-spec-compiler.md`, which supersedes/folds PROP-034) and built as a
new host crate **`crates/vibe-spec`**. **9 slices landed, 66 tests, fmt/clippy green — the whole inline
compiler works end-to-end** (`spec://` address → `INLINE.md`), driven on a throwaway demo corpus.

**The earlier cultural-refactor is DONE and merged: `cultural-refactor` was fast-forwarded into `main`
(42 commits) and pushed. All work is now on `main`; `main == origin == github @ 2f12a85`.** No blocker —
each slice is a safe stop.

**Immediate next:** continue PROP-035 — the remaining compiler pieces (source-merge into the pipeline,
§9/§10/§11/§12, the structural loader §13), then the payoff: **wire the compiler into `bootgen`**.

## Where work stands

- Branch **`main`** @ `2f12a85`, working tree **clean**, **`main == origin/main == github/main`** (pushed
  both remotes). The host now lives on `main` (the old `cultural-refactor` branch still points at the same
  commit and can be deleted; a `cultural-backup` branch + `pre-cultural-refactor` tag remain as history).
- **Gate state:** `cargo test -p vibe-spec` = 66 passing; `cargo clippy -p vibe-spec --all-targets --
  -D warnings` clean; `cargo fmt -p vibe-spec` applied. (The full-workspace `bash tools/self-check.sh` was
  NOT re-run this session — the work is an isolated new crate that nothing else depends on yet.)
- **specmap:** `specmap.json` was left parked (the crate adds no `spec/**` units; PROP-035.md is prose).

## Active blocker + the exact unblock

**None.** The work is a clean incremental build on `main`; the next slice can start immediately.

## Next-steps recipe

The crate `crates/vibe-spec` is the router + directive layer + inline pipeline. Remaining PROP-035 work,
in a sensible order:

1. **Source-merge into the pipeline (§8 phase 3).** `merge.rs` (`merge_contract_source`) exists but the
   `pipeline.rs` `compile_inline` does not yet call it — `#source` contract→impl resolution needs wiring
   (find a contract's source via its `#source` directive, merge, then embed-expand). Add to `compile_inline`
   between topo and embed.
2. **Contract-cycle admission (§9).** `use_graph.rs` reports *all* cycles as errors; §9 makes a `#use`
   cycle *between contracts* legal (forward declaration). Admit contract-only cycles at the emission layer.
3. **Link tables (§10)** — the vtable analogue: an install-time edge index (`contract→source`, use-graph,
   anchor-index) so the structural side gets "global knowledge" cheaply. Reuses `specmap.json` infra.
4. **Full markers (§11)** — file/package open+close markers (the crate has node + embed markers today).
5. **transitive-inline (§12)** — fold in PROP-034's transitive-link/dedup/topo emission.
6. **Structural loader prompt (§13)** — the first-loaded instructions that make an LLM honour the directives.
7. **The payoff — wire into `bootgen`** (`crates/vibe-workspace/src/install/bootgen.rs`): replace the naive
   `INLINE.md` concatenation with `compile_inline`. This is the point of the whole mission.

Migration posture (§15): keep testing on the demo corpus; then migrate `org.vibevm.world`; vibevm's own
core specs last.

## Non-obvious findings (do not re-learn)

- **Gate loop for `vibe-spec`:** `cargo fmt -p vibe-spec && cargo test -p vibe-spec && cargo clippy -p
  vibe-spec --all-targets -- -D warnings`. Edition 2024 clippy is strict — it flagged `collapsible-if`
  (use let-chains), `map-entry` (use `Entry`), `manual-pattern-char-comparison` (pass a `[char]` to
  `trim_end_matches`), all fixed inline.
- **No `spec:// → path` resolver existed before this crate** (verified by two recon passes): the specmap
  engine only mints `path → URI` into `specmap.json`, with `PROP-NNN` truncation, `packages/` vs
  `vibedeps/` split, and no version in the URI. `vibe-spec` is that missing router.
- **`spec://` grammar is our own** — the vendored `specmark-grammar` (`packages/…/vendor/
  core-ai-native-specmark-grammar`) rejects both `@version` and dotted tree-path anchors and is a
  sync-engines-frozen snapshot, so it must not be edited from the host; we reproduce its flat-anchor kebab
  rule per-segment for byte-compatibility.
- **Router resolves against `vibedeps/` + host `spec/`, never `packages/`** — consistent with the specmap
  engine (ratified §6).
- **`#embed` is materialization-time / mode-independent** — `vibedeps` stores embeds pre-expanded; editing
  an embedded source needs re-materialization (a decision ratified this session).
- **Commits:** heredoc only (`git commit -F - <<'MSG'`), **never** `-m` with backticks. **No AI-authorship
  trailers, ever** (Rule 1 overrides the harness default).
- **Editing:** Edit/Write only — PowerShell 5.1 corrupts UTF-8-no-BOM round-trips. WAL is too big for the
  Read tool (it token-counts the whole file); read its head via `Get-Content -TotalCount`, edit via unique
  anchors.
- **Trio byte-identity:** `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` must stay byte-identical (self-check's
  `sync-engines`) — apply every trio edit to all three. (Not touched this session.)

## Repository map

- `crates/vibe-spec` — **the new spec compiler** (this mission). Modules: `address` (spec:// grammar),
  `doctree` (hierarchical IR), `resolver` (doc→file + `tests/fixtures/ws` demo corpus), `directives`
  (scan), `merge` (contract/source), `embed` (#embed expand), `use_graph` (#use topo), `pipeline`
  (`compile_inline`). Integration tests: `tests/resolve.rs`, `tests/embed.rs`, `tests/compile.rs`.
- `crates/vibe-*` — the rest of the Rust workspace: `vibe-cli`, `vibe-core` (manifests/graph), `vibe-workspace`
  (install + **bootgen** — where the compiler gets wired in), `vibe-registry`, `vibe-resolver`, `vibe-index`,
  `vibe-check`, `vibe-publish`, `vibe-llm`, `vibe-mcp`, `vibe-wire`, `vibe-graph`; `xtask/` (specmap, mirror,
  health).
- `packages/org.vibevm.*/**` — the extracted practice flows + language stacks + fractality (all UPL-1.0). The
  specmap/conform/specmark engines are vendored under `…/rust-ai-native-lang/v0.7.0/crates/vendor/`.
- `spec/` — `boot/` (00-core, 90-user, generated INDEX.md + INLINE.md), `common/` (PROP-000, PROP-028,
  PROP-029, …), `modules/` (per-crate PROP/FEAT incl. PROP-009 loading-model, **PROP-034**, **PROP-035**),
  `design/`, `WAL.md`.
- Root: `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (byte-identical trio), `vibe.toml`, `vibe.lock`, `refs/`
  (third-party / owner notes incl. `inline-vision.md`).

## Architectural / policy decisions in force

- **PROP-035 is the flagship design** (provisional): two build modes (inline algorithmic AOT / structural
  lazy JIT) with an equivalence invariant (inline compiler = reference semantics; differential testing
  deferred, §16); `simple` vs `normal` package formats; the `contract`/`source` split; a hierarchical
  document IR (Markdown now, XML later); the deterministic `spec://` router; the five-phase pipeline with a
  fixed embed order; C++-derived cycle rules (contract cycles legal, source topological); link tables as the
  vtable analogue.
- **Delegation was deliberately *not* used for `vibe-spec`** — it is load-bearing fundational code whose
  context lives in the design dialogue; per the delegation calculus (generate when verification isn't
  cheaper), the boss authored it. Delegation stays reserved for the bulk parallel work (migrating real
  packages onto the new format).
- Repo rules: Rules 1–4 (human-authored attribution, Conventional Commits, atomicity, autonomy) are the
  `git-practices` family; source is dual-homed (GitVerse `origin` canonical + GitHub mirror).

## Recent commits (last 25)

```
2f12a85 feat(vibe-spec): compile the inline pipeline (PROP-035)
314ec01 feat(vibe-spec): topo-sort the #use graph (PROP-035)
02209fc feat(vibe-spec): expand #embed to a fixed point (PROP-035)
49b0082 feat(vibe-spec): merge contract/source sections (PROP-035)
aa64f25 feat(vibe-spec): scan directives and in-place uses (PROP-035)
4b8dc04 feat(vibe-spec): resolve addresses to files + demo corpus (PROP-035)
b4dbeb0 feat(vibe-spec): resolve tree-path anchors to nodes (PROP-035)
8b65a74 feat(vibe-spec): add the document IR tree (PROP-035)
d98fd15 feat(vibe-spec): add the spec:// address parser (PROP-035)
b833e26 spec(vibe-workspace): draft PROP-035 spec-compiler design
c76e568 docs(wal): session-end checkpoint — the cultural-refactor
8cf097f docs(continue): cold-resume checkpoint for the cultural-refactor
3e46162 spec(vibe-workspace): PROP-034 — transitive links + the static boot-link graph
1d9aa2f docs(backlog): mark B4 done — trio delegation block thinned
4720d65 refactor(delegation): thin the trio's fractality operational block (B4)
661e842 docs(backlog): record B4 — finish thinning the trio delegation block
ca9356a refactor(boot): reduce the trio's commit rules to a git-practices pointer
71971e6 refactor(delegation): drop the general obligations from the trio block
a470a77 refactor(fractality): reshape delegation-first per owner review
c8a1aa8 refactor(delegation): thin the trio directive to a delegation-first pointer
09151bf feat(host): depend on delegation-first (static)
0ef57b2 feat(fractality): author the delegation-first flow package
4d5ccf8 refactor(spec): reduce PROP-006 to an operating-modes pointer
ebffebf docs(refactor): light remainder done; §3 settled; Section B/D remain
a210598 refactor(spec): cite the decision-records genre from the design README
```

## Quick-start

```sh
# the spec-compiler crate (this mission)
cargo fmt -p vibe-spec
cargo test -p vibe-spec        # expect 66 passing
cargo clippy -p vibe-spec --all-targets -- -D warnings

# build the working-tree vibe binary (never the PATH vibe) when touching install/bootgen
cargo build -p vibe-cli
```
