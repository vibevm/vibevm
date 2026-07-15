# HYBRID-LINKING-PLAN v0.1 — per-package boot compilation units with soft/hard static edges

_Status: EXECUTING (Phase 2 next) · written against tree `a9fdd63` · cold-executable: Phase 0 is
spikes and commits nothing; every later phase ends with the floor green and is a
safe stop. The **contract** is [PROP-038](../modules/vibe-workspace/PROP-038-hybrid-boot-linking.md);
this plan is the recipe that executes it, phase by phase, each phase citing the
PROP-038 §-anchors it delivers._

---

## 2 — Execution record (prepended at close)

_Empty at authoring._

---

## 3 — The mandate

Owner (2026-07-15), across a multi-turn design dialogue. Verbatim essentials:

> "мне нужна локальная вложенная статичность — большой трек с основательным
> решением этого вопроса, без костылей. И нужно написать много тестов, которые
> всё это проверят — особенно что мы не потеряли или не забыли перегенерить
> какие-то зависимости, если мы добавили или удалили зависимости."

> "Я хочу чтобы у каждой библиотеки были свои STATIC.md в vibedeps тоже (если
> нет — этот механизм надо создать) … факт включения A как dynamic означает, что
> в том STATIC.md который относится к root не должно появиться B, C, D … А вот
> внутри материализованного A, в его личном STATIC.md уже должен появиться
> вкомпилированный B … Но эта вкомпилированная версия не будет нести в себе C,
> потому что зависимость на C в свою очередь динамическая."

> "по-умолчанию будет static-soft … Если у нас убираем библиотеку в общий
> STATIC.md через hoisting, то в пакетный локальный STATIC.md все равно нужно
> создать … инструкцию типа #use этой библиотеки, чтобы агент точно не промазал."

Ratified in the dialogue: `static-soft` is the default (§2.3); hoisting targets
the LCA static-zone, global root only cross-zone (§2.4); the single-version
invariant makes the multi-version-hint feature unnecessary (§2.6); `#use` + a
shared-by hint are the two hoist markers (§2.5). Scope questions resolve against
PROP-038 + this mandate; the executor never re-litigates them.

---

## 4 — Target arithmetic

This is a subsystem-build campaign, so the "counts" are **capabilities**, each a
PROP-038 §-anchor, reconciled baseline → exit:

```
Baseline (shipped bootgen, verified §5): 4 capabilities present but partial —
  single-GLOBAL STATIC.md/INDEX.md per workspace node; static seeded ROOT-ONLY;
  dedup+topo per-global-graph; boot regen WHOLE-TREE (no fingerprint).
Exit (PROP-038 delivered): 9 capabilities —
  §2.1 per-unit artifacts · §2.2 per-edge recursive compilation · §2.3 soft/hard
  modes · §2.4 LCA hoisting · §2.5 hoist markers · §2.6 single-version rest
  (no new code — invariant recorded) · §2.7 Merkle fingerprint · §2.8
  dirty-subgraph regen · §2.9 read-set dedup — each with tests (§3).
Reconciliation: the 4 baseline capabilities are subsumed (per-global → per-unit;
  root-only seed → per-edge; whole-tree → dirty-subgraph); §2.6 is a recorded
  invariant, not code; everything else is net-new behind §3's oracle.
```

The definition of done is the differential oracle (§3): `incremental == full`,
byte-identical, under mutation fuzzing.

---

## 5 — Current-state facts (verified at authoring, 2026-07-15; do not re-discover)

From a code fact-check of the shipped `bootgen` (cite these, trust them):

