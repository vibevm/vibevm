# E14-B035-PARITY — parity-audit loop, pass after batch 4 (wave-B exit)

Read-only parity audit for `BACKLOG.md {#b-035}` (owner principle, now discipline
law: manifesto `##PARITY-ACROSS-PROJECTIONS` — “no language projection enforces
the discipline more weakly than another; a rule the pilot enforces is either
enforced in every projection or its absence carries a recorded reason”). This
pass re-cuts the table **by the fact of the tree** after wave-B **batch 4** —
the wave’s final batch — landed its two mechanisms: the
acknowledged-deviation status (B-025, “mark, don’t suppress”) and the SARIF
ingest that turns a foreign linter’s diagnosis into a gate fact (B-026). The
previous cuts are pass №1 `harvest/e10-b035-parity-pass.md`, pass №2
`harvest/e12-b035-parity-pass.md`, pass №3 `harvest/e13-b035-parity-pass.md`;
only the CHANGED / NEW rows are re-stated here, the rest hold at pass №1/№2/№3.

Every cell carries `path:line` relative to the worktree root, on the
worktree’s own non-vendored copies. The engine lives at
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/…` (v0.8.0 is the
canonical home batch 4 landed in — verified: the B-025 commit `05f8bdff` and
the B-026 commit `decc5d0a` touch v0.8.0 only; the prior `core-ai-native/v0.7.0`
copy is untouched by batch 4, its `finding.rs` last moved only by the
`2b02996c`/`788e67c8` spec-authority refactors). “Unchanged” is a claim too —
it was verified by reading, not by memory.

**Headline.** Batch 4’s two mechanisms land at **opposite ends of the parity
spectrum**, and neither closes any of the three recorded build-complete debts
pass №3 left open:

- **B-026 (SARIF ingest) reaches full parity.** One engine fact
  (`Fact::LintDiagnosis`), one citation primitive (`Fact::cites_lint`), one
  rule (`LintSuppressionNeedsReason`), mounted in **all three** drivers, all
  three loading the same root `sarif_reports` the same way before the rules
  run. No language weaker — the foreign-linter channel is one engine surface
  every projection cites identically.
- **B-025 (mark, don’t suppress) reaches parity on its CORE contract, with one
  honest recorded sub-asymmetry.** The mechanism is symmetric: a `FindingStatus`
  with a `DeviationAcknowledged` variant, **six** source-deviation rules stamp
  it instead of skipping (3 Rust + 1 TS + 2 Go), `baseline::diff`/`freezeable`
  keep acknowledged findings gate-inert in one place, and SARIF renders them
  via `suppressions{kind:"inSource"}`. The sub-asymmetry: the three Rust
  stampers emit `reason: None` (the Rust facts carry only the boolean
  `in_deviation`), so their SARIF `justification` is a **fixed marker**, not
  the human’s text — whereas the TS/Go stampers carry the directive’s reason
  text (`@ts-expect-error -- reason` / `//spec:deviates reason="…"`) straight
  into `justification`. Here **Rust is the weaker cell**, and the gap is
  recorded in the code that implements it (`finding.rs:74-83`) with a measured
  cost and a not-now decision — so it satisfies the recorded-reason bar and
  joins the build-complete debt.
- **The three surviving Go/inversion gaps of pass №3 are unchanged.** Row 6
  (Go flag/registry rule), rows 8/12 (Go-floor `vet`/`tests`/`staticcheck`
  `./...`), and the custom-lint inversion (TS built, Rust/Go routed
  `{#b-050}`). Batch 4 touched the conform engine and the driver loaders; it
  did not touch the Go flag rule, the Go floor, or any custom-lint vehicle.
  Verified by reading, below.

**M-PARITY verdict (stated plainly, two bars — pass №3’s split holds).**
M-PARITY = “the B-035 table shows no language cell weaker than Rust without a
recorded reason” (`TOOLING-MAP.md:107`).

- **Bar 1 — recorded-reason (the literal milestone): REACHED.** Every weaker
  cell carries a recorded reason: row 6 (`registry_pkg` “carries no rule”),
  rows 8/12 (the named Go-floor residual), `{#b-050}` (Rust dylint + Go
  `analysis.Analyzer`, recorded in `new-rule-classes.xml` §3 + BACKLOG), and
  now the B-025 Rust reason-text sub-gap (recorded in `finding.rs:74-83` + the
  build report + this table). No cell is weaker in silence; batch 4 added no
  silent gap. Being the wave’s final batch, the recorded-reason condition
  holds **as wave B exits**.
