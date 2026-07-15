# PROP-038: Hybrid boot linking — per-package compilation units with soft/hard static edges {#root}

**Status:** ACCEPTED — 2026-07-15. Requirements captured from an owner design dialogue; **ratified by the owner's directive to implement in full** (2026-07-15, "реализуй всю гибридную линковку, включая спеки, код и тесты"). The §5 open questions are **resolved** (Phase 0 of the campaign, recorded inline in §5 with each resolution); implementation follows the [HYBRID-LINKING campaign](../../terraforms/HYBRID-LINKING-PLAN-v0.1.md).
**Extends:** [PROP-009](PROP-009-loading-model.md) (the loading model — the `STATIC.md` / `INDEX.md` artifacts §2.3, the `static` / `dynamic` link types §2.4), [PROP-035](PROP-035-spec-compiler.md) (the two-mode boot linker §2, `#use` §7.2, the `@spec` read-set §7.4, link tables §10).
**Supersedes / evolves:** [PROP-034](PROP-034-transitive-links-boot-graph.md) — its **single global** static-link graph and the precedence lattice ([§2.2](PROP-034-transitive-links-boot-graph.md#precedence)) are replaced by **per-edge recursive** linking plus hoisting (§2.2, §2.4 below). PROP-034's dedup + topological-order + cycle-rejection invariants are retained, applied **per compilation unit**.
**Related:** [PROP-017 §3](../vibe-resolver/PROP-017-resolvo-resolver.md#encoding) (resolvo — the single-version-per-name invariant this rests on), [PROP-011 §2.4](PROP-011-incremental-install.md#boot-regen) (whole-tree boot regeneration — revised here to a dirty-subgraph), [PROP-022](PROP-022-materialization-modes.md) / [PROP-014](../../../packages/org.vibevm.ai-native/core-ai-native/v0.7.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md) (`content_hash`, the specmap/link-table index), [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md) (the static/dynamic-linking metaphor this PROP completes).

---

## 1. Motivation — the boot must link like a real linker, per unit {#motivation}

PROP-009 gives each dependency edge an inclusion type and emits **one** `STATIC.md` + `INDEX.md` per entry-point workspace node. PROP-034 resolves the whole closure as one **global** static-link graph, seeded from the root manifest. Verified against the shipped `bootgen` (2026-07-15), two limitations block the model the owner wants:

1. **Static propagates only from the root.** The `static-transitive` closure is seeded **exclusively** from the root manifest's direct edges; `link` declarations **inside** an intermediate (dynamically-linked) package are never read for boot. So a `dynamic`-linked package `A` cannot declare "I statically link my own dependency `B`" — `B` falls back to `dynamic`. The effective-mode lattice (PROP-034 §2.2) is only half-implemented: "static wins", but only from the root.

2. **A single global `STATIC.md` cannot express *local* static.** Modes are a global property of a node. "`B` is static" means "`B` is in the one root `STATIC.md`, read first, always" — even when `B`'s parent `A` is `dynamic` and may never load. There is no notion of "static **within** `A`": static compiled *relative to* a package, loaded *with* that package, and only when it loads.

The owner's target is **local nested static linking** — a dynamically-linked package that statically links its own dependencies, recursively, exactly as a real linker composes objects into a `.so` (statically-linked, inside) while leaving other `.so`s as `DT_NEEDED` dynamic references (late-bound by the loader). This PROP makes boot a **hybrid linker**: it composes AOT (static, *within* a compilation unit) and JIT (dynamic, *across* unit boundaries) **at every edge**, and adds a soft/hard dedup axis on the static side.

---

## 2. Decisions {#decisions}

### 2.1 Every materialised package is a compilation unit {#units}

**Decision.** Every package materialised under `vibedeps/` carries its **own** boot artifacts — `vibedeps/<slot>/spec/boot/STATIC.md` (what is compiled **into** this unit, verbatim) and `.../INDEX.md` (this unit's **external dynamic** references, resolved when the unit loads) — not only entry-point workspace nodes. This changes PROP-009 §2.3's "for every entry-point node" to "for every compilation unit (entry-point node **or** materialised package)".

A unit's `STATIC.md` is self-contained and reversible (open/close markers, PROP-035 §11): reading it, an agent gets this package and everything statically linked into it, in dependency order, once each — the PROP-034 dedup + topological-order + cycle-rejection invariants, applied **within the unit**.

### 2.2 The edge is the linker instruction; compilation is recursive and dynamic-bounded {#edge-recursion}

**Decision.** `link` is a property of the **edge** (consumer-side, declared in the parent's manifest), never baked into the pulled package (as PROP-034 §2.1 already states). A unit `P` is compiled by walking its **own** direct edges `P→X`:

- **`static`** — `X`'s `STATIC.md` is compiled into `P`'s `STATIC.md`. Compilation **recurses down** `X`'s own static edges; a nested `dynamic` edge inside `X` breaks the recursion (that target stays an `INDEX.md` reference). `static` therefore **respects** the modes below it.
- **`dynamic`** — `X` is **not** compiled; it becomes an `[[entry]]` in `P`'s `INDEX.md`. The static zone **breaks** at this edge.
- **`static-transitive`** — `X` and its **entire** subtree are forced `static`, **ignoring** any `dynamic` edges inside — "rewrite the whole tree under `X`". This is the one mode that overrides nested breaks.

The difference between `static` and `static-transitive` is exactly this treatment of nested `dynamic` edges: `static` honours them (breaks), `static-transitive` overrides them (forces).

**Worked example** — `root → A(dynamic) → B(static) → C(dynamic) → D(static-transitive)`:

| Unit artifact | Contains | Because |
|---|---|---|
| `root/…/STATIC.md` | — (no A, B, C, D) | `root→A` **dynamic** → break; A is a reference in `root/INDEX.md` |
| `vibedeps/A/…/STATIC.md` | **A + B** (no C) | `A→B` **static** → B compiled in; `B→C` **dynamic** → C breaks |
| `vibedeps/B/…/STATIC.md` | B | `B→C` **dynamic** → C is a reference in `B/INDEX.md` |
| `vibedeps/C/…/STATIC.md` | **C + D + all under D** | `C→D` **static-transitive** → forces the subtree |

### 2.3 Two static modes — `static-soft` (default) and `static-hard` {#modes}

**Decision.** The static side has two modes, differing in **where** duplication is deduplicated:

- **`static-soft`** — **the default**, the meaning of a bare `link = "static"`. Hoisting dedup at **compile time**: a package statically linked by more than one consumer is **hoisted** to a shared location (§2.4) and linked **once**; each consumer references it. Deterministic; does not depend on read-time behaviour.
- **`static-hard`** — explicit opt-in (`link = "static-hard"`). **Pure local** compilation: every consumer compiles the package into its own `STATIC.md` independently, with no hoisting. Duplication is deduplicated at **read time** by the read-set (§2.9).

**Why soft is the default (owner decision, 2026-07-15).** A forgotten qualifier must fail toward **correctness**, not toward implicit duplication. When the same package is compiled into several units unhoisted, the model sees the same prompt several times and can be confused about which copy is authoritative — a correctness hazard the owner weighs above the "explicit-over-implicit" cost of a smart default. `static-hard` remains for the deliberate case where a package must load **only** with its consumer (lazy locality preferred over dedup) even at the price of on-disk duplication.

### 2.4 Soft hoisting targets the LCA of the static-zone, not always the global root {#hoisting}

**Decision.** A hoisted package rises to the **least common ancestor within a continuous static zone** of its consumers, not unconditionally to the global root:

- **Within one static zone** (consumers share a static ancestor `Z` reached by an unbroken chain of static edges) → hoist into `Z`'s `STATIC.md`. Dedup achieved **and** the package still loads only when `Z` loads: **laziness is preserved**. Within-zone hoisting is free and always done.
- **Across dynamic zones** (the consumers' common static ancestor does not exist because a `dynamic` edge separates them) → the only shared always-loaded location is the **global root** `STATIC.md`, and hoisting there makes the package **eager** (loaded even when its dynamic consumers are not). This is the one real cost of soft, paid only for cross-zone sharing.

**Consequences, all deliberate:**

- Hoisting needs a **global pass** counting the static-consumers of each package (partially re-introducing global analysis the per-edge model otherwise avoids) — the price of compile-time dedup. Recorded as a change-detection cost in §2.7.
- **Hoist transitivity.** Hoisting `L` hoists `L`'s own static sub-zone with it (else the hoisted `L` references code not present at the hoist point).
- A `static`-declared package can be **hoisted past** an edge that named it `static` only within its static zone; a `dynamic` edge is always a hoist barrier (crossing it is the eager cross-zone case above).

### 2.5 Hoist markers — the two ends of a lifted edge {#markers}

**Decision.** When soft hoists `L` out of a unit `P`'s local `STATIC.md`, two markers preserve correctness — the reversible two-ended shape PROP-035 §7/§11 already defines:

- **In `P`'s `STATIC.md`**, where `L`'s text used to be: a **`#use spec://…/L`** directive (PROP-035 §7.2). It preserves the `P→L` graph edge locally and tells the agent "`L` is part of me; its text is lifted and already read above — do not duplicate." The read-set (§2.9) gates the re-read, so no duplication reaches context.
- **In the hoist target** (`Z`'s or the root's `STATIC.md`), at the lifted block: a **shared-by hint comment** naming the consumers (`shared by P, Q, R`). It explains to the model why `L` is here and not local, and asserts this is **one shared version** — not a duplicate to reconcile.

Both markers are generated, are part of the reversible marker set, and must be regenerated on recompilation.

### 2.6 The single-version invariant this rests on {#single-version}

**Decision (recording a closed question).** Soft dedup is correct **because** the resolver guarantees **one version per `(kind, name)`** across the workspace — resolvo enforces single-version-per-name automatically (PROP-017 §3), and genuinely incompatible constraints fail as `Unsatisfiable` (PROP-017 §2.4) rather than coexisting. Therefore:

- A hoisted package is one shared version; there is never "two versions of `B` side by side" in a `STATIC.md`.
- The proposed "group different versions together + a divergence hint" feature is **not needed** — the situation it guards against cannot occur (confirmed 2026-07-15). Should the resolver model ever change to permit coexisting majors (a large, separate decision), this section is the trigger to revisit the hint mechanism.

Conflict resolution (how an author forces the single chosen version) is out of scope here and documented in [`docs/faq/version-conflicts.md`](../../../docs/faq/version-conflicts.md) (`[[override]]`, git-source, `version.var`).

### 2.7 Change-detection — a Merkle fingerprint over the boot graph {#change-detection}

**Decision.** Each unit's `STATIC.md` carries a **fingerprint** of the inputs it was compiled from — a Merkle hash over the unit's compilation zone:

```
fp(P) = hash(
    content_hash(own_boot(P)),                       // P's own boot text
    [ link_type(P→X) for each edge ],                // dynamic↔static switches
    [ fp(X) for each static / static-transitive edge P→X ],   // recurse into the zone
    [ identity(Y) = (group,name,version) for each dynamic edge P→Y ],  // dyn edge: identity only
    soft_hoist_inputs(P)                             // §2.4 global static-use counts touching P
)
```

Properties:

- **A `dynamic` edge breaks fingerprint propagation** — exactly as it breaks compilation. A change *behind* a dynamic edge changes `fp(Y)` but not `fp(P)` (only `Y`'s identity enters `fp(P)`); `Y`'s unit recompiles independently.
- Any change **inside** a static zone — content, version, edge set, **or a `link`-type switch** (which resolution does **not** see, §2.8) — flips `fp` up the continuous static chain to the first dynamic break.
- The soft-hoist term makes a **single→multi static-use transition** (a new consumer statically links `L`, so `L` must now hoist) flip `fp` for the affected units — the nonlocal invalidation soft costs, made explicit so tests target it (§3).

Fingerprint storage location and granularity are open (§5).

### 2.8 Incremental regeneration — the dirty subgraph {#incremental}

**Decision.** Boot regeneration recompiles **only** the units whose `fp` changed (the dirty subgraph), replacing PROP-009's / PROP-011's whole-tree regeneration. PROP-011 §2.4 kept boot regeneration whole-tree because it was cheap (a small `INDEX.md` per node); with **verbatim per-package compilation** (§2.1) that rationale no longer holds — a `STATIC.md` is now real concatenated text — so the incremental path becomes load-bearing. The fast path: an unchanged root `fp` ⇒ **zero** recompilation, **zero** git churn (idempotency).

This is the standard build-system shape — a `cargo`-fingerprint / Bazel-action-graph dirty-subgraph. The materialisation step is already incremental (PROP-011 §2.3); this brings boot regeneration to parity.

### 2.9 Read-set — the read-time dedup {#read-set}

**Decision.** The `@spec`/`#use` read-set (PROP-035 §7.4 — a persistent `{ specpath, content_hash }` record, "read once") is the dedup mechanism for (a) `static-hard` duplication across units, and (b) the `#use` markers soft leaves in local units (§2.5). It is a load-bearing prerequisite, not optional: without it, `static-hard` duplicates and lifted `#use` targets would re-enter context. Its known weakness across context compaction (PROP-035 open question #2) applies; soft's compile-time dedup is the mitigation for the common case.

---

## 3. Test obligations {#tests}

This system's central risk is **losing or failing to regenerate a dependency** when the graph changes. The contract:

- **The differential oracle is mandatory and central.** `incremental_regen(any mutation sequence)` MUST equal `full_regen_from_scratch()`, byte-for-byte. Full regeneration is the reference semantics (it cannot silently drop anything); incremental must match it. This is the AI-Native Rust differential-oracle idiom applied to bootgen.
- **Property-based mutation fuzzing.** Generate random DAGs (packages + edges with random link modes), apply random sequences of `add-edge` / `remove-edge` / `change-link` / `bump-version` / `edit-content`, assert `incremental == full` after each. Targets the combinatorial "forgot to regenerate in a rare topology" — including the §2.7 nonlocal soft invalidation.
- **Invariants as characterization goldens:** *no-loss / reachability* (units reachable through `STATIC.md`+`INDEX.md` == resolved closure; nothing dropped, nothing dangling); *completeness* (every static child is compiled in; every dynamic child is a reference, not compiled); *no-stale* (recomputed `fp` == stored `fp` for every unit); *boundary isolation* (a mutation behind a dynamic edge does not change the parent unit's `STATIC.md`); *idempotency* (a no-op `vibe install` recompiles nothing, zero git diff); *dedup-at-read* (the read-set reads a duplicated/hoisted package once).
- **`vibe check` boot-graph integrity.** The existing `vibe-check` `boot_directory` check gains a boot-graph pass: fingerprints current, reachability complete — so "did everything regenerate?" is answerable in CI and by hand.

---

## 4. Compatibility and migration {#compat}

- **Evolves PROP-009 §2.3** — boot artifacts now generated per compilation unit, not only per entry-point node. Existing single-node projects are the degenerate case (one unit) and keep working.
- **Retires PROP-034 §2.2** (the global precedence lattice) — the effective-mode join is unnecessary once mode is a per-edge property resolved per unit; a package may be `static` in one unit's `STATIC.md` and `dynamic` in another's `INDEX.md` with no conflict and no global join. PROP-034's dedup / topological-order / cycle-rejection survive, applied per unit.
- **Revises PROP-011 §2.4** — boot regeneration moves from whole-tree to dirty-subgraph (§2.8); the "boot is cheap, keep it whole-tree" decision is re-opened by the verbatim-compilation cost and its recorded trigger has fired.
- **Depends on PROP-035** — the structural/JIT concepts (`#use`, read-set, link tables, reversible markers) become load-bearing rather than best-effort. This PROP is the concrete evolution of PROP-035's two-mode boot linker (§2) and its emission layer (§12).
- Migration is demo-corpus-first (PROP-035 §15): build and prove on throwaway fixtures before converting any real package; vibevm itself converts last, and only where a package opts into the hybrid shape.

---

## 5. Resolved questions {#open}

The five questions opened in the design dialogue were resolved 2026-07-15 (Phase 0):

1. **`soft` × `static-transitive` — orthogonal axes.** `static-transitive` decides *which* packages are static (it forces the subtree); `soft`/`hard` decides *how* duplicates are deduped (hoist vs. local). They compose: a `static-transitive` edge's forced subtree is deduped by `soft` (hoisting) by default. No separate `static-transitive-hard` variant ships in v1 — the matrix stays 2×1 (`soft`/`hard`) × (`direct`/`transitive`) with hard-transitive deferred (no use case).
2. **Static-use counter — both direct and forced count.** A package reached by a direct `static`/`static-hard` edge **and** a package forced static by a `static-transitive` ancestor both increment its static-use count for hoisting (§2.4). `dynamic` edges never count. This keeps hoisting correct across a forced subtree.
3. **Fingerprint storage — the `STATIC.md`/`INDEX.md` header (§2.7).** A generated header comment carries the unit's `fp`, self-describing and reversible (PROP-035 §11), with **no `vibe.lock` schema bump** — avoiding an observable-contract change to the lockfile (the lighter of the RP2 options). A link-table cache (PROP-035 §10) may memoise it later; the header is the source of truth.
4. **Granularity — per package (v1).** Fingerprint and invalidation are per compilation unit (package). Section-level granularity (PROP-035 §5 IR) is deferred ([plan DEF-1](../../terraforms/HYBRID-LINKING-PLAN-v0.1.md)).
5. **Dynamic-boundary representation — aggregated into the unit's `INDEX.md`.** When a unit's static zone is compiled, every `dynamic` edge inside that zone is surfaced into the **unit's own `INDEX.md`** (not left as an inline directive in the compiled text). A unit's `INDEX.md` is thus the complete "what to load dynamically once you have read my `STATIC.md`" manifest — one manifest per unit, no inline resolution the agent must perform mid-text.

**Migration-safety corollary (Phase 0 finding).** Per-unit artifacts (§2.1) are **additive**: generating `STATIC.md`/`INDEX.md` inside a `vibedeps/` slot is new output, expected on migration. An entry-point node's **existing** artifacts stay **byte-identical** for a tree with no intermediate static edges (today's vibevm: `static` reaches the boot lane only through the root's `static-transitive` redbook edge, so root recursion reproduces the current root `STATIC.md`). P5's acceptance therefore checks *root artifacts unchanged* **plus** *new per-unit artifacts appear*, not "no new files".

---

## 6. Version history {#history}

- **2026-07-15 — drafted (owner-requested).** Captures the hybrid-linking design dialogue: per-package compilation units (§2.1); the edge as linker instruction with recursive, dynamic-bounded compilation and the `static` / `dynamic` / `static-transitive` semantics (§2.2); the `static-soft` (default) / `static-hard` modes and why soft is the default (§2.3); LCA-scoped hoisting with the within-zone/cross-zone split and hoist transitivity (§2.4); the two-ended hoist markers — local `#use` + shared-by hint (§2.5); the single-version invariant the dedup rests on and the closed multi-version-hint question (§2.6); the Merkle fingerprint over the boot graph (§2.7); dirty-subgraph incremental regeneration revising PROP-011 §2.4 (§2.8); the read-set as read-time dedup (§2.9); and the differential-oracle-centred test obligations (§3). Implementation is the [HYBRID-LINKING campaign](../../terraforms/HYBRID-LINKING-PLAN-v0.1.md).
- **2026-07-15 — ACCEPTED; §5 resolved (Phase 0).** Ratified by the owner's implement-in-full directive. The five open questions resolved inline (§5): soft/hard × transitive are orthogonal; both direct and forced edges increment the static-use count; the fingerprint lives in the artifact header (no lockfile bump); granularity is per-package; dynamic boundaries aggregate into the unit's `INDEX.md`. The migration-safety corollary pins per-unit artifacts as additive with entry-point artifacts byte-stable for the current tree.