- **Artifacts are workspace-node-only.** `write_boot_artifacts` writes
  `<node>/spec/boot/{INDEX,STATIC}.md` (`crates/vibe-workspace/src/boot_artifacts.rs:424-449`),
  driven by `regenerate_boot_from` over `workspace.iter_nodes()` = root + members
  (`bootgen.rs:38-55`). **No artifacts are written inside `vibedeps/` packages** —
  the per-unit mechanism (§2.1) does not exist yet. (`render_static` *reads*
  contributions from the materialised `vibedeps/` slots, `boot_artifacts.rs:196`,
  but *writes* only to consuming nodes.) The one other writer,
  `regenerate_published_boot` (`publish/staging.rs:191-200`), emits own-boot-only
  for a staged publish copy — not a nested artifact.
- **Static is seeded root-only.** `forced_inline = static_transitive_closure(node_manifest, index)`
  is seeded **exclusively** from the root manifest's direct `static-transitive`
  edges (`bootgen.rs:271-278`); the BFS walks `dep.requires` **without reading the
  link types on those edges** (`bootgen.rs:280-289`). A `static` declared inside an
  intermediate package is never read for boot (`bootgen.rs:248-252`).
- **Precedence chain (one value/node), not a per-edge lattice.**
  `declared_link → suggested_link → default_link → Dynamic`, `when` forces
  `Dynamic` (`boot.rs:226-246`); the only join to static is `forced_inline`
  membership (`bootgen.rs:248-252`, fold `StaticTransitive→Static` at
  `boot.rs:234-237`).
- **Dedup + topo are present and reusable per-unit.** BFS `visited`
  (`bootgen.rs:200-217`); Kahn topo-sort with a `(group,name)` min-heap tie-break
  and cycle → `WorkspaceError::BootDependencyCycle` (`boot.rs:279-327`).
- **`LinkType` enum:** `Static` / `Dynamic` / `StaticTransitive`
  (`vibe-core/src/manifest/package.rs:307-327`); **no `dynamic-transitive`**
  (reserved). `static-transitive` already lands
  (`bootgen.rs:267-291`, `boot.rs:234-237`, test `boot/tests.rs:288-297`).
- **Boot regen is whole-tree, no fingerprint.** `apply_resolution` step 3
  (`install.rs:146`) → `regenerate_boot_from` rewrites `INDEX.md`/`STATIC.md` for
  **every** node unconditionally (`boot_artifacts.rs:433-449`); no
  hashing/dirty-tracking for boot artifacts anywhere. Materialisation *is*
  incremental (slot-present skip, `install.rs:245-253`).
- **Structural side is spec'd, best-effort.** `#use` / `@spec` / read-set / link
  tables live in PROP-035 (§7, §10) and the `vibe-spec` crate (`render_static`
  runs `expand_embeds`); the structural loader is prompt-driven today (PROP-035
  §13), not an algorithmic agent.
- **Single-version is guaranteed.** resolvo enforces single-version-per-name
  (PROP-017 §3); incompatible constraints fail `Unsatisfiable` (§2.4). No
  coexisting versions — the premise soft dedup rests on (§2.6).
- **Floor:** `bash tools/self-check.sh` green; `vibe-workspace` is a gated crate
  (conform + specmap). Machine: Edit/Write only (PS5.1 UTF-8), heredoc commits,
  self-check via Git Bash.

---

## 6 — Decisions

- **D1 — a new PROP, not an edit to PROP-034.** PROP-038 is a fresh contract;
  PROP-034 is retained as design history and superseded in place (its §2.2 lattice
  retires, §2.3 dedup/topo survives per-unit). Rejected: rewriting PROP-034 —
  loses the fork-by-fork record and the global-linker rationale that motivates the
  per-unit move.
