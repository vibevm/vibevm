# PROP-032 — The project model as a universal typed graph; the agent-first IDE substrate {#root}

**Status.** Design proposal v0.1 — not implementation-locked. Drafted for review; every decision below is open to challenge until ratified. This PROP names a *model and a direction*; it schedules no implementation of its own. It is the umbrella under which [PROP-014](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) (traceability) and [PROP-031](spec://vibevm/common/PROP-031#root) (refactoring) become **consumers of one model**, and it fixes the one foundational extension both need: **code as a first-class addressable node.**

**Companions.** [PROP-014 — specmap bidirectional traceability](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) (the code↔spec *projection* of this model; its `#edges`, `#queries`, `#runtime` are generalised here) · [PROP-031 — algorithmic refactoring](spec://vibevm/common/PROP-031#root) (the *mutations* over this model) · [PROP-003 — dependency evolution](spec://vibevm/modules/vibe-resolver/PROP-003) §2.5.3 and [PROP-014 §2.7](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#llm-boundary) (the LLM boundary this PROP makes the primary interface) · [PROP-000 §3](spec://vibevm/common/PROP-000#license) (permissive-only dependencies) · prior art: LSP, SCIP/LSIF, rustdoc intra-doc links, Sphinx domains (§6).

---

## 1. Problem statement — the reframe {#problem}

`prop r1` — specmap (PROP-014) gives us a real, deterministic, committed graph of the project — but it is **asymmetric by construction**. Its `Edge` type is hard-coded `(code item) --verb--> (spec unit)`: a code symbol is always the source, a spec unit always the target, across exactly five verbs (`implements`, `verifies`, `documents`, `deviates`, `informs`). Consequences we keep hitting:

- **Code has no stable address.** A `CodeItem` carries a symbol-path, file, and line — all *derived* and *volatile* — but no minted, refactor-stable address. Code can be *pointed from*, never *pointed at*.
- **Only one of four directions exists.** Prose specs cannot cite each other as tracked edges (spec→spec); a spec or a doc cannot point *at* code as the authority (spec→code) — even though sometimes **the code is the best, most precise description** (an algorithm, a wire schema, a canonical example); code cannot reference code across the package boundary as a tracked link (code→code).

And the ambition is larger than traceability. We want **navigation** (go-to-definition, find-references, impact), **refactoring** (PROP-031), and eventually **surfaces** — a library API, a command line, and one day a graphical view — *all agent-first*. That is not a traceability index; that is a **language server for the whole project model**: one graph, a query surface, a mutation surface, driven primarily by an agent.

This PROP names that model — a **symmetric typed graph over addressable nodes** — and the substrate built on it. PROP-014 and PROP-031 do not compete with it; they instantiate it. The model must be symmetric first, or none of navigation, refactoring, or surfaces can be complete, because every one of those is inherently bidirectional.

## 2. Decisions {#decisions}

### 2.1 The project model is a typed, directed graph over addressable nodes {#graph}

`prop r1` — The canonical model of the project is a **typed, directed property graph**: **nodes** are anything with a stable address (spec units today; code items, packages, boot entries as the model grows); **edges** are typed and directed and may connect **any node kind to any node kind**. specmap's `code→spec` edges are **one projection** of this graph, not its definition. The graph is the single source of truth for navigation and refactoring; everything else (the index file, the queries, the operations, the surfaces) is a view or an action over it.

### 2.2 Every node carries a stable, minted, location-independent address {#addressing}

`req r1` — A node's **address of record is minted and travels with the artifact**; its *location* (line number, symbol-path, doc-path, file) is **derived decoration, never the address**. This is the property that lets the graph survive refactoring — the lesson [PROP-014 §5.2](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#rejected) learned when it rejected line/range anchors as "maximally fragile," here promoted to a model-wide invariant:

- **spec node** → `spec://<ns>/<doc-path>#<anchor>`; the `{#anchor}` is minted and immutable, travels via edit-in-place.
- **code node** → `code://<ns>/<id>`; the `<id>` is minted and travels **on an attribute on the item** (§2.3).
- **package node** → the FQID (PROP-029).

Because the address is minted, **renaming or moving the artifact does not change its address** — the address moves *with* it. Location-based addressing is inverted: the symbol-path and line are computed *from* the item at index time, for human navigation, and are free to churn.

### 2.3 Code is a first-class node (`code://`), not only a source {#code-node}

`req r1` — The concrete extension this PROP exists to fix. A code item becomes an **addressable node** under `code://<namespace>/<id>`, where `<id>` is a minted, immutable, kebab-case identifier **carried by a per-language marker on the item** — the `specmark` projection pattern PROP-014 §2.9 already uses (a Rust attribute `#[addr("resolver-fixpoint")]` or a field on `#[spec]`; JSDoc/decorator for the other stacks). The item's symbol-path, file, and line are **derived decoration**, exactly as a spec unit's `line` is.

Two rules keep it honest:

- **Mint sparingly.** A `code://` address is minted **only where the code is meant to be pointed at** — a canonical algorithm, a wire schema, a reference example — not on every function. Most code stays addressed only *derivedly* (by symbol-path in the index, which is enough for find-references). Ceremony is proportional to authority.
- **The id is the address, the name is not.** Because the id lives on the attribute, `rename-symbol` / `move-item` (PROP-031) **do not break `code://` links** — the same robustness that makes `#[spec]` survive refactors. This is the whole reason to mint rather than address by symbol-path.

**Where the marker lives — on the item, never external.** The address is carried by whatever metadata construct is *idiomatic and scannable* in each language, attached to the item itself, because an address that does not travel with the artifact reintroduces exactly the fragility [PROP-014 §5.1](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#rejected) rejected (a sidecar map rots on every refactor). The neutral grammar is the address; the carrier is per-language (PROP-014 §2.9): a **structured attribute** where the language has one (Rust `#[spec(addr = "…")]`, Java/Kotlin annotation, Python decorator), a **structured doc-comment tag** where it does not (TypeScript/JS `/** @addr code://… */`, matching the `@spec` JSDoc choice), and a **comment-directive** for languages with neither (Go `//spec:addr …`, cf. `//go:generate`). Three constraints bind every carrier: (1) **on the item** — travels on refactor; (2) **scannable without execution** — read as AST/text, as `#[spec]` is today; (3) **structured, not free prose** — a defined grammar the scanner parses, never a human sentence. **Recommendation:** make `addr` a **facet of the existing `specmark` marker**, not a new construct — one code-marker family then carries both a node's *identity* (`addr`) and its *outgoing edges* (the verbs), the whole code-side of the graph in one place.

This single change **removes the asymmetry of §1**: code can now be a *target*, so spec→code, doc→code, and code→code edges become expressible — and the graph is symmetric.

### 2.4 Edges are typed by authority direction {#edges}

`prop r1` — An edge's **verb carries which end is the source of truth**, which is what makes the graph queryable and honest. Generalising [PROP-014 §2.4](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#edges) from five code→spec verbs to a directional set:

| From → To | Verb(s) | Meaning |
|---|---|---|
| code → spec | `implements` `verifies` `documents` `deviates` `informs` | exist — code defers to the spec as authority |
| spec → spec | `references` `refines` `supersedes` | a prose citation as a tracked edge (the D3 gap, PROP-031 §3.3) |
| **spec → code** | **`defined-by` / `canonical`** | **the normative description of this concept is the code at `code://…`** — the authority *inversion* |
| doc → code | `exemplifies` | the canonical example / reference usage is here |
| code → code | `uses` `see-also` | a cross-item reference (rustdoc intra-doc links, generalised across packages) |

`canonical` / `defined-by` is a **marked, deliberate, rare inversion** — the peer of `deviates`. It says "this concept's normative content genuinely *is* the code; do not restate it in prose." It is **not** a licence for shadow-code: [PROP-014 §3.1.6](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) ("spec states *what* and *why*, never *how*; a spec that mirrors code is drift fuel") still governs the default. The inversion is the exception you *mark*, exactly as a deviation is.

### 2.5 Three operations over one model: query, mutate, render {#operations}

`prop r1` — The substrate exposes exactly three operation families over the graph, and "the IDE" is nothing more than these three:

- **Query — navigation.** *Go-to-definition* = follow an edge to its target; *find-references* = the reverse edges into a node; *impact* = the transitive closure. specmap already ships these as `explain` / coverage / impact ([PROP-014 §2.6](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#queries)); symmetry (§2.3) is what makes them work in *both* directions.
- **Mutate — refactoring.** The typed, atomic, gated operations of PROP-031 (`rename-address`, `move-unit`, `rename-symbol`, …), now spanning `spec://` **and** `code://` addresses.
- **Render — explanation.** The deterministic subgraph plus its optional prose rendering (PROP-014 §2.6); the data layer is always available without an LLM.

### 2.6 Agent-first: the primary client emits typed commands; surfaces are progressive {#agent-first}

`req r1` — The **primary consumer of the substrate is an agent**, not a human at a keyboard. An agent drives navigation and refactoring by **emitting typed query/mutation commands** — the LLM boundary of [PROP-031 §2.2](spec://vibevm/common/PROP-031#llm-boundary) and PROP-014 §2.7: *the model proposes a typed command; the deterministic engine executes and gates it.* The transport is MCP ([PROP-014 §2.8](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#runtime), already shipping `specmap_query` / `specmap_explain`).

The consequence reorders the usual notion of "IDE": **the IDE is a headless model-plus-operations server; the GUI is the last, optional client, not the IDE itself.** Surfaces are progressive projections of the *same* command set:

```
library API  →  command line  →  MCP / agent  →  (last, optional) graphical view
   (exists)       (exists)        (embryo §2.8)        (future, human-facing)
```

Building GUI-first would invert the dependency — a graphical shell over an incomplete, asymmetric model. The model and the operations are the product; every surface is a client.

### 2.7 Integrity and refactoring fall out of the model, not bespoke code {#free}

`req r1` — Because everything is one graph with minted addresses, the hard properties are **free**: a link to an address that resolves to no node is a **dangling** edge (the existing gate); a duplicate minted id is a **duplicate-address** warning (the existing `duplicate-anchor` machinery); a rename is a **`rename-address`** operation (PROP-031); a stale pin is a **suspect** (PROP-014 §2.2). This is the reason symmetry is *cheap once the node model generalises*: no new subsystem, only more node kinds and edge directions in the graph that already computes all of this.

## 3. Layering — what this owns, versus PROP-014 and PROP-031 {#layering}

`prop r1` — To avoid duplication (the one real overlap risk), the boundary is explicit:

- **PROP-032 (this) owns the *model shape and the vision*:** the universal typed graph (§2.1), the addressing invariant (§2.2), the code node (§2.3), the directional edge set (§2.4), the three operation families (§2.5), and the agent-first substrate (§2.6). It specifies *what the model is*, not *how edges are extracted or gated*.
- **PROP-014 owns the *traceability instantiation and mechanics*:** the `#[spec]`/`scope!` grammar, extraction (`rscan`/`mdspec`), revisions/suspects, the committed `specmap.json`, and the gate. It is the **code↔spec projection** of this model — the first and canonical one — and it **grows** (per this PROP) a `code://` node kind and the spec→spec / spec→code directions. It is not superseded; it is generalised (§7 open question 5).
- **PROP-031 owns the *mutations*:** the typed refactoring operations over the model, gated by re-checking it.

PROP-032 introduces **no extraction or gate mechanics of its own**; it names the model those mechanics populate and the extensions they must grow to cover. The engine still lives in `core-ai-native`; the host PROPs (031, 032) drive it, exactly as PROP-031 already does.

## 4. Build-in-anticipation {#anticipation}

`req r1` — Extends PROP-031 §3 with the model-level disciplines, in force from ratification:

1. **Address every node kind; never location-address.** Generalises PROP-031 §3.1 to code and packages: the address of record is always the minted id/anchor/FQID, never a line or a symbol-path. Author nothing that can only be reached by location.
2. **Mint `code://` addresses sparingly and deliberately** — only where code is the canonical description (§2.3). Over-minting is noise; under-minting leaves authoritative code unpointable.
3. **Generalise `Edge` from `code→spec` to `node→node` deliberately** (a schema v3 step, §7), not piecemeal — the byte-stable `specmap.json` must migrate once, cleanly.
4. **Build the model and the operations, not the GUI.** The substrate is complete when navigation + refactoring run agent-first over a symmetric, gated graph. A graphical surface is a later, separate, human-facing decision.

## 5. Rejected alternatives {#rejected}

1. **Address code by location (line or symbol-path).** Fragile — exactly what refactoring changes (PROP-014 §5.2). Retained only as *derived* index decoration, never the address.
2. **A bespoke "spec→code link type."** Special-cases what should be symmetric. The universal move is a code *node* (§2.3); then spec→code is an edge like any other, and doc→code / code→code come for free. A one-off link type would need a second one for every new direction.
3. **A GUI-first IDE.** Inverts the dependency — a shell over an incomplete model. The model + operations are the IDE; the GUI is the last client (§2.6).
4. **A separate code-navigation tool (a standalone SCIP/LSIF server) beside specmap.** Two graphs, two truths, two things to keep in sync. The code node lives in the *same* graph, so navigation and traceability compose (find every REQ *and* every doc that points at a function, in one query).
5. **Keep the graph `code→spec` only, with a computed inverse.** The inverse answers "what implements this REQ" but cannot represent an *authored* spec→code or spec→spec edge, cannot gate a prose citation, and cannot be refactored — the whole point of §1.

## 6. Prior art & license posture {#prior-art}

Conventions and ideas are free; code is not (PROP-000 §3). License fields re-verified before any code-level reuse.

| System | License (verify) | Role here |
|---|---|---|
| **LSP** (Language Server Protocol) | n/a (protocol) | The architecture: one model, a query surface, a mutation surface, many clients. This PROP is "LSP for spec + code + packages, agent-first." |
| **SCIP / LSIF** (Sourcegraph) | Apache-2.0 | **The code-node prior art.** A serialised, committed graph of code symbols + references for navigation without a live server — precisely `specmap.json` for code. They solved the stable-symbol *moniker* problem (§7). |
| rustdoc intra-doc links | n/a (rustc) | Path-based, compiler-checked in-code references — the `code→code`, integrity-checked precedent. |
| Sphinx domains + `:ref:` | BSD-2 | Prose→object references resolved and checked at build — the `doc→code` precedent. |
| OpenRewrite (via PROP-031) | Apache-2.0 | Typed, gated operations over a lossless model — the mutation half. |

**Differentiators.** (i) spec, code, and packages live in **one** graph, so traceability and navigation compose; (ii) edges are **typed by authority direction**, so the graph states who is the source of truth; (iii) the primary client is an **agent** emitting typed commands, with GUI as the last surface, not the first; (iv) every relation is **gated** by the discipline's own invariant, not merely "compiles."

## 7. Open questions {#open}

1. **The `code://` id scheme.** A free-minted slug (`resolver-fixpoint`) maximises rename-stability but adds a namespace to manage; a **structured moniker** (SCIP-style: package + descriptor path) needs no minting but moves under refactor. Lean: **free-minted for authoritative nodes** (stability is the point), structured monikers as the *derived* address for everything else.
2. **How many node kinds.** Packages (FQID) and boot entries (INDEX id) are the obvious next nodes (the product tier of PROP-031). Config? Manifests? Grow by demonstrated need, not speculation.
3. **Schema v3 migration.** Generalising `Edge` from `code→spec` (`from_symbol` + `uri`) to `node→node` (two typed addresses) is a `specmap.json` schema change; plan the byte-stable migration (the `check-codegen` idiom, PROP-014 §2.5).
4. **Reverse-edge storage vs computation.** Find-references can be computed by inverting the edge set (as today) or materialised; decide when the graph grows enough that inversion cost matters.
5. **PROP-014's identity. Decided (owner, 2026-07-13): it grows *in place*.** PROP-014 keeps its title ("specmap: bidirectional traceability") and gains the `code://` node kind + the new edge directions (spec→spec, spec→code); PROP-032 references it as the canonical first projection, not a replacement. No re-scope, no rename — the extension lands as new sections in PROP-014 and new node/edge kinds in its engine.
6. **When (and whether) a GUI, and by whom.** Explicitly deferred (§2.6, §4.4). The substrate must be complete and agent-first first; a graphical client is a separate, later, human-facing project.

---

*This PROP is a design proposal. Ratification happens through PR review against this document, PROP-014, and PROP-031. It commits to no implementation of its own; its first concrete step is the `code://` node (§2.3), sequenced by the SPECMAP Unit-Mobility Plan and PROP-031's operation roadmap. Any mechanism specified here that is not exercised by the second shipped node kind or edge direction is removed from the spec rather than carried as aspirational documentation (the PROP-014 §335 discipline, inherited).*
