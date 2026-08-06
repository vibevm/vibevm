# E13-R1-COMMENT-POSITION — census of inputs for the «invariants don't drown in the middle» rule (B-036)

Read-only census of the inputs a B-036 implementation would consume, taken on
branch `wt/E13-R1-COMMENT-POSITION`. Every factual claim carries a `path:line`,
relative to the worktree root, over the **non-vendored** copies under
`packages/org.vibevm.ai-native/…`. `vibedeps/**` copies are regenerated mirrors
and are never cited; in-package `crates/vendor/core-ai-native-conform/` copies
are byte-identical mirrors of the canonical engine and are only noted as a
fan-out fact. "Not found" is recorded explicitly as a fact about the perimeter,
never silently omitted. This is a measurement file for the B-036 design
(`BACKLOG.md` `{#b-036}`) — **measurements and named missing inputs only, no
design recommendations.**

The canonical engine is `core-ai-native-conform` **v0.8.0** at
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/`
(not v0.7.0). Unless a copy is named otherwise, every `budget.rs` /
`facts.rs` / `config.rs` / `finding.rs` / `baseline.rs` line citation below
refers to that canonical v0.8.0 tree.

---

## Q1 — The promise, verbatim ×3, and where else it is promised

### The three `##POSITION-IS-A-RESOURCE` clauses

**Rust** — `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:59` (one line):

> `##POSITION-IS-A-RESOURCE` **Position is a resource** (R3-003): safety-critical invariants live at file top or bottom, never the diluted middle. Prefer more, smaller, single-purpose files at equal token mass. A conform check warns on files over a length threshold **and on invariant-bearing comments in the middle third.** `@impl/done`

**TypeScript** — `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:128` (one line):

> `##POSITION-IS-A-RESOURCE` **Position is a resource** (R3-003): module-level invariants and the public surface live at the top; prefer more, smaller, single-purpose modules over long files at equal token mass. A conform check warns on files over a length threshold **and on invariant-bearing comments in the diluted middle third** (for `.ts` that structural gate runs through the `typescript-ai-native-conform-frontend` crate — `typescript/tools/conform-frontend-typescript.md` — feeding the same language-neutral engine the Rust stack ships…). `@impl/done`

**Go** — `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:232-236` (multi-line):

> `##POSITION-IS-A-RESOURCE` **Position is a resource** (R3-003): package-level invariants live in the package doc block (`doc.go`) or at file top; safety-critical facts never sit in a file's diluted middle third. Prefer more, smaller, single-purpose files at equal token mass — Go packages are natively multi-file, so splitting costs nothing (§15). A conform check warns on files over the length budget. `@impl/done`

### Differences across the three formulations

| axis | Rust (`:59`) | TypeScript (`:128`) | Go (`:232-236`) |
|---|---|---|---|
| what counts as the "invariant" | **safety-critical invariants** | **module-level invariants and the public surface** | **package-level invariants** / **safety-critical facts** |
| where they must live | "file top **or bottom**" | "at the top" | "package doc block (`doc.go`) or at file top" |
| length-threshold check promised? | yes ("files over a length threshold") | yes ("files over a length threshold") | yes ("files over the length budget") |
| **comment-position (middle-third) check promised?** | **yes** — "invariant-bearing comments in the middle third" | **yes** — "invariant-bearing comments in the diluted middle third" | **NO** — the "conform check warns on …" clause names **only** "files over the length budget"; the middle-third language appears only in the *principle* sentence ("safety-critical facts never sit in a file's diluted middle third"), not in the check that is promised |
| numeric threshold stated? | no (vague "a length threshold") | no (vague "a length threshold") | no ("the length budget") |
| warn vs blocking? | **warn** ("warns") | **warn** ("warns") | **warn** ("warns") |
| impl status marker | `@impl/done` | `@impl/done` | `@impl/done` |

Net: Rust and TypeScript **explicitly promise a comment-position check**; Go
states the principle but its promised *check* covers only the length budget.
All three are marked `@impl/done` despite no comment-position check existing
anywhere in the engine (see Q2) — the drift B-036 was filed on. All three say
**warn**, not blocking.

### Every other place in the corpus where this check / rule is named (non-`vibedeps`)

`grep` for `middle third`, `diluted middle`, `R3-003`, `POSITION-IS-A-RESOURCE`,
and `position is a resource` over the whole tree excluding `vibedeps/`:

- **Engine budget rule (the only `middle third` in code):** canonical
  `core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/budget.rs:119`
  (doc-comment), `:121` (doc-comment), `:147` (the `why()` string), `:148`. The
  v0.7.0 canonical copy carries the same at `budget.rs:119,121,147,148`; the six
  in-package vendor copies (`rust-ai-native-lang`, `rust-ai-native-mcp`,
  `go-ai-native-lang`, `go-ai-native-mcp`, `typescript-ai-native-lang`,
  `typescript-ai-native-mcp`, each under `crates/vendor/core-ai-native-conform/`)
  are byte-identical mirrors of the same four lines.
