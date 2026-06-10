# The Discipline — Charter v0.1

**Status.** Founding draft — the seed from which the full rulebook grows. Working name "The Discipline" is a placeholder; naming is Open Question 1.

**What this is.** The constitution of a discipline for AI-first software development: how specifications, code, and the knowledge connecting them are written, transformed, cached, and verified — across Rust, TypeScript/JavaScript, Python, and C++ (current standard). vibevm is the first carrier and reference transformation (legacy → disciplined), not the scope boundary.

**What this is not.** Not a style guide (formatting is tooling's job), not a pattern encyclopedia (patterns are vocabulary, not targets), not advice (rules here are enforced or explicitly counted as wishes).

---

## 0. Axioms {#axioms}

Everything below derives from five axioms. A rule that cannot trace its "why" to an axiom does not enter the rulebook.

- **A1 — Explainability invariant.** No artifact ships without a machine-resolvable explanation chain: `binary → symbol → item → cell → REQ → rationale`. Unexplainable is unmergeable. *(Source: Priority 1.)*
- **A2 — Never pay twice for the same understanding.** Every derived cognition — link, intent, explanation, classification — is materialized in content-addressed, dependency-tracked form. LLM cost scales with the novelty of the diff, never with the size of the repository. *(Source: Priority 2.)*
- **A3 — The algorithmic floor.** Where a deterministic decision procedure exists, the LLM is forbidden. The LLM works only above the floor (design, judgment, naming the why) — and each of its outputs is materialized per A2, sinking below the floor for all future occasions. A2+A3 compose into the **ratchet of understanding**: expensive cognition becomes cheap infrastructure; the floor rises with every turn. *(Source: Priority 3.)*
- **A4 — The human is the accountability point.** Diffs remain reviewable by a human; deviations from rules are formal records, not silent exceptions. AI participates as checker and proposer, never as the accountable author. *(Industry precedent: the Linux kernel's 2026 acceptance condition for AI — review, not authorship; submitter accountable.)*
- **A5 — A rule is code.** Every rule ships with its checker (lint, structural test, gate) or carries the explicit status `WISH`. The health of the rulebook is measured by its **wish-ratio**, not its page count. Thick rulebooks are fine — MISRA/AUTOSAR-thick — *because* their bulk is machine-held.
- **A6 — Reality before aspiration (brownfield axiom; amendment v0.1.1).** Gates measure deltas against *inventoried reality*, never against an imagined healthy state. Debt, unimplemented intent, and contradiction are first-class tracked objects with dispositions and tripwires — legal when labeled, fatal when ambient. *(Machinery: `BROWNFIELD-PROTOCOL-v0.1.md`.)*

**Headline metric.** LLM cost per merged change must **decrease** as the codebase grows. This is the inversion of the industry curve and the proof that the terraforming loop (A2+A3) is alive.

---

## 1. Architecture of the discipline {#architecture}

Four tiers. Prose shrinks and machine-checkability grows as you descend.

| Tier | Content | Form | Size budget |
|---|---|---|---|
| **T0** | The five axioms | prose, frozen | 1 page |
| **T1** | Universal rules (language-neutral): cells, seams, flags, errors-as-contract, intent ledger, naming grammar, import pipelines | rule records (§2 schema) | ~30–60 rules |
| **T2** | Language guides: Rust, TypeScript/JavaScript, Python, C++ — derivations of T1 + language-specific rules + pattern translation tables | rule records + idiom tables | thick; bounded by wish-ratio, not count |
| **T3** | The machine layer: checkers, templates, codemods, registries | code | unbounded |

**Distribution.** The rulebook ships as vibevm packages (`flow:discipline-core`, `stack:rust-discipline`, …). Rule clusters carry **lazy-push activation descriptions**, so an agent pages in only the rules relevant to the task at hand. The thickness problem is solved by the tool the rulebook governs — a deliberate self-referential closure satisfying A2.

**Self-application.** Every rule is a spec unit (anchored, `r`-revisioned, content-hashed) in the discipline's own specmap. Checkers carry `verifies` edges to their rules. Deviations use the `deviates` verb with mandatory reason — the same record MISRA calls a formal deviation.

---

## 2. The rule record {#rule-schema}

```
id:        R-NNN (T1) / <lang>-NNN (T2)
level:     MUST | SHOULD | MAY
statement: one sentence, RFC-2119 voice
why:       → axiom(s), one short paragraph
check:     { tool, check-id, status: enforced | partial | WISH }
deviation: allowed? required record fields
examples:  minimal good / minimal bad
bindings:  per-language realisations (T1 rules only)
sunset:    evidence that would retire the rule
```

A rule without `check.status = enforced` is technical debt with a name. A rule without `sunset` is dogma.

### 2.1 Genre samples (normative once ratified) {#samples}

**R-001 · MUST · Flag at the seam, never in the veins.**
A runtime flag is read exactly once, at the composition root, to select a cell. `if flag` inside domain logic is forbidden.
*Why:* A1 (selection is explainable as data), A3 (flag removal becomes a one-line algorithmic diff).
*Check:* structural lint — flag-read calls permitted only in registry modules. `enforced`.
*Bad:* `if cfg.flags.new_solver { … } else { … }` inside `solve()`. *Good:* registry selects `SatDepSolver` vs `NaiveDepSolver`; both implement `DepSolver`.

**R-002 · MUST · One cell, one registration point.**
Every cell (feature unit behind a trait seam) is registered in exactly one registry location. Cells never import sibling cells; only seams and core.
*Why:* A1 (the registry is the map of the system), review locality (A4).
*Check:* import-graph lint over the workspace. `enforced`.

**R-010 · MUST · Errors are contract surface.**
Public error variants that signal a requirement carry the requirement's edge; user-facing failures cite the violated REQ URI.
*Why:* A1 — every failure is a doorway into the metamodel.
*Check:* specmap coverage query over error enums. `enforced`.

**R-020 · MUST · Names are derived, not composed.**
Meaning-dense identifiers are encouraged and may be machine-length, but a name asserting structure (Bridge, Factory, Strategy, …) must be derivable from — or lint-consistent with — the cell's declared manifest. A name is a serialization of declared structure, never an unverified claim.
*Why:* A1 (an unverifiable structural claim is slop in a good hat), A2 (canonical names are computable, hence cacheable).
*Check:* naming-grammar lint against the cell manifest. `partial` until grammar v1 lands.

**R-021 · MUST · Verbose-explicit over terse-magical.**
Long names, explicit wiring, visible data flow: allowed and cheap. Hidden control flow, implicit effects, action-at-a-distance: forbidden regardless of elegance.
*Why:* A4 — review is the accountability point; you may not hide flow from the reviewer.
*Check:* per-language lint sets (e.g. forbidden reflection/proxy idioms in TS; `partial`).

**R-030 · MUST · Anchors in code, caches beside it.**
Authored truth (spec tags, cell identity) travels inside the source and survives refactors. Derived knowledge (intents, explanations, renders) lives in the content-addressed **intent ledger**, keyed by `(item hash, spec rev, query kind)`, and never pollutes diffs.
*Why:* A2; plus merge hygiene (A4).
*Check:* ledger-write lint (no generated prose committed into source files). `enforced` by construction once the ledger exists.

**R-040 · MUST · Replacements ship with a differential oracle.**
A new cell replacing an old one keeps the old cell alive behind the flag until a property-level equivalence (or documented-divergence) test holds for the agreed window.
*Why:* A1, A4; the cheapest oracle we will ever have is the previous implementation.
*Check:* CI requires a `#[verifies]`-tagged differential test for any cell marked `replaces = …`. `enforced`.

**R-050 · MUST · Rules live on evidence.**
Every rule carries a sunset clause; a periodic audit retires rules whose checker has not fired (or whose firing produced no accepted fix) within the evidence window.
*Why:* A5 — dead rules are wish-ratio in disguise.
*Check:* checker-telemetry report in the audit run. `partial` until telemetry lands.

**R-060 · SHOULD · Test matrices are declared, not exhaustive.**
CI tests the default flag set, each flag toggled individually, and explicitly declared interacting pairs — never 2^n.
*Why:* A3 (matrix generation is algorithmic), economics.
*Check:* CI config generated from the flag registry. `enforced` once flag registry exists.

---

## 3. Naming grammar (T1 sketch) {#naming}

- Identifiers are for the dominant reader (the model): length is free, ambiguity is not.
- Structural tokens (Cell, Seam, Strategy, Bridge, Factory, Oracle, …) come from a closed, versioned vocabulary; their composition order is canonical.
- The canonical name of a cell is **computed** from its manifest (kind, seam, capabilities, delivery); hand-written names are linted against the computation (R-020).
- Human review reads structure, not poetry: one glance at the registry tells the reviewer what exists; `vibe explain <name>` tells them why (A1).

---

## 4. Language guides (T2) — template and translation {#languages}

Each guide contains: (1) bindings for every T1 rule; (2) the language's risk table (footguns the checker set must cover); (3) the pattern translation table; (4) language-specific rules; (5) the T3 vehicle (which linter ecosystem hosts the checks).

Pattern translation — sample rows (full tables live in the guides):

| Intent | Rust | TypeScript | Python | C++ (current) |
|---|---|---|---|---|
| Swappable behaviour ("Strategy") | trait + impls | interface + DI token | Protocol + injected callable | concept/virtual seam + registry |
| Closed-set dispatch ("Visitor") | enum + match | discriminated union + switch | match on tagged union | `std::variant` + `std::visit` |
| Construction policy ("Factory") | registry fn + manifest | token-bound factory | registry callable | factory fn behind seam |
| One-instance ("Singleton") | forbidden; explicit wiring | forbidden; module scope is not DI | forbidden; pass it | forbidden; no magic statics |

Notes already fixed by this charter: Rust dissolves much of the classical catalog into language features — its guide is the thinnest. C++ is the stress case: the guide's primary instrument is **subset selection** (profiles in the Core-Guidelines sense), and its T3 vehicle is clang-tidy; modules-era build realities get an honest caveat section. TypeScript's guide owns the R-020/R-021 tension most directly (the ecosystem loves both verbose names and hidden magic).

---

## 5. Importing legacy code (T1 pipeline) {#legacy-code}

Six stages; every stage persists everything it learned (A2 — the first pass over legacy is the most expensive read the project will ever do).

1. **Inventory.** Scan → metamodel skeleton where *everything* is an orphan; mine latent links (commit messages, doc comments, issue refs).
2. **Characterize.** Golden/characterization tests pin current behaviour — bugs included — before any transformation. Legacy is code without tests; this is the oracle that makes agent-scale work safe.
3. **Seam-ify.** Introduce trait/interface seams at natural boundaries; no behaviour change.
4. **Cell-ify.** Extract features into cells behind flags; the old path stays as the differential oracle (R-040). Strangler-fig, never big-bang.
5. **Spec-ify.** Reverse specmap: propose spec units mined from code + history; human affirmation moves them from `proposed` to authored (Sync-from-Code at scale).
6. **Ratchet.** Conformance gates flip per-module/per-crate as each is touched; the repo is never globally gated on day one.

vibevm itself is the reference run of this pipeline.

## 6. Importing legacy specifications and skills (T1 pipeline) {#legacy-specs}

Prose has no code form; the move is **normalization**, not refactoring:

1. **Segment** arbitrary documents into candidate units.
2. **Classify** each unit: `req` / `design` / `guide` / noise (LLM-proposed, above the floor; the classification is cached per A2).
3. **Anchor** — propose stable IDs, normativity markers, `r1` revisions.
4. **Affirm** — a human accepts/edits; only affirmed units enter the metamodel. Proposals are never silently ground truth.
5. **Adapt** known dialects with dedicated importers. First-class case: agent skill files (`SKILL.md`-style name + activation description + body) map almost 1:1 onto subskill manifests with lazy-push delivery — the skills→packages adapter is a product feature, not an exercise.

Honesty clause: imported corpora will contain units that resist classification. They enter as `design` with a `low-confidence` mark rather than being forced into `req` — a wrong MUST is worse than an honest "unclear".

## 7. Metrics and governance {#metrics}

- **LLM-$/merged change** — must trend down as the repo grows (the headline; A2+A3 alive).
- **Wish-ratio** — rules without enforced checkers / total (A5; the only honest measure of rulebook bloat).
- **Cache hit rate** of the intent ledger; **suspect half-life** (how fast invalidated knowledge is re-affirmed); **slop-escape rate** (post-merge defects whose item lacked explanation chain — A1 leak detector).
- Rule lifecycle mirrors flag lifecycle: evidence windows, sunset audits (R-050), formal deviations.

## 8. Prior art and posture {#prior-art}

Ideas are free; code and texts are licensed. Reference roles: MISRA / AUTOSAR / CERT (thick-but-enforced rulebooks; formal deviation records — proprietary/standards texts, concepts only), C++ Core Guidelines + safety profiles (subset-selection as discipline), 12-factor (the genre of few checkable boundary assertions — our T0/T1 voice), Feathers' *Working Effectively with Legacy Code* (characterization, seams), Fowler's strangler fig, Salsa / rustc query system (incremental memoized computation — the intent ledger's execution model; MIT/Apache), Bazel/sccache (action caching; Apache-2.0), Nix (content-addressed derivations — ideas only; code LGPL), the 2026 Linux kernel AI-review settlement (checker-not-author; A4's precedent).

## 9. Open questions {#open}

1. **Name** of the discipline and of this document's eventual home (own repo, distinct from vibevm — the carrier is not the product).
2. **T3 engine fork:** per-ecosystem plugins (clippy/dylint, eslint, ruff, clang-tidy) vs one cross-language structural engine (tree-sitter-based). Leverage vs consistency; gates all of T3.
3. **Cell granularity** (crate vs module in Rust; package vs module elsewhere) — unresolved from the vibevm pilot; measured decision.
4. **Intent ledger format** — storage layout, eviction, and the semantic-staleness problem (hash-valid but contextually obsolete explanations): epochs keyed to dependency/lockfile changes, provenance labels on every render. Needs its own PROP.
5. **C++ subset depth** — how aggressive the profile is (exceptions? RTTI? coroutines?); decided by the C++ guide's pilot project, not by taste.
6. **Rule-retrieval grammar** — activation descriptions for rule clusters (the lazy-push surface): authored per cluster or derived from rule records?

---

*Ratification of T0 freezes the axioms. Everything else in this charter is molten by design and revised on pilot evidence. Any mechanism not exercised by the first two carrier milestones is removed rather than carried as aspiration.*
