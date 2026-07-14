# CONTINUE.md — cold-resume checkpoint (2026-07-15)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

The **spec-compiler mission (PROP-035)** — the owner's "inline vision": boot loading as a real
**preprocessor + linker for the context budget**, a two-mode compiler (inline = algorithmic AOT,
structural = lazy JIT) over one directive semantics (`#embed` / `#use` / `#source` / `@spec`) — is
**COMPLETE**. It is specified in `spec/modules/vibe-workspace/PROP-035-spec-compiler.md` and built as the
new host crate **`crates/vibe-spec`**, then **wired into the live `bootgen`** (the payoff), with the
**transitive-inline link** (§12) added. **Full-workspace `bash tools/self-check.sh` is green.**

All work is on **`main`** @ `4d41f5a`, pushed to both remotes (`origin` GitVerse + `github`).

**No blocker.** The remaining work is post-mission and non-urgent: §16 equivalence testing (empirical,
deferred by design) and the §15 migration of real packages onto the format (demo → `org.vibevm.world` →
vibevm's own specs last).

## Where work stands

- Branch **`main`** @ `4d41f5a`, working tree **clean**, `main == origin/main == github/main` (pushed).
- **Gate state: full `bash tools/self-check.sh` GREEN** — fmt, `cargo test --workspace`, clippy
  `-D warnings`, `vibe check` (0/0/0), `cargo xtask conform check`, and the specmap ratchet gate all pass.
- The live boot is **untouched**: the payoff is guarded, and vibevm's own boot lane carries zero
  directives, so `INLINE.md` is byte-identical (verified: `grep '^#\(embed\|use\|source\) '` over
  `vibedeps/` + `spec/boot/` = 0 matches).

## Active blocker + the exact unblock

**None.** The mission is complete and green.

## What was built (this session)

The whole PROP-035 system, each piece committed + pushed, all green:

1. **`crates/vibe-spec`** — the compiler, 13 slices: `address` (spec:// grammar), `doctree` (hierarchical
   IR + `resolve_path` + `:add`/`:replace` trailing), `resolver` (`doc_path→file` + `tests/fixtures/ws`
   demo corpus), `directives` (scan `#embed`/`#use`/`#source` + `@spec`), `merge` (contract↔source +
   `fold_source`), `embed` (`#embed` expand to fixed point + cycle guard), `use_graph` (`#use` topo-sort,
   tree-shaking, contract-cycle admission §9), `pipeline` (`compile_inline`, the 5 phases incl. source-fold),
   `link_table` (§10 vtable analogue), `markers` (reversible §11 + `decompile`).
2. **Structural loader §13** — `spec/design/structural-loader.md` (provisional prompt; **not** wired into
   live boot).
3. **The payoff** — `crates/vibe-workspace/src/boot_artifacts.rs::render_inline` now runs the assembled
   inline lane through `vibe_spec::expand_embeds`, **guarded** by `has_embed_directive` so a directive-free
   lane is byte-identical. `vibe-workspace` depends on `vibe-spec`.
4. **transitive-inline §12** — `vibe_core::manifest::LinkType::InlineTransitive` (wire `"inline-transitive"`)
   + `bootgen::inline_transitive_closure` propagation, resolved to `inline` at emission (`boot.rs`).

## Next-steps recipe (post-mission, optional)

1. **Migration (§15)** — adopt the format on real packages, in order: the demo corpus (done) → all of
   `org.vibevm.world` → vibevm's own core specs **last**. A package adopts by splitting into `contract/` +
   `source/`, using directives, and loading `spec/design/structural-loader.md` (or its successor) first.
2. **Equivalence testing (§16)** — differential-test the inline compiler (reference semantics) against the
   structural loader on a corpus. Empirical, expensive; deferred by design until a working base existed —
   which now exists.
3. **Nested-section source-merge** — `fold_source` folds at the top level (the flat-contract norm, §4);
   nested-section merge is the documented refinement.
4. **Fold `vibe-spec` into `gated_crates`** once it is spec-tagged + REQ-edged (it is `exempt` today).

## Non-obvious findings (do not re-learn)

- **Gate loop:** `cargo fmt`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, then the full
  `bash tools/self-check.sh` before finishing (it runs `vibe check` + `conform` + specmap too). A **new
  crate must be classified in `conform.toml`** (gated or exempt) or `every_crate_is_gated_or_exempt` fails.
  The **600-line file budget** is a conform gate — keep tests out-of-line (`#[cfg(test)] #[path] mod tests;`).
- **The payoff is guarded** — `render_inline` only compiles when `has_embed_directive` is true, so legacy
  boot is byte-identical. `#use` / `#source` in boot content are **not** processed by the payoff (they are
  mode-dependent, left to the structural loader §13).
- **`spec://` grammar is our own** (`vibe-spec::address`) — the vendored `specmark-grammar` rejects
  `@version` + dotted tree-path and is sync-engines-frozen. Router resolves against `vibedeps/` + host
  `spec/`, never `packages/`.
- **Commits:** heredoc only (`git commit -F - <<'MSG'`), never `-m` with backticks. **No AI-authorship
  trailers, ever** (Rule 1 overrides the harness default).
- **Editing:** Edit/Write only (PS 5.1 corrupts UTF-8-no-BOM). WAL is too big for the Read tool — read its
  head via `Get-Content -TotalCount`, edit via unique anchors.

## Repository map

- `crates/vibe-spec` — **the spec compiler** (this mission): `address`, `doctree` (+ `doctree`… inline),
  `resolver`, `directives`, `merge`, `embed`, `use_graph`, `pipeline`, `link_table`, `markers`; integration
  tests `tests/{resolve,embed,compile}.rs`; demo corpus `tests/fixtures/ws`.
- `crates/vibe-workspace` — install + **bootgen** (the payoff lives in `boot_artifacts::render_inline`;
  transitive-inline in `install/bootgen.rs` + `boot.rs`). `crates/vibe-core` — manifests incl. `LinkType`.
  Other `crates/vibe-*` as before.
- `packages/org.vibevm.*/**` — practice flows + language stacks + fractality (all UPL-1.0); vendored engines
  under `…/rust-ai-native-lang/v0.7.0/crates/vendor/`.
- `spec/` — `boot/`, `common/`, `modules/` (PROP-009, PROP-034, **PROP-035**), `design/` (incl.
  **`structural-loader.md`**), `WAL.md`.
- Root: `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (byte-identical trio), `conform.toml`, `vibe.toml`, `vibe.lock`.

## Architectural / policy decisions in force

- **PROP-035 is the shipped design**: two build modes (inline algorithmic AOT / structural lazy JIT) with an
  equivalence invariant (inline compiler = reference semantics; differential testing is §16, deferred);
  `simple`/`normal` package formats; the `contract`/`source` split; a hierarchical document IR (Markdown now,
  XML later); the deterministic `spec://` router; the five-phase pipeline with a fixed embed order;
  C++-derived cycle rules (contract cycles legal, source topological); link tables as the vtable analogue.
- **The payoff is conservative by §15**: the compiler is wired in but only acts on directive-bearing lanes,
  so vibevm's own boot is untouched until it deliberately adopts the format (vibevm migrates last).
- **Delegation was deliberately not used** for `vibe-spec` — load-bearing fundational code whose context
  lived in the design dialogue; per the delegation calculus the boss authored it. Delegation stays reserved
  for the bulk parallel migration work.
- Repo rules: Rules 1–4 (human-authored attribution, Conventional Commits, atomicity, autonomy); source is
  dual-homed (GitVerse `origin` canonical + GitHub mirror).

## Recent commits (last 25)

```
4d41f5a refactor(vibe-workspace): move boot tests out-of-line (file budget)
8a3ce96 chore(conform): classify the vibe-spec crate as exempt
2416f34 feat(vibe-workspace): the inline-transitive link (PROP-035 §12)
abe0c59 feat(vibe-workspace): compile the inline boot lane (PROP-035)
4d7a643 docs(spec-compiler): author the structural loader (PROP-035 §13)
edbe976 feat(vibe-spec): reversible block markers and decompile (PROP-035)
4eb83f9 feat(vibe-spec): build directive link tables (PROP-035 §10)
7adbaa1 feat(vibe-spec): wire the source-fold into compile_inline (PROP-035)
5fd2232 feat(vibe-spec): fold source into contract document (PROP-035)
5e949e1 feat(vibe-spec): admit contract-only #use cycles (PROP-035)
2f12a85 feat(vibe-spec): compile the inline pipeline (PROP-035)
314ec01 feat(vibe-spec): topo-sort the #use graph (PROP-035)
02209fc feat(vibe-spec): expand #embed to a fixed point (PROP-035)
49b0082 feat(vibe-spec): merge contract/source sections (PROP-035)
aa64f25 feat(vibe-spec): scan directives and in-place uses (PROP-035)
4b8dc04 feat(vibe-spec): resolve addresses to files + demo corpus (PROP-035)
b4dbeb0 feat(vibe-spec): resolve tree-path anchors to nodes (PROP-035)
8b65a74 feat(vibe-spec): add the document IR tree (PROP-035)
d98fd15 feat(vibe-spec): add the spec:// address parser (PROP-035)
c46a45f docs(continue): cold-resume checkpoint — spec-compiler on main
d39d1ff docs(wal): checkpoint — the spec-compiler mission on main
b833e26 spec(vibe-workspace): draft PROP-035 spec-compiler design
c76e568 docs(wal): session-end checkpoint — the cultural-refactor
8cf097f docs(continue): cold-resume checkpoint for the cultural-refactor
3e46162 spec(vibe-workspace): PROP-034 — transitive links + the static boot-link graph
```

## Quick-start

```sh
# the spec-compiler crate
cargo test -p vibe-spec        # ~83 tests
cargo clippy -p vibe-spec --all-targets -- -D warnings

# the full gate (run before finishing — covers vibe check + conform + specmap)
bash tools/self-check.sh       # expect "self-check: all green"

# build the working-tree vibe binary (never the PATH vibe)
cargo build -p vibe-cli
```
