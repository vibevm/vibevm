# Refactoring Engine — Meta-Plan v0.1 (the program map)

**status: the umbrella over the whole refactoring-engine program · design captured 2026-07-13 · NEXT = the OpenRewrite research campaign, run cold in a fresh session**

> **What this is.** A single at-a-glance map of everything planned in the refactoring-engine design arc — the vision, the documents, the decisions, the sequencing, the clean-room firewall — so no part is forgotten across sessions. It **indexes** the detailed documents; it does not repeat them. `spec/WAL.md` is the living state; this is the program map that state points at.

---

## 0. TL;DR

We are building an **agent-first, algorithmic refactoring engine** as a **discipline-neutral vibevm capability**, informed by a **clean-room study of OpenRewrite** (+ kin), built **iteratively, essential-first**. It is the highest-leverage investment available: it turns the most expensive work in AI-assisted development (refactoring) into cheap, deterministic, gated tool calls.

## 1. Why — the strategic thesis {#why}

Refactoring is the **largest and most expensive activity in AI-assisted development**: an agent rewriting files is `O(files)`, recurs on every change, and silently misses sites. Making it **algorithmic + gated** is maximum leverage on three axes:

- **Cost** — `O(files)` LLM file-walking → `O(decision)` tool call (below even the cheap-model tier — the delegation floor).
- **Reliability** — deterministic + gated is correct-by-construction; no silently-missed site.
- **Compounding** — every future change gets cheaper.

And it **unblocks the big cultural-pattern extraction refactoring** — itself a migration too large to do by hand or by LLM file-walking.

## 2. The document map {#docs}

| Document | Role | Home |
|---|---|---|
| **PROP-032** | **the model** — the project as a universal typed graph (spec + code nodes, typed directional edges); the agent-first IDE substrate; `code://` first-class node (§2.3); three-tier packaging (§2.8) | `spec/common/PROP-032` |
| **PROP-031** | **the mutations** — algorithmic refactoring / the codemod engine; the *write-side* of the model; typed commands the model emits, executed + gated; the three-tier stack; the operation algebra | `spec/common/PROP-031` |
| **PROP-033** | **the registry** — refactorings as package-declared capabilities (`[[refactoring]]`), discovered from the lockfile, precompiled + cached; three kinds (algorithmic/llm/hybrid) | `spec/common/PROP-033` |
| **PROP-014** *(grows in place)* | **the code↔spec traceability projection** of PROP-032's model; gains the `code://` node + the spec→spec / spec→code directions | `packages/org.vibevm.ai-native/core-ai-native/.../mechanisms/PROP-014` |
| **SPECMAP-UNIT-MOBILITY-PLAN** | the **first operation's** execution plan — move spec units across package boundaries with edges intact, gated | `spec/terraforms/SPECMAP-UNIT-MOBILITY-PLAN-v0.1` |
| **OPENREWRITE-RESEARCH-PLAN** | the **clean-room study** that precedes and informs everything (← the next action) | `spec/research/OPENREWRITE-RESEARCH-PLAN-v0.1` |

Conceptual layering: **PROP-032 (model) ⊃ PROP-014 (its code↔spec projection) + PROP-031 (its mutations) + PROP-033 (their packaging/discovery).**

## 3. The key decisions — so they are not lost {#decisions}

1. **The model is a symmetric typed graph** over addressable nodes (spec, code, package), edges in any direction typed by authority. specmap is the read-side; refactoring is the write-side of the *same* model (PROP-032 §2.1, PROP-031 §2.1).
2. **Agent-first.** The IDE is a **headless model+operations server**; the primary client is an agent emitting typed query/mutation commands over MCP; the GUI is the *last, optional* client, not the IDE (PROP-032 §2.6).
3. **Code is a first-class addressable node** — `code://<ns>/<id>`, the id **minted on a marker on the item** (per-language carrier: Rust attribute / TS JSDoc / Go comment-directive), **never external, never location-based**, a facet of `specmark` (PROP-032 §2.3).
4. **A refactoring is done only when the model re-checks clean** — atomic, deterministic, dry-run, gated. The LLM emits a typed command; a deterministic engine executes + gates it (PROP-031 §2.2–2.3).
5. **Three-tier operation stack** (product / discipline / language); **wrap permissive engines** (rust-analyzer / ast-grep / ts-morph), never reimplement AST surgery (PROP-031 §2.4–2.5).
6. **Operation algebra**, composable: `rename-address` → `move-unit` = `rename-address ∘ relocate-text ∘ upkeep` → `rename-symbol`, `rename-package`, … (PROP-031 §2.6). Build `rename-address` first (the purest instance).
7. **Refactorings are a package-declared capability** (`[[refactoring]]`), discovered from the lockfile and **precompiled into a cached registry manifest** (the `INDEX.md` / `.mcp.json` pattern); dispatched via `vibe bin`; three kinds under one gated interface; namespaced ids; **the library + the spec are the center, CLI/MCP are thin surfaces** (PROP-033).
8. **Three-tier product model** — base vibevm / the SDD substrate (specmark + specmap, under `org.vibevm.world`) / the ai-native discipline; **dependency inverted** so a legacy tree gets traceability + refactoring **without** conform/cards (PROP-032 §2.8).
9. **Prose spec→spec links + spec→code become graph edges** (refactorable, not merely gated) — extend `mdspec` (PROP-032 §3.3, SPECMAP plan D3/M5).
10. **PROP-014 grows in place** (owner decision) — keeps its title, gains the code node + directions.
11. **Clean-room OpenRewrite study; iterative essential-first; a three-session firewall** (study → redesign → implement) with the findings doc as the only interface (OPENREWRITE-RESEARCH-PLAN §1).

