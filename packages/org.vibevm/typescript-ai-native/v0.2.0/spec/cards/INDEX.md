# Card Registry — INDEX (TypeScript projection)
**Discipline v0.2 · BETA · T2 · TypeScript**

*The navigable registry of the TypeScript projection's cards. The harness uses this to resolve a trigger to a card and to deliver the Band-3 extract for a `.ts` edit. These are the TypeScript shape of the nine language-neutral scaffold patterns catalogued in the core `02-EXECUTABLE-SCAFFOLDS.md`; this stack ships its own `cards/` so the weak-reader runtime surface for TypeScript is a TypeScript Band-3 block, never the Rust core's (`GUIDE-AI-NATIVE-TYPESCRIPT.md` §13). Generated/maintained as a derived index (A2/R-030); hand edits are a defect.*

## Scaffold cards (the nine executable-scaffold patterns)

| Card | Layer | Mechanism | Trigger mode | Transfer | Checker status |
|---|---|---|---|---|---|
| `scaffold-a-generators` | A+C | scaffold A | raid/gate | [E-strong] | specified |
| `scaffold-b-typed-builders` | E | scaffold B | gate | [E-mid] | specified |
| `scaffold-c-runnable-contracts` | E | scaffold C | inline | [E-mid] | specified |
| `scaffold-d-differential-oracle` | E | scaffold D | gate | [E-mid] | specified (pilot) |
| `scaffold-e-fast-loop` | E+H | scaffold E | gate | [E-strong] | specified |
| `scaffold-f-structured-diagnostics` | E+C | scaffold F | inline | [E-mid] | specified |
| `scaffold-g-doctests` | C+G | scaffold G | gate | [E-strong] | specified |
| `scaffold-h-simulators` | E+H | scaffold H | gate | [E-strong] | specified |
| `scaffold-i-codemods` | H+A | scaffold I | raid | **[E-hyp]** | WISH (pilot-gated) |

The classification axes (layer, mechanism, trigger mode, transfer tag) are language-neutral and carried verbatim from the core catalog so the Rust and TypeScript projections stay comparable. What differs per row is the **checker** (a TypeScript tool, not a Rust one) and the per-language Band-3 routine.

## Trigger-mode delivery summary
- **inline** (per-edit, lint-detectable): C, F. Most frequent; cheapest — `@typescript-eslint` rules in the editor.
- **gate** (per-merge): B, D, E, G, H — `tsc --noEmit` / `vitest` / type-level tests at the cell's verification gate.
- **raid** (scheduled/on-adoption): A, I — `ts-morph` codegen and codemods swept across a layer.
- **review** (human/strong-agent): none yet; reserved for judgment-heavy cards.

## TypeScript checker surface (what each card's checker stands on)
- **`@typescript-eslint` custom rules** — the inline checkers (C, F) and the `unsafe`-set bans (guide §8): `no-explicit-any`, no-unchecked-`as`, no-`!`, no-`@ts-ignore`, no-bare-primitive-at-seam, diagnostic-cites-REQ.
- **`tsc --noEmit` + project references** — the per-cell compile gate (E); the maxed `tsconfig` (guide §1) IS a large part of the checker surface.
- **`tsd` / `expectTypeOf` (vitest)** — type-level assertions (B, and the type-level-testing scaffold, guide §12).
- **Twoslash** — type-checked examples (G).
- **`fast-check`** — property/differential oracles (D); `vitest` snapshots for characterization.
- **codemod post-checks** — atomic apply + `tsc` + `vitest` green (I).

All checker statuses are `specified` (defined, not yet implemented): there is **no TypeScript pilot codebase yet** — the forthcoming VibeVM TypeScript surface (UI + scripting) is the pilot, exactly as vibevm-Rust was the Rust pilot. The cards are authored so a card graduates from BETA when its checker is implemented on that pilot AND its evidence IDs are non-empty AND pilot evidence has not falsified it. This mirrors the state Rust's cards were in *before* the terraform implemented their checkers.

## Axis coverage (research frame A–H)
- A language-shape: A (generators), I (codemods)
- B names & tokens: covered by guide §4 (naming + branding) — candidate future card `rule-closed-vocabulary-naming` / `rule-branding-at-seam`
- C meta-layer: A, F, G
- D context & repo: covered by guide §3 (cells, closure) — candidate `rule-cell-closure`
- E verification: B, C, D, E, F, H
- F spec-binding: specmap (PROP-014, guide §9) — mechanism, not a card
- G empirics: G
- H weak-reader: E, H, I

## TypeScript-specific additive coverage (beyond the nine)
- **Type-level testing** (guide §12) — Class C/D applied to the types themselves (`expectTypeOf`/`tsd`/`@ts-expect-error`). Rust has no readily-available analogue, so this is additive over the shared nine, not a tenth scaffold — it is folded into cards B (type-level surface) and D (type-level differential) as the TypeScript-unique facet of their checkers. A dedicated `rule-type-level-test` card is a candidate if the pilot shows its trigger fires often enough.
- **The erasure boundary** (guide §2) and the **`unsafe` set** (guide §8) are TypeScript-specific *rules*, enforced through cards C (boundary validators) and the bans' eslint rules; candidate dedicated cards `rule-erasure-boundary-validated` and `antipattern-erased-type-lie` if pilot triggers warrant.

## Pending cards (named, not yet authored — pilot will prioritize)
- `rule-closed-vocabulary-naming` (R3-004) — names from a closed vocabulary; no shadowing.
- `rule-branding-at-seam` (R3-008, TS) — meaning-bearing primitives crossing a seam are branded.
- `rule-cell-closure` (R3-001) — editable units declare their full semantic dependency set; no barrel sprawl.
- `rule-contract-first-ordering` (R3-002) — intent before body.
- `rule-position-is-a-resource` (R3-003) — critical invariants at file edges; file-length bound.
- `rule-uniformity` (R3-006/H6) — one idiom per operation; mark exceptions.
- `rule-erasure-boundary-validated` (TS) — untyped exterior enters as `unknown` + schema, never `any`/`as`.
- `antipattern-god-file` (R3-013) — fan-in per file/barrel bounds swarm throughput.
- `antipattern-lying-prose` (R2C-004/H4) — unverified prose/JSDoc claims near code.

These are deferred not because they are unimportant but to honor minimal sufficiency: the nine scaffold cards are the runnable-capital core; rule/anti-pattern cards are added as the pilot shows which triggers actually fire.

## Notes on status
- **specified** = checker is defined but not yet implemented (no TypeScript pilot yet).
- **specified (pilot)** = checker defined; implementation is a named pilot task (D's `replacement-has-oracle`).
- **WISH** = no checker yet (A5); the card is advisory until one exists (I, until weak-agent parameterization is pilot-validated).
- A card graduates from BETA when its checker is implemented AND its evidence IDs are non-empty AND pilot evidence has not falsified it.
