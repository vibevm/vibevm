# PROP-038: Hybrid boot linking — per-package compilation units with soft/hard static edges {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: IMPLEMENTED 2026-07-15, all five campaign phases shipped"/>

@fact:status-line **Status:** IMPLEMENTED — 2026-07-15 (all five campaign phases shipped, `d487d4e`…`381095e`; see §6). Requirements captured from an owner design dialogue; **ratified by the owner's directive to implement in full** (2026-07-15, "реализуй всю гибридную линковку, включая спеки, код и тесты"). The §5 open questions are **resolved** (Phase 0 of the campaign, recorded inline in §5 with each resolution); implementation follows the [HYBRID-LINKING campaign](../../../legacy-spec/terraforms/HYBRID-LINKING-PLAN-v0.1.md). @status:impl/done

@fact:extends **Extends:** [PROP-009](PROP-009-loading-model.md) (the loading model — the `STATIC.md` / `INDEX.md` artifacts §2.3, the `static` / `dynamic` link types §2.4), [PROP-035](PROP-035-spec-compiler.md) (the two-mode boot linker §2, `#use` §7.2, the `@spec` read-set §7.4, link tables §10). @status:spec/done

@fact:INPUT-IS-THE-EFFECTIVE-SET **Input narrowed by the visibility layer (2026-08-23, PROP-050 §3):** the unit table this linker compiles is built from the resolved `ResolvedDep` slice, which since the W2 landing carries only the consumer root's **effective set E(R)**. Zones, `static-transitive` forcing, hoisting counts and surfaced dynamic edges therefore all operate within `E(R)` by construction — forcing can make a visible unit static, never make an invisible one visible (PROP-050 ##FORCING-NEVER-WIDENS). @status:impl/done