- **Bar 2 — build-complete: NOT REACHED.** The recorded gaps are not all
  built. By name: (1) row 6 — the Go flag/registry rule; (2) rows 8/12 — the
  Go-floor `./...` scoping; (3) `{#b-050}` — the Rust `dylint` and Go
  `analysis.Analyzer` custom-lint vehicles (P3, owner “don’t build now, don’t
  drop the promise”); (4) the B-025 Rust reason-text plumbing (~33 frontend
  sites + a frontend version bump, not-now decision — recorded, low priority).

One honest qualifier on Bar 1: the B-025 reason-text gap is recorded in the
**code doc and the build report**, but — unlike `{#b-050}` — it does **not**
(yet) carry a named `BACKLOG.md {#b-xxx}` route. It is one notch softer than
the other recorded gaps; this table now names it so it is not lost. And the
`TOOLING-MAP.md:107` milestone tag is still `@doc/work` even though the literal
recorded-reason condition is met — flipping it is the owner’s call, and the
build-complete debts above are the likely reason it has not flipped.

---

## Rows that are NEW against pass №3

| # | Mechanism | Rust | TypeScript | Go | Delta vs pass №3 | Verdict |
|---|---|---|---|---|---|---|
| 18 | Acknowledged-deviation status — a recorded deviation is MARKED, never suppressed (B-025) | **yes (core) + weaker sub-cell (reason text).** `FindingStatus` enum `core-ai-native/v0.8.0/crates/core-ai-native-conform/src/finding.rs:85`, variant `DeviationAcknowledged { reason: Option<String> }` `:94`; **three** rules stamp `reason: None` (the Rust facts carry only the bool `in_deviation`): `UnsafeGate` `rules/budget.rs:59` (stamp `:110`), `NoUnwrapInDomain` `budget.rs:244` (stamp `:294`), `AmbientEnv` `budget.rs:364` (stamp `:424`); `baseline::diff` excludes acknowledged from `new` `baseline.rs:73-89` (filter `:80`) and `freezeable` from the frozen set `:98-107` (filter `:101`); SARIF renders acknowledged via `suppressions{ kind:"inSource" }` `sarif.rs:63-70`, with `justification` falling back to the fixed marker `"acknowledged in-source deviation (#[spec(deviates)] testimony)"` when `reason` is `None` `sarif.rs:64-66` — **non-empty, but generic, not the human’s text**. Mounted via the Rust driver’s `build_rules` `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:53` | **yes.** `TsUnsafeInDomain` `rules/typescript.rs:48`; its `ts_expect_error` reasoned arm stamps `DeviationAcknowledged { reason: reason.clone() }` `typescript.rs:105-110` — the `@ts-expect-error -- reason` text rides the status straight into the SARIF `justification`. Same baseline/SARIF path (shared engine) | **yes.** **two** rules stamp `reason: reason.clone()`: `GoUnsafeInDomain` `rules/go.rs:59` (stamp `:133-134`, the `//spec:deviates` deviation arm), and `GoSeamErrorCitesReq` `rules/go_parity.rs:56` (stamp `:112-113`, a seam-error obligation covered by a recorded deviation). Same baseline/SARIF path | **NEW.** The `Finding` struct gained `status: FindingStatus` `finding.rs:50` and `evidence: String` (the involved-facts field B-025-BUILD names — a compact render of the birthing fact, on every finding, language-neutral) `finding.rs:56`; declaration order keeps the `Ord` sort byte-identical so no counter/golden/baseline shifts `:39-47`. The six stampers are exactly the packet’s map (3 Rust + 1 TS + 2 Go); a seventh, `LintSuppressionNeedsReason`, stamps SARIF-suppression deviations and is row 19 (language-neutral). The asymmetry is **inside** B-025: TS/Go carry the human reason; Rust carries only the bool → `reason: None` → fixed-marker `justification`. Recorded in `finding.rs:74-83` and `sarif.rs:52-62` (“a measured, recorded leftover … plumbing the reason through the rust-syn frontend”), measured at ~33 frontend sites + a frontend version bump, decision: don’t build now | **CORE PARITY ACHIEVED; reason-text sub-cell: Rust WEAKER, RECORDED (build-complete debt)** |
| 19 | SARIF ingest — a foreign linter’s diagnosis becomes a gate fact (B-026) | **yes.** engine fact `Fact::LintDiagnosis { tool, rule_id, file, line, message, suppressed, reason }` `facts.rs:246-254`; citation primitive `Fact::cites_lint(tool, rule_id, suppressed)` `facts.rs:370-379`; rule `LintSuppressionNeedsReason` `rules/citations.rs:69` (id `lint-suppression-needs-reason` `:72-74`; a reasoned suppression → `DeviationAcknowledged { reason: Some }` `:115-118`, a reasonless one → `Live` `:121-123`); mounted in the Rust driver `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:102`; the driver loads reports before the rules run in BOTH `run_check` `:147` and `run_freeze` `:224` (`sarif::load_reports(root, &config.sarif_reports)`); a broken report is the absence of facts, never a refusal `sarif.rs:256-259` (`:270-291`: unreadable/unparseable/no-`runs` → stderr + skip) | **yes.** same engine fact + primitive + rule; mounted in the TS driver `typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-conform/src/lib.rs:82`; loads reports `:108-109` (`conform_core::sarif::load_reports`) in `run_check` (and the freeze path) | **yes.** same engine fact + primitive + rule; mounted in the Go driver `go-ai-native-lang/v0.1.0/crates/go-ai-native-conform/src/lib.rs:89`; loads reports `:115-116` in `run_check` (and the freeze path) | **NEW.** One root config key `sarif_reports: Vec<String>` `core-ai-native/v0.8.0/crates/core-ai-native-conform/src/config.rs:140` (default empty `:174`) feeds all three drivers identically — the vocabulary is language-neutral, so it lives at the root like `invariant_comment_markers`. The read half (`sarif::ingest`/`load_reports` `sarif.rs:125,264`) is engine code every driver calls; the citation form `check: { tool, id, status }` is the one `cites_lint` primitive. A suppressed foreign diagnosis reuses the B-025 `DeviationAcknowledged` path for free (no new status) — the two batch-4 mechanisms interlock | **PARITY ACHIEVED** |

