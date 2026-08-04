# E13-B035-PARITY — parity-audit loop, pass after batch 3

Read-only parity audit for `BACKLOG.md {#b-035}` (owner principle, now
discipline law: manifesto `##PARITY-ACROSS-PROJECTIONS` — “no language
projection enforces the discipline more weakly than another; a gap carries a
recorded reason, never silent”). This pass re-cuts the table **by the fact of
the tree** after wave-B batch 3 landed (three new engine rules — invariant
position, computed cell names, declared test matrices — plus the TS custom-lint
layer of B-037). The previous cuts are pass №1
`harvest/e10-b035-parity-pass.md` and pass №2 `harvest/e12-b035-parity-pass.md`;
only the CHANGED / NEW rows are re-stated here, the rest hold at pass №1/№2.

Every cell carries `path:line` (relative to the worktree root, on the
worktree’s own non-vendored copies under `packages/org.vibevm.ai-native/...`).
“Unchanged” is a claim too — it was verified by reading, not by memory.

**Headline.** Batch 3 landed three engine rules and the TS half of B-037.
Against the parity bar the picture is now sharper than pass №2’s
“recorded-honest but not build-complete”:

- **Two new mechanisms reach full parity** — `invariant-comment-position`
  (B-036) and `declared-test-matrices` (R-060): one engine rule each, mounted
  in **all three** drivers, fed by a fact **all three** extractors emit, no
  language weaker.
- **One is built-and-recorded** — `cell-name-is-computed` (B-038): one engine
  rule mounted in Rust **and** Go (closing a real Go gap — Go practised the
  convention with no machine check), TypeScript out with a recorded reason
  (no cell manifest to compute from).
- **One is an inversion, and it is recorded** — B-037’s custom-lint layer:
  TypeScript **built** it (`eslint-plugin-ai-native`); Rust and Go did not,
  each with a recorded reason (dylint needs nightly, the project pins stable;
  Go names no vehicle at all). The manifesto’s
  `##PARITY-PILOT-IS-A-BAR-NOT-A-PRIVILEGE` says the bar rises to the
  language that matured past the pilot — so here Rust and Go are the weaker
  cells, and per `##PARITY-GAP-IS-NEVER-SILENT` the absence carries a recorded
  reason and a named route (`BACKLOG.md {#b-050}`), never silence.
- **The two Go-only content gaps of pass №2 survive batch 3 unchanged** — row
  6 (Go flag/registry rule) and rows 8/12 (Go-floor `./...` residual). Batch 3
  touched the conform engine and the TS tool layer; it did not touch the Go
  flag rule or the Go floor. Verified by reading, below.

**M-PARITY verdict (stated plainly).** M-PARITY = “the table shows no language
cell weaker than Rust without a recorded reason.” After batch 3 the literal bar
**IS met**: every weaker cell now carries a recorded reason — the two surviving
Go gaps (row 6: `registry_pkg` “carries no rule”; rows 8/12: routed
verify/build), and the new inversion (Rust/Go custom-lint vehicles, routed
`{#b-050}`). No cell is weaker in silence. What remains is **build-completion
of the recorded gaps**, not silencing them: the Go flag rule (row 6), the Go
floor `./...` scoping (rows 8/12), and the Rust/Go custom-lint vehicles
(`{#b-050}`, P3, owner-ruled “don’t build now, don’t drop the promise”). So:
**M-PARITY (recorded-reason bar) reached; M-PARITY (build-complete) not yet.**

---

## Rows that are NEW or CHANGED against pass №2