@fact:supersedes-line **Supersedes / evolves:** [PROP-034](PROP-034-transitive-links-boot-graph.md) — its **single global** static-link graph and the precedence lattice ([§2.2](PROP-034-transitive-links-boot-graph.md#precedence)) are replaced by **per-edge recursive** linking plus hoisting (§2.2, §2.4 below). PROP-034's dedup + topological-order + cycle-rejection invariants are retained, applied **per compilation unit**. @status:spec/done

@fact:related **Related:** [PROP-017 §3](../vibe-resolver/PROP-017-resolvo-resolver.md#encoding) (resolvo — the single-version-per-name invariant this rests on), [PROP-011 §2.4](PROP-011-incremental-install.md#boot-regen) (whole-tree boot regeneration — revised here to a dirty-subgraph), [PROP-022](PROP-022-materialization-modes.md) / [PROP-014](../../../packages/org.vibevm.ai-native/core-ai-native/v0.7.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md) (`content_hash`, the specmap/link-table index), [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md) (the static/dynamic-linking metaphor this PROP completes). @status:spec/done

---

## 1. Motivation — the boot must link like a real linker, per unit {#motivation}

- @fact:status-quo-linking PROP-009 gives each dependency edge an inclusion type and emits **one** `STATIC.md` + `INDEX.md` per entry-point workspace node. PROP-034 resolves the whole closure as one **global** static-link graph, seeded from the root manifest. @status:impl/done
- @fact:two-limitations Verified against the shipped `bootgen` (2026-07-15), two limitations block the model the owner wants: @status:impl/done

1. @fact:LIMIT-ROOT-ONLY **Static propagates only from the root.** The `static-transitive` closure is seeded **exclusively** from the root manifest's direct edges; `link` declarations **inside** an intermediate (dynamically-linked) package are never read for boot. So a `dynamic`-linked package `A` cannot declare "I statically link my own dependency `B`" — `B` falls back to `dynamic`. The effective-mode lattice (PROP-034 §2.2) is only half-implemented: "static wins", but only from the root. @status:impl/done

2. @fact:LIMIT-GLOBAL-STATIC **A single global `STATIC.md` cannot express *local* static.** Modes are a global property of a node. "`B` is static" means "`B` is in the one root `STATIC.md`, read first, always" — even when `B`'s parent `A` is `dynamic` and may never load. There is no notion of "static **within** `A`": static compiled *relative to* a package, loaded *with* that package, and only when it loads. @status:impl/done

- @fact:OWNER-TARGET The owner's target is **local nested static linking** — a dynamically-linked package that statically links its own dependencies, recursively, exactly as a real linker composes objects into a `.so` (statically-linked, inside) while leaving other `.so`s as `DT_NEEDED` dynamic references (late-bound by the loader). @status:impl/done
- @fact:HYBRID-LINKER This PROP makes boot a **hybrid linker**: it composes AOT (static, *within* a compilation unit) and JIT (dynamic, *across* unit boundaries) **at every edge**, and adds a soft/hard dedup axis on the static side. @status:impl/done

---

## 2. Decisions {#decisions}

### 2.1 Every materialised package is a compilation unit {#units}

@fact:UNIT-PER-PACKAGE **Decision.** Every package materialised under `vibedeps/` carries its **own** boot artifacts — `vibedeps/<slot>/spec/boot/STATIC.md` (what is compiled **into** this unit, verbatim) and `.../INDEX.md` (this unit's **external dynamic** references, resolved when the unit loads) — not only entry-point workspace nodes. @status:impl/done

@fact:UNITS-CHANGES-009 This changes PROP-009 §2.3's "for every entry-point node" to "for every compilation unit (entry-point node **or** materialised package)". @status:impl/done

@fact:UNIT-SELF-CONTAINED A unit's `STATIC.md` is self-contained and reversible (open/close markers, PROP-035 §11): reading it, an agent gets this package and everything statically linked into it, in dependency order, once each — the PROP-034 dedup + topological-order + cycle-rejection invariants, applied **within the unit**. @status:impl/done

@fact:UNIT-ARTIFACT-STATIC-CONSUMER-ELIDES The unit artifact serves **dynamic** consumers (an `INDEX.md` reference loads the whole zone through it) and standalone unit reads; a **static** consumer whose lane already compiles the unit's zone member-by-member **elides** the aggregate entry to a provenance stub instead of embedding the artifact — the once-each rule of PROP-009 §2.3 `##STATIC-EMITS-ONCE-EACH` (B-006, owner-approved 2026-08-04), applied at compose time and distinct from §2.4's hoisting, which shares one copy *across* consumers rather than deduplicating within one lane. @status:spec/work

### 2.2 The edge is the linker instruction; compilation is recursive and dynamic-bounded {#edge-recursion}

@fact:EDGE-IS-INSTRUCTION **Decision.** `link` is a property of the **edge** (consumer-side, declared in the parent's manifest), never baked into the pulled package (as PROP-034 §2.1 already states). A unit `P` is compiled by walking its **own** direct edges `P→X`: @status:impl/done

- @fact:EDGE-STATIC **`static`** — `X`'s `STATIC.md` is compiled into `P`'s `STATIC.md`. Compilation **recurses down** `X`'s own static edges; a nested `dynamic` edge inside `X` breaks the recursion (that target stays an `INDEX.md` reference). `static` therefore **respects** the modes below it. @status:impl/done
- @fact:EDGE-DYNAMIC **`dynamic`** — `X` is **not** compiled; it becomes an `[[entry]]` in `P`'s `INDEX.md`. The static zone **breaks** at this edge. @status:impl/done
- @fact:EDGE-STATIC-TRANSITIVE **`static-transitive`** — `X` and its **entire** subtree are forced `static`, **ignoring** any `dynamic` edges inside — "rewrite the whole tree under `X`". This is the one mode that overrides nested breaks. @status:impl/done

@fact:STATIC-VS-TRANSITIVE The difference between `static` and `static-transitive` is exactly this treatment of nested `dynamic` edges: `static` honours them (breaks), `static-transitive` overrides them (forces). @status:impl/done

@fact:worked-example-lead **Worked example** — `root → A(dynamic) → B(static) → C(dynamic) → D(static-transitive)`: @status:impl/done

| Unit artifact | Contains | Because |
|---|---|---|
| @fact:ROW-EX-ROOT `root/…/STATIC.md` @status:impl/done | — (no A, B, C, D) @status:impl/done | `root→A` **dynamic** → break; A is a reference in `root/INDEX.md` @status:impl/done |
| @fact:ROW-EX-A `vibedeps/A/…/STATIC.md` @status:impl/done | **A + B** (no C) @status:impl/done | `A→B` **static** → B compiled in; `B→C` **dynamic** → C breaks @status:impl/done |
| @fact:ROW-EX-B `vibedeps/B/…/STATIC.md` @status:impl/done | B @status:impl/done | `B→C` **dynamic** → C is a reference in `B/INDEX.md` @status:impl/done |
| @fact:ROW-EX-C `vibedeps/C/…/STATIC.md` @status:impl/done | **C + D + all under D** @status:impl/done | `C→D` **static-transitive** → forces the subtree @status:impl/done |

### 2.3 Two static modes — `static-soft` (default) and `static-hard` {#modes}

@fact:TWO-STATIC-MODES **Decision.** The static side has two modes, differing in **where** duplication is deduplicated: @status:impl/done

- @fact:MODE-STATIC-SOFT **`static-soft`** — **the default**, the meaning of a bare `link = "static"`. Hoisting dedup at **compile time**: a package statically linked by more than one consumer is **hoisted** to a shared location (§2.4) and linked **once**; each consumer references it. Deterministic; does not depend on read-time behaviour. @status:impl/done
- @fact:MODE-STATIC-HARD **`static-hard`** — explicit opt-in (`link = "static-hard"`). **Pure local** compilation: every consumer compiles the package into its own `STATIC.md` independently, with no hoisting. Duplication is deduplicated at **read time** by the read-set (§2.9). @status:impl/done

@fact:SOFT-DEFAULT-WHY **Why soft is the default (owner decision, 2026-07-15).** A forgotten qualifier must fail toward **correctness**, not toward implicit duplication. @status:impl/done

- @fact:duplication-hazard When the same package is compiled into several units unhoisted, the model sees the same prompt several times and can be confused about which copy is authoritative — a correctness hazard the owner weighs above the "explicit-over-implicit" cost of a smart default. @status:impl/done
- @fact:HARD-REMAINS `static-hard` remains for the deliberate case where a package must load **only** with its consumer (lazy locality preferred over dedup) even at the price of on-disk duplication. @status:impl/done

### 2.4 Soft hoisting targets the LCA of the static-zone, not always the global root {#hoisting}

@fact:HOIST-LCA **Decision.** A hoisted package rises to the **least common ancestor within a continuous static zone** of its consumers, not unconditionally to the global root: @status:impl/done

- @fact:HOIST-WITHIN-ZONE **Within one static zone** (consumers share a static ancestor `Z` reached by an unbroken chain of static edges) → hoist into `Z`'s `STATIC.md`. Dedup achieved **and** the package still loads only when `Z` loads: **laziness is preserved**. Within-zone hoisting is free and always done. @status:impl/done
- @fact:HOIST-CROSS-ZONE **Across dynamic zones** (the consumers' common static ancestor does not exist because a `dynamic` edge separates them) → the only shared always-loaded location is the **global root** `STATIC.md`, and hoisting there makes the package **eager** (loaded even when its dynamic consumers are not). This is the one real cost of soft, paid only for cross-zone sharing. @status:impl/done

@fact:hoist-consequences-lead **Consequences, all deliberate:** @status:impl/done

- @fact:HOIST-GLOBAL-PASS Hoisting needs a **global pass** counting the static-consumers of each package (partially re-introducing global analysis the per-edge model otherwise avoids) — the price of compile-time dedup. Recorded as a change-detection cost in §2.7. @status:impl/done
- @fact:HOIST-TRANSITIVITY **Hoist transitivity.** Hoisting `L` hoists `L`'s own static sub-zone with it (else the hoisted `L` references code not present at the hoist point). @status:impl/done
- @fact:HOIST-BARRIER A `static`-declared package can be **hoisted past** an edge that named it `static` only within its static zone; a `dynamic` edge is always a hoist barrier (crossing it is the eager cross-zone case above). @status:impl/done

### 2.5 Hoist markers — the two ends of a lifted edge {#markers}

@fact:HOIST-MARKERS **Decision.** When soft hoists `L` out of a unit `P`'s local `STATIC.md`, two markers preserve correctness — the reversible two-ended shape PROP-035 §7/§11 already defines: @status:impl/done

- @fact:MARKER-USE **In `P`'s `STATIC.md`**, where `L`'s text used to be: a **`#use spec://…/L`** directive (PROP-035 §7.2). It preserves the `P→L` graph edge locally and tells the agent "`L` is part of me; its text is lifted and already read above — do not duplicate." The read-set (§2.9) gates the re-read, so no duplication reaches context. @status:impl/done
- @fact:MARKER-SHARED-BY **In the hoist target** (`Z`'s or the root's `STATIC.md`), at the lifted block: a **shared-by hint comment** naming the consumers (`shared by P, Q, R`). It explains to the model why `L` is here and not local, and asserts this is **one shared version** — not a duplicate to reconcile. @status:impl/done

@fact:MARKERS-REGENERATED Both markers are generated, are part of the reversible marker set, and must be regenerated on recompilation. @status:impl/done

### 2.6 The single-version invariant this rests on {#single-version}

@fact:SINGLE-VERSION-INVARIANT **Decision (recording a closed question).** Soft dedup is correct **because** the resolver guarantees **one version per `(kind, name)`** across the workspace — resolvo enforces single-version-per-name automatically (PROP-017 §3), and genuinely incompatible constraints fail as `Unsatisfiable` (PROP-017 §2.4) rather than coexisting. Therefore: @status:impl/done

- @fact:ONE-SHARED-VERSION A hoisted package is one shared version; there is never "two versions of `B` side by side" in a `STATIC.md`. @status:impl/done
- @fact:HINT-NOT-NEEDED The proposed "group different versions together + a divergence hint" feature is **not needed** — the situation it guards against cannot occur (confirmed 2026-07-15). Should the resolver model ever change to permit coexisting majors (a large, separate decision), this section is the trigger to revisit the hint mechanism. @status:impl/done

@fact:CONFLICT-RESOLUTION-OOS Conflict resolution (how an author forces the single chosen version) is out of scope here and documented in [`docs/faq/version-conflicts.md`](../../../docs/faq/version-conflicts.md) (`[[override]]`, git-source, `version.var`). @status:spec/done

### 2.7 Change-detection — a Merkle fingerprint over the boot graph {#change-detection}

@fact:MERKLE-FINGERPRINT **Decision.** Each unit's `STATIC.md` carries a **fingerprint** of the inputs it was compiled from — a Merkle hash over the unit's compilation zone: @status:impl/done

```
fp(P) = hash(
    content_hash(own_boot(P)),                       // P's own boot text
    [ link_type(P→X) for each edge ],                // dynamic↔static switches
    [ fp(X) for each static / static-transitive edge P→X ],   // recurse into the zone
    [ identity(Y) = (group,name,version) for each dynamic edge P→Y ],  // dyn edge: identity only
    soft_hoist_inputs(P)                             // §2.4 global static-use counts touching P
)
```

@fact:fp-properties-lead Properties: @status:impl/done

- @fact:FP-DYNAMIC-BREAK **A `dynamic` edge breaks fingerprint propagation** — exactly as it breaks compilation. A change *behind* a dynamic edge changes `fp(Y)` but not `fp(P)` (only `Y`'s identity enters `fp(P)`); `Y`'s unit recompiles independently. @status:impl/done
- @fact:FP-STATIC-FLIP Any change **inside** a static zone — content, version, edge set, **or a `link`-type switch** (which resolution does **not** see, §2.8) — flips `fp` up the continuous static chain to the first dynamic break. @status:impl/done
- @fact:FP-SOFT-TRANSITION The soft-hoist term makes a **single→multi static-use transition** (a new consumer statically links `L`, so `L` must now hoist) flip `fp` for the affected units — the nonlocal invalidation soft costs, made explicit so tests target it (§3). @status:impl/done

@fact:fp-storage-note Fingerprint storage location and granularity were open here; §5 resolved both on 2026-07-15 — header storage (`RES-FP-STORAGE`) and per-package granularity (`RES-GRANULARITY`). @status:impl/done

### 2.8 Incremental regeneration — the dirty subgraph {#incremental}

@fact:DIRTY-SUBGRAPH **Decision.** Boot regeneration recompiles **only** the units whose `fp` changed (the dirty subgraph), replacing PROP-009's / PROP-011's whole-tree regeneration. @status:impl/done

- @fact:RATIONALE-CHANGED PROP-011 §2.4 kept boot regeneration whole-tree because it was cheap (a small `INDEX.md` per node); with **verbatim per-package compilation** (§2.1) that rationale no longer holds — a `STATIC.md` is now real concatenated text — so the incremental path becomes load-bearing. @status:impl/done
- @fact:FAST-PATH-IDEMPOTENT The fast path: an unchanged root `fp` ⇒ **zero** recompilation, **zero** git churn (idempotency). @status:impl/done

@fact:build-system-shape This is the standard build-system shape — a `cargo`-fingerprint / Bazel-action-graph dirty-subgraph. The materialisation step is already incremental (PROP-011 §2.3); this brings boot regeneration to parity. @status:impl/done

### 2.9 Read-set — the read-time dedup {#read-set}

@fact:READ-SET-DEDUP **Decision.** The `@spec`/`#use` read-set (PROP-035 §7.4 — a persistent `{ specpath, content_hash }` record, "read once") is the dedup mechanism for (a) `static-hard` duplication across units, and (b) the `#use` markers soft leaves in local units (§2.5). @status:impl/done

- @fact:READ-SET-PREREQ It is a load-bearing prerequisite, not optional: without it, `static-hard` duplicates and lifted `#use` targets would re-enter context. @status:impl/done
- @fact:read-set-weakness Its known weakness across context compaction (PROP-035 open question #2) applies; soft's compile-time dedup is the mitigation for the common case. @status:impl/done

---

## 3. Test obligations {#tests}

@fact:TEST-CENTRAL-RISK This system's central risk is **losing or failing to regenerate a dependency** when the graph changes. The contract: @status:impl/done

- @fact:TEST-DIFFERENTIAL-ORACLE **The differential oracle is mandatory and central.** `incremental_regen(any mutation sequence)` MUST equal `full_regen_from_scratch()`, byte-for-byte. Full regeneration is the reference semantics (it cannot silently drop anything); incremental must match it. This is the AI-Native Rust differential-oracle idiom applied to bootgen. @status:impl/done
- @fact:TEST-MUTATION-FUZZ **Property-based mutation fuzzing.** Generate random DAGs (packages + edges with random link modes), apply random sequences of `add-edge` / `remove-edge` / `change-link` / `bump-version` / `edit-content`, assert `incremental == full` after each. Targets the combinatorial "forgot to regenerate in a rare topology" — including the §2.7 nonlocal soft invalidation. **Shipped:** `boot/hybrid/fuzz.rs` runs the proptest sweep and names this DEF-5 in its own header. @status:impl/done
- @fact:TEST-GOLDEN-INVARIANTS **Invariants as characterization goldens:** *no-loss / reachability* (units reachable through `STATIC.md`+`INDEX.md` == resolved closure; nothing dropped, nothing dangling); *completeness* (every static child is compiled in; every dynamic child is a reference, not compiled); *no-stale* (recomputed `fp` == stored `fp` for every unit); *boundary isolation* (a mutation behind a dynamic edge does not change the parent unit's `STATIC.md`); *idempotency* (a no-op `vibe install` recompiles nothing, zero git diff); *dedup-at-read* (the read-set reads a duplicated/hoisted package once). @status:impl/done
- @fact:TEST-CHECK-INTEGRITY **`vibe check` boot-graph integrity.** The existing `vibe-check` `boot_directory` check gains a boot-graph pass: fingerprints current, reachability complete — so "did everything regenerate?" is answerable in CI and by hand. @status:impl/done

---

## 4. Compatibility and migration {#compat}

- @fact:COMPAT-EVOLVES-009 **Evolves PROP-009 §2.3** — boot artifacts now generated per compilation unit, not only per entry-point node. Existing single-node projects are the degenerate case (one unit) and keep working. @status:impl/done
- @fact:COMPAT-RETIRES-034 **Retires PROP-034 §2.2** (the global precedence lattice) — the effective-mode join is unnecessary once mode is a per-edge property resolved per unit; a package may be `static` in one unit's `STATIC.md` and `dynamic` in another's `INDEX.md` with no conflict and no global join. PROP-034's dedup / topological-order / cycle-rejection survive, applied per unit. @status:impl/done
- @fact:COMPAT-REVISES-011 **Revises PROP-011 §2.4** — boot regeneration moves from whole-tree to dirty-subgraph (§2.8); the "boot is cheap, keep it whole-tree" decision is re-opened by the verbatim-compilation cost and its recorded trigger has fired. @status:impl/done
- @fact:COMPAT-DEPENDS-035 **Depends on PROP-035** — the structural/JIT concepts (`#use`, read-set, link tables, reversible markers) become load-bearing rather than best-effort. This PROP is the concrete evolution of PROP-035's two-mode boot linker (§2) and its emission layer (§12). @status:impl/done
- @fact:COMPAT-MIGRATION-DEMO-FIRST Migration is demo-corpus-first (PROP-035 §15): build and prove on throwaway fixtures before converting any real package; vibevm itself converts last, and only where a package opts into the hybrid shape. @status:spec/done

---

## 5. Resolved questions {#open}

@fact:resolved-lead The five questions opened in the design dialogue were resolved 2026-07-15 (Phase 0): @status:impl/done

1. @fact:RES-ORTHOGONAL-AXES **`soft` × `static-transitive` — orthogonal axes.** `static-transitive` decides *which* packages are static (it forces the subtree); `soft`/`hard` decides *how* duplicates are deduped (hoist vs. local). They compose: a `static-transitive` edge's forced subtree is deduped by `soft` (hoisting) by default. No separate `static-transitive-hard` variant ships in v1 — the matrix stays 2×1 (`soft`/`hard`) × (`direct`/`transitive`) with hard-transitive deferred (no use case). @status:impl/done
2. @fact:RES-USE-COUNTER **Static-use counter — both direct and forced count.** A package reached by a direct `static`/`static-hard` edge **and** a package forced static by a `static-transitive` ancestor both increment its static-use count for hoisting (§2.4). `dynamic` edges never count. This keeps hoisting correct across a forced subtree. @status:impl/done
3. @fact:RES-FP-STORAGE **Fingerprint storage — the `STATIC.md`/`INDEX.md` header (§2.7).** A generated header comment carries the unit's `fp`, self-describing and reversible (PROP-035 §11), with **no `vibe.lock` schema bump** — avoiding an observable-contract change to the lockfile (the lighter of the RP2 options). A link-table cache (PROP-035 §10) may memoise it later; the header is the source of truth. @status:impl/done
4. @fact:RES-GRANULARITY **Granularity — per package (v1).** Fingerprint and invalidation are per compilation unit (package). Section-level granularity (PROP-035 §5 IR) is deferred ([plan DEF-1](../../../legacy-spec/terraforms/HYBRID-LINKING-PLAN-v0.1.md)). @status:impl/done
5. @fact:RES-DYN-BOUNDARY **Dynamic-boundary representation — aggregated into the unit's `INDEX.md`.** When a unit's static zone is compiled, every `dynamic` edge inside that zone is surfaced into the **unit's own `INDEX.md`** (not left as an inline directive in the compiled text). A unit's `INDEX.md` is thus the complete "what to load dynamically once you have read my `STATIC.md`" manifest — one manifest per unit, no inline resolution the agent must perform mid-text. @status:impl/done

@fact:MIGRATION-SAFETY-COROLLARY **Migration-safety corollary (Phase 0 finding).** Per-unit artifacts (§2.1) are **additive**: generating `STATIC.md`/`INDEX.md` inside a `vibedeps/` slot is new output, expected on migration. An entry-point node's **existing** artifacts stay **byte-identical** for a tree with no intermediate static edges (today's vibevm: `static` reaches the boot lane only through the root's `static-transitive` redbook edge, so root recursion reproduces the current root `STATIC.md`). P5's acceptance therefore checks *root artifacts unchanged* **plus** *new per-unit artifacts appear*, not "no new files". @status:impl/done

---

## 6. Version history {#history}

- @fact:HISTORY-DRAFTED **2026-07-15 — drafted (owner-requested).** Captures the hybrid-linking design dialogue: per-package compilation units (§2.1); the edge as linker instruction with recursive, dynamic-bounded compilation and the `static` / `dynamic` / `static-transitive` semantics (§2.2); the `static-soft` (default) / `static-hard` modes and why soft is the default (§2.3); LCA-scoped hoisting with the within-zone/cross-zone split and hoist transitivity (§2.4); the two-ended hoist markers — local `#use` + shared-by hint (§2.5); the single-version invariant the dedup rests on and the closed multi-version-hint question (§2.6); the Merkle fingerprint over the boot graph (§2.7); dirty-subgraph incremental regeneration revising PROP-011 §2.4 (§2.8); the read-set as read-time dedup (§2.9); and the differential-oracle-centred test obligations (§3). Implementation is the [HYBRID-LINKING campaign](../../../legacy-spec/terraforms/HYBRID-LINKING-PLAN-v0.1.md). @status:spec/done
- @fact:HISTORY-ACCEPTED **2026-07-15 — ACCEPTED; §5 resolved (Phase 0).** Ratified by the owner's implement-in-full directive. The five open questions resolved inline (§5): soft/hard × transitive are orthogonal; both direct and forced edges increment the static-use count; the fingerprint lives in the artifact header (no lockfile bump); granularity is per-package; dynamic boundaries aggregate into the unit's `INDEX.md`. The migration-safety corollary pins per-unit artifacts as additive with entry-point artifacts byte-stable for the current tree. @status:spec/done
- @fact:HISTORY-IMPLEMENTED **2026-07-15 — IMPLEMENTED.** All five phases of the [HYBRID-LINKING campaign](../../../legacy-spec/terraforms/HYBRID-LINKING-PLAN-v0.1.md) landed on `main` (`d487d4e`…`381095e`), floor green throughout. In `vibe-workspace`: the per-unit recursive compiler (`boot::hybrid` — `resolve_zone` / `topo_zone`), soft hoisting (`hybrid::hoist`) with `#use` markers and shared-by hints, the `static-hard` opt-out on `LinkType`, Merkle fingerprints (`hybrid::fingerprint`) driving the emit-side dirty-subgraph skip (§2.8), and the `verify_boot_graph` integrity check (§3). Emission lives in `install/bootgen` + `bootgen/hybrid_emit`. 178 tests, specmap 0 orphans. Deferred (plan §15): broad conversion of real packages (DEF-3), a `proptest` fuzz sweep (DEF-5), and the `vibe check` CLI wiring (DEF-6). Today's tree is byte-stable — `static` reaches the lane only through the root's `static-transitive` edge, so nothing is per-unit-emitted yet. @status:spec/done
