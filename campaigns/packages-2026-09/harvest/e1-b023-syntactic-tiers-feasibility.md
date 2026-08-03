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
syntactic frontend but *honest labels* — and, the owner's counter-probe
established (§counter-probe), **a depth audit against the discipline's
intent**: today's depth satisfies today's roster, and in at least one named
place (`as_cross`) the intent already demands more. For Python: the frontend
is buildable on a ready-made sidecar shape, but no Python stack package
exists to drive it — a *consumer* is what is missing there.

## Verdict table {#verdicts}

| # | question | feasible? | effort | blocked on | recommendation (proposal) |
|---|---|---|---|---|---|
| T1 | a T-syn tier for TS/JS via tree-sitter/SWC | yes | M (new C-class or swc dependency + a second extractor) | nothing technical | **do not build** — it would duplicate the depth the shipped frontend already delivers while adding a dependency edge that today is zero; the honest fix is relabelling (T3) |
| T2 | checker-grade (true T-sem) TS facts | yes | S-M *inside the existing sidecar* (`createProgram` + checker calls in `extract.ts`) — **plus the cache-key redesign, the real cost** | ~~a rule that needs a checker fact~~ **superseded 2026-08-04 (§counter-probe):** the demand already exists in the discipline's own intent — `as_cross` *names* a type property its test cannot see | **REVISED (owner counter-probe): a planned deepening of the existing sidecar**, opened by a four-point research prelude (§counter-probe below); never a separate frontend |
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

## RULED — the owner's word, 2026-08-04 {#ruled}

**Verbatim:** «давай B-023 отложим до тех пор, пока не появится ещё
какое-то правило кроме "as_cross с не локальной областью". Не нужно
забывать об этом, это нормальное продолжение развития, просто это
кандидат на середину или конец бэклога.»

**What this supersedes:** the §counter-probe's «planned deepening +
research prelude» proposal. The ruling keeps the deferral but sharpens
the trigger — `as_cross` alone does NOT fire it; the checker deepening
waits for a **second** type-requiring rule to appear, and sits at
mid-to-late backlog priority, outside the current waves. Standing in
full: the no-build on a tree-sitter/SWC duplicate; Python behind a
stack decision; the honest-labels re-annotation now names this ruling
as the deferral it records (executed with the two F-146 anchors'
re-judgement, verdict-first, when that pass runs).

## The owner's counter-probe (2026-08-04) — the depth verdict re-judged {#counter-probe}

**The probe, near-verbatim:** «А ты уверен вообще, что это приемлемая
глубина? А почему наш семантический экстрактор не вызывает type checker,
если типы есть? Может наоборот, мы недостаточно глубоко копнули?»

**The honest answer: no, the original T2 was not proven — it was measured
against today's roster, not against the discipline's intent.** Three
findings, one per question:

1. **The depth is provably below intent in one named place.** The fact is
   called `as_cross` — a *cross-type* cast — and its test is «an `as` that
   is not `as const`», which sees no types at all. The landing commit
   (`47cc4978`) itself says «`as const` is discriminated from cross-type
   `as`» — the name promises semantics the implementation approximates
   syntactically. The guide's own rule («`as` only after a check») is
   inexpressible at this depth: the rule cannot tell a checked narrowing
   from a blind cast, and today that gap is papered over by procedure
   (ratchet baseline + recorded deviations), not by precision.
2. **Why no checker — no recorded decision exists.** The landing commit
   records *what* shipped, not why the checker half was left out; the
   guide carries no rationale; there is no decision record. The choice the
   owner is now legitimately re-opening was never written down — itself a
   finding. The *architectural* candidates visible in the tree: the fact
   store's per-file cache key `(file content-hash, frontend id+version)`
   is only correct for **local** facts, and checker facts are non-local
   (file A's types depend on file B — an edit to B stales A's facts under
   an unchanged key); `createProgram` costs a whole-project check per run
   vs a per-file parse; B5 degradation is wider for a checker on a broken
   tree; and the stack's floor already runs the full semantic pass
   (`tsc --noEmit`, strictest config) — conform hunts discipline patterns
   tsc considers legal. Real reasons — but they explain a *default*, not
   a *decision*.
3. **Verdict revised.** T2 rises from «defer with a trigger» to **a
   planned deepening of the existing sidecar**, opened by a research
   prelude of four points: *(i)* inventory the discipline-intent rules
   that need type facts (the true `as_cross`; what else the guide wants
   that the roster cannot express — seam-crossing branded types, typed
   seam failures); *(ii)* a cost spike: `createProgram` wall-clock on a
   real consumer tree; *(iii)* the cache-key design for non-local facts —
   the actual engineering fork (project-epoch key vs a separate
   non-incremental fact class); *(iv)* the API caveat: the TS compiler
   exposes no *public* assignability check (`isTypeAssignableTo` is
   internal — model knowledge, verify at build time), so a true
   `as_cross` costs either an internal-API pin or a workaround. **What
   does not change:** no tree-sitter/SWC duplicate in any outcome — the
   deepening goes into the sidecar that already holds the full Compiler
   API, never into a new frontend.

## Owner decision points, exactly three {#owner-points}

1. **Ratify the no-build pair** — T1 (no tree-sitter/SWC duplicate frontend)
   and P1/P2's gating on a future Python-stack product decision. This study
   does not ask for that product decision now; it only records that the
   engineering path is ready when it comes.
2. **Approve the honest-labels spec edit (T3)** — the §2 table re-annotation,
   executed verdict-first through the registry's two deferred F-146 anchors
   (mirror → merge-verdicts → seal, never chained).
3. **Commission the T2 research prelude** (superseding the original
   passive trigger, per §counter-probe): the four-point study — intent-rule
   inventory, `createProgram` cost spike, the non-local-fact cache-key
   design, the assignability-API caveat — after which the owner rules on
   the deepening's scope. The deepening itself lands inside
   `tools/ts-extract`, never as a new frontend.

## Method and honesty {#method}

The evidence half is the worker's (`e1-b023-evidence.md`, runs archived at
`cache/agents/sorted/E1-B023-SWEEP/` — one main run and one rework that
supplied the initially-missing worker report; the acceptance map and the
lesson are in that directory's `meta.md`). Every verdict, effort class and
recommendation here is the boss's judgment over it, per the campaign's
delegation law. Figures are properties of HEAD `779b3aaa`/`33a0308f`;
anyone re-running takes HEAD's own measurements. The two F-146 registry
anchors deferred to this study re-judge only after the owner's ruling.