- **The `position is a resource` signal is, in code, the file-LENGTH fact, not a
  position check:** `core-ai-native/v0.8.0/.../core-ai-native-conform/src/facts.rs:75`
  documents `Fact::FileMetrics` as the "position is a resource" signal
  (file-length budget); `budget.rs:146` renders the same phrase in the
  `file-length` finding's `why`.
- **Card INDEX — named, not authored:** `rust-ai-native-lang/v0.7.0/spec/cards/INDEX.md:40`,
  `typescript-ai-native-lang/v0.6.0/spec/cards/INDEX.md:57`,
  `go-ai-native-lang/v0.1.0/spec/cards/INDEX.md:87` each list
  `rule-position-is-a-resource (R3-003)` among pending cards. **No authored card
  file exists** — `Glob rule-position-is-a-resource*` over the package tree
  returns zero files; the three INDEX lines say "named, not yet authored".
- **ATLAS finding catalog:** `core-ai-native/v0.8.0/spec/appendix/ATLAS.md:95`
  (`@fact:FINDING-R3-003 — Position is a resource: critical invariants at file
  edges, file length bounded`) and the v0.7.0 twin
  `core-ai-native/v0.7.0/spec/appendix/ATLAS.md:92`. ATLAS names the *finding
  family* ("invariants at file edges, file length bounded"); it describes no
  checker.
- **Tool docs tie the phrase to the file-LENGTH rule only:**
  `go-ai-native-lang/v0.1.0/spec/go/tools/conform-frontend-go.md:34`
  (`@fact:RULE-FILE-LENGTH-BUDGET the file-length budget (position is a resource,
  guide §3)`) and
  `typescript-ai-native-lang/v0.6.0/spec/typescript/tools/conform-frontend-typescript.md:23`
  (`##RULE-FILE-LENGTH-BUDGET the file-length budget (position is a resource)`).
  Neither tool doc promises a comment-position check.
