# PROP-034: Transitive inclusion links and the static boot-link graph {#root}

<status stage="impl" state="done" comment="C 2026-07-25: the transitive semantics shipped under the renamed terms; PROP-035 §12 / PROP-038 absorbed the graph as their emission layer; body predates the 2026-07-16 rename"/>

@fact:status-line **Status: IMPLEMENTED under renamed terms** (requirements authored 2026-07-14 at the owner's request; verified against the tree 2026-07-25 by the spec-actualization campaign). `static-transitive` is live on `LinkType`, and dedup + topological ordering + tie-break emission run in `bootgen` — this repository's own `STATIC.md` is the §3 redbook closure in the flesh. [PROP-035 §12](PROP-035-spec-compiler.md) and [PROP-038](PROP-038-hybrid-boot-linking.md) absorbed the graph as their emission layer; read the `TERMINOLOGY-RENAME` note below before the body. Extends [PROP-009](PROP-009-loading-model.md) (the loading model). @status:impl/done

@fact:TERMINOLOGY-RENAME **Terminology (2026-07-16):** the `inline` / `static` / `dynamic` link types this document describes were **renamed** — read `inline` as `static` (the verbatim `STATIC.md` lane), `static` as `dynamic` (the default, a by-reference read), and the old `dynamic` as a `dynamic` entry carrying a `when`; `inline-transitive` is now `static-transitive`. See PROP-009 §2.4. The body below predates the rename and keeps the old names. @status:spec/done

@fact:related **Related:** [PROP-009 §2.4](PROP-009-loading-model.md#inclusion-types) (the direct `inline` / `static` / `dynamic` link types this PROP makes transitive), [PROP-028](../../common/PROP-028-package-families.md) (families / collections — the motivating consumer), [PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) / [PROP-017](../vibe-resolver/PROP-017-resolvo-resolver.md) (version unification — one node per resolved package), [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md) (the static/dynamic-linking metaphor this PROP completes). @status:spec/done

@fact:supersedes-line **Supersedes:** backlog `B1` (`transitive-inline` / `transitive-static`) — promoted to this PROP. @status:spec/done

---

## 1. Motivation — the boot closure must link like a static linker {#motivation}

@fact:two-gaps-lead PROP-009 §2.4 gives each **direct** dependency edge an inclusion type — `inline` (verbatim into `INLINE.md`, the priority lane), `static` (a path in `INDEX.md`), or `dynamic` (a conditional INCLUDE). Two gaps make a growing dependency set fragile, and a collection like `redbook` un-loadable the way its consumer wants: @status:spec/done

@fact:GAP-NO-PROPAGATION **Gap 1 — inclusion does not propagate transitively.** A consumer sets `link` only on its own direct edges. @status:spec/done

- @fact:GAP1-CLOSURE-DEFAULT A dependency's transitive closure takes its type from each member's own `[boot_snippet]` suggested link, or the `static` default — `bootgen` resolves `declared_link.or(suggested_link)`, and a transitive dependency's `declared_link` reads back as `None`. @status:spec/done
- @fact:GAP1-COLLECTION-BLOCKED So a **collection** (e.g. `redbook`, PROP-028) cannot say "load my whole closure inline." @status:spec/done
- @fact:GAP1-WORKAROUND-WRONG The only workaround — each member self-suggesting `inline` — is wrong: it forces inline on **every** consumer of that member, not just the one collection that wanted it. @status:spec/done
- @fact:GAP1-CONSUMER-PROPERTY Inclusion strength is a property of *how a consumer pulls a subtree*, and today that cannot be expressed. @status:spec/done

@fact:GAP-NO-LINK-GRAPH **Gap 2 — the boot closure is not resolved as a static link graph.** A dependency closure is a graph, and assembling a boot from it is exactly **static linking**. @status:spec/done

@fact:INVARIANTS-INFORMAL Three linker invariants are currently only informally met by PROP-009's "ordered list" and are load-bearing: @status:spec/done

- @fact:INV-LINK-ONCE **Each package linked exactly once.** A package reachable through several dependency paths must appear **once** in `INLINE.md` / `INDEX.md`, never N times. Double-inclusion wastes context, and for `inline` it duplicates verbatim text and its anchors (a `duplicate-anchor` hazard). @status:spec/done
- @fact:INV-DEP-ORDER **Dependency order (topological).** A package must be emitted **before** every package that requires it, so that when a dependent's boot text is read, everything it builds on is already in context. A package needed by another (or several) comes **earlier** in the sequence. @status:spec/done
- @fact:INV-DETERMINISTIC **Deterministic and acyclic.** The walk must be deterministic (stable output across runs) and reject cycles loudly at generate time — the agent-side read stays recursion-free (PROP-009 §2), so all graph work, including cycle detection, happens once in `vibe`. @status:spec/done

@fact:RISK-WITHOUT-PINNING Without these pinned, adding the whole `redbook`, `delegation-first`, and future collections risks a boot that double-includes packages and orders a dependent before its dependency — the failure the owner names as "we might never load cleanly." @status:spec/done

---

## 2. Decisions {#decisions}

### 2.1 Transitive inclusion links {#transitive-links}

@fact:TRANSITIVE-VARIANTS Extend the §2.4 `link` value set with **transitive** variants: @status:spec/done

```toml
[requires.packages]
"flow:org.vibevm.world/redbook"     = { version = "^0.2", link = "inline-transitive" }
"stack:org.vibevm.ai-native/rust"   = { version = "^0.7", link = "static-transitive" }
"flow:org.vibevm.world/wal"         = { version = "^0.2", link = "static" }   # direct (this edge only)
```

- @fact:LINK-INLINE-TRANSITIVE `link = "inline-transitive"` — this package **and its entire transitive closure** are pulled `inline`. @status:spec/done
- @fact:LINK-STATIC-TRANSITIVE `link = "static-transitive"` — this package and its entire transitive closure are pulled `static`. @status:spec/done
- @fact:LINK-DYNAMIC-RESERVED `dynamic-transitive` — reserved (§5). @status:spec/done

- @fact:DIRECT-UNCHANGED The existing `inline` / `static` / `dynamic` remain **direct**: they set the mode of *this edge's target only*; the target's own closure resolves by its own rules. @status:spec/done
- @fact:TRANSITIVE-SUBTREE-DECL A transitive link is the consumer's declaration that the mode applies to *the whole subtree reached through this edge* — the missing "how I pull this subtree" expressiveness of Gap 1. @status:spec/done
- @fact:CONSUMER-SIDE-PROPERTY This is a **consumer-side** property of the edge, not a property baked into the pulled package — the same package can be pulled `inline-transitive` by one consumer and `static` by another; nothing is written into the package. @status:spec/done

### 2.2 Effective inclusion mode — the precedence lattice {#precedence}

@fact:multi-path-reality A package can be reached by several edges and several transitive closures carrying different modes. @status:spec/done

@fact:EFFECTIVE-MODE-JOIN Its **effective inclusion mode** is the join of every mode that reaches it, under the lattice: @status:spec/done

```
inline ⊐ static ⊐ dynamic
```

@fact:resolution-lead Resolution, for each package in the closure: @status:spec/done

1. @fact:RES-COLLECT Collect every mode reaching it: @status:spec/done
   - @fact:RES-SRC-DIRECT (a) a **direct** edge to it contributes that edge's mode; @status:spec/done
   - @fact:RES-SRC-TRANSITIVE (b) a **transitive** link contributes its mode to *every* package in the closure reached through it; @status:spec/done
   - @fact:RES-SRC-SUGGESTED (c) the package's own `[boot_snippet]` suggested link; @status:spec/done
   - @fact:RES-SRC-DEFAULT (d) the `static` default when nothing else applies. @status:spec/done
2. @fact:RES-STRONGEST The effective mode is the **strongest** (left-most) of those. @status:spec/done

@fact:consequences-lead Consequences, all deliberate: @status:spec/done

- @fact:INLINE-WINS **Inline wins, monotonically.** If any path pulls a package inline (a direct `inline`, or an `inline-transitive` ancestor, or a self-suggested `inline`), it is inline. Adding a stronger link never demotes a package — a package's priority can only rise, never silently fall, as the graph grows. @status:spec/done
- @fact:DYNAMIC-OVERRIDDEN **`dynamic` is overridden by any unconditional need.** If one path needs a package `dynamic` (context-gated) but another needs it `static`/`inline` unconditionally, it loads unconditionally — the `when` gate cannot hide a package something else requires outright. @status:spec/done
- @fact:NO-DIRECT-DEMOTION A **direct** link never demotes a package an ancestor pulled `inline-transitive` (inline is sticky, per the monotonicity above). A consumer that truly needs a subtree *not* inline must not sit under an `inline-transitive` edge to it. @status:spec/done

@fact:LATTICE-DETERMINISM The lattice makes the effective mode a deterministic function of the graph, independent of walk order. @status:spec/done

### 2.3 The static boot-link graph {#link-graph}

@fact:LINKER-ONCE `vibe` resolves the boot exactly once, at install / generate time, as a static linker: @status:spec/done

1. @fact:STEP-BUILD-GRAPH **Build the graph.** Nodes = the resolved package versions in the root's dependency closure (one node per unified `(group, name, version)` — the resolver has already unified versions, PROP-003 / PROP-017). Edges = `requires`. @status:spec/done
2. @fact:STEP-ASSIGN-MODES **Assign effective modes** to every node (§2.2). @status:spec/done
3. @fact:STEP-DEDUP **Deduplicate.** Each node contributes its boot **exactly once**, in its effective mode — regardless of how many paths reach it. @status:spec/done
4. @fact:STEP-TOPO-SORT **Topologically sort.** Order the nodes so that **every dependency precedes every dependent**. Independent nodes (no path between them) are ordered by a deterministic tie-break — category, then boot-snippet slot, then fully-qualified name — so the emitted sequence is byte-stable across runs. @status:spec/done
5. @fact:STEP-REJECT-CYCLES **Reject cycles.** `requires` is expected acyclic; a cycle is a **hard error at generate time**, reported with the offending cycle path. The boot is never emitted half-linked. (This is the one place cycle detection lives; the agent-side read is a flat, recursion-free parse per PROP-009 §2.) @status:spec/done

### 2.4 Emission — a dependency-ordered priority lane and index {#emission}

@fact:emission-lead From the sorted, deduplicated, mode-assigned node list: @status:spec/done

- @fact:EMIT-INLINE **`inline`** nodes → concatenated verbatim into `INLINE.md`, **in topological order** — a dependency's boot text precedes its dependents' within the priority lane. This is the **static-linked inline lane**: inline content, but resolved, deduplicated, and dependency-ordered by the linker rather than pasted in discovery order. @status:spec/done
- @fact:EMIT-STATIC **`static`** nodes → `[[entry]]` `kind = "static"` in `INDEX.md`, in topological order. @status:spec/done
- @fact:EMIT-DYNAMIC **`dynamic`** nodes → `[[entry]]` `kind = "dynamic"` with their `when`, in topological order. @status:spec/done

@fact:LANES-ORDERED Both lanes are dependency-ordered: a reader (human or agent) always meets a package **before** anything that builds on it, and never meets the same package twice. @status:spec/done

### 2.5 Boot budget note {#budget}

- @fact:BUDGET-COST `inline-transitive` over a large collection puts that collection's **whole** boot closure into `INLINE.md`, read first and in full every session. @status:spec/done
- @fact:BUDGET-TRADEOFF That is the point when priority must be *guaranteed by position* (critical practices that must never be missed) — and a real cost when the closure is large. @status:spec/done
- @fact:BUDGET-GUIDANCE Choose `inline-transitive` for closures whose every member is boot-critical; keep `static-transitive` (still deduplicated and dependency-ordered, but read on demand from `INDEX.md`) as the default for large practice sets where INDEX-lane resolution is acceptable. @status:spec/done

---

## 3. `redbook` as `inline-transitive` — the motivating case {#redbook}

@fact:REDBOOK-CONSUMER vibevm pulls `flow:org.vibevm.world/redbook` as `inline-transitive`. The effect, under §2: @status:spec/done

- @fact:redbook-effect-inline Every practice `redbook` collects, plus their transitive deps, resolves to effective mode `inline`. @status:spec/done
- @fact:redbook-effect-dedup The graph is deduplicated: a practice several members share (e.g. `git-practices`, or a common dependency) appears **once**. @status:spec/done
- @fact:redbook-effect-order The graph is topologically ordered: a practice needed by another (or by `redbook` itself) is emitted **earlier** in `INLINE.md`. @status:spec/done
- @fact:redbook-effect-guarantee The whole tested edition is thus in the priority lane, verbatim, dependency-ordered, once each — the practices are guaranteed to load, which is the safety the owner is buying (over `static`, which trusts agent-side INDEX resolution). @status:spec/done

@fact:B1-INTERIM-DROPPED The per-member `[boot_snippet].link = "inline"` self-suggestions used as the B1 interim (e.g. on the `git-practices` members) become unnecessary once the consumer declares `inline-transitive`, and can be dropped — the inclusion strength moves back to the consumer where §2.1 puts it. @status:spec/done

---

## 4. Compatibility and migration {#compat}

- @fact:COMPAT-ADDITIVE The direct `inline` / `static` / `dynamic` types and their semantics are unchanged; the transitive variants are purely additive to the `link` enum. @status:spec/done
- @fact:COMPAT-PINS-ORDER §2.3's dedup + topological order **pins** what PROP-009 §2 left as an "ordered list": existing manifests keep working and simply gain guaranteed single-inclusion and dependency order. Any boot that silently relied on a *non*-topological order is a latent bug this surfaces. @status:spec/done
- @fact:COMPAT-SCHEMA Manifest schema (`vibe-core`): the `link` field accepts the new variants; unknown `link` values remain a manifest error. @status:spec/done
- @fact:COMPAT-IMPL-SITE Implementation lands in `vibe-workspace` boot resolution (`bootgen`): mode propagation (§2.2), dedup, topological sort with the deterministic tie-break, and cycle rejection. @status:spec/done

---

## 5. Open questions {#open}

1. @fact:OPEN-DYNAMIC-TRANSITIVE **`dynamic-transitive`.** Semantics of gating a whole closure behind one `when` (does the gate distribute to every member, or gate the subtree as a unit?) — reserved until a use case appears. @status:spec/work
2. @fact:OPEN-WHEN-INSIDE-INLINE **`when` inside a transitive-inline closure.** A member carrying its own `when` OS-gate that is pulled `inline-transitive`: does inline (unconditional, verbatim) or the gate win? Provisional: an explicit member `when` keeps the member `dynamic` (the gate is more specific than the ancestor's blanket inline); confirm against a real case. @status:spec/work
3. @fact:OPEN-EXPLICIT-DEMOTION **Explicit demotion.** Whether to add a `link = "static-here"` escape that lets a consumer pull a package `static` *even under* an ancestor's `inline-transitive` (breaking the monotonicity of §2.2). Deferred — no demotion use case today, and monotonic priority is the safer default. @status:spec/work

---

## 6. Version history {#history}

- @fact:HISTORY-DRAFTED **2026-07-14 — drafted (owner-requested).** Promotes backlog B1. Defines the transitive `inline-transitive` / `static-transitive` links (§2.1), the effective-mode precedence lattice (§2.2), and the static boot-link graph — dedup + topological order + cycle rejection (§2.3) — with dependency-ordered emission into the `INLINE.md` priority lane and `INDEX.md` (§2.4). `redbook` is the motivating `inline-transitive` consumer (§3). Implementation (manifest schema + `bootgen` resolution) is the next milestone. @status:spec/done
