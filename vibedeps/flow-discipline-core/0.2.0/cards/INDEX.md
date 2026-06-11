# Card Registry — INDEX
**Discipline v0.2 · BETA**

*The navigable registry of all Discipline cards. The harness uses this to resolve a trigger to a card and to deliver the Band-3 extract. Generated/maintained as a derived index (A2/R-030); hand edits are a defect.*

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

## Trigger-mode delivery summary
- **inline** (per-edit, lint-detectable): C, F. Most frequent; cheapest.
- **gate** (per-merge): B, D, E, G, H.
- **raid** (scheduled/on-adoption): A, I.
- **review** (human/strong-agent): none yet; reserved for judgment-heavy cards.

## Axis coverage (research frame A–H)
- A language-shape: A (generators), I (codemods)
- B names & tokens: covered by guide §2 (naming rules) — candidate future card `rule-closed-vocabulary-naming`
- C meta-layer: A, F, G
- D context & repo: covered by guide §1 (cells, closure) — candidate `rule-cell-closure`
- E verification: B, C, D, E, F, H
- F spec-binding: specmap (PROP-014, guide §7) — mechanism, not a card
- G empirics: G
- H weak-reader: E, H, I

## Pending cards (named, not yet authored — pilot will prioritize)
- `rule-closed-vocabulary-naming` (R3-004) — names from a closed vocabulary; no shadowing.
- `rule-cell-closure` (R3-001) — editable units declare their full semantic dependency set.
- `rule-contract-first-ordering` (R3-002) — intent before body.
- `rule-position-is-a-resource` (R3-003) — critical invariants at file edges; file-length bound.
- `rule-uniformity` (R3-006/H6) — one idiom per operation; mark exceptions.
- `antipattern-god-file` (R3-013) — fan-in per file bounds swarm throughput.
- `antipattern-lying-prose` (R2C-004/H4) — unverified prose claims near code.

These are deferred not because they are unimportant but to honor minimal sufficiency: the nine scaffold cards are the runnable-capital core; rule/anti-pattern cards are added as the pilot shows which triggers actually fire.

## Notes on status
- **specified** = checker is defined but not necessarily implemented in the pilot.
- **specified (pilot)** = checker defined; implementation is a named pilot task.
- **WISH** = no checker yet (A5); the card is advisory until one exists.
- A card graduates from BETA when its checker is implemented AND its evidence IDs are non-empty AND pilot evidence has not falsified it.
