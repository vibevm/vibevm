# E13-R3-PENDING-CARDS — census of pending rule cards (R-060, R3-004) and the cell-naming fork

Read-only census. Every claim is anchored `path:line` relative to the worktree root.
`vibedeps/**` and `**/vendor/**` copies are excluded throughout (regenerated /
vendored mirrors, not cited). Discipline engine + specs are `core-ai-native/v0.8.0`
(NOT v0.7.0); v0.7.0 appears only where it is the live stack version pinned under a
language package. "Not found" is stated as an explicit fact with a count.

---

## Q1 — Card format and where cards live

### Card schema (mandatory sections / fields)

Source: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/01-PATTERN-CARD-FORMAT.xml`.
A card has three bands; every field below is a load-bearing section. The
authoring stub is `:65-94`.

**Band 1 — Identity & Recognition** (`:23-28`, GoF parent):
- Card ID & Name — stable slug = a `spec://` anchor (`:24`)
- Classification — `layer` ∈ {A..H}, `mechanism` ∈ {scaffold A-I | rule | anti-pattern} (`:25`)
- Intent (`:26`)
- Also Known As (`:27`)
- Applicability / Recognition — the detector seed (`:28`)

**Band 2 — Justification & Tradeoffs** (`:30-38`, GoF + JEP):
- Motivation (`:31`), Structure & Participants (`:32`), Collaborations (`:33`)
- Goals / Non-Goals (`:34`), Consequences (`:35`), Alternatives (`:36`)
- Risks & Assumptions + sunset condition (`:37`)
- Evidence & Transfer-strength (`:38`)

**Band 3 — Operation** (`:40-47`, machine-extractable), authored as a fenced
` ```card-ops ` block (`:41`) of `key: value` fields:
- `trigger` (`:42`), `mode` (`:43`), `routine` (`:44`), `checker` (`:45`),
  `raid_role` (`:46`), `budget` (`:47`).

### Card addressing (`discipline://` URI)

There is **no standalone grammar document** for the `discipline://` scheme. The
form is defined only by usage and by one code comment:
- `core-ai-native-conform/src/rules/mod.rs:36-39` — "`discipline://` URIs cite the
  installed Discipline package (resolved against `vibevm.discipline.lock`) … The
  convention is recorded in `spec/discipline/README.md`."
- The referenced `spec/discipline/README.md` exists only as
  `legacy-spec/discipline/README.md`; it documents the `spec://` scheme, **not** the
  `discipline://` grammar (`legacy-spec/discipline/README.md:1-39`).

Observed URI grammar (from rule constants, `core-ai-native-conform/src/rules/*.rs`):
`discipline://<stack-package>/<doc>#<anchor>` where
`<stack-package>` ∈ {`go-ai-native-lang`, `rust-ai-native-lang`, `typescript-ai-native-lang`}
and `<doc>` ∈ {`guide`, `cards/<card-name>`}. Examples:
- `discipline://go-ai-native-lang/guide#cells` — `rules/go.rs:16`
- `discipline://rust-ai-native-lang/cards/scaffold-g-doctests#ops` — `rules/diagnostics.rs:88`

### The seven pending cards (verbatim, format doc)

`01-PATTERN-CARD-FORMAT.xml:7` names seven cards "listed by name under 'Pending
cards (named, not yet authored)' in every stack's index":

1. `rule-closed-vocabulary-naming`
2. `rule-cell-closure`
3. `rule-contract-first-ordering`
4. `rule-position-is-a-resource`
5. `rule-uniformity`
6. `antipattern-god-file`
7. `antipattern-lying-prose`

**Measured divergence (fact):** the format doc claims the *same* seven appear in
*every* stack's index. In fact the three indices carry **different** pending lists;
only `rule-closed-vocabulary-naming` is common to all three:
- Rust `rust-ai-native-lang/v0.7.0/spec/cards/INDEX.md:36-43` — the canonical seven above.
- Go `go-ai-native-lang/v0.1.0/spec/cards/INDEX.md:79-84` — `rule-closed-vocabulary-naming`
  (R3-004), `rule-cell-closure` (R3-001), `rule-owned-concurrency` (guide §5). NOT the same seven.
- TS `typescript-ai-native-lang/v0.6.0/spec/cards/INDEX.md:53-58` — `rule-closed-vocabulary-naming`
  (R3-004), `rule-branding-at-seam` (R3-008 TS), `rule-cell-closure` (R3-001),
  `rule-contract-first-ordering` (R3-002), `rule-position-is-a-resource` (R3-003),
  `rule-uniformity` (R3-006). A TS-specific superset.