## 4. The sequencing — the program roadmap {#roadmap}

- **Phase R — Research.** Run `OPENREWRITE-RESEARCH-PLAN` in a **fresh, clean session**. Product: a findings document in our words + a gap-map + a minimal-first recommendation. *(The next action.)*
- **Phase D — Redesign.** From the findings doc **only**, revise PROP-031 / 032 / 033 into the final specs. *(A separate session; does not read the studied sources.)*
- **Phase I — Implement, essential-first slices** *(from the redesigned specs; the detailed build plan is authored after Phase D)*:
  - **Slice 0 — M1:** gate the host specmap index in `self-check` (+ fix the `EmbeddedPrecedence` orphan). Host-only, the foundation. *(Currently parked — see §6.)*
  - **Slice 1 — `rename-address`:** the first gated recipe; rename a `spec://` / `code://` address and retarget every citer.
  - **Slice 2 — `move-unit`:** the SPECMAP Unit-Mobility operation.
  - **Slice 3 — composition:** recipes compose into migrations.
  - **Slice 4 — search recipes / find→fix:** the pipeline connecting conform (find) → refactoring (fix).
  - **Slice 5+ —** more kinds (LLM/hybrid), more languages, preconditions, recipe-goldens as a standing gate.
- **Parallel packaging track:** extract specmark + specmap out of `ai-native` into the neutral `org.vibevm.world` SDD-substrate package (dependency inversion, PROP-032 §2.8).
- **The bootstrap track (runs FIRST, does *not* wait for the engine):** the **cultural-pattern extraction refactoring** — pull vibevm's general programming-culture material into reusable packages, minimally-tooled (the existing specmap gate) and gated. Executable plan: [`CULTURAL-EXTRACTION-PLAN-v0.1`](CULTURAL-EXTRACTION-PLAN-v0.1.md), launchable under `/goal`. It cleans and layers vibevm's own specs **and** its `report.md` hands the engine build a concrete requirements list drawn from doing every move by hand. The engine, once built, becomes the tool that maintains this at scale — so the bootstrap both *precedes* and is later *re-served by* the engine. This resolves the chicken-and-egg: you do not need the engine to do the refactoring, only the gate.

## 5. The clean-room firewall — the campaign's first law {#firewall}

OpenRewrite (Apache-2.0, verify) is **inspiration-only, never a code source** — the `eth-sri/type-constrained-code-generation` posture (`90-user.md`). Read to understand; write structurally different code from scratch or on our own permissive deps. **Three separated sessions:** (a) **study** reads `refs/src/` and writes the findings doc; (b) **redesign** works from the findings doc; (c) **implement** works from the redesigned specs. The findings doc is the **only** interface; the sources never cross into (b)/(c). Java→Rust is an extra structural firewall.

## 6. The gaps to close vs OpenRewrite — the redesign's checklist {#gaps}

The study confirms/expands these (our preliminary gap-read): **the two-layer tree model** (coarse project graph ⊕ borrowed per-language type-attributed tree, instead of rebuilding an LST) · **composable/declarative recipes** · **search recipes + the find→fix pipeline** · **preconditions / pattern-matched applicability** · **recipe testing (goldens)** · **impact/blast-radius preview before apply**. Where we aim to *beat* OpenRewrite: spec+code+package graph, agent-first, three implementation kinds, package-delivered recipes, borrowing the real compiler tree, the three-tier product model.

## 7. Parked / deferred {#parked}

Deferred until **after** Phase R → D (the engine may change, so gating now is premature):

- **M1** — gate the host specmap index in `self-check`.
- **The `EmbeddedPrecedence` orphan** (`crates/vibe-resolver/src/embedded_provider.rs:18`, untagged `pub enum` from the PROP-030 work) — blocks the ratchet gate; tag it when M1 lands.
- **The host `specmap.json` regen** — it was found **silently drifted** (editorial naming-campaign edits + PROP-030 code-tag evolution went un-regenerated because the index is not gated — itself the proof M1 is needed).

## 8. Provisional status {#provisional}

**PROP-031 / 032 / 033 are provisional input to the Phase-D redesign.** Expect material change after the research. Do not treat them as frozen; the research is precisely what tells us what to keep, change, and add.

## 9. This session's commit chain (newest first) {#commits}

```
6c27eea docs(research): OpenRewrite clean-room research plan
39b78b9 docs(spec): refactoring registry (PROP-033) + three-tier packaging (PROP-032 §2.8)
5d2c510 docs(terraform): SPECMAP unit-mobility plan under PROP-031/032
037de30 docs(spec): PROP-031 - algorithmic refactoring, the codemod engine
782752c docs(spec): PROP-032 - project model and agent-first IDE substrate
```

---

*This meta-plan is a map, not a spec. The binding detail lives in the documents it indexes (§2); where this map and a document disagree, the document wins, and where a document and the WAL disagree, the WAL wins. The next concrete action is Phase R: run `spec/research/OPENREWRITE-RESEARCH-PLAN-v0.1.md` in a fresh, clean session.*