---

## Rows unchanged, still OPEN — re-verified by reading (batch 4 did not touch them)

The packet’s instruction was to confirm these by reading, not memory. All
three survive batch 4 exactly as pass №3 left them — batch 4 added engine
status/ingest code and driver report-loaders; it did not touch the Go flag
rule, the Go floor, or any custom-lint vehicle.

- **Row 6 — the Go flag/registry rule (unchanged).** Rust `FlagSites` (R-001)
  + TS `ts-flag-sites` exist; Go still has neither. The Go `build_rules`
  (`go-ai-native-lang/v0.1.0/crates/go-ai-native-conform/src/lib.rs:51-90`)
  mounts `GoUnsafeInDomain`, `GoSeamErrorCitesReq`, `GoCellIsolation`,
  `GoConformanceAssertion`, `CellNameIsComputed`, `FileLength`,
  `InvariantCommentPosition`, `DeclaredTestMatrices`, `LintSuppressionNeedsReason`
  — **no flag rule**. The `registry_pkg` config field still reads “carries no
  rule” (`core-ai-native/v0.8.0/crates/core-ai-native-conform/src/config.rs:286,289`,
  field `:291`, default `None` `:307`). Recorded not silent; straight build,
  routed to a later batch (the Go §6 promise is still the remaining large
  asymmetry).
- **Rows 8 / 12 — the Go floor `./...` residual (unchanged).** `gofmt` is
  `exclude_substrings`-scoped (B-003):
  `go-ai-native-lang/v0.1.0/crates/go-ai-native-cli/src/floor.rs:103-113`
  + `filter_gofmt_listed`. `vet`/`tests`/`staticcheck` still walk `./...`
  unfiltered — `go vet ./...` `:134-136`, `go test ./...` `:146-148`,
  `staticcheck ./... && exhaustive ./...` `:158-171`. The B-048 TS twin of
  B-003 and this Go residual are siblings; verify/build routed. Recorded not
  silent.