### Where existing cards physically live + counts

Cards live **only** in the three language stacks; the core has **no** `cards/` dir.
`find … -type d -name cards` (excluding vendor):
- `go-ai-native-lang/v0.1.0/spec/cards/` — `INDEX.md` + `scaffold-a` … `scaffold-i` = **10 files**
- `rust-ai-native-lang/v0.7.0/spec/cards/` — `INDEX.md` + 9 scaffolds = **10 files**
- `typescript-ai-native-lang/v0.6.0/spec/cards/` — `INDEX.md` + 9 scaffolds = **10 files**
- core `core-ai-native/v0.8.0/` — **0** (no `cards/` directory exists)

Total shipped card files: 30 (3 × 10). No rule or anti-pattern card file exists in
any stack (`find -name '*R-*'` under `cards/` → 0; pending names appear only as
INDEX text and guide citations).

### One existing card as a sample (full structure)

`rust-ai-native-lang/v0.7.0/spec/cards/scaffold-g-doctests.md` (49 lines), structure by section:
- `:1` `# CARD: scaffold-g-doctests — …` (slug + name) `{#root}`
- `:3` `<status stage="spec" state="done"/>`; `:5` `##status-line`
- Band 1 (`:7-15`): `##CLASSIFICATION` (`:9`), `##INTENT` (`:11`), `##ALSO-KNOWN-AS`
  (`:13`), `##APPLICABILITY-RECOGNITION` (`:15`)
- Band 2 (`:17-33`): `##MOTIVATION` (`:19`), `##STRUCTURE-AND-PARTICIPANTS` (`:21`),
  `##COLLABORATIONS` (`:23`), `##GOALS-AND-NON-GOALS` (`:25`), `##CONSEQUENCES` (`:27`),
  `##ALTERNATIVES` (`:29`), `##RISKS-AND-ASSUMPTIONS` (`:31`), `##EVIDENCE-AND-TRANSFER-STRENGTH` (`:33`)
- Band 3 (`:35-48`): a fenced ` ```card-ops ` block (`:37`) with `trigger` (`:38`),
  `mode: gate` (`:39`), `routine` (`:40-44`), `checker` (`:45`), `raid_role` (`:46`),
  `budget` (`:47`).

---

## Q2 — How an engine rule references a card

A rule does **not** resolve a card. It cites a `discipline://` URI as the REQ
identifier inside the finding `message`, built by `req_message`:
- `core-ai-native-conform/src/rules/mod.rs:53-55` — `req_message(uri, why, fix)` →
  `"violates REQ {uri}: {why}; fix surface: {fix}"`.
- The URI is passed as the first argument at the call site, e.g.
  `rules/go.rs:116/121/126` pass `GO_GUIDE_CELLS` (`:16`
  `"discipline://go-ai-native-lang/guide#cells"`); `rules/diagnostics.rs:88` passes
  the literal `"discipline://rust-ai-native-lang/cards/scaffold-g-doctests#ops"`.

Four live rules and the URI each emits:
- `rules/go.rs` — `GoCellIsolation` / `GoUnsafeInDomain` cite
  `discipline://go-ai-native-lang/guide#cells` (`:16`), `#errors` (`:17`), `#bans`
  (`:18`), `#replacement` (`:19`).
- `rules/diagnostics.rs` — `PubDoctest`/`SeamHasDoctest` cite
  `discipline://rust-ai-native-lang/cards/scaffold-g-doctests#ops` (`:88`, `:185`);
  `ErrorEnumCitesReq`/`ErrorMessageCitesReq` cite
  `discipline://rust-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops` (`:269`, `:351`).
- `rules/structure.rs` — `CellHasOracle` cites
  `discipline://rust-ai-native-lang/cards/scaffold-d-differential-oracle#ops` (`:218`);
  `CellIsolation`/`FlagSites` cite `discipline://rust-ai-native-lang/guide#…` (`:57`, `:130`).
- `rules/go_parity.rs` — `GoConformanceAssertion` cites
  `discipline://go-ai-native-lang/guide#conformance-is-made-loud` (`:17`).

### Resolution verdict: not resolved by code; files mostly exist; one anchor dangles

- **No resolver code.** `grep -rn "discipline://"` for any `read|open|path|file|fs::`
  use returns **0** matches; no function resolves a `discipline://` URI to a file.
  The only check is `matches_req_grammar` (`mod.rs:66-74`), which validates the
  *message shape* (prefix `violates REQ `, a known scheme ∈ {`spec://`,`discipline://`,`misra://`}
  (`:70`), `: ` and `; fix surface: ` markers) — it never opens the cited file.