| # | Mechanism | Rust | TypeScript | Go | Delta vs pass №2 | Verdict |
|---|---|---|---|---|---|---|
| 14 | Invariant-comment position (B-036 — a marker comment buried in a file’s middle third) | **yes.** `InvariantCommentPosition` `core-ai-native-conform/src/rules/position.rs:74-176` (id `invariant-comment-position` `:88`, middle-third `lines/3 < l <= 2·lines/3` `:119-120`, ordinal fingerprint `:166-169`); mounted `rust-ai-native-conform/src/lib.rs:85-88`; the Rust frontend’s raw-text scan emits `Fact::InvariantComment` `rust-ai-native-conform-frontend/src/lib.rs:301` | **yes.** same rule, mounted `typescript-ai-native-conform/src/lib.rs:73-76`; the TS bridge lowers its `RawFact::InvariantComment` into the engine fact `typescript-ai-native-extract-bridge/src/lib.rs:276` | **yes.** same rule, mounted `go-ai-native-conform/src/lib.rs:80-83`; the Go bridge lowers it `go-ai-native-extract-bridge/src/lib.rs:335` | **NEW.** Two root config keys feed all three identically — `invariant_comment_markers` (default the five labeled tags `INVARIANT:`/`WARNING:`/`PANICS:`/`MUST:`/`NEVER:`) `core-ai-native-conform/src/config.rs:118` (`:149-155`) and `invariant_comment_min_file_lines` (default 120) `config.rs:123` (`:156`); root because the vocabulary is language-neutral (`config.rs:95-97`). The fact variant is `facts.rs:195-199`. Card `rule-position-is-a-resource` authored (status done) | **PARITY ACHIEVED** |
| 15 | Computed cell name (B-038, fork №1 — a cell’s type name is `Pascal(variant)` + the seam as written) | **yes.** `CellNameIsComputed` `core-ai-native-conform/src/rules/naming.rs:86-153` (id `cell-name-is-computed` `:90`, compose `Pascal(variant)+seam` `:124`, verbatim seam never re-cased `:42-44`, fingerprint by file+declared name `:146`); mounted `rust-ai-native-conform/src/lib.rs:78`; reads `#[cell(seam, variant)]` lowered verbatim into the `cell(...)` attr | **recorded reason (out of rule).** TS has no cell manifest, so there is nothing to compute — stated in the rule itself `naming.rs:10-11` and the card `rule-closed-vocabulary-naming.md:33`, held by the parity law `…00-MANIFESTO#PARITY-GAP-IS-NEVER-SILENT`, not in silence. `CellNameIsComputed` is NOT in the TS `build_rules` `typescript-ai-native-conform/src/lib.rs:48-79` | **yes (closes a Go gap).** same rule, mounted `go-ai-native-conform/src/lib.rs:76` (comment `:71-75`: one engine rule reads both); the Go bridge renders `//spec:cell seam= variant=` into the SAME `cell(seam=…, variant=…)` attr string `go-ai-native-extract-bridge/src/lib.rs` (the single place the rust notation is born for Go) | **NEW.** Go practised `{Variant}{Seam}` by hand with **no machine check anywhere** (card `:33`, design `spec/design/new-rule-classes.md:183-188`); the build closes that gap in the same move rather than creating an asymmetry. Card `rule-closed-vocabulary-naming` authored (status done); honest non-goal: composition only, the closed-vocabulary/one-referent/no-synonym halves of R3-004 stay unbuilt (`naming.rs:25`-equivalent, card `:25`) | **Rust+Go BUILT; TS RECORDED** |
| 16 | Declared test matrices (R-060, B-038 — a test matrix is declared as data, never a `2^n` sweep) | **yes.** `DeclaredTestMatrices` `core-ai-native-conform/src/rules/matrices.rs:78-142` (id `declared-test-matrices` `:82`, bitmask + nested-loops arms `:106-128`, ordinal fingerprint `:135`); mounted `rust-ai-native-conform/src/lib.rs:97`; the Rust frontend emits `Fact::TestSweep` in test context `rust-ai-native-conform-frontend/src/lib.rs:566` | **yes.** same rule, mounted `typescript-ai-native-conform/src/lib.rs:77`; the TS bridge lowers `RawFact::TestSweep` `typescript-ai-native-extract-bridge/src/lib.rs:281` | **yes.** same rule, mounted `go-ai-native-conform/src/lib.rs:84`; the Go bridge lowers it `go-ai-native-extract-bridge/src/lib.rs:340` | **NEW.** One engine rule serves all three (card `rule-declared-test-matrices.md:23`, design `new-rule-classes.md:197-204`); the fact is `facts.rs:216-220`. The line drawn is **WHAT THE LOOP ITERATES**: a generated numeric range sweeps (`bitmask` `1<<n` / `2**n` / `math.Pow(2,n)`, or `nested-loops` ≥3-deep of range/C-style-for); a loop over a DECLARED collection/array/constant does NOT count, so exhausting a closed set by nesting collection loops is compliant (`matrices.rs:36-42`, card `:31`, commit `4f53e053`). Every frontend emits only in test context, so the rule needs no path filter of its own. Card authored (status done); R-060 | **PARITY ACHIEVED** |
| 17 | Custom REQ-citing lint layer (B-037 — the third Scaffold-F channel: own lints whose messages name the rule + remedy) | **recorded reason (inversion — Rust is now the weaker cell).** No `dylint`/`declare_lint`/`LateLintPass`/`rustc_private` anywhere in source (grep over `packages/…/**/*.{rs,toml,go}` → 0 matches). Reason recorded: `dylint` links rustc internals behind `#![feature(rustc_private)]` and cannot build on the pinned `stable`; the custom-**check** layer that DOES exist is the conform engine itself (19 call sites of the one `req_message` renderer), so the grammar is honoured — what is genuinely missing is a **type-aware** vehicle, an owner toolchain-policy decision routed `BACKLOG.md {#b-050}` (`spec/design/new-rule-classes.md:135-148`, `:226-227`) | **yes (built — the inversion source).** `typescript-ai-native-lang/v0.6.0/tools/eslint-plugin-ai-native/` ships rule `diagnostic-cites-req` via `ESLintUtils.RuleCreator` `src/diagnostic-cites-req.ts:62,139-145`; `src/req-message.ts:19-30` reproduces the engine’s `violates REQ <uri>: <why>; fix surface: <where>` grammar exactly; `RuleTester` battery `test/diagnostic-cites-req.test.ts`; wired through the consumer’s flat config, no floor change (`package.json:6`, design `new-rule-classes.md:123-133`) | **recorded reason (inversion — Go is now the weaker cell).** No `analysis.Analyzer`/`flag.Analyzer` custom lint in source (same grep → 0 matches). Reason recorded: the Go guide promises **no vehicle at all** — only “custom checks emit the same grammar”; the floor already distributes single-binary analyzers (`staticcheck`, `exhaustive`), so the shape is exercised, but the decision that Go’s carrier is an `analysis.Analyzer` is named-not-taken, routed with the Rust half `BACKLOG.md {#b-050}` (`spec/design/new-rule-classes.md:150-155`, `:1060`) | **NEW, and it inverts the usual direction.** TS matured past Rust/Go on this axis; `##PARITY-PILOT-IS-A-BAR-NOT-A-PRIVILEGE` (`00-MANIFESTO.md:101`) says the bar rises to it, so Rust and Go are the weaker cells — and per `##PARITY-GAP-IS-NEVER-SILENT` (`:103`) each carries a recorded reason + named route, never silence. The Go guide’s Scaffold-F clause already reads “Two of the three channels are built” `GUIDE-AI-NATIVE-GO.md:295-298` | **TS BUILT; Rust/Go RECORDED (open debt `{#b-050}`)** |

