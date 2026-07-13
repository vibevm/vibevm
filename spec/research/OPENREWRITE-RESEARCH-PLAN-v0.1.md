# OpenRewrite Research Plan v0.1 — a clean-room study feeding our refactoring engine

**status: PLANNED · not started · runs COLD in its own dedicated session · feeds the spec redesign, then the implementation**

> **Read-first / boot.** This plan is executed **cold, in a fresh session the owner launches for it**. Boot the normal way (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md` → `CONTINUE.md`), then read this whole file. It is self-contained: the strategic frame, the **clean-room firewall (non-negotiable)**, what to acquire, the question-driven study agenda, the deliverables, the phases, and the risks are all here.
>
> **Its output is a findings document written in OUR words** — never OpenRewrite's code — that feeds a *separate* spec-redesign session, which feeds a *separate* implementation session. The studied sources never cross those boundaries (§1).

---

## 0. Why this exists — the strategic thesis {#why}

**Refactoring is the single largest and most expensive activity in AI-assisted development.** An agent asked to rename an address, move a unit, or migrate an idiom spends enormous effort and tokens walking files, and every missed site is a silent defect. That cost is `O(files)` and it recurs on every change. The whole delegation-first thesis of this repository (`CLAUDE.md`) says the opposite is possible: a mechanical transform should be a **deterministic tool call**, `O(decision)` — below even the cheap-model tier. Our design already names this (PROP-031 the codemod engine, PROP-032 the model, PROP-033 the registry). **Making refactoring algorithmic is therefore the highest-leverage investment available to this project** — it converts the most expensive AI work into the cheapest.

**OpenRewrite is the most mature proof that this works** — typed, composable, *gated* refactoring "recipes" over a lossless, type-attributed tree, run as framework migrations across thousands of repositories. Before we finalise our design or write a line of engine code, we study OpenRewrite and its neighbours **rigorously and clean-room**, distil the lessons in our own words, and let that distillation drive the spec redesign.

Two guard-rails keep the "insanely large" scope tractable:

1. **Iterative, essential-first.** We do not build OpenRewrite. We build the *smallest useful slice* first (§7), prove it, and grow. The research's job is partly to tell us what the essential slice *is*.
2. **Clean-room (§1).** We steal not one line. We read to understand; we write from scratch or on our own permissively-licensed dependencies.

## 1. Clean-room discipline — GATING, non-negotiable {#clean-room}

This section governs the entire campaign. Violating it is worse than not doing the research at all.

**Posture.** OpenRewrite is **Apache-2.0** (permissive — *verify at acquire time*), and its permissive licence would technically allow dependency use. **We do not rely on that.** OpenRewrite (and every studied project) is treated as **inspiration-only, never a code source** — the exact posture `spec/boot/90-user.md` sets for `eth-sri/type-constrained-code-generation`. The method is: **READ to understand the approach; then write STRUCTURALLY DIFFERENT code, from scratch or on our own permissive deps.** No copying, no line-by-line porting, no adaptation of their expression. Identical *behaviour* is fine; borrowed *expression* is not. Rule 1 (the human-authored surface) still governs; nothing is attributed to any tool.

**A clean-room advantage we have by construction:** OpenRewrite is Java/JVM; our engine is Rust. You *cannot* paste Java into Rust — the language boundary forces re-expression. Study the **concepts** (the LST, the recipe model, the visitor pattern, data tables); implement them idiomatically in Rust. "On existing libraries" means we may *depend on* permissive Rust libraries (rust-analyzer crates, `tree-sitter`, `ast-grep`, `syn`) and *wrap* them (PROP-031 §2.5) — never OpenRewrite itself.

**The firewall — three separated sessions (the owner's own insight, formalised).** The sources and the product code must never share a context:

- **(a) STUDY session** — *this campaign.* Reads the sources in `refs/`, produces the **findings document** (§4) in our words. Reads their code; writes **no** product code.
- **(b) REDESIGN session** — revises PROP-031 / 032 / 033 from the **findings document only**. Does **not** open the studied sources.
- **(c) IMPLEMENT session(s)** — build from the redesigned specs. Do **not** open the studied sources.

The findings document is the *only* interface across the firewall. `refs/` is already the repository's read-only reference zone (off-limits for extraction, per the memory-discipline and the big-refactoring scope rules); the studied sources land there and stay there.

## 2. What to acquire — into `refs/src/` {#acquire}

Clone (shallow is fine) each project into `refs/src/<name>/`, capture its `LICENSE`, verify the licence is permissive, and record it in the findings doc's provenance table **regardless** (the clean-room posture binds whatever the licence says).

| Project | Why study it | Land at |
|---|---|---|
| **OpenRewrite** — `rewrite` (core), `rewrite-java`, `rewrite-docs`, a recipe module | **Primary.** The LST, recipes, visitors, composition, data tables, recipe testing, execution model | `refs/src/openrewrite/` |
| **ast-grep** | Lightweight structural search/replace without a full LST — the "cheap tier" of transforms; the pattern language | `refs/src/ast-grep/` |
| **comby** | Structural rewriting across many languages; the template model | `refs/src/comby/` |
| **tree-sitter** | Incremental parsing; the concrete-syntax substrate several tools share | `refs/src/tree-sitter/` |
| **SCIP / LSIF** (Sourcegraph) | A serialised code graph for navigation — the `code://` node prior art (§PROP-032); stable symbol monikers | `refs/src/scip/` |
| **rust-analyzer** | How a real language server exposes rename / assists / SSR over a type-attributed tree — what we *borrow* instead of rebuilding an LST | `refs/src/rust-analyzer/` (subset) |
| **ts-morph / jscodeshift** | The TypeScript AST-refactoring model (for the TS stack) | `refs/src/ts-morph/` |
| **LSP specification** | The client/server, code-action, rename, executeCommand model — our agent-first surface prior art | `refs/src/lsp-spec/` |
| **OpenFastTrace / Doorstop** | Requirements↔code traceability (ideas only; both are copyleft — study-only, never a dep) | `refs/src/traceability/` |

## 3. Study questions — grounded in OUR design gaps {#questions}

The study is **question-driven**, not aimless reading. The questions are our preliminary gap-read (from designing PROP-031/032/033), to be confirmed and expanded:

1. **The tree / model layer (the deepest question).** How does OpenRewrite build and maintain the **LST** — lossless (round-trips to byte-identical source) and **type-attributed**? What does type attribution cost? **Our hypothesis: we do NOT rebuild an LST per language — we borrow the language server's own tree** (rust-analyzer / TS LSP) for deep code work, and keep our own **coarse project graph** (specmap: spec + code nodes, addresses, edges, cross-language, cross-package). Confirm the **two-layer model** (coarse project graph ⊕ rich per-language borrowed tree) and define **the minimal tree the first refactorings actually need**.
2. **Recipes + composition.** Declarative (YAML: compose + configure sub-recipes) vs imperative (a Visitor). How small recipes compose into large migrations. **Our gap: we have atomic operations but no composition model** — design it, because the cultural-pattern extraction *is* a large migration.
3. **Visitors.** The traversal-and-transform pattern; how a recipe expresses "match X, rewrite to Y."
4. **Search recipes + data tables.** OpenRewrite recipes can **emit structured findings**, not only transform. **Our gap: query and mutate are split (PROP-032 §2.5); the find→fix pipeline is missing.** Study how they unify — and note this is exactly the bridge from **conform (find violations) → refactoring (fix them)** in our stack.
5. **Preconditions / applicability.** Running a recipe only where a structural pattern matches (richer than PROP-033 `applies_to`).
6. **Catalog, discovery, distribution.** OpenRewrite's recipe catalog vs **our package-delivered registry (PROP-033)**. What their marketplace gets right; how our `[[refactoring]]` + install-time composition compares.
7. **Recipe testing.** Their before/after fixture framework. **Our gap: no story yet for testing that a refactoring is correct** — design "refactoring goldens."
8. **Markers, provenance, dry-run/diff, multi-module scale.** How they annotate the tree, track where an edit came from, preview diffs, and run across a whole build.
9. **Adjacent lessons.** ast-grep/comby (lightweight structural transforms — the tier below a full LST); tree-sitter usage; SCIP (stable code monikers for our `code://`); LSP rename/code-actions (our agent-first surface); traceability tools (spec↔code, ideas only).

## 4. Deliverables — in OUR words, clean-room {#deliverables}

1. **The `refs/src/` corpus** + a provenance/licence table.
2. **The findings document** — `spec/research/openrewrite-findings.md` — the architecture and behaviour of OpenRewrite + kin **in our own words**, decision-relevant, structured by the §3 questions. This is the clean-room interface; it contains no copied code.
3. **The gap-map** — our design (PROP-031/032/033) vs OpenRewrite + kin, each item marked **match / beat / skip**, with rationale.
4. **The minimal-first recommendation** — the smallest useful engine slice and the growth path (§7 made concrete).

## 5. Where we aim to be *better* than OpenRewrite {#beat}

The study keeps these north-star differentiators in view (the owner's "not worse, even better"):

- **A spec ↔ code ↔ package graph** — OpenRewrite is code-only; ours spans the whole project model (PROP-032).
- **Agent-first** — typed query/mutation commands over MCP (PROP-032 §2.6); OpenRewrite is not agent-driven.
- **Three implementation kinds** — algorithmic / LLM / hybrid (PROP-033 §2.4); OpenRewrite has no LLM tier.
- **Package-delivered, discoverable recipes** — install a package, gain its refactorings (PROP-033); vs Maven artifacts + a central marketplace.
- **Borrow the language's real compiler tree** (rust-analyzer) instead of reconstructing an LST per language — more accurate, and it sidesteps OpenRewrite's largest engineering burden.
- **The three-tier product model** — base / SDD substrate / discipline (PROP-032 §2.8), so the engine serves everyone from a spec-collection user to a strict-discipline shop.

## 6. Phases {#phases}

- **Phase 0 — Acquire.** Clone the §2 corpus into `refs/src/`, capture licences, verify permissive, seed the findings-doc provenance table.
- **Phase 1 — OpenRewrite deep study.** The core: LST, recipe model, visitors, composition, data tables, recipe testing, execution. Distil into the findings doc, question by question (§3.1–3.8).
- **Phase 2 — Adjacent projects.** ast-grep, comby, tree-sitter, SCIP, rust-analyzer, LSP, ts-morph, traceability (§3.9). Distil.
- **Phase 3 — Synthesis.** The gap-map (§4.3) + the beat-OpenRewrite assessment (§5) + the minimal-first recommendation (§4.4, §7).
- **Phase 4 — Hand back.** The findings doc is complete and self-contained; it feeds the redesign session (firewall (b)). The study session writes no product code and touches no PROP.

## 7. The iterative build framing — essential-first {#iterative}

The research feeds the redesign, which feeds implementation; the **detailed build plan is authored after the research**. But the essential-first shape is already visible, and the research should confirm or correct it:

- **Slice 1 (essential, max leverage):** the two-layer model wired minimally + **`rename-address`** as the first gated recipe — rename a `spec://` (later `code://`) address and retarget every citer, atomic + `specmap --check`-gated. This converts the commonest expensive multi-file rename into a tool call.
- **Slice 2:** **`move-unit`** = `rename-address` ∘ relocate-text ∘ upkeep (the SPECMAP Unit-Mobility plan's operation).
- **Slice 3:** **composition** (recipes compose into migrations) — unlocks the cultural-pattern extraction at scale.
- **Slice 4:** **search recipes + the find→fix pipeline** (conform → refactoring).
- **Slice 5+:** more kinds (LLM/hybrid), more languages, preconditions, recipe-goldens as a standing gate.

Each slice is gated, shippable, and grows the PROP-033 registry by one or more `[[refactoring]]` entries.

## 8. Risks & fallbacks {#risks}

- **Licence contamination.** Mitigated by the clean-room firewall (§1), `refs/` isolation, the three-session separation, and never-copy. The Java→Rust language boundary is an additional structural firewall.
- **Scope explosion.** Mitigated by the question-driven agenda (§3), essential-first slicing (§7), and time-boxed phases (§6). The research answers "what is essential," not "how do we build all of OpenRewrite."
- **Cross-session contamination.** The redesign and implementation sessions work from the findings doc, never the sources (§1). If a redesign session finds the findings doc insufficient, it files a follow-up question for a *study* session — it does not open `refs/src/` itself.
- **The provisional design churns.** PROP-031 / 032 / 033 are **provisional input** to the redesign; expect them to change materially after the research. That is intended — do not treat them as frozen.

## 9. Quick-start (for the research session) {#quick-start}

```sh
# boot, then acquire (Phase 0) — shallow clones into the reference zone:
mkdir -p refs/src
git clone --depth 1 <openrewrite-core-url>  refs/src/openrewrite/rewrite
git clone --depth 1 <openrewrite-java-url>  refs/src/openrewrite/rewrite-java
# … the rest of the §2 table …
# capture each LICENSE into the findings provenance table; verify permissive.

# then Phase 1: open refs/src/openrewrite, study by the §3 questions, write
# spec/research/openrewrite-findings.md in OUR words. Write NO product code.
```

---

*This is a research plan, not a build plan. It commits to no implementation. Its product is the findings document, which drives a separate spec-redesign session; the redesigned specs drive separate implementation sessions. The clean-room firewall (§1) is the campaign's first law: study the sources, write none of their code.*