- **Files exist (manually resolvable).** `discipline://rust-ai-native-lang/cards/scaffold-g-doctests#ops`
  → `rust-ai-native-lang/v0.7.0/spec/cards/scaffold-g-doctests.md` (exists); its
  `#ops` target is the ` ```card-ops ` fence at `:37`. `discipline://go-ai-native-lang/guide#cells`
  → `go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:163` `## 2. Cells, closure, ownership {#cells}` (exists).
- **One dangling anchor.** `discipline://rust-ai-native-lang/cards/scaffold-d-differential-oracle#ops`
  (`structure.rs:218`) → file exists, but `#ops` does **not**: that card authors
  Band 3 as anchored prose (`##TRIGGER` `:65`, `##MODE` `:67`, `##ROUTINE-*` `:70-76`)
  with **no** ` ```card-ops ` fence (grep `card-ops` in that file → 0). This is the
  divergence `01-PATTERN-CARD-FORMAT.xml:41` already annotates.

Net: URIs are citations, not machine-resolved references; the conform engine never
verifies that a cited card or anchor exists.

---

## Q3 — R-060 today

### R-060 occurrences (excluding `vibedeps/` and `vendor/`); 48 raw, the live cites:

- `BACKLOG.md:888,892,893,898` — B-038 entry (R-060 = "тест-матрицы объявляются
  данными, никогда полный перебор 2^n").
- `crates/vibe-cli/src/registry.rs:65` — `/// the R-060 flag-matrix generator is its Phase 4+ runtime consumer.`
- `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/GUIDE-AI-NATIVE-RUST.xml:127`
  — `##DECLARED-TEST-MATRICES-NEVER-EXPONENTIAL *(R-060, retained.)* Declared test matrices, never \`2^n\`.`
- `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/vibevm/vibespecs/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.xml:233`
  — `##MATRIX-IS-AUTHORED-DATA … (R-060, projected).` (also `##TEST-MATRICES-ARE-DECLARED`
  and `##MATRIX-TOOLING` at `:231` region).
- `vibevm/vibespecs/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml:2669,4436`; `TOOLING-MAP.md:41`;
  `CONTINUE.md:57`; campaign harvest docs (`campaigns/packages-2026-09/harvest/d7d-stacks-sync-reverify.md:1135-1157,2307,2350`, `…/PHASE-D-HOST-OBLIGATIONS.md:317`).

### Card and checker absent (confirmed by grep)

- Card file: `find . -name '*R-060*'` (excl. vibedeps) → **0**.
- Checker: `grep -rni "r.060|r_060|r060"` in `core-ai-native-conform/src` → **0**.
- ATLAS record: `grep "R-060"` in `core-ai-native/v0.8.0/spec/appendix/ATLAS.md` → **0** (see Q6).

### Rule intent (the subject the checker operates on)

R-060 = "declared test matrices, never `2^n`." Per-stack expression:
- Rust `GUIDE-AI-NATIVE-RUST.xml:127` — "Declared test matrices, never `2^n`."
- TS `GUIDE-AI-NATIVE-TYPESCRIPT.xml:231-233` — "Declared test matrices, never an
  implicit `2^n`"; tooling `test.each`/`it.each` over "a named, bounded case table
  (`as const`)", `fast-check` for behavioral surfaces.
- Go `GUIDE-AI-NATIVE-GO.xml:496-504` — `##TABLE-DRIVEN-TESTS-ARE-THE-DECLARED-MATRIX`
  "a named, bounded case slice with `t.Run` subtests, never an implicit `2^n`"
  (note: the Go guide states the rule but does **not** cite the id `R-060`).

### Measured test-matrix constructs in the tree (checker input)

Combinatorial / exponential constructs (the `2^n` smell): **0 occurrences.**
- `grep -rnE "cartesian|\.product\(|combos|permutations|powerset|power_set|2\^n|2\*\*n"`
  in `*.rs/*.go/*.ts` (excl. vendor) → **0**.
- `grep -rnE "rstest|#\[test_case|proptest"` in `*.rs` (excl. vendor) → **0**.
  No property-based-testing framework is used in the discipline's own crates.

Declared matrices (the sanctioned form) are present:
- Rust fixed array: `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-tcg-bridge/src/lib.rs:184`
  `let cases: [(TcgBridgeError, &str, &str); 5] = [ … ]` — bounded array of 5 tuples.
- Go table-driven: `go-ai-native-lang/v0.1.0/tools/go-extract/extract_test.go:132`
  `cases := map[string]string{ … }` iterated `for name, src := range cases { t.Run(name, …) }` (`:134`).

