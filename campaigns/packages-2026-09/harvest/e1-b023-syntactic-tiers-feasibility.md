# B-023 · TS/JS syntactic tier + Python frontend — the feasibility study {#root}

**Date:** 2026-08-03 · **HEAD at synthesis:** `33a0308f` (the evidence commit) ·
**Owner directive:** `BACKLOG.md` B-023 — «давай в бэклог положим исследование,
что мы можем реализовать синтаксически для JS/TS и PYTHON» (2026-08-01),
modelled on B-012.

**What this document is.** The boss synthesis over one evidence pass —
[`e1-b023-evidence.md`](e1-b023-evidence.md) (worker-gathered, evidence-only,
boss-reviewed: spot-checks re-run against the tree at `facts.rs:176-189` (the
real `Frontend` trait), `extract.ts` (zero checker calls), and the TS driver's
`build_rules`). The **verdicts and recommendations below are the boss's**,
marked as proposals: nothing is scheduled until the owner rules. Genre:
campaign harvest, non-binding.

**The one-line answer, and it inverts the question:** the TS/JS *syntactic
tier already exists in substance* — the shipped `ts-tsc` frontend, labelled
T-sem by the spec's table, never calls the type checker (it uses
`createSourceFile`/`createScanner` only), and every fact the three TS gate
rules read is parse-, lexical- or trivia-derivable. What is missing is not a
syntactic frontend but *honest labels* — and, for Python, a *consumer*: the
frontend is buildable on a ready-made sidecar shape, but no Python stack
package exists to drive it.

## Verdict table {#verdicts}

| # | question | feasible? | effort | blocked on | recommendation (proposal) |
|---|---|---|---|---|---|
| T1 | a T-syn tier for TS/JS via tree-sitter/SWC | yes | M (new C-class or swc dependency + a second extractor) | nothing technical | **do not build** — it would duplicate the depth the shipped frontend already delivers while adding a dependency edge that today is zero; the honest fix is relabelling (T3) |
| T2 | checker-grade (true T-sem) TS facts | yes | S-M *inside the existing sidecar* (`createProgram` + checker calls in `extract.ts`) | a rule that needs a checker fact — none of the 15 does today | **defer with a recorded trigger:** build the checker half in the *existing* sidecar the day a rule needs a resolved-type fact (e.g. a true cross-type `as_cross`); never as a separate frontend |
| T3 | the §2 frontend-table rows made honest | n/a (spec edit) | S | the owner's ruling on this study | **do:** re-annotate `##ROW-FRONTEND-TS-JS` (the shipped frontend is Compiler-API *parser*-depth; the T-syn cell's tree-sitter/SWC stays a named non-build) and the trait sketch divergence (no `lang()`/`tier()`); executed verdict-first through the two deferred F-146 anchors |
| P1 | a Python frontend — CPython `ast`/`symtable` sidecar | yes | M (~by precedent: extractor ~670 LOC class, bridge ~360, adapter ~135, + `Fact::PyUnsafe` + `Store::for_python` + 2 rules) | **a consumer**: no `python-ai-native-lang` package exists — the frontend would be dead engine code | **feasible, form ready — build only with a Python stack decision**, which is a product call outside B-023 |
| P2 | a Python frontend — RustPython parser in-process | yes | M (the rust-syn shape, ~405 LOC class) | same consumer gap; plus a large new parser dependency whose grammar must track CPython's | **prefer P1's sidecar if/when Python is greenlit** — the sidecar parses with the consumer's own interpreter (the Go precedent: parser ships in the runtime, no `*Unresolvable` class), and the bridge skeleton is copied, not re-derived |

## What the evidence settled {#settled}

1. **The engine's seam is small and frontend-agnostic** — `Frontend =
   {id, version, extract(file, crate, module, text) → Vec<Fact>, warm(batch)}`
   (`facts.rs:176-189`); the store keys the cache on `(id+version,
   content-hash)` and two languages' frontends coexist without collision. A
   new language costs: a `Fact` variant, a `Store::for_<lang>` view + walker,
   an extractor, and its rules. No engine redesign.
2. **The «T-sem» TS frontend is parser-depth.** `extract.ts` calls
   `createSourceFile`, `forEachChild`, `createScanner`, JSDoc readers — and
   nothing from the checker surface (`createProgram`/`getTypeChecker`/
   `getSymbolAtLocation`: zero hits). Its `as_cross` fact is a syntactic
   as-that-isn't-as-const test; it cannot know whether a cast crosses types.
   The node sidecar therefore pays process-spawn + consumer-resolved
   `typescript` for *depth it does not use* — but ripping it out for
   tree-sitter would buy nothing the gate reads today, and the node
   prerequisite is already paid by the stack's tsc floor step.
3. **A third sidecar has a ready-made shape.** The TS and Go bridges are
   cell-for-cell copies (PROTOCOL const, byte-identical `parse_ndjson`,
   `FileRecord`, content-addressed materialisation, warm/extract/probe
   adapter); the five divergences are named and small. CPython is
   structurally the Go case — parser ships in the runtime, so no
   `*Unresolvable` error class and no consumer-side package resolution.
4. **The parity gap is in the rules, not the frontends.** The roster is 15
   at this HEAD (F-146's «thirteen» is stale — `ambient-env` and
   `pub-doctest` joined); a TS project runs at most 3. The nine
   engine-neutral rules TS skips read Rust-only facts. Widening the TS
   *ruleset* (wave Б's B-029/B-034/B-039 family) needs new facts from the
   *existing* frontend, not a new frontend.
5. **All four candidate technologies are absent in-tree** (zero hits over
   11 lockfiles + every manifest) — any adoption is a new dependency edge;
   licence families are permissive across the board (model-knowledge,
   marked for pin-time verification).

## Owner decision points, exactly three {#owner-points}

1. **Ratify the no-build pair** — T1 (no tree-sitter/SWC duplicate frontend)
   and P1/P2's gating on a future Python-stack product decision. This study
   does not ask for that product decision now; it only records that the
   engineering path is ready when it comes.
2. **Approve the honest-labels spec edit (T3)** — the §2 table re-annotation,
   executed verdict-first through the registry's two deferred F-146 anchors
   (mirror → merge-verdicts → seal, never chained).
3. **Record the T2 trigger** — one sentence at the table: «checker-depth
   facts land inside `tools/ts-extract` the day a rule consumes a
   resolved-type fact» — so the deferral carries its build, per BUILD-FIRST.

## Method and honesty {#method}

The evidence half is the worker's (`e1-b023-evidence.md`, runs archived at
`cache/agents/sorted/E1-B023-SWEEP/` — one main run and one rework that
supplied the initially-missing worker report; the acceptance map and the
lesson are in that directory's `meta.md`). Every verdict, effort class and
recommendation here is the boss's judgment over it, per the campaign's
delegation law. Figures are properties of HEAD `779b3aaa`/`33a0308f`;
anyone re-running takes HEAD's own measurements. The two F-146 registry
anchors deferred to this study re-judge only after the owner's ruling.