- **Campaign evidence already established the absence:** `campaigns/.../tasks/evidence/ev-C3-rust.json:330`
  ("no rule inspects comment position, so the second half of the claim … has no
  checker"), `ev-C3-typescript.json:669` ("No rule inspects comment position"),
  `harvest/d7d-stacks-sync-reverify.md:966-991`, `harvest/d14-b012-part-B.md:611`.

So: the comment-position check is promised by 2 of 3 guides, named (not built)
in 3 card INDEXes, catalogued as a finding family in ATLAS, and present as
**prose only** (doc-comment + `why` string) inside the existing `file-length`
engine rule. No checker exists. The `@impl/done` markers on all three guide
clauses are the drift.

---

## Q2 — The length rule as paradigm: `budget.rs` in full

Source: `core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/budget.rs`.

### What fact the rule stands on, and where the file length comes from

- `FileLength` (`budget.rs:137-139`) is `pub struct FileLength { pub max_lines: u32 }`.
- `check()` (`budget.rs:150-183`) iterates `Fact::FileMetrics { lines }`
  (`budget.rs:157`) — the file length is a **fact field emitted by the
  extractor**, not computed by the rule and not a separate pass. The rule does
  not count lines itself.
- The line count originates in each frontend as `text.lines().count()` /
  `text.split("\n").length` / `physicalLines(src)` (see Q4) and is carried as
  `Fact::FileMetrics { lines }` (`facts.rs:76`).
- Path filter: `super::in_src(&sf.file)` (`budget.rs:153`; predicate at
  `rules/mod.rs:82-84`).
- Threshold comparison: `if *lines <= self.max_lines { continue; }`
  (`budget.rs:160`). `max_lines` is cloned out of `config.max_file_lines` at
  `build_rules` time (Rust `rust-ai-native-conform/src/lib.rs:81-83`; Go
  `go-ai-native-conform/src/lib.rs:71-73`; TS `typescript-ai-native-conform/src/lib.rs:70-72`).

### Finding shape

A `file-length` finding (`budget.rs:163-178`): `rule: "file-length"`
(`budget.rs:142-144`), `file`, **`line: 1` always** (`budget.rs:166`), `message`
via `req_message(REQ-URI, text, remedy)` (`budget.rs:167-175`) producing the
Class-F grammar `"violates REQ discipline://rust-ai-native-lang/guide#surface-form: {lines} lines exceeds the {N}-line file budget; fix surface: …"`,
`why` (`budget.rs:176`), `fingerprint: "file-length|{file}"`
(`budget.rs:177`). The REQ-URI is `discipline://rust-ai-native-lang/guide#surface-form`.

### Is there a notion of "warning" vs "blocking finding" in this rule?

**No.** `FileLength` emits one uniform kind of `Finding`. The `Finding` struct
(`finding.rs:27-36`) has fields `rule, file, line, message, why, fingerprint`
and **no severity field**; the `Rule` trait (`finding.rs:53-57`) is
`id/why/check` with no severity method. There is no warn/block distinction at
the rule level — see Q3 for the gate-level mechanism.

### What is actually at `:119` and `:147`

- `:119` is a **doc-comment line** of the `FileLength` struct
  (`budget.rs:118-123`): "a source file over the line budget pages badly and
  **buries invariants in its middle third**; prefer more, smaller,
  single-purpose files at equal token mass (R3-003 …)". It is rationale prose
  on the struct, not a computation.
- `:147` is the **`why()` return string** (`budget.rs:145-149`): "position is a
  resource: past the budget a file pages badly and **its middle third buries
  invariants** — prefer more, smaller, single-purpose files (…R3-003)". It is
  the axiom text rendered into SARIF/terminal when the rule fires, not a check.

Both occurrences are prose. The rule's only logic is `lines > max_lines`
(`budget.rs:160`); it never reads any comment's position or content.

---

## Q3 — Severity / warning: is there a non-blocking finding class today?

### Engine

- **No severity anywhere.** `Finding` (`finding.rs:27-36`) has no severity field;
  `Rule` (`finding.rs:53-57`) has no severity method; `check()`
  (`finding.rs:79-86`) runs every rule and treats every finding uniformly. A
  targeted grep for `severity|advisory|warning` over the engine source
  (`core-ai-native-conform/src/**`) returns only prose: `config.rs:359` (doc
  "everything advisory") and `config/coverage.rs:330-335` (the empty-scope
  warning helper's own comments). **No `Severity` type, no `advisory` flag, no
  per-finding level exists.**

### How the gate decides pass/fail (the only "block" notion)

All three drivers fail the gate **iff there are new findings vs the ratchet
baseline** — identical `if !new.is_empty() { bail!(…) }`:
- Rust `rust-ai-native-conform/src/lib.rs:180-182`.
- Go `go-ai-native-conform/src/lib.rs:165-167`.
- TS `typescript-ai-native-conform/src/lib.rs:169-171`.

### What is printed but does NOT fail the gate

These are `eprintln!` output, never `Finding`s, never a fail condition:
- `ConfigOrigin` announcement (Rust `rust-ai-native-conform/src/lib.rs:32-36`;
  Go `go-ai-native-conform/src/lib.rs:28-35`; TS analogous).
- Vacuous-gate warning `warn_vacuously_gated` (Rust `rust-ai-native-conform/src/lib.rs:100-108`);
  Go/TS equivalents `announce_go_coverage` (`go-ai-native-conform/src/lib.rs:103-115`)
  calling `go_vacuously_gated` / `go_scope_warnings` (defined in
  `config/coverage.rs`, re-exported `config.rs:27-30`).
- Empty-scope warnings `rust_scope_warnings` / `go_scope_warnings` /
  `ts_scope_warnings` (Rust `rust-ai-native-conform/src/lib.rs:136-138`; Go
  `go-ai-native-conform/src/lib.rs:105-107`; helpers in
  `config/coverage.rs:330+`).
- Stale baseline entries ("baseline entry no longer fires — prune it", Rust
  `rust-ai-native-conform/src/lib.rs:159-161`; Go `:144-146`; TS analogous) —
  printed, non-blocking.

### How the ratchet baseline is built (the "tolerate N existing" mechanism)

- `Baseline { schema: u32, findings: Vec<String> }` (`baseline.rs:22-27`) — a
  list of **frozen fingerprints**.
- `diff(baseline, findings) -> (new, stale)` (`baseline.rs:62-76`): `new` =
  findings whose fingerprint is not in the baseline (these fail); `stale` =
  baseline entries no longer produced (prune candidates).
- An absent baseline file ⇒ empty baseline ⇒ "no findings allowed at all"
  (`baseline.rs:29-30`, `:38-44`).
- The freeze path (`run_freeze`, Rust `rust-ai-native-conform/src/lib.rs:190-227`;
  Go `go-ai-native-conform/src/lib.rs:177+`; TS analogous) rewrites the baseline
  to the current fingerprint set; the documented legal moments are "a NEW rule
  landing (its pre-existing findings freeze once), and a re-freeze after work
  that shrank the set" (Rust `rust-ai-native-conform/src/lib.rs:187-189`).

### Named missing input

`BACKLOG.md` `{#b036-sut}` specifies the position rule should ship as
"предупреждение — не блокирующий гейт на старте (урок B-021)". **The engine has
no advisory / warning finding class** — a finding is either a uniform
gate-failing `Finding` or it does not exist. The only mechanism that makes a
finding non-failing is the **ratchet baseline** (freeze all current
position-violations once via `run_freeze`, tolerate them, then shrink). There is
no severity/level field on `Finding` to mark a finding warn-only. **This is a
concrete missing input**: either an advisory-finding class must be added to the
engine (`Finding` + the gate's fail condition), or B-036 must be realised
purely through the ratchet freeze.

---

## Q4 — What the extractors see about comments, per language

### The shared fact model — every `Fact` variant and what it carries

Source: `core-ai-native/v0.8.0/.../core-ai-native-conform/src/facts.rs`. The
`Frontend::extract(&self, file, crate_name, module, text: &str) -> Vec<Fact>`
trait (`facts.rs:227-240`) hands each frontend the **full file `text`**, so every
frontend has access to all comments by construction.

| `Fact` variant | line | carries comment text? | carries a comment line? | doc-comment? | file line-count? |
|---|---|---|---|---|---|
| `Item { kind, symbol, line, attrs, is_pub, has_doctest }` | `facts.rs:27-40` | no (`attrs` = `spec/cell/verifies` attr text only) | `line` = the **item's** line, not a comment's | `has_doctest: bool` — doc fence presence, **text/line discarded** | — |
| `Import { from_module, to_path, line }` | `:42-46` | no | item line | — | — |
| `Ctor { type_name, line }` | `:48` | no | item line | — | — |
| `UnsafeUse { context, line, in_test, in_deviation }` | `:58-63` | no | item line | — | — |
| `ErrorVariant { enum_symbol, variant, message, line, enum_attrs }` | `:66-73` | `message` = `#[error("…")]` text | item line | — | — |
| **`FileMetrics { lines }`** | `:76` | — | — | — | **YES — total file lines** |
| `UnwrapUse { method, line, in_test, in_deviation }` | `:85-90` | no | item line | — | — |
| `EnvRead { method, line, in_test, in_deviation }` | `:97-102` | no | item line | — | — |
| `TsUnsafe { kind, line, in_test, reason }` | `:113-118` | `reason: Option<String>` = `@ts-expect-error -- …` text | item/comment line | — | — |
| `GoUnsafe { kind, line, in_test, reason }` | `:135-140` | `reason: Option<String>` = `//spec:deviates … reason="…"` / suppression reason | item/comment line | — | — |
| `GoConformance { seam, impl_type, line, in_test }` | `:148-153` | no | item line | — | — |
| `TsEnvRead { source, line, in_test }` | `:165-169` | no | item line | — | — |
| `TsSeamError { symbol, cites_req, line, in_test }` | `:180-185` | no (`cites_req: bool`) | item line | — | — |

Cross-cutting answers to the four sub-questions:

- **(a) comment text:** **No** variant carries free-form source-comment text.
  The closest are `reason: Option<String>` on `TsUnsafe` / `GoUnsafe`
  (deviation/suppression testimony, a narrow directive) and `ErrorVariant.message`
  (an error-string template). Neither is an arbitrary comment.
- **(b) comment line:** **No dedicated comment-line fact.** Twelve variants carry
  a `line`, but it is the line of the **code construct** they describe (item /
  call / use), not of a comment. The exception-shaped `reason` facts point at a
  directive comment, but only for the spec/deviation vocabulary.
- **(c) doc-comment / doc-string:** **Partial — reduced to a boolean.** Rust
  `Item.has_doctest` (facts.rs:38-39); Go `item.has_doc_example`; TS
  `item.has_doc_example`. Each frontend *reads* the doc comment but discards its
  text and its line, keeping only "does it contain a fenced example".
- **(d) total file lines:** **YES — already a fact in all three.** `FileMetrics {
  lines }` (`facts.rs:76`), emitted once per file by every frontend.

### Per-frontend detail

**Rust frontend** — `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform-frontend/src/lib.rs`
(version `"6"`, `lib.rs:57`):
- `extract()` seeds `Fact::FileMetrics { lines: text.lines().count() }`
  (`lib.rs:66-68`) — **(d) from raw `text`**.
- Doc comments are read: `has_doc_fence()` (`lib.rs:169-182`) walks `#[doc = …]`
  attributes (syn desugars `///`/`//!` to `#[doc]`) and tests for a ```` ``` ````
  fence, producing `Item.has_doctest` (`lib.rs:200,281,293,339`) — **(c) bool
  only, text/line discarded**.
- **Plain `//` and `/* */` comments are dropped by `syn::parse_file`**
  (`lib.rs:61`) — they never enter the AST, so a `// SAFETY:` line is invisible
  to the syn walk. Only `#[doc]` (from `///`/`//!`) survives. **(a)/(b) absent
  for plain comments.**

**go-extract** — `go-ai-native-lang/v0.1.0/tools/go-extract/extract.go`:
- `file_metrics { lines: physicalLines(src) }` emitted first
  (`extract.go:180`; `physicalLines` `:200-209`) — **(d)**.
- Parses with `parser.ParseComments` (`extract.go:183`), so **all comments
  (incl. doc comments) are in the AST**.
- `collectMarkers()` (`extract.go:348-396`) walks `ex.file.Comments`, and for
  each `//spec:` directive emits `marker{Tag, URI, R, Reason, Symbol, Line}`
  (`marker` struct `:74-81`; `marker.Line = ex.line(c.Pos())` `:362`). — **(a)
  partial (directive text via `Reason`/`Tag`), (b) partial (directive line).**
- `suppressions()` (`extract.go:777-806`) walks `ex.file.Comments` again and
  computes `line := ex.line(c.Pos())` for **every** comment, emitting
  `reasonless_suppression` facts at comment lines. — the comment-walk loop with
  per-comment `text` + `line` **already exists** here.
- Doc comments: `docOwners()` (`extract.go:298-327`) maps a doc `CommentGroup`
  to its declaration name; `funcItem`/`genItems` set `HasDocExample: &noExample`
  (always false, `:439,455,475` — example coverage is joined later, package-level).
  — **(c) not recorded as a fact (always false).**

**ts-extract** — `typescript-ai-native-lang/v0.6.0/tools/ts-extract/extract.ts`:
- `file_metrics { lines }` where `lines = text.split("\n").length`
  (`extract.ts:501-502`) — **(d)**.
- `item.has_doc_example = /```|@example/.test(docText.slice(0,2000))`
  (`extract.ts:602`, `docText` from JSDoc-bearing node text) — **(c) bool only,
  text/line discarded**.
- JSDoc `@implements/@verifies/@documents/@deviates/@informs/@scope` tags →
  `Marker{tag, uri, reason, symbol, line}` via `markerFromTag()`
  (`extract.ts:284-301`; `line: lineOf(sf, tag.getStart(sf))` `:299`). — **(a)
  partial (tag/reason text), (b) partial (tag line).**
- **A comment-stream scanner walks every comment with its text + line**
  (`extract.ts:626-671`): `ts.createScanner` over the whole `text`; for each
  `SingleLineCommentTrivia`/`MultiLineCommentTrivia` it reads
  `commentText = scanner.getTokenText()` and `lineOf(sf, start)`, then emits
  only `@ts-expect-error`/`@ts-ignore` (`SUPPRESSION` `:272`, `:643-651`) and
  detached `@scope` (`:658-667`). — the loop that sees **every comment's full
  text + line already exists**; non-suppression comments are discarded.

### Minimal new input per language to compute "comment position / file length"

File length already exists everywhere (`FileMetrics`). The missing half is a
per-comment fact carrying **(comment line, invariant-marker signal)** — a new
`Fact` variant (e.g. shaped `{ line, marker }` or `{ line, text }`). Per
frontend the emission effort differs:
- **Rust:** syn drops plain `//` comments, so a **new text-scan pass over
  `text`** is required for `//`/`/* */` comment lines (the frontend already scans
  `text` for line-count at `lib.rs:67`); Rust `//!` module-doc and `///` doc
  comments are already reachable as `#[doc]` attributes. → ~2 new things: a
  text-scan for plain-comment lines + the new fact variant.
- **Go:** the comment-walk loop already exists (`extract.go:350-395` and
  `:778-806`); package doc and doc comments are already in `ex.file.Comments`.
  → ~1–2 new things: emit the new fact from inside the existing comment loop.
- **TypeScript:** the comment-stream scanner loop already exists
  (`extract.ts:626-671`) with `commentText` + `lineOf(sf, start)` for every
  comment; JSDoc `/** @invariant */` is reachable via `getJSDocTags`. → ~1–2 new
  things: emit the new fact from inside the existing scanner loop.

---

## Q5 — Marker vocabulary: measurement over the live tree

`rg --count-matches` over **non-vendored code only** (`*.rs *.go *.ts *.tsx
*.mts *.cts`, excluding `**/vendor/**`, `**/vibedeps/**`, `**/target/**`) of the
three lang packages + host `crates/`. Counts are real, not estimates. "Top
files" = the 5 files with the most matches.

| marker class | occurrences | files | concentration |
|---|---|---|---|
| `SAFETY:` (and `// SAFETY`) | **6** | 3 | `rust-ai-native-env-audit/src/lib.rs:4`, `crates/vibe-test-support/src/isolate.rs:1`, `crates/vibe-cli/src/main.rs:1` |
| `INVARIANT:` (uppercase colon) | **0** | 0 | — |
| `NOTE:` | **12** | 3 | `crates/vibe-cli/tests/cli_registry_mgmt.rs:10`, `crates/vibe-test-support/src/lib.rs:1`, `crates/vibe-cli/tests/cli_pkg_cycle.rs:1` |
| `WARNING:` (uppercase colon) | **0** | 0 | — |
| `MUST` (uppercase word) | **21** | 17 | `crates/vibe-resolver/src/local_composite_provider.rs:2`, `crates/vibe-registry/src/shippable.rs:2`, `crates/vibe-registry/src/git_backend/shell.rs:2`, `crates/vibe-publish/src/orchestrator.rs:2`, `typescript-ai-native-cli/tests/fresh_ts_project.rs:1` |
| `NEVER` (uppercase word) | **5** | 5 | `typescript-ai-native-lang/tools/ts-oracle/oracle.ts:1`, `crates/vibe-workspace/src/boot_artifacts.rs:1`, `crates/vibe-publish/src/post_hook.rs:1`, `crates/vibe-index/src/scanner/from_github.rs:1`, `crates/vibe-cli/tests/cli_registry_mgmt.rs:1` |
| `PANICS` (word) | **0** | 0 | — |
| rustdoc sections `# Safety` / `# Panics` / `# Invariants` / `# Errors` | **0** | 0 | — |
| `##`-anchor inside a comment (`##[A-Z]`) | **114** | 14 | `crates/progress-core/src/parse/facts.rs:22`, `crates/vibe-spec/src/qualify.rs:19`, `crates/vibe-spec/src/pipeline/tests.rs:16`, `crates/vibe-spec/src/facts.rs:12`, `crates/progress-core/src/parse/delimiters.rs:12` |
| Rust module-doc `//!` | **6005** | 628 | `crates/vibe-resolver/src/lib.rs:50`, `crates/vibe-cli/src/commands/mcp/mod.rs:48`, `crates/vibe-cli/tests/cli_live_e2e.rs:46`, `crates/vibe-check/src/lib.rs:46`, `crates/vibe-workspace/tests/lane_dedup.rs:41` |
| Go package doc `// Package ` (real `.go` files) | **3** | 3 (`.go`) | `go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/dirty/internal/registry/registry.go:1`, `…/dirty/internal/cells/plan/plan.go:1`, `…/clean/internal/cells/greet/greet.go:1` |
| TS `@invariant` | **0** | 0 | — |

### Two measurement caveats (recorded, not silent)

- **`// Package ` total is 23, but 20 of those are in `.rs` files** (test
  fixtures / templates that emit Go source), not real Go. Only **3 real `.go`**
  files carry a Go package doc, and all three are go-extract **test fixtures**.
  No real Go package doc lives in shipped Go code in this tree.
- **The `##`-anchor count (114) is concentrated in 14 files** — the spec /
  progress parser crates' own doc-comments that *describe* the `##ANCHOR`
  grammar (e.g. `crates/progress-core/src/parse/facts.rs:40,42,44,50-51` cite
  `##COUNTABLE-UNITS`, `##FACT-ANCHOR-SYNTAX`). These are doc-comments about the
  anchor token, used as inline section references — a real comment signal, but
  not broadly distributed.

### Measurement reading (fact, not a design recommendation)

The explicit uppercase invariant-marker vocabulary the guides' prose evokes
(`SAFETY:`, `INVARIANT:`, `PANICS`, rustdoc `# Safety`/`# Panics`) has
**near-zero presence** in this codebase (0–6 occurrences). The only broadly
present "invariant-bearing" signals are the doc-comment forms — Rust `//!`
(every module: 6005) and `///`, Go doc comments (3, all fixtures), TS JSDoc.
A marker-keyed heuristic would need to key on **doc-comment presence** (not on
`SAFETY:`/`INVARIANT:`/`PANICS`) to have any recall here.

---

## Q6 — Where the thresholds land: the config v2 surface

Source: `core-ai-native/v0.8.0/.../core-ai-native-conform/src/config.rs`. The
v2 surface (B-029 + B-034 + B-049) is per-language-symmetric.

### Root `Config` (`config.rs:62-101`; `#[serde(default, deny_unknown_fields)]` `:63`; `Default` `:103-121`)

- 9 **loud-tombstone** `Option<Value>` fields — the retired flat root keys
  (`roots`, `exclude_substrings`, `gated_crates`, `gated_pub_doctest`,
  `audit_crates`, `env_roots`, `registry_file`, `registry_gated_crate`, `exempt`)
  at `config.rs:70-86`; any set value is a targeted move-hint error via
  `tombstones::check` (`config.rs:371`).
- **`max_file_lines: u32`** (`config.rs:90`; default `600` `:115`) — the **only
  cross-language budget at the root**, read by all three frontends.
- `rust: RustConfig` (`config.rs:92`); `typescript: TsConfig` (`:96`);
  `go: GoConfig` (`:100`).

### Per-language section shape (one uniform shape + language extras)

`RustConfig` (`config.rs:138-174`, `Default` `:176-191`): `roots`, `exclude_substrings`,
`gated`, `exempt: Vec<ExemptEntry>`, `gated_pub_doctest`, `audit_crates`,
`env_roots`, `registry_file: Option`, `registry_gated_crate: Option`,
`floor_disable: Vec<FloorDisable>` (`:173`).

`GoConfig` (`config.rs:207-237`, `Default` `:239-252`): `roots`, `exclude_substrings`,
`gated`, `exempt`, `cells_dir: Option`, `seams_pkg: Option`, `registry_pkg:
Option`, `floor_disable` (`:236`).

`TsConfig` (`config.rs:268-297`, `Default` `:299-312`): `roots`, `exclude_substrings`,
`gated`, `exempt`, `cells_dir: Option`, `seam: String`, `composition_root:
Option`, `floor_disable` (`:296`).

`ExemptEntry { unit: String, reason: String }` (`config.rs:338-345`,
`#[serde(deny_unknown_fields)]` — both required). `FloorDisable { step: String,
reason: String }` (`config.rs:315-323`, `#[serde(deny_unknown_fields)]` — both
required); `step` is one of `prettier`/`tsc`/`tests`/`eslint`/`conform`/`specmap`/`test-gate`
(`config.rs:318-319`). `ConfigOrigin { Loaded, Defaulted }` (`config.rs:354-361`).

### Where `max_file_lines` reaches the rule, and how config is loaded

- All three drivers clone `max_file_lines` into `FileLength { max_lines }` at
  `build_rules`: Rust `rust-ai-native-conform/src/lib.rs:81-83`; Go
  `go-ai-native-conform/src/lib.rs:71-73`; TS
  `typescript-ai-native-conform/src/lib.rs:70-72`.
- All three load via `Config::load_or_default` (`config.rs:380-390`): Rust
  `rust-ai-native-conform/src/lib.rs:29`; Go `go-ai-native-conform/src/lib.rs:26`;
  TS `typescript-ai-native-conform/src/lib.rs:23`. `Config::load`
  (`config.rs:366-373`) parses then runs `tombstones::check` (`config.rs:371`).
- The gated-or-exempt validators are methods on `Config` in
  `config/coverage.rs`: `validate_against_tree` (Rust, `:235`),
  `validate_go_against_tree` (`:250`), `validate_typescript_against_tree`
  (`:264`).

### Is there a field parameterising a rule threshold per language today?

**No.** `max_file_lines` is the **single** numeric rule threshold in the whole
schema, and it lives at the **root** (cross-language, one value for all three).
Every per-language field is a string or a list (`roots`, `gated`, `exempt`,
`cells_dir`, `seams_pkg`, `composition_root`, `floor_disable`, …). There is no
precedent for a per-language numeric rule threshold. A position rule's
thresholds would either reuse a root-level field (`max_file_lines`-style) or
introduce the first per-language numeric — no existing field parameterises a
rule threshold by language.

### Live `conform.toml` (host root) is already v2-shaped

`conform.toml` (host root) carries `max_file_lines = 600` (`:16`) and a `[rust]`
section (`:18` onward: `roots`, `exclude_substrings`, `registry_file`,
`registry_gated_crate`, `audit_crates`, `gated` (13 crates, `:46-60`),
`gated_pub_doctest`, `env_roots` (many, `:76-105`), and `[[rust.exempt]]` ×6
(`:111,:115,:119,:123,:127,:131`, each `unit`+`reason`)). It is Rust-only; no
`[go]`/`[typescript]`. The flat root keys are gone (migrated), so it parses
cleanly under the tombstone gate.

---

## Q7 — Fixtures and goldens

### Rust stack — the gate runs on the live workspace (no synthetic conform fixtures)

- **Scope:** the real `crates/*` + `xtask` tree (`conform.toml:21`); the gate
  extracts and checks the actual workspace, not a fixture tree.
- **Golden (frozen fingerprints):** `conform-baseline.json` (host root) — the
  ratchet baseline; grows when a new rule fires on host code and is rewritten by
  `run_freeze` (`rust-ai-native-conform/src/lib.rs:190-227`). The demo twin
  `research/rust-demo/conform-baseline.json` is `{ "schema": 1, "findings": [] }`
  (clean demo).
- **By-rule / count tests:**
  - Engine per-rule unit tests: `core-ai-native-conform/src/rules/tests.rs`
    (`assert_eq!(found.len(), …)` at `:56, :82, :114, :151, :303, :354, :367`)
    cover the structure/diagnostics rules. **`FileLength`'s only test is its
    doctest** (`budget.rs:125-136`); there is no dedicated `file-length` test in
    `tests.rs`.
  - Frontend integration: `rust-ai-native-conform-frontend/tests/engine.rs`
    (`cold.extracted.len()==4` `:64`; `findings_a.len()==2` `:128`;
    `scoped.len()==1` `:140`; `new.is_empty()` `:150`).
  - Host guard: `xtask/src/conform.rs:29` `every_crate_is_gated_or_exempt`
    (gated-or-exempt invariant, not a by-rule count); `run_conform_check`
    (`:13`), `run_conform_freeze` (`:17`).
- **TCG parity:** `rust-ai-native-tcg` crate (`rust-ai-native-lang/v0.7.0/crates/rust-ai-native-tcg/`),
  test module `src/lib/tests.rs` — oracle parity, not conform by-rule counts.

### Go stack

- **Extractor fixtures:** `go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/{clean,dirty}/`
  (clean: `internal/cells/greet/greet.go`; dirty: `internal/cells/plan/plan.go`,
  `plan_test.go`, `internal/registry/registry.go`); each fixture tree carries a
  `conform.toml` (E8 census).
- **Extractor unit test:** `tools/go-extract/extract_test.go` — reads fixtures,
  asserts by-kind via helpers `hasUnsafeKind` (`:24-31`) / `unsafeLine`
  (`:33-40`); checks `seam_error_message_no_req` line `== 22` (`:63`); asserts
  clean does not emit the dirty kinds (`:77-81`). It does **not** assert
  `file_metrics` directly.
- **Bridge golden (inline NDJSON):**
  `go-ai-native-lang/v0.1.0/crates/go-ai-native-extract-bridge/src/lib.rs` —
  `EXTRACTOR_SOURCE = include_str!("../../../tools/go-extract/extract.go")`
  (`:156`); an inline `REPLAY` NDJSON const is replayed by
  `replay_parses_and_lowers_into_engine_facts` (`:335`) asserting
  `records.len()==2`, `facts.len()==6`, exact `file_metrics`/`import`/`go_unsafe`/`item`/`markers`
  records. **Adding a new fact kind changes `facts.len()` and the inline golden.**
- **Driver test module:** `go-ai-native-conform/src/lib.rs:205` `mod tests`
  (`#[test]` `:213`); `count_by_rule` is used for the run summary
  (`go-ai-native-conform/src/lib.rs:147,186`), not as an assertion.
- **Baseline golden:** `research/go-demo/go-ai-native-conform-baseline.json`.
- **TCG parity:** `go-ai-native-tcg` crate
  (`go-ai-native-lang/v0.1.0/crates/go-ai-native-tcg/`), `src/lib.rs` + test
  module; `[go].seams_pkg` read at `go-ai-native-tcg/src/lib.rs:299`.

### TypeScript stack

- **Extractor fixtures:**
  `typescript-ai-native-lang/v0.6.0/tools/ts-extract/test/fixtures/{clean,dirty,seam}/`
  (clean: `src/cells/{greet,parse}/index.ts`, `src/core/text.ts`; dirty:
  `src/cells/greet/{index,internal}.ts`, `src/cells/parse/logic.ts`,
  `src/rubble.ts`; seam: `src/errors.ts`); each carries a `conform.toml`.
- **Extractor unit test:** `tools/ts-extract/test/extract.test.ts` —
  `records.length==4` (`:64`); `deepEqual(kinds, […])` exact by-kind array
  (`:84`); `deepEqual(imports, […])` (`:106`); **each record has exactly one
  `file_metrics`** (`metrics.length==1`, `:197-198`); seam `deepEqual(symbols,
  […])` (`:162`).
- **Bridge golden (inline NDJSON):**
  `typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-extract-bridge/src/lib.rs`
  — `const REPLAY` (`:268`) replayed by `replay_parses_and_lowers_into_engine_facts`
  (`:282-288`, `records.len()==2`, `facts.len()==5`); plus `REPLAY_ENV` (`:304`,
  `:317`) and `REPLAY_SEAM` (`:343`, `:356`) goldens. **Adding a new fact kind
  changes `facts.len()` and these inline goldens.**
- **Driver test module:** `typescript-ai-native-conform/src/lib.rs:209` `mod tests`
  (`#[test]` `:217, :265, :300`); `count_by_rule` for the run summary
  (`:151, :190`), not an assertion.
- **Baseline golden:** `research/ts-demo/typescript-ai-native-conform-baseline.json`.
- **TCG parity:** `typescript-ai-native-tcg` crate
  (`typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-tcg/`).

### Tests that break when a new rule / new fact lands (the counters)

- **New fact variant in the extractor** (the Q4 need): the two **inline bridge
  replay goldens** (Go `go-ai-native-extract-bridge/src/lib.rs:335` asserting
  `facts.len()==6`; TS `typescript-ai-native-extract-bridge/src/lib.rs:288/317/356`
  asserting `facts.len()==5`) and the **extractor unit tests** (TS
  `extract.test.ts:84` by-kind `deepEqual`, `:197-198` one-`file_metrics`-per-file;
  Go `extract_test.go` by-kind) — these hard-assert the record shape.
- **New rule that fires on existing code:** the **frozen baselines** (host
  `conform-baseline.json`, `research/{rust,go,ts}--demo/*-baseline.json`,
  `packages/org.vibevm.fractality/fractality/v0.1.0/conform-baseline.json`) gain
  fingerprints; by design these are re-frozen via `run_freeze`, not "broken"
  (`rust-ai-native-conform/src/lib.rs:187-189`).
- **Engine per-rule counts:** `core-ai-native-conform/src/rules/tests.rs` exact
  counts break if an existing rule's output changes; a new rule adds its own
  unit test there (and its own doctest, as `FileLength` does at `budget.rs:125`).