### Typical declared matrices (full examples)

1. Rust — `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-tcg-bridge/src/lib.rs:184-205`:
   `let cases: [(TcgBridgeError, &str, &str); 5] = [ (RustAnalyzerMissing{..}, "rust-analyzer-missing", "rustup component add"), … ]`,
   iterated to assert every error variant carries a kind + recipe. Bounded, named, finite.
2. Go — `go-ai-native-lang/v0.1.0/tools/go-extract/extract_test.go:132-140`:
   `cases := map[string]string{ "blank no type": "package plan\n\nvar _ = New\n", "star type": "package plan\n\nvar _ *Type\n", }`
   then `for name, src := range cases { t.Run(name, func(t *testing.T){ … }) }`. Named map.
3. TS — `typescript-ai-native-lang/v0.6.0/tools/ts-extract/test/extract.test.ts` uses
   `node:test`'s `test` (no `.each`); it runs the extractor over a committed fixture
   tree (`runExtract(DIRTY)`, `:46`) and asserts on records — fixture-driven, not parametric.
   `grep "\.each\("` in `*.ts` (excl. vendor) → **0**.

### Syntactic signals a checker could distinguish (measured in this tree)

- **Declared (sanctioned, present):** a named finite literal collection iterated once —
  Rust `let cases: [T; N] = […]` / `let cases = &[…]`; Go `cases := map[…]{…}` or
  `cases := []struct{…}{…}` + `for … range cases { t.Run(…) }`; TS a typed `as const`
  case table. Signal: one literal binder + one iteration + bounded length visible at the literal.
- **Exponential (to flag, currently ABSENT = 0):** nested `for` over independent
  boolean dimensions; `itertools::iproduct!`/`cartesian_product`; `powerset`;
  bitmask enumeration `for mask in 0..(1<<n)` / `0..2usize.pow(n)`; recursive
  enumeration generators. None of these occur in the non-vendored tree.

A checker today would find **0** violations in this corpus; the rule is preventive.

---

## Q4 — closed-vocabulary-naming (R3-004) today

### Occurrences

`R3-004` (excl. vibedeps/vendor) is cited widely:
- `core-ai-native/v0.8.0/spec/appendix/ATLAS.md:55` — `##FINDING-R3-004 **R3-004** — Names are token programs: closed-vocabulary composition, one name one referent, no shadowing`.
- All three stack indices: `rust-ai-native-lang/…/cards/INDEX.md:37`,
  `go-ai-native-lang/…/cards/INDEX.md:80`, `typescript-ai-native-lang/…/cards/INDEX.md:53`.
- All three guides: `rust …/GUIDE-AI-NATIVE-RUST.xml:57`, `go …/GUIDE-AI-NATIVE-GO.xml:213`,
  `ts …/GUIDE-AI-NATIVE-TYPESCRIPT.xml:125` — each `##NAMES-ARE-TOKEN-PROGRAMS (R3-004, R-020)`.
- `BACKLOG.md:892,898`; campaign docs.

`R-020` is paired with R3-004 in every guide citation ("R3-004, R-020"). The ATLAS
record for R3-004 (`ATLAS.xml:55-56`) carries `_… refines:R-020_`, i.e. R-020 is
referenced as the finding R3-004 refines. A standalone `##FINDING-R-020` / `**R-020**`
ATLAS record: `grep "R-020" ATLAS.xml` → only the `:56` `refines:R-020` tail; **no R-020 record**.

### Closed vocabulary of structural naming tokens — as DATA