---

## Rows unchanged, still OPEN — re-verified by reading (batch 3 did not touch them)

The packet’s instruction was to confirm these by reading, not memory. Both
survive batch 3 exactly as pass №2 left them — batch 3 added engine rules and a
TS tool package; it did not touch the Go flag rule or the Go floor.

- **Row 6 — the Go flag/registry rule (unchanged).** Rust `FlagSites` (R-001)
  + TS `ts-flag-sites` exist; Go still has neither. The Go `build_rules`
  (`go-ai-native-conform/src/lib.rs:51-86`) mounts `GoUnsafeInDomain`,
  `GoSeamErrorCitesReq`, `GoCellIsolation`, `GoConformanceAssertion`,
  `CellNameIsComputed`, `FileLength`, `InvariantCommentPosition`,
  `DeclaredTestMatrices` — **no flag rule**. The `registry_pkg` config field
  still says “carries no rule” (`core-ai-native-conform/src/config.rs:268,271`,
  field `:273`, default `None` `:289`). Recorded not silent; straight build,
  routed to a later batch (the Go §6 promise is the remaining large asymmetry).
- **Rows 8 / 12 — the Go floor `./...` residual (unchanged).** `gofmt` is
  `exclude_substrings`-scoped (B-003): `go-ai-native-cli/src/floor.rs:103-109`
  + `filter_gofmt_listed` `:61-70`. `vet`/`tests`/`staticcheck` still walk
  `./...` unfiltered — `go vet ./...` `:134-136`, `go test ./...` `:146-148`,
  `staticcheck ./... && exhaustive ./...` `:158-171`. The B-048 TS twin of
  B-003 and this Go residual are siblings; verify/build routed. Recorded not
  silent.