- **D2 — `static-soft` is the default meaning of `static`** (PROP-038 §2.3, owner).
  Rejected: `static-hard` default — simpler and local, but a forgotten qualifier
  then duplicates silently, and dedup correctness (owner's stated priority)
  outranks explicit-over-implicit here.
- **D3 — hoist to the LCA static-zone, global root only cross-zone** (§2.4).
  Rejected: always-global-root hoist — needlessly eager, loses the laziness the
  whole "local nested static" track buys.
- **D4 — fingerprint the compilation zone; a dynamic edge breaks propagation**
  (§2.7). Storage location (STATIC.md header / lockfile / link table) is a Phase-0
  spike (§5.3), not pre-decided. Rejected: mtime-based freshness (PROP-011 §6
  already rejects it — mtimes don't survive `git clone`).
- **D5 — dirty-subgraph regen replaces whole-tree** (§2.8). Rejected: keep
  whole-tree (PROP-011 §2.4) — its "boot is cheap" premise dies once STATIC.md is
  verbatim compiled text, not a small INDEX.
- **D6 — the differential oracle (`incremental == full`) is the central test**
  (§3). Rejected: hand-written per-case tests only — they miss the combinatorial
  "forgot to regenerate in a rare topology" the owner explicitly fears.
- **D7 — demo-corpus-first migration** (PROP-035 §15). Build and prove on
  throwaway fixtures; convert vibevm itself last. Rejected: convert real packages
  early — a bug then breaks vibevm's own boot.
- **D8 — AI-Native Rust, granular addressable REQs.** Every new file `scope!`s a
  PROP-038 §-anchor; conform + specmap stay green (the crate baseline is zero-slack).

---

## 7 — Predictions

- **P1** — per-edge recursive compilation (§2.2) expresses the mandate's chain
  (`root→A(dyn)→B(static)→C(dyn)→D(static-transitive)`) with the §5 table's exact
  per-unit contents and **zero** global-lattice code. Falsifiable: a case needs
  the retired PROP-034 §2.2 join.
- **P2** — the differential oracle holds: `incremental == full` byte-identical
  across ≥10k fuzzed mutation sequences (§3). Falsifiable: any divergence — and a
  divergence is a real dropped/stale dependency, exactly the owner's fear.
- **P3** — within-zone hoisting preserves laziness: a package whose only consumers
  sit under an unloaded `dynamic` parent does **not** appear in the root
  `STATIC.md` (§2.4). Falsifiable: it leaks eager.
- **P4** — dirty-subgraph: editing one package's boot text recompiles only that
  package's continuous static-ancestor chain (to the first dynamic break), not the
  whole tree (§2.8); a no-op install recompiles nothing. Falsifiable: a unit
  behind a dynamic break recompiles, or an idempotent install churns git.
- **P5** — retiring the global lattice (PROP-034 §2.2) breaks no existing project:
  the single-node degenerate case is byte-identical before/after for vibevm's own
  boot until a package opts into the hybrid shape. Falsifiable: vibevm's boot
  artifacts change on the no-op migration.

---

## 8 — Phases

Each phase ends floor-green and cites the PROP-038 §-anchors it delivers. Tests
land **with** their phase (not deferred to the end); Phase 5 is the exhaustive
oracle + fuzzing + the vibevm self-migration.

**Phase 0 — spikes (NO commits).** Resolve PROP-038 §5 open questions on real
data: (a) a Merkle `fp` over vibevm's actual boot graph — is a dynamic break a
clean propagation barrier? (b) the differential oracle on a toy DAG harness —
`incremental == full` skeleton; (c) LCA-static-zone computation + the
within/cross-zone split; (d) the `soft × static-transitive` matrix (§5.1); (e)
what increments the static-use counter (§5.2); (f) `fp` storage location (§5.3);
(g) read-set behaviour on a `dyn→static` chain. Findings fold into §6/PROP-038.

**Phase 1 — per-unit emission + per-edge recursion** (§2.1, §2.2). Every
materialised package emits `vibedeps/<slot>/spec/boot/{STATIC,INDEX}.md`; compile
by walking the unit's own edges, recursively, dynamic-bounded; reuse the existing
dedup/topo (`boot.rs:279-327`) per unit; retire root-only seeding
(`bootgen.rs:271-289`). The mandate's chain example (§5 table) is the acceptance.
The big enabling phase.

**Phase 2 — soft hoisting + markers + hard opt-in** (§2.3, §2.4, §2.5). The global
static-use pass; LCA hoist with the within/cross-zone split; `#use` markers in
local units + shared-by hints at the hoist target; `link = "static-hard"` opt-in
(pure-local). Proves dedup without losing laziness (P3).

**Phase 3 — fingerprint + dirty-subgraph** (§2.7, §2.8). The Merkle `fp` over each
unit's zone (storage per Phase-0 finding); recompile only the dirty subgraph;
idempotent no-op install (P4). Revises PROP-011 §2.4 in place.

**Phase 4 — read-set + `vibe check` integrity** (§2.9, §3). Read-set dedup for
hard duplication and lifted `#use`; the `vibe-check` boot-graph pass (fingerprints
current, reachability complete).

**Phase 5 — the oracle, the fuzzer, the self-migration** (§3, PROP-035 §15). The
full differential oracle + property-based mutation fuzzing + the invariant goldens
(no-loss / completeness / no-stale / boundary / idempotency / dedup-at-read); then
migrate vibevm's own boot onto the hybrid shape, demo-corpus-proven first.

---

## 9 — Risks and fallbacks

- **R1 — nonlocal soft invalidation misses a dirty unit** (a new static consumer
  should re-hoist `L`, but some unit isn't recompiled). Detection: the differential
  oracle + mutation fuzzing (§3), specifically the single→multi static-use case
  (§2.7). Fallback: the `fp` soft-hoist term is the fix surface; worst case a phase
  falls back to whole-tree regen for hoisted packages until the term is right.
- **R2 — read-set weakness across context compaction** (PROP-035 open q#2). Soft's
  compile-time dedup is the mitigation for the common case; `static-hard` is
  documented as read-set-dependent. Fallback: prefer soft; do not ship hard as the
  default.
- **R3 — per-unit verbatim compilation bloats `vibedeps/` on disk.** Detection: a
  P0/P1 size measurement on vibevm's own tree. Hoisting reduces it; the corpus is
  committed and diffable, so on-disk duplication is acceptable (the static-linking
  cost the owner accepted). Fallback: raise the hard→soft default pressure.
- **R4 — a cycle inside a static zone.** Per-unit cycle rejection (PROP-034 §2.3,
  retained → `BootDependencyCycle`) catches it at generate time; the build never
  emits half-linked.
- **R5 — the global hoist pass is expensive.** It counts static-uses, not content;
  cheap. Measured at P0; fallback is memoising counts in the link table (PROP-035
  §10).
- **R6 — PS5.1 UTF-8 / CRLF** (machine). Edit/Write only; heredoc commits;
  self-check via Git Bash; a `vibe install` reinstall's CRLF noise across vibedeps
  is staged-then-`git -c core.autocrlf=false checkout -- .`.

---

## 10 — Non-goals

- **Multi-version support.** PROP-038 §2.6 — single-version is invariant; the
  version-grouping hint is not needed. Owner-closed; revisit only if the resolver
  model changes.
- **A hard algorithmic structural agent** (PROP-035 §14). This campaign uses the
  prompt-driven structural loader + read-set; the deterministic executor is future
  work.
- **Section-granularity fingerprints** (§5.4). v1 is per-package; section-level is
  deferred to a follow-up if measurement pulls it.
- **Converting real vibevm packages beyond a proof.** Demo-corpus-first (D7);
  broad conversion is the next campaign, seeded from §15.
- **`dynamic-transitive`.** Reserved (PROP-034 §5, PROP-035 §16); no use case.

---

## 11 — Quick-start for the executing session

```sh
git log --oneline -1                 # a9fdd63 — matches the status line
bash tools/self-check.sh             # floor GREEN before Phase 0
cargo test -p vibe-workspace         # bootgen tests green (boot/tests.rs)
# read the contract first, then the current-state facts:
sed -n '1,90p' spec/modules/vibe-workspace/PROP-038-hybrid-boot-linking.md
sed -n '/^## 5/,/^## 6/p' spec/terraforms/HYBRID-LINKING-PLAN-v0.1.md
```

---

## 12 — Whole-campaign acceptance

```sh
bash tools/self-check.sh; echo "EXIT=$?"                       # 0
cargo test -p vibe-workspace                                    # bootgen + oracle green
# the mandate chain compiles to the exact per-unit contents (§5 table):
#   vibedeps/<A>/spec/boot/STATIC.md contains A + B, NOT C
#   root/spec/boot/STATIC.md contains neither A nor B/C/D
# the differential oracle holds under fuzzing:
cargo test -p vibe-workspace differential_incremental_vs_full   # byte-identical
# every PROP-038 REQ the code implements is scope!-cited:
cargo xtask specmap && cargo xtask conform check                # clean
# vibe check reports boot-graph integrity (fingerprints current, reachability full):
./target/debug/vibe check                                       # green
```

---

## 13 — Review points

- **RP1 — `soft × static-transitive` composition** (PROP-038 §5.1): orthogonal
  axes or transitive-implies-hard? Executor proposes at Phase 0 close; owner rules.
- **RP2 — fingerprint storage location** (§5.3): STATIC.md header vs `vibe.lock`
  schema bump vs link table. Proposed at Phase 0; owner rules (a lockfile bump is
  an observable-contract change).
- **RP3 — retiring PROP-034 §2.2** (the global precedence lattice): confirmed as
  dead code once §2.2-per-edge lands (P5 asserts no behaviour change first). Owner
  confirms the retirement.

---

## 14 — Execution ledger

- **Phase 0 — spikes + spec resolution.** `d487d4e` `docs(spec): PROP-038
  accepted — resolve Phase 0 open questions`. Code reconnaissance folded the
  five §5 questions into resolutions (soft/hard × transitive orthogonal; both
  direct + forced edges count static-use; fingerprint in the artifact header;
  per-package granularity; dynamic boundaries aggregate into the unit INDEX)
  and the migration-safety corollary (per-unit artifacts additive, entry-point
  artifacts byte-stable). Baseline floor green. No code committed (spikes only).
- **Phase 1 — per-unit emission + per-edge recursion** (§2.1, §2.2). Two commits:
  - `542befd` `feat(vibe-workspace): hybrid per-unit boot compiler` — the pure
    core `boot/hybrid.rs`: `resolve_zone` (membership recursion, dynamic-bounded,
    static-transitive forces, when-gate stays dynamic) + `topo_zone` (Kahn +
    pkgref tie-break for byte-stability). 5 unit tests: the owner's chain, diamond
    dedup, forced subtree, the when gate.
  - `29388fe` `feat(vibe-workspace): wire per-unit boot compilation into install`
    — `build_unit_table` (edges carry each package's OWN manifest's link modes,
    fixing root-only seeding) + `emit_package_units` (writes STATIC.md/INDEX.md
    into a slot when a package statically links a child) + the node dynamic-edge
    refinement (→ the child's STATIC.md). Confirms **P1** (per-edge recursion, no
    global lattice) and **P5** (byte-identical node artifacts on the current
    tree: `with_static` empty ⇒ no-op). New end-to-end test proves the owner's
    core case. Floor green (152 workspace tests + specmap 0 orphans).

---

## 15 — Deferrals ledger

- **DEF-1** — section-granularity fingerprints (PROP-038 §5.4) · owner · deferred;
  v1 is per-package, revisit if measurement pulls it.
- **DEF-2** — `dynamic-transitive` (PROP-034 §5) · owner · reserved, no use case.
- **DEF-3** — broad conversion of real vibevm packages to the hybrid shape · owner
  · demo-corpus-first this campaign; broad conversion is the next campaign's mandate.
- **DEF-4** — a hard algorithmic structural agent (PROP-035 §14) · owner · future;
  this campaign is prompt-driven structural + read-set.