Not found. `grep -rnE "VOCABULARY|allowed_tokens|TOKEN_VOCAB|NAME_TOKENS|structural.tokens|const.*VOCAB|fn.*vocabulary"`
in `*.rs/*.go/*.ts` (excl. vendor) → only `typescript-ai-native-tcg-bridge/src/lib.rs:320`
(`file_record_assembly_matches_the_extractor_vocabulary` — a test name about the
extractor's *fact* vocabulary, unrelated to naming tokens). The other "closed
vocabulary" string is `crates/progress-core/src/doc.rs:104` ("Attribute value
outside the closed vocabulary" — progress-doc attributes, not naming). A data-defined
list of allowed name tokens: **0**.

### Naming / shadow / synonym lint in the engine

Not found. `grep -rniE "naming|shadow|synonym"` in
`core-ai-native-conform/src` → **3** incidental prose hits
(`config/coverage.rs:27`, `rules/go_parity.rs:184`, `specmap/src/index.rs:395`) —
none is a naming/shadow/synonym rule. A rule that inspects names: **0**.

---

## Q5 — THE FORK: measuring cell naming, Go vs Rust vs TS

### a) Go — the live `{Variant}{Seam}` specimen

**Cell definition.** A Go cell is a package directory under `cells_dir`
(`GUIDE-AI-NATIVE-GO.xml:170` `##CELL-IS-A-PACKAGE-UNDER-INTERNAL-CELLS`;
`go-ai-native-cli/src/codemod.rs:92-98`, `fast_loop.rs:49-56`). The cell *manifest*
is a `//spec:cell seam=… variant=… replaces=… flag=…` directive, plus the
loud-conformance assertion `var _ seams.<Seam> = (*<Impl>)(nil)` extracted as
`Seam`/`Impl` facts (`tools/go-extract/extract.go:68-70, 504-524`).

**Naming rule (quoted).** `GUIDE-AI-NATIVE-GO.xml:213-217` `##NAMES-ARE-TOKEN-PROGRAMS`:
"Canonical cell type name is computed from the manifest: `{Variant}{Seam}` →
`BatchPlanner`; the package is the lower-case variant (`batchplanner`)." The
template cell shown is `//spec:cell seam=Planner variant=batch replaces=naive flag=planner`
/ `package batchplanner` (`GUIDE-AI-NATIVE-GO.xml:199-201`).

**Actual Go cell names (full list, with paths):**
- `research/go-demo/internal/cells/batchplanner/doc.go:8` `//spec:cell seam=Planner variant=batch replaces=naive flag=planner`;
  `doc.go:9` / `planner.go:1` `package batchplanner`; `planner.go:14`
  `var _ seams.Planner = (*BatchPlanner)(nil)`; `planner.go:17` `func New() *BatchPlanner`.
  → type `BatchPlanner` = `{Batch}{Planner}`; package `batchplanner` = lower(variant). **Matches.**
- `research/go-demo/internal/cells/naiveplanner/doc.go:7` `package naiveplanner`
  (no `//spec:cell` directive — only `//spec:scope`); `planner.go:1` `package naiveplanner`;
  `planner.go:15` `var _ seams.Planner = (*NaivePlanner)(nil)`; `planner.go:18` `func New() *NaivePlanner`.
  → type `NaivePlanner` = `{naive}{Planner}` (variant derivable from package). **Matches.**
- Fixture `go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/clean/internal/cells/greet/greet.go:6`
  `package greet`; `:39` `var _ Greeting = (*Greeter)(nil)`; `:44` `func New(clk clock) *Greeter`.
  → type `Greeter`, seam `Greeting`. `Greeter` does **not** decompose as `{Variant}{Seam}`
  (`Greet`+`Greeting` would be `GreetGreeting`); no `//spec:cell` directive. **Free name.**
- Fixture `…/fixtures/dirty/internal/cells/plan/plan.go:5` `package plan` — deliberately
  dirty fixture (trips census kinds); carries `//spec:scope`/`//spec:implements` but no
  clean conformance assertion. Not a naming exemplar.

**Machine check of the name today? No.** The `go-conformance-assertion` rule
(`rules/go_parity.rs`) checks only that the assertion `var _ <seam> = (*<Impl>)(nil)`
is *present* in a gated cell (`go_parity.rs:116, 183, 241-243` "add `var _ <seam> = (*<Impl>)(nil)` …").
`grep "Variant|variant|composed|decompos"` in `go_parity.rs` → no name-composition check.
So the computed name `{Variant}{Seam}` is **practiced but not machine-verified**.

### b) Rust — free naming; manifest data present but unused for the name

**Cell definition.** A Rust cell is a type carrying the `#[cell(seam = "…", variant = "…" [, replaces = "…"] [, flag = "…" )]`
attribute — `GUIDE-AI-NATIVE-RUST.xml:51` `##CELL-CARRIES-A-CELL-MANIFEST-ATTRIBUTE`;
attribute grammar `core-ai-native-specmark-grammar/src/lib.rs:449` (v0.8.0), which
**requires** `seam` and `variant` (`:517` "requires `seam`", `:522` "requires `variant`").
The `CellIsolation` rule (R-002) identifies a cell by exactly this signal:
`rules/mod.rs:96-113` `cell_types()` collects items whose `attrs` contain `cell(`;
`rules/structure.rs:98, 102` filters files that declare such a type.

**Naming rule (quoted).** `GUIDE-AI-NATIVE-RUST.xml:57` `##NAMES-ARE-TOKEN-PROGRAMS`:
"Canonical cell name is **computed** from the manifest (`{Variant}{Seam}`) … no
shadowing, no synonym pairs. Structural tokens come from a closed vocabulary."

**Actual Rust cell type names (full list, with paths):**
- `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/codemod.rs:28-44` — the
  `add-cell` codemod names the type `let ty = pascal(cell)` (`:28`), emitting
  `pub struct {ty};` (`:44`) beside `#[cell(seam = "{seam}", variant = "{variant}")]` (`:40`).
  Test `codemod.rs:104-108`: `module_source("sat", "DepSolver", "sat", …)` → expects
  `pub struct Sat;` with `#[cell(seam = "DepSolver", variant = "sat")]`. The type `Sat`
  is `pascal("sat")` (`pascal` tests `:100-102`: `pascal("sat")`→`Sat`, `pascal("sat_solver")`→`SatSolver`),
  **not** `{Variant}{Seam}` (`SatDepSolver`).
- `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform-frontend/src/lib/tests.rs:19`
  `#[cell(seam = "S", variant = "v")] … pub struct Thing;` — type `Thing`, not `VS`.
- `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform-frontend/tests/engine.rs:26`
  `#[cell(seam = "S", variant = "x")] pub struct X;` — type `X`, not `XS`.
- `…/tests/engine.rs:31` `#[cell(seam = "S", variant = "y")] pub struct Y;` — type `Y`, not `YS`.
- `…/tests/engine.rs:77` `#[cell(seam = "S", variant = "y")] pub struct Y; pub struct Extra;`.
  (Each duplicated in the `rust-ai-native-mcp/v0.7.0/…` mirror.)

**Live consumer (rust-demo) — no manifest at all.** `grep -rn "#\[cell(" research/rust-demo`
→ **0**. Its cells carry only `#[spec(implements = "spec://rust-demo/PROP-001#…")]`:
`research/rust-demo/crates/rust-demo/src/cells/greeting.rs:25,42,71,95`,
`farewell.rs:18`, `core/text.rs:15`. Type names: `GuestName` (`greeting.rs:26`),
`ParseError` (`greeting.rs:43`) — free names, no seam/variant to compute from.

**Is there a Rust manifest from which the name could be computed?** Yes for
`#[cell]`-bearing types: the `seam`/`variant` key/value pairs ARE in the attribute
(`#[cell(seam="DepSolver", variant="sat")]`) — the same data shape as Go's `//spec:cell`.
But (i) no code computes `{Variant}{Seam}` from it (the codemod uses `pascal(cell)`),
(ii) the live consumer `rust-demo` has **no** `#[cell]` manifest at all, so for those
cells the data source is **absent**.

### c) TypeScript — directory-is-the-cell; free function names

A TS cell is "a module (file) or a small directory with a single public entry
(`index.ts` as the seam)" — `GUIDE-AI-NATIVE-TYPESCRIPT.xml` §3 `##CELL-GRANULARITY`
(cell dir under `src/cells/<name>`). No `#[cell]`-equivalent manifest attribute
exists. Actual names: fixture
`typescript-ai-native-lang/v0.6.0/tools/ts-extract/test/fixtures/clean/src/cells/greet/index.ts:2`
`export function greet(…)`, `…/cells/parse/index.ts:5` `export function parseAndGreet(…)`.
The TS naming rule (`GUIDE-AI-NATIVE-TYPESCRIPT.xml:125`) states closed-vocabulary +
one-referent but does **not** mention `{Variant}{Seam}` computed names.

### d) Cost of each fork variant — facts only, no choice

**Variant A — "Rust accepts computed names `{Variant}{Seam}`":**
concrete files/names that would have to change so `type == {Pascal(variant)}{Pascal(seam)}`:
1. `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/codemod.rs:28` — change
   `let ty = pascal(cell)` to a `{Pascal(variant)}{Pascal(seam)}` composer; update the
   generated `pub struct {ty};` (`:44`) and the two codemod tests (`:104-108`
   `pub struct Sat;` → `pub struct SatDepSolver;`; `:100-102` `pascal` tests lose their premise).
2. `rust-ai-native-conform-frontend/src/lib/tests.rs:19` — `pub struct Thing;` → `pub struct VS;` (seam S, variant v).
3. `rust-ai-native-conform-frontend/tests/engine.rs:26` — `pub struct X;` → `pub struct XS;`.
4. `…/tests/engine.rs:31` — `pub struct Y;` → `pub struct YS;`.
5. `…/tests/engine.rs:77` — `pub struct Y;` → `pub struct YS;` (and `Extra` is non-cell).
   Mirror set in `rust-ai-native-mcp/v0.7.0/…`: codemod.rs, lib/tests.rs, tests/engine.rs — same 5 sites again.
   Total rename sites: **~5 distinct × 2 stacks = ~10**, plus the codemod generator logic + its 2 tests.
   **Missing data source:** the live consumer `rust-demo` has **no** `#[cell]` manifest
   (`grep "#\[cell(" research/rust-demo` → 0), so `GuestName`, `ParseError`, `greet`,
   `parse_guest_name` cannot be computed without first authoring manifests; and there is
   no `Pascal(variant){Pascal(seam)}` composer function today (only `pascal(slug)` at `codemod.rs:28`).

**Variant B — "free naming + lint on vocabulary and uniqueness":** inputs the lint needs:
1. A closed vocabulary of structural naming tokens — **does not exist** (0; see Q4).
2. A registry of contract-surface names for the one-name-one-referent / uniqueness
   check — the `#[cell]`-carrying types are discoverable via `cell_types()`
   (`rules/mod.rs:96-113`) and the frontend emits `Fact::Item { symbol, attrs }`
   (`facts.rs:27`); but this covers only `#[cell]`-bearing items (rust-demo has none),
   and there is no "contract surface" predicate (only `in_src`/`is_lib_root` helpers, `mod.rs:82-94`).
3. A synonym/shadow detector — **does not exist** (0; see Q4).
   What already exists: the fact-extraction pipeline (`Fact::Item` with `symbol`+`attrs`),
   `cell_types()` name discovery, the REQ-grammar renderer (`req_message`). What is
   missing: the vocabulary, the uniqueness-registry scope, synonym/shadow detection,
   and the contract-surface scope predicate.

---

## Q6 — ATLAS and the rule id-space

**What ATLAS is.** `core-ai-native/v0.8.0/spec/appendix/ATLAS.md:1` — "Atlas —
Findings Ledger (human view)". It is a GENERATED human view of research findings
(`:5` "GENERATED from findings.jsonl (A2: derived, do not hand-edit)"); totals
`ATLAS.xml:7` — "Total records: 98 · unique (non-duplicate): 87 · passes: DR-1, DR-2,
blind-control, R3, R2c, seeds". Organized by research axis A–H
(`:9` "By axis: A=10, B=8, C=9, D=10, E=8, F=4, G=8, H=30").

**Structure of one record.** Each is a `##FINDING-<ID>` bullet: ID + title, then a
`_evidence-class · strength · status/refines_` trailing tag, then prose. Three sample rows:
- `ATLAS.xml:35` `##FINDING-R3-002 **R3-002** — Contract-first ordering: intent before body within every item _theory · med · new_`
- `ATLAS.xml:55` `##FINDING-R3-004 **R3-004** — Names are token programs: closed-vocabulary composition, one name one referent, no shadowing _theory · high · refines:R-020_`
- `ATLAS.xml:111` `##FINDING-R3-008 **R3-008** — Misuse-resistant API shape converts probable hallucinations into compile errors`

**Rule id assignment — two series, one registered, one not:**
- `R3-0NN` (first-principles research rules): **registered in ATLAS.** All of
  R3-001 … R3-015 carry `##FINDING-R3-NNN` records (e.g. `:93` R3-001, `:35` R3-002,
  `:95` R3-003, `:55` R3-004, `:73` R3-005, `:37` R3-006, `:109` R3-007, `:111` R3-008,
  `:113` R3-009, `:201` R3-010, `:115` R3-011, `:97` R3-012, `:203` R3-013, `:205` R3-014,
  `:75` R3-015). **Max occupied: R3-015.** The registry IS ATLAS.
- `R-0NN` (engine rules): **no registry.** Distinct ids observed (excl. vibedeps/vendor):
  R-001, R-002, R-010, R-020, R-021, R-030, R-040, R-050, R-060. **Max occupied: R-060.**
  ATLAS entries for `R-0NN`: `grep -c "FINDING-R-0"` in ATLAS → **0**. Only R-001 and R-002
  are shipped as code rules (`rules/structure.rs:35` `"R-001"`, `:91` `"R-002"`); the rest
  are prose citations. There is no separate rule-id registry file
  (`grep -rln "rule registry|RULE-REGISTRY|id registry"` → only prose mentions in
  `ENGINE-CONFORM-v0.1.xml` and a legacy-projection guide, not a registry).

**Reservation needed for the new checker?** No new id is required:
- R3-004 (closed-vocabulary-naming) already has its ATLAS record (`ATLAS.xml:55`).
- R-060 (test-matrices) is already the max cited id (no ATLAS entry, because the
  R-series keeps none; a finding entry for it would be net-new with no R-series precedent).
Registry fact: **R3-series registry exists (ATLAS); R-0NN-series registry does not.**

---

## Q7 — Where new checkers mount

Template = the recently-mounted parity rules `rules/go_parity.rs`
(`GoSeamErrorCitesReq` `:52`, `GoConformanceAssertion`) and `rules/typescript_parity.rs`
(`TsSeamErrorCitesReq` `typescript_parity.rs:52`, id `:55`).

**Full mount path, fact → rule → finding:**
1. **Fact variant.** Declared in `core-ai-native-conform/src/facts.rs:25` `pub enum Fact`.
   Variants (`:27-180`): `Item { symbol, attrs, … }` (`:27`), `Import` (`:42`),
   `Ctor { type_name, line }` (`:48`), `UnsafeUse` (`:58`), `ErrorVariant` (`:66`),
   `FileMetrics { lines }` (`:76`), `UnwrapUse` (`:85`), `EnvRead` (`:97`),
   `TsUnsafe` (`:113`), `GoUnsafe` (`:135`), `GoConformance { seam, impl, … }` (`:148`),
   `TsEnvRead` (`:165`), `TsSeamError` (`:180`). A new rule either reuses these or needs
   a new variant the language frontend must populate (e.g. `GoConformance` is fed by
   `tools/go-extract/extract.go:504-524`).
2. **Rule struct.** A file under `core-ai-native-conform/src/rules/<family>.rs`
   implementing the `Rule` trait. Trait + `Finding` struct:
   `core-ai-native-conform/src/finding.rs:25-34` `Finding { rule: &'static str, file, line: u32, message, why: &'static str, fingerprint: String }`;
   `Rule` requires `id()`, `why()`, `check(&[SourceFacts]) -> Vec<Finding>`
   (`finding.rs:36+`). Example body: `go_parity.rs:54-60` `impl Rule for GoSeamErrorCitesReq { fn id(&self) -> &'static str { "go-seam-error-cites-req" } … }`.
3. **Registered in `build_rules`** — three drivers, one per language:
   - `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:53` `pub fn build_rules(config: &Config) -> Vec<Box<dyn Rule>>`.
   - `go-ai-native-lang/v0.1.0/crates/go-ai-native-conform/src/lib.rs:51` `pub fn build_rules(config: &Config) -> Vec<Box<dyn Rule>>`.
   - `typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-conform/src/lib.rs:48` `pub fn build_rules(config: &Config) -> Vec<Box<dyn Rule>>`.
   Each rule is pushed as `out.push(Box::new(rules::X::new(config.…)))`; config is
   threaded from `Config` (e.g. `config.go.cells_dir`, `config.go.gated`,
   `config.max_file_lines` — see the Go body `go-ai-native-conform/src/lib.rs:51-74`).
4. **Finding emitted.** Via `req_message(uri, why, fix)` (`mod.rs:53`) carrying a
   `discipline://…` URI; a stable `fingerprint` (`"rule|file|carrier"`,
   `finding.rs:33`) feeds the ratchet.
5. **Ratchet.** `rust-ai-native-conform/src/lib.rs:140-155` — `build_rules` →
   `check(&rule_refs, &facts, scope)` → `baseline::load` + `baseline::diff`; new
   findings print `conform: NEW <rule> <file>:<line>` and the run compares
   `findings.len()` vs `base.findings.len()` (Go `lib.rs:150/153`, TS `:154/157`,
   Rust `:165/168`). The host ratchet file `conform-baseline.json` is currently empty:
   `{"findings":[],"schema":1}`.

**Gate tests that assert exact finding counts (the ones a new rule breaks):**
- `rust-ai-native-conform-frontend/tests/engine.rs:128` `assert_eq!(findings_a.len(), 2, "{findings_a:?}")`.
- `rust-ai-native-conform-frontend/tests/engine.rs:140` `assert_eq!(scoped.len(), 1)`.
- `rust-ai-native-conform-frontend/tests/engine.rs:64` `assert_eq!(cold.extracted.len(), 4, …)`.
- Per-rule doctests: `go_parity.rs:49` `assert_eq!(findings.len(), 1)`;
  `typescript_parity.rs:49` and `:123` `assert_eq!(findings.len(), 1)`, `:140`/`:154`
  `assert!(…check(&facts).is_empty())`.
- Each `*-conform/src/lib.rs` test compares live `findings.len()` to
  `base.findings.len()` (Go `:150/:153`, TS `:154/:157`, Rust `:165/:168`) — a new
  rule that fires on a baselined fixture flips these unless the baseline is updated.
