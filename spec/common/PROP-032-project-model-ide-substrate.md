# PROP-032 — The project model as a universal typed graph; the agent-first IDE substrate {#root}

<status stage="spec" state="done" action="continue" comment="B0 2026-07-24: design proposal v0.1, drafted for review; open to challenge until ratified; fact grain 2026-07-24"/>

@fact:status-line **Status.** Design proposal v0.1 — not implementation-locked. Drafted for review; every decision below is open to challenge until ratified. This PROP names a *model and a direction*; it schedules no implementation of its own. It is the umbrella under which [PROP-014](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) (traceability) and [PROP-031](spec://org.vibevm.core/vibevm/common/PROP-031#root) (refactoring) become **consumers of one model**, and it fixes the one foundational extension both need: **code as a first-class addressable node.** @status:spec/done

@fact:companions **Companions.** [PROP-014 — specmap bidirectional traceability](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) (the code↔spec *projection* of this model; its `#edges`, `#queries`, `#runtime` are generalised here) · [PROP-031 — algorithmic refactoring](spec://org.vibevm.core/vibevm/common/PROP-031#root) (the *mutations* over this model) · [PROP-003 — dependency evolution](spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003) §2.5.3 and [PROP-014 §2.7](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#llm-boundary) (the LLM boundary this PROP makes the primary interface) · [PROP-000 §3](spec://org.vibevm.core/vibevm/common/PROP-000#license) (permissive-only dependencies) · prior art: LSP, SCIP/LSIF, rustdoc intra-doc links, Sphinx domains (§6). @status:spec/done

---

## 1. Problem statement — the reframe {#problem}

- @fact:ASYMMETRY-CLAIM `prop r1` — specmap (PROP-014) gives us a real, deterministic, committed graph of the project — but it is **asymmetric by construction**. @status:spec/done
- @fact:EDGE-HARDCODED Its `Edge` type is hard-coded `(code item) --verb--> (spec unit)`: a code symbol is always the source, a spec unit always the target, across exactly five verbs (`implements`, `verifies`, `documents`, `deviates`, `informs`). @status:spec/done

@fact:consequences-lead Consequences we keep hitting: @status:spec/done

- @fact:NO-CODE-ADDRESS **Code has no stable address.** A `CodeItem` carries a symbol-path, file, and line — all *derived* and *volatile* — but no minted, refactor-stable address. Code can be *pointed from*, never *pointed at*. @status:spec/done
- @fact:ONE-DIRECTION-ONLY **Only one of four directions exists.** Prose specs cannot cite each other as tracked edges (spec→spec); a spec or a doc cannot point *at* code as the authority (spec→code) — even though sometimes **the code is the best, most precise description** (an algorithm, a wire schema, a canonical example); code cannot reference code across the package boundary as a tracked link (code→code). @status:spec/done

- @fact:AMBITION And the ambition is larger than traceability. We want **navigation** (go-to-definition, find-references, impact), **refactoring** (PROP-031), and eventually **surfaces** — a library API, a command line, and one day a graphical view — *all agent-first*. @status:spec/done
- @fact:LANGUAGE-SERVER-FRAME That is not a traceability index; that is a **language server for the whole project model**: one graph, a query surface, a mutation surface, driven primarily by an agent. @status:spec/done

- @fact:MODEL-NAMED This PROP names that model — a **symmetric typed graph over addressable nodes** — and the substrate built on it. @status:spec/done
- @fact:CONSUMERS-INSTANTIATE PROP-014 and PROP-031 do not compete with it; they instantiate it. @status:spec/done
- @fact:SYMMETRY-FIRST The model must be symmetric first, or none of navigation, refactoring, or surfaces can be complete, because every one of those is inherently bidirectional. @status:spec/done

## 2. Decisions {#decisions}

### 2.1 The project model is a typed, directed graph over addressable nodes {#graph}

- @fact:GRAPH-DEF `prop r1` — The canonical model of the project is a **typed, directed property graph**: **nodes** are anything with a stable address (spec units today; code items, packages, boot entries as the model grows); **edges** are typed and directed and may connect **any node kind to any node kind**. @status:spec/done
- @fact:PROJECTION-NOT-DEFINITION specmap's `code→spec` edges are **one projection** of this graph, not its definition. @status:spec/done
- @fact:GRAPH-SSOT The graph is the single source of truth for navigation and refactoring; everything else (the index file, the queries, the operations, the surfaces) is a view or an action over it. @status:spec/done

### 2.2 Every node carries a stable, minted, location-independent address {#addressing}

- @fact:ADDRESS-MINTED-LAW `req r1` — A node's **address of record is minted and travels with the artifact**; its *location* (line number, symbol-path, doc-path, file) is **derived decoration, never the address**. @status:spec/done
- @fact:FRAGILITY-LESSON This is the property that lets the graph survive refactoring — the lesson [PROP-014 §5.2](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#rejected) learned when it rejected line/range anchors as "maximally fragile," here promoted to a model-wide invariant: @status:spec/done

- @fact:ADDR-SPEC-NODE **spec node** → `spec://<ns>/<doc-path>#<anchor>`; the `{#anchor}` is minted and immutable, travels via edit-in-place. @status:spec/done
- @fact:ADDR-CODE-NODE **code node** → `code://<ns>/<id>`; the `<id>` is minted and travels **on an attribute on the item** (§2.3). @status:spec/done
- @fact:ADDR-PACKAGE-NODE **package node** → the FQID (PROP-029). @status:spec/done

- @fact:ADDRESS-SURVIVES-RENAME Because the address is minted, **renaming or moving the artifact does not change its address** — the address moves *with* it. @status:spec/done
- @fact:LOCATION-DERIVED Location-based addressing is inverted: the symbol-path and line are computed *from* the item at index time, for human navigation, and are free to churn. @status:spec/done

### 2.3 Code is a first-class node (`code://`), not only a source {#code-node}

- @fact:CODE-NODE-EXTENSION `req r1` — The concrete extension this PROP exists to fix. @status:spec/done
- @fact:CODE-ADDR-FORM A code item becomes an **addressable node** under `code://<namespace>/<id>`, where `<id>` is a minted, immutable, kebab-case identifier **carried by a per-language marker on the item** — the `specmark` projection pattern PROP-014 §2.9 already uses (a Rust attribute `#[addr("resolver-fixpoint")]` or a field on `#[spec]`; JSDoc/decorator for the other stacks). @status:spec/done
- @fact:CODE-LOCATION-DERIVED The item's symbol-path, file, and line are **derived decoration**, exactly as a spec unit's `line` is. @status:spec/done

@fact:code-rules-lead Two rules keep it honest: @status:spec/done

- @fact:MINT-SPARINGLY **Mint sparingly.** A `code://` address is minted **only where the code is meant to be pointed at** — a canonical algorithm, a wire schema, a reference example — not on every function. Most code stays addressed only *derivedly* (by symbol-path in the index, which is enough for find-references). Ceremony is proportional to authority. @status:spec/done
- @fact:ID-IS-ADDRESS **The id is the address, the name is not.** Because the id lives on the attribute, `rename-symbol` / `move-item` (PROP-031) **do not break `code://` links** — the same robustness that makes `#[spec]` survive refactors. This is the whole reason to mint rather than address by symbol-path. @status:spec/done

- @fact:MARKER-ON-ITEM **Where the marker lives — on the item, never external.** The address is carried by whatever metadata construct is *idiomatic and scannable* in each language, attached to the item itself, because an address that does not travel with the artifact reintroduces exactly the fragility [PROP-014 §5.1](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#rejected) rejected (a sidecar map rots on every refactor). @status:spec/done
- @fact:CARRIER-PER-LANGUAGE The neutral grammar is the address; the carrier is per-language (PROP-014 §2.9): a **structured attribute** where the language has one (Rust `#[spec(addr = "…")]`, Java/Kotlin annotation, Python decorator), a **structured doc-comment tag** where it does not (TypeScript/JS `/** @addr code://… */`, matching the `@spec` JSDoc choice), and a **comment-directive** for languages with neither (Go `//spec:addr …`, cf. `//go:generate`). @status:spec/done
- @fact:CARRIER-CONSTRAINTS Three constraints bind every carrier: @status:spec/done
  - @fact:CC-ON-ITEM **on the item** — travels on refactor; @status:spec/done
  - @fact:CC-SCANNABLE **scannable without execution** — read as AST/text, as `#[spec]` is today; @status:spec/done
  - @fact:CC-STRUCTURED **structured, not free prose** — a defined grammar the scanner parses, never a human sentence. @status:spec/done
- @fact:ADDR-FACET-RECOMMENDATION **Recommendation:** make `addr` a **facet of the existing `specmark` marker**, not a new construct — one code-marker family then carries both a node's *identity* (`addr`) and its *outgoing edges* (the verbs), the whole code-side of the graph in one place. @status:spec/done

@fact:ASYMMETRY-REMOVED This single change **removes the asymmetry of §1**: code can now be a *target*, so spec→code, doc→code, and code→code edges become expressible — and the graph is symmetric. @status:spec/done

### 2.4 Edges are typed by authority direction {#edges}

@fact:EDGE-AUTHORITY-LAW `prop r1` — An edge's **verb carries which end is the source of truth**, which is what makes the graph queryable and honest. Generalising [PROP-014 §2.4](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#edges) from five code→spec verbs to a directional set: @status:spec/done

| From → To | Verb(s) | Meaning |
|---|---|---|
| @fact:EDGE-CODE-SPEC code → spec @status:spec/done | `implements` `verifies` `documents` `deviates` `informs` @status:spec/done | exist — code defers to the spec as authority @status:spec/done |
| @fact:EDGE-SPEC-SPEC spec → spec @status:spec/done | `references` `refines` `supersedes` @status:spec/done | a prose citation as a tracked edge (the D3 gap, PROP-031 §3.3) @status:spec/done |
| @fact:EDGE-SPEC-CODE **spec → code** @status:spec/done | **`defined-by` / `canonical`** @status:spec/done | **the normative description of this concept is the code at `code://…`** — the authority *inversion* @status:spec/done |
| @fact:EDGE-DOC-CODE doc → code @status:spec/done | `exemplifies` @status:spec/done | the canonical example / reference usage is here @status:spec/done |
| @fact:EDGE-CODE-CODE code → code @status:spec/done | `uses` `see-also` @status:spec/done | a cross-item reference (rustdoc intra-doc links, generalised across packages) @status:spec/done |

- @fact:INVERSION-RARE `canonical` / `defined-by` is a **marked, deliberate, rare inversion** — the peer of `deviates`. It says "this concept's normative content genuinely *is* the code; do not restate it in prose." @status:spec/done
- @fact:NO-SHADOW-CODE It is **not** a licence for shadow-code: [PROP-014 §3.1.6](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) ("spec states *what* and *why*, never *how*; a spec that mirrors code is drift fuel") still governs the default. @status:spec/done
- @fact:INVERSION-MARKED The inversion is the exception you *mark*, exactly as a deviation is. @status:spec/done

### 2.5 Three operations over one model: query, mutate, render {#operations}

@fact:THREE-OPERATIONS `prop r1` — The substrate exposes exactly three operation families over the graph, and "the IDE" is nothing more than these three: @status:spec/done

- @fact:OP-QUERY **Query — navigation.** *Go-to-definition* = follow an edge to its target; *find-references* = the reverse edges into a node; *impact* = the transitive closure. specmap already ships these as `explain` / coverage / impact ([PROP-014 §2.6](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#queries)); symmetry (§2.3) is what makes them work in *both* directions. @status:spec/done
- @fact:OP-MUTATE **Mutate — refactoring.** The typed, atomic, gated operations of PROP-031 (`rename-address`, `move-unit`, `rename-symbol`, …), now spanning `spec://` **and** `code://` addresses. @status:spec/done
- @fact:OP-RENDER **Render — explanation.** The deterministic subgraph plus its optional prose rendering (PROP-014 §2.6); the data layer is always available without an LLM. @status:spec/done

### 2.6 Agent-first: the primary client emits typed commands; surfaces are progressive {#agent-first}

- @fact:AGENT-PRIMARY `req r1` — The **primary consumer of the substrate is an agent**, not a human at a keyboard. @status:spec/done
- @fact:TYPED-COMMANDS An agent drives navigation and refactoring by **emitting typed query/mutation commands** — the LLM boundary of [PROP-031 §2.2](spec://org.vibevm.core/vibevm/common/PROP-031#llm-boundary) and PROP-014 §2.7: *the model proposes a typed command; the deterministic engine executes and gates it.* @status:spec/done
- @fact:TRANSPORT-MCP The transport is MCP ([PROP-014 §2.8](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#runtime), already shipping `specmap_query` / `specmap_explain`). @status:spec/done

- @fact:IDE-HEADLESS The consequence reorders the usual notion of "IDE": **the IDE is a headless model-plus-operations server; the GUI is the last, optional client, not the IDE itself.** @status:spec/done
- @fact:SURFACES-PROGRESSIVE Surfaces are progressive projections of the *same* command set: @status:spec/done

```
library API  →  command line  →  MCP / agent  →  (last, optional) graphical view
   (exists)       (exists)        (embryo §2.8)        (future, human-facing)
```

- @fact:GUI-FIRST-INVERTS Building GUI-first would invert the dependency — a graphical shell over an incomplete, asymmetric model. @status:spec/done
- @fact:MODEL-IS-PRODUCT The model and the operations are the product; every surface is a client. @status:spec/done

### 2.7 Integrity and refactoring fall out of the model, not bespoke code {#free}

@fact:FREE-PROPERTIES `req r1` — Because everything is one graph with minted addresses, the hard properties are **free**: @status:spec/done

- @fact:FREE-DANGLING a link to an address that resolves to no node is a **dangling** edge (the existing gate); @status:spec/done
- @fact:FREE-DUPLICATE a duplicate minted id is a **duplicate-address** warning (the existing `duplicate-anchor` machinery); @status:spec/done
- @fact:FREE-RENAME a rename is a **`rename-address`** operation (PROP-031); @status:spec/done
- @fact:FREE-SUSPECT a stale pin is a **suspect** (PROP-014 §2.2). @status:spec/done

@fact:SYMMETRY-CHEAP This is the reason symmetry is *cheap once the node model generalises*: no new subsystem, only more node kinds and edge directions in the graph that already computes all of this. @status:spec/done

### 2.8 The substrate is a discipline-neutral, independently-installable tier {#packaging}

@fact:SUBSTRATE-NEUTRAL `req r1` — The substrate (specmark + specmap-core + the refactoring operations and their registry, [PROP-033](spec://org.vibevm.core/vibevm/common/PROP-033#root)) is packaged **independently of the ai-native discipline** and delivered as its own installable tier, so vibevm serves a spectrum of users through a **three-tier product model**: @status:spec/done

1. @fact:TIER-BASE **Base vibevm** — the package manager itself (resolve / install / lockfile / boot; working with `vibe.toml` projects; loading spec collections). No traceability, no refactoring, no discipline. The "just load a collection of specs" user lives here. @status:spec/done
2. @fact:TIER-SDD **+ the SDD substrate** (a package under `org.vibevm.world`, not `ai-native`) — installs specmark + specmap + the refactoring registry: the `spec://` / `code://` model, integrity checking, navigation, and the algorithmic refactoring core. Proper spec-driven development, **without** the strict discipline. @status:spec/done
3. @fact:TIER-AI-NATIVE **+ the ai-native discipline** (`rust-ai-native`, …) — the strict opt-in: conform, cards, cells, the nine scaffolds. It **depends on** tier 2, contributing its own refactorings to the registry; it never owns the substrate. @status:spec/done

- @fact:DEP-INVERSION The dependency runs `ai-native → SDD substrate → base vibevm` — a **dependency inversion** from today, where `rust-ai-native` owns specmap. @status:spec/done
- @fact:LEGACY-TIERS A legacy tree that cannot adopt the discipline still gets tiers 1–2. @status:spec/done
- @fact:REOPENS-RELOCATION This re-opens what the Traceability-Relocation plan §1 deferred, for the stronger reason of *product surface* (not cross-language DRY). @status:spec/done
- @fact:TIER-CENTER The **center of each tier is its Rust library + its spec**, so agents work with it directly (§2.6); the CLI (`vibe refactor …`) and MCP are thin surfaces, never the center. @status:spec/done

## 3. Layering — what this owns, versus PROP-014 and PROP-031 {#layering}

@fact:layering-lead `prop r1` — To avoid duplication (the one real overlap risk), the boundary is explicit: @status:spec/done

- @fact:OWNS-032 **PROP-032 (this) owns the *model shape and the vision*:** the universal typed graph (§2.1), the addressing invariant (§2.2), the code node (§2.3), the directional edge set (§2.4), the three operation families (§2.5), and the agent-first substrate (§2.6). It specifies *what the model is*, not *how edges are extracted or gated*. @status:spec/done
- @fact:OWNS-014 **PROP-014 owns the *traceability instantiation and mechanics*:** the `#[spec]`/`scope!` grammar, extraction (`rscan`/`mdspec`), revisions/suspects, the committed `specmap.json`, and the gate. It is the **code↔spec projection** of this model — the first and canonical one — and it **grows** (per this PROP) a `code://` node kind and the spec→spec / spec→code directions. It is not superseded; it is generalised (§7 open question 5). @status:spec/done
- @fact:OWNS-031 **PROP-031 owns the *mutations*:** the typed refactoring operations over the model, gated by re-checking it. @status:spec/done

- @fact:NO-OWN-MECHANICS PROP-032 introduces **no extraction or gate mechanics of its own**; it names the model those mechanics populate and the extensions they must grow to cover. @status:spec/done
- @fact:ENGINE-HOME The engine still lives in `core-ai-native`; the host PROPs (031, 032) drive it, exactly as PROP-031 already does. @status:spec/done

## 4. Build-in-anticipation {#anticipation}

@fact:ANTICIPATION-EXTENDS `req r1` — Extends PROP-031 §3 with the model-level disciplines, in force from ratification: @status:spec/done

1. @fact:ANT-ADDRESS-ALL-KINDS **Address every node kind; never location-address.** Generalises PROP-031 §3.1 to code and packages: the address of record is always the minted id/anchor/FQID, never a line or a symbol-path. Author nothing that can only be reached by location. @status:spec/done
2. @fact:ANT-MINT-DELIBERATE **Mint `code://` addresses sparingly and deliberately** — only where code is the canonical description (§2.3). Over-minting is noise; under-minting leaves authoritative code unpointable. @status:spec/done
3. @fact:ANT-EDGE-V3-ONCE **Generalise `Edge` from `code→spec` to `node→node` deliberately** (a schema v3 step, §7), not piecemeal — the byte-stable `specmap.json` must migrate once, cleanly. @status:spec/done
4. @fact:ANT-MODEL-NOT-GUI **Build the model and the operations, not the GUI.** The substrate is complete when navigation + refactoring run agent-first over a symmetric, gated graph. A graphical surface is a later, separate, human-facing decision. @status:spec/done

## 5. Rejected alternatives {#rejected}

1. @fact:REJ-LOCATION-ADDRESS **Address code by location (line or symbol-path).** Fragile — exactly what refactoring changes (PROP-014 §5.2). Retained only as *derived* index decoration, never the address. @status:spec/done
2. @fact:REJ-BESPOKE-LINK **A bespoke "spec→code link type."** Special-cases what should be symmetric. The universal move is a code *node* (§2.3); then spec→code is an edge like any other, and doc→code / code→code come for free. A one-off link type would need a second one for every new direction. @status:spec/done
3. @fact:REJ-GUI-FIRST **A GUI-first IDE.** Inverts the dependency — a shell over an incomplete model. The model + operations are the IDE; the GUI is the last client (§2.6). @status:spec/done
4. @fact:REJ-SEPARATE-NAV **A separate code-navigation tool (a standalone SCIP/LSIF server) beside specmap.** Two graphs, two truths, two things to keep in sync. The code node lives in the *same* graph, so navigation and traceability compose (find every REQ *and* every doc that points at a function, in one query). @status:spec/done
5. @fact:REJ-COMPUTED-INVERSE **Keep the graph `code→spec` only, with a computed inverse.** The inverse answers "what implements this REQ" but cannot represent an *authored* spec→code or spec→spec edge, cannot gate a prose citation, and cannot be refactored — the whole point of §1. @status:spec/done

## 6. Prior art & license posture {#prior-art}

@fact:prior-art-lead Conventions and ideas are free; code is not (PROP-000 §3). License fields re-verified before any code-level reuse. @status:spec/done

| System | License (verify) | Role here |
|---|---|---|
| @fact:PA-LSP **LSP** (Language Server Protocol) @status:spec/done | n/a (protocol) @status:spec/done | The architecture: one model, a query surface, a mutation surface, many clients. This PROP is "LSP for spec + code + packages, agent-first." @status:spec/done |
| @fact:PA-SCIP **SCIP / LSIF** (Sourcegraph) @status:spec/done | Apache-2.0 @status:spec/done | **The code-node prior art.** A serialised, committed graph of code symbols + references for navigation without a live server — precisely `specmap.json` for code. They solved the stable-symbol *moniker* problem (§7). @status:spec/done |
| @fact:PA-RUSTDOC rustdoc intra-doc links @status:spec/done | n/a (rustc) @status:spec/done | Path-based, compiler-checked in-code references — the `code→code`, integrity-checked precedent. @status:spec/done |
| @fact:PA-SPHINX Sphinx domains + `:ref:` @status:spec/done | BSD-2 @status:spec/done | Prose→object references resolved and checked at build — the `doc→code` precedent. @status:spec/done |
| @fact:PA-OPENREWRITE OpenRewrite (via PROP-031) @status:spec/done | Apache-2.0 @status:spec/done | Typed, gated operations over a lossless model — the mutation half. @status:spec/done |

@fact:DIFFERENTIATORS **Differentiators.** @status:spec/done

1. @fact:DIFF-ONE-GRAPH spec, code, and packages live in **one** graph, so traceability and navigation compose; @status:spec/done
2. @fact:DIFF-AUTHORITY-TYPED edges are **typed by authority direction**, so the graph states who is the source of truth; @status:spec/done
3. @fact:DIFF-AGENT-CLIENT the primary client is an **agent** emitting typed commands, with GUI as the last surface, not the first; @status:spec/done
4. @fact:DIFF-GATED every relation is **gated** by the discipline's own invariant, not merely "compiles." @status:spec/done

## 7. Open questions {#open}

<status stage="spec" state="work" comment="B1 2026-07-24: five questions open (q5 decided in place 2026-07-13); ratification pending"/>

1. @fact:open-code-id-scheme **The `code://` id scheme.** A free-minted slug (`resolver-fixpoint`) maximises rename-stability but adds a namespace to manage; a **structured moniker** (SCIP-style: package + descriptor path) needs no minting but moves under refactor. Lean: **free-minted for authoritative nodes** (stability is the point), structured monikers as the *derived* address for everything else. @status:spec/work
2. @fact:open-node-kinds **How many node kinds.** Packages (FQID) and boot entries (INDEX id) are the obvious next nodes (the product tier of PROP-031). Config? Manifests? Grow by demonstrated need, not speculation. @status:spec/work
3. @fact:open-schema-v3 **Schema v3 migration.** Generalising `Edge` from `code→spec` (`from_symbol` + `uri`) to `node→node` (two typed addresses) is a `specmap.json` schema change; plan the byte-stable migration (the `check-codegen` idiom, PROP-014 §2.5). @status:spec/work
4. @fact:open-reverse-edges **Reverse-edge storage vs computation.** Find-references can be computed by inverting the edge set (as today) or materialised; decide when the graph grows enough that inversion cost matters. @status:spec/work
5. @fact:decided-prop-014-grows **PROP-014's identity. Decided (owner, 2026-07-13): it grows *in place*.** PROP-014 keeps its title ("specmap: bidirectional traceability") and gains the `code://` node kind + the new edge directions (spec→spec, spec→code); PROP-032 references it as the canonical first projection, not a replacement. No re-scope, no rename — the extension lands as new sections in PROP-014 and new node/edge kinds in its engine. @status:spec/done
6. @fact:open-gui-when **When (and whether) a GUI, and by whom.** Explicitly deferred (§2.6, §4.4). The substrate must be complete and agent-first first; a graphical client is a separate, later, human-facing decision. @status:spec/work

---

- @fact:ratification-note *This PROP is a design proposal. Ratification happens through PR review against this document, PROP-014, and PROP-031.* @status:spec/done
- @fact:first-step *It commits to no implementation of its own; its first concrete step is the `code://` node (§2.3), sequenced by the SPECMAP Unit-Mobility Plan and PROP-031's operation roadmap.* @status:spec/done
- @fact:unexercised-removed *Any mechanism specified here that is not exercised by the second shipped node kind or edge direction is removed from the spec rather than carried as aspirational documentation (the PROP-014 §335 discipline, inherited).* @status:spec/done