- **Row 17 — the custom-lint inversion (unchanged).** TypeScript **built** it
  (`typescript-ai-native-lang/v0.6.0/tools/eslint-plugin-ai-native/`); Rust and
  Go did not — grep over `vibevm/vibepacks/org.vibevm.ai-native/**` for
  `dylint`/`declare_lint!`/`LateLintPass`/`analysis.Analyzer` returns matches
  only in the three `GUIDE-AI-NATIVE-*.md` and `scaffold-f-structured-diagnostics.xml`
  cards (promises), never in source. Per
  `##PARITY-PILOT-IS-A-BAR-NOT-A-PRIVILEGE` the bar rose to TS, so Rust and Go
  are the weaker cells, each with a recorded reason + named route
  (`vibevm/vibespecs/design/new-rule-classes.xml` §3, `BACKLOG.md {#b-050}`, P3). Recorded
  not silent.
- **Rows 1–5, 7, 9–16 — hold at pass №1/№2/№3.** Unchanged this pass: the
  infrastructure rows (2–5), seam-error REQ-citation (1), conformance-assertion
  (7), the record-reason rows (9–11), floor-disable (13), and the batch-3
  engine rules (14 invariant-position, 15 computed-name, 16 declared-matrices).

---

## On the packet’s “27 acknowledged / 0 live on the live host”

The packet’s map states the live host run produced **27 acknowledged, 0 live**
findings once batch 4 landed. The **mechanism** that yields that split is
source-verified here: the six source-deviation stampers + `LintSuppressionNeedsReason`
emit `DeviationAcknowledged`, `baseline::diff` keeps them out of `new`
(`baseline.rs:80`), and SARIF marks them `inSource` (`sarif.rs:67-70`). The
**count itself (27/0)** is a runtime observation a read-only pass cannot
re-derive — there is no committed `report.sarif` or host conform baseline in the
tree to corroborate it (the two `baseline.json` under `campaigns/` are
campaign-drift baselines, unrelated). It is attributed to the packet’s map, not
re-measured; the split it implies (every deviation acknowledged, none left to
fail the gate) is exactly what the B-025 mechanism guarantees by construction.

---

## M-PARITY status

M-PARITY = “the B-035 table shows no language cell weaker than Rust without a
recorded reason” (`TOOLING-MAP.md:107`, tag still `@doc/work`). After batch 4
— wave B’s final batch:

- **B-026 is at full parity** (one engine fact + one rule + one citation
  primitive, mounted identically in all three drivers, all three loading the
  same root `sarif_reports`). No weaker cell.
- **B-025 is at parity on its core contract** (status enum, six stampers, one
  gate-inert exclusion, SARIF `inSource` marking — symmetric across languages)
  **with one recorded sub-asymmetry** (Rust `reason: None` → fixed-marker
  `justification`; TS/Go carry the human reason). Rust is the weaker cell on
  that sub-axis; the gap is recorded in `finding.rs:74-83` + the build report,
  measured, decided not-now. It satisfies the recorded-reason bar and is named
  here so it is not lost.
- **The three surviving gaps of pass №3 (row 6, rows 8/12, `{#b-050}`) carry
  their recorded reasons exactly as before.**

**So the literal recorded-reason bar (Bar 1) IS met: no language cell is weaker
than Rust in silence.** What remains is **build-completion of the recorded
gaps** (Bar 2), all routed/owner-acknowledged, none silent:

1. Row 6 — the Go flag/registry rule (straight build; the Go §6 promise).
2. Rows 8/12 — the Go-floor `vet`/`tests`/`staticcheck` `./...` scoping (the
   B-048 sibling).
3. `{#b-050}` — the Rust `dylint` and Go `analysis.Analyzer` custom-lint
   vehicles (P3, owner “don’t build now, don’t drop the promise”).
4. (new, minor) B-025 Rust reason-text plumbing (~33 frontend sites + a
   frontend version bump, not-now decision — recorded in code + build report;
   no formal `BACKLOG.md {#b-xxx}` route yet, named here so it is not lost).

Net: pass №3’s **“M-PARITY (recorded-reason bar) reached; M-PARITY
(build-complete) not yet”** is **unchanged at the wave-B exit** — batch 4
landed two mechanisms at parity (one full, one core-parity-with-a-recorded
sub-gap) and closed none of the recorded build-complete debts, but it added no
silent gap either. The recorded-reason condition the milestone names literally
holds; the four builds above are what stand between recorded-honest and
build-complete.