- **Rows 1, 7, 13 — closed by pass №2** (seam-error REQ-citation both halves ×
  3; conformance-assertion Go-built/Rust-compiler-reason/TS-routed; floor
  disable ×3 via B-049). Unchanged this pass; hold at pass №2.
- **Rows 2–5, 9–11 — the infrastructure + record-reason rows.** Unchanged;
  hold at pass №1 (rows 2–5 parity-achieved; rows 9–11 load-bearing language
  differences recorded in the guides).

---

## Card-index coordination — one honest gap (documentation, not enforcement)

The packet states “индексы Go/Rust согласованы” as a map; verified by reading,
it holds for two of the three new cards and slips on the third — a
card-**registry** coordination gap, not a discipline-enforcement gap (the
checker is mounted either way):

- **Rust index** lists all three new cards as shipped:
  `rust-ai-native-lang/v0.7.0/spec/cards/INDEX.md:26-28` (and `:53` confirms
  the three checkers are live).
- **Go index** lists `rule-position-is-a-resource` (`INDEX.md:38`) and
  `rule-closed-vocabulary-naming` (`:37`), each pointing to the Rust-authored
  card as the shared rule’s lineage home — coordinated. **But
  `rule-declared-test-matrices` is NOT listed** in the Go index (no row in the
  Rule-cards table `:35-38`, absent from the pending list `:89-99`), even
  though Go mounts `DeclaredTestMatrices` (`go-ai-native-conform/src/lib.rs:84`)
  and the Rust card explicitly states it “serves Go and TypeScript too”
  (`rule-declared-test-matrices.md:5,23`). The enforcement is equal; the index
  entry is missing.
- **TS index** is the stalest: it still lists `rule-position-is-a-resource`
  and `rule-closed-vocabulary-naming` as **pending** “candidate future card”
  (`typescript-ai-native-lang/v0.6.0/spec/cards/INDEX.md:53,57`) even though TS
  now mounts `invariant-comment-position` (`typescript-ai-native-conform/src/lib.rs:73-76`); `rule-declared-test-matrices` is absent entirely. (The naming
  card legitimately staying pending for TS is consistent — TS is outside that
  rule by recorded reason.) The TS lint layer does appear in the TS checker
  surface prose (`INDEX.md:29` names `diagnostic-cites-REQ`).

These are navigation/registry doc gaps. They do not weaken any language’s
**enforcement** (every checker that should mount, mounts), so they are outside
the B-035 parity bar — but they are surfaced here, not left silent, per the
same recorded-not-silent posture the audit enforces.

---

## M-PARITY status

M-PARITY = “the B-035 table shows no language cell weaker than Rust without a
recorded reason.” After batch 3:

- The three new mechanisms are at parity (position, matrices) or
  built-and-recorded (naming: Rust+Go built, TS recorded) — no weaker cell
  there.
- The one inversion batch 3 introduces (custom-lint layer: TS built, Rust/Go
  not) is itself **recorded** (`spec/design/new-rule-classes.md` §3 +
  `BACKLOG.md {#b-050}`) — the manifesto’s own words: “the checker is built or
  the reason is recorded, never the rule quietly relaxed”
  (`00-MANIFESTO.md:103`).
- The two surviving Go gaps (row 6, rows 8/12) carry their recorded reasons
  exactly as in pass №2.

**So the literal recorded-reason bar IS met: no language cell is weaker than
Rust in silence.** What remains is **build-completion of the recorded gaps**,
all routed, none silent:

1. Row 6 — the Go flag/registry rule (straight build; the Go §6 promise).
2. Rows 8/12 — the Go-floor `vet`/`tests`/`staticcheck` `./...` scoping (the
   B-048 sibling).
3. `{#b-050}` — the Rust `dylint` and Go `analysis.Analyzer` custom-lint
   vehicles (P3, owner-ruled “add to BACKLOG with low priority; do not build
   now, do not drop the guide promise”).

Net: pass №2’s “recorded-honest but not build-complete” sharpens to
**“M-PARITY (recorded-reason bar) reached; M-PARITY (build-complete) not yet”**
— batch 3 closed no recorded gap with a build on the Go side, but it added no
silent gap either, and the inversion it created is recorded. The three
builds above are what stand between recorded-honest and build-complete.
