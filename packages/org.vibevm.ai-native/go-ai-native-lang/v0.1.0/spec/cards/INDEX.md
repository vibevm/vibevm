# Card Registry — INDEX (Go projection)
**Discipline v0.2 · BETA · T2 · Go**

*The navigable registry of the Go projection's cards. The harness uses this to resolve a
trigger to a card and to deliver the Band-3 extract for a `.go` edit. These are the Go
shape of the nine language-neutral scaffold patterns catalogued in the core
`02-EXECUTABLE-SCAFFOLDS.md`; this stack ships its own `cards/` so the weak-reader
runtime surface for Go is a Go Band-3 block, never another language's
(`GUIDE-AI-NATIVE-GO.md` §12). Generated/maintained as a derived index (A2/R-030); hand
edits are a defect.*

## Scaffold cards (the nine executable-scaffold patterns)

| Card | Layer | Mechanism | Trigger mode | Transfer | Checker status |
|---|---|---|---|---|---|
| `scaffold-a-generators` | A+C | scaffold A | raid/gate | [E-strong] | specified |
| `scaffold-b-typed-builders` | E | scaffold B | gate | [E-mid] | specified |
| `scaffold-c-runnable-contracts` | E | scaffold C | inline | [E-mid] | specified |
| `scaffold-d-differential-oracle` | E | scaffold D | gate | [E-mid] | specified (pilot: `research/go-demo` fuzz differential) |
| `scaffold-e-fast-loop` | E+H | scaffold E | gate | [E-strong] | shipped (`go-ai-native fast-loop`; per-package `go test` needs no project machinery) |
| `scaffold-f-structured-diagnostics` | E+C | scaffold F | inline | [E-mid] | shipped (`seam-error-cites-req`, go-ai-native-conform) |
| `scaffold-g-doctests` | C+G | scaffold G | gate | [E-strong] | specified (the health collector counts exported-item `Example` coverage; the gate lands with the pilot) |
| `scaffold-h-simulators` | E+H | scaffold H | gate | [E-strong] | specified |
| `scaffold-i-codemods` | H+A | scaffold I | raid | **[E-hyp]** | pilot prototype shipped (`go-ai-native codemod add-cell`); free parameterization stays the open R4 question |

The classification axes (layer, mechanism, trigger mode, transfer tag) are
language-neutral and carried verbatim from the core catalog so the three projections
stay comparable. What differs per row is the **checker** (a Go tool, not a Rust or TS
one) and the per-language Band-3 routine.

## Trigger-mode delivery summary
- **inline** (per-edit, lint-detectable): C, F — go vet / staticcheck / conform findings
  in the editor loop.
- **gate** (per-merge): B, D, E, G, H — `go build` / per-package `go test -race` /
  Example execution at the cell's verification gate.
- **raid** (scheduled/on-adoption): A, I — `go:generate` regeneration and codemods swept
  across a layer.
- **review** (human/strong-agent): none yet; reserved for judgment-heavy cards.

## Go checker surface (what each card's checker stands on)
- **`go build` / `go vet`** — the compile gate: defined-type nominal safety (B),
  unused-import/variable hygiene, `context` placement, printf shapes.
- **`go test` per package, `-race`** — the Class E loop (E); `Example` functions with
  `// Output:` are compiled AND executed (G — a behavioral doctest, stronger than
  compilation-only).
- **`go test -fuzz` + committed `testdata/` seed corpus** — the differential engine (D).
- **staticcheck / `exhaustive`** — evidence providers (MIT / BSD-2): the unused-code and
  correctness census, and the closed-set switch exhaustiveness the compiler cannot check
  (guide §5 — the one rule a linter carries entirely).
- **go-ai-native-conform (`go-extract` facts → conform-core rules)** — the structural
  gate: cell isolation, the §7 ban census (`init`, ambient defaults, naked `go`,
  error-string matching), file budget, `seam-error-cites-req` in the Class-F grammar.
- **codemod post-checks** — atomic apply + `go build` + per-package `go test` green (I).

Checker statuses marked `shipped` land with this package's own toolchain (the
GO-AI-NATIVE-PLAN campaign); `specified` rows await the pilot
(`research/go-demo` is the first carrier — vibevm itself carries no Go until the
Kubernetes work begins). A card graduates from BETA when its checker is implemented AND
its evidence IDs are non-empty AND pilot evidence has not falsified it.

## Axis coverage (research frame A–H)
- A language-shape: A (generators), I (codemods)
- B names & tokens: covered by guide §3 (naming + the free nominal types) — candidate
  future card `rule-closed-vocabulary-naming`
- C meta-layer: A, F, G
- D context & repo: covered by guide §2 (cells, closure) — candidate `rule-cell-closure`
- E verification: B, C, D, E, F, H
- F spec-binding: specmap (PROP-014, guide §8) — mechanism, not a card
- G empirics: G
- H weak-reader: E, H, I

## Go-specific additive coverage (beyond the nine)
- **Goroutine ownership** (guide §5) is a Go-specific *rule* enforced through the ban
  census (`naked_go_in_cell`) and review; candidate dedicated card
  `rule-owned-concurrency` if pilot triggers warrant.
- **The loud-conformance assertion** (`var _ Seam = (*Impl)(nil)`, guide §2) rides card
  B's checker as a presence check; candidate `rule-loud-conformance`.

## Pending cards (named, not yet authored — pilot will prioritize)
- `rule-closed-vocabulary-naming` (R3-004) — names from a closed vocabulary; no
  shadowing on contract surfaces.
- `rule-cell-closure` (R3-001) — cells declare their full semantic dependency set;
  no ambient state.
- `rule-owned-concurrency` (guide §5) — every goroutine has an owner; channels are
  implementation.
- `rule-contract-first-ordering` (R3-002) — intent before body.
- `rule-position-is-a-resource` (R3-003) — invariants at file edges; file-length bound.
- `rule-uniformity` (R3-006/H6) — one idiom per operation; mark exceptions.
- `antipattern-init-registration` (guide §2/§7) — the stdlib-blessed import side effect,
  banned in cells.
- `antipattern-god-file` (R3-013) — fan-in per file bounds swarm throughput.
- `antipattern-lying-prose` (R2C-004/H4) — unverified godoc claims near code.

These are deferred not because they are unimportant but to honor minimal sufficiency:
the nine scaffold cards are the runnable-capital core; rule/anti-pattern cards are added
as the pilot shows which triggers actually fire.

## Notes on status
- **shipped** = the checker ships in this stack (go-ai-native-conform / go-ai-native)
  and runs on any consumer tree.
- **specified** = checker is defined but not yet implemented.
- **specified (pilot)** = checker defined; implementation is a named pilot task.
- **WISH** = no checker yet (A5); the card is advisory until one exists.
