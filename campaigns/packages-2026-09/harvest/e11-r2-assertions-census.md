# E11-R2-ASSERTIONS — conformance-assertion-presence census (B-030)

Read-only census for `BACKLOG.md {#b-030}` (the "conformance assertion is
present" check — build it for Go, survey Rust and TS). It is the input to
both halves of the build: (a) a Go syntactic scan for the assertion, and
(b) a Rust/TS survey of what their guides promise about
assertions/registration, what their gates already check, and what would
need detecting. The parity frame for this row is
`campaigns/packages-2026-09/harvest/e10-b035-parity-pass.md` row 7
("Conformance-assertion presence … none at all three").

Every claim carries `path:line`. "Not found" is stated as an explicit
fact. No design recommendations.

## Path convention

Paths are worktree-relative on the non-vendored originals under
`packages/org.vibevm.ai-native/`. The shared engine is
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/` (the v0.8.0
release is the one that carries `rules/go.rs`; v0.7.0 does not). Each
language stack vendors a byte-identical copy of the engine under its own
`crates/vendor/core-ai-native-conform/` — line numbers cited against
`core-ai-native/v0.8.0/...` match those vendored copies. Language-specific
files (rosters, guides, extractors, bridges) are cited under their own
stacks: `go-ai-native-lang/v0.1.0/`, `rust-ai-native-lang/v0.7.0/`,
`typescript-ai-native-lang/v0.6.0/`.

## Q1 — Go idiom live; what the Go guide promises about the assertion and its gate check

### Live `var _ <Iface> = (*Type)(nil)` occurrences (the idiom, as written)

In the Go consumer demo (`research/go-demo`):

- `research/go-demo/internal/cells/batchplanner/planner.go:14` —
  `var _ seams.Planner = (*BatchPlanner)(nil) // silent conformance made loud`
- `research/go-demo/internal/cells/naiveplanner/planner.go:15` —
  `var _ seams.Planner = (*NaivePlanner)(nil) // silent conformance made loud`
- `research/go-demo/internal/sim/world.go:17` —
  `var _ seams.Store = (*World)(nil) // silent conformance made loud`
- `research/go-demo/internal/sim/world.go:89` —
  `var _ seams.Clock = (*FixedClock)(nil)`

The guide's own canonical example (the `— MUST` form):

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:201` —
  `var _ seams.Planner = (*BatchPlanner)(nil) // silent conformance made loud — MUST`

Reinforced in the package boot and scaffold cards (prose, same idiom):

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/boot/20-stack-go-ai-native-lang.md:32` —
  "Every cell carries `var _ Seam = (*Impl)(nil)`."
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/cards/INDEX.md:76` —
  "The loud-conformance assertion (`var _ Seam = (*Impl)(nil)`, guide §2) rides card".
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/cards/scaffold-b-typed-builders.md:21` and `:44` —
  "Conformance assertion (`var _ Seam = (*Impl)(nil)` — structural typing made loud, guide §2)" / "Add the loud-conformance assertion `var _ Seam = (*Impl)(nil)` beside the impl."

### What the Go guide verbatim promises about the assertion and the gate

The load-bearing promise (and its honest self-annotation) is one block:

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:191-192` —
  `##CONFORMANCE-IS-MADE-LOUD`: "Conformance is made loud — every cell
  carries the compile-time assertion, and conform checks its presence
  (T-syn)" — then, in the same row: "*Specified, not built (→ B-030): the
  Go gate registers exactly three rules — `GoUnsafeInDomain`,
  `GoCellIsolation`, `FileLength` (`build_rules`, `lib.rs:53-60`) — and
  none parses for the assertion's presence; the assertion itself is real,
  idiomatic Go and the pattern below is correct.*"

So the guide PROMISES "conform checks its presence (T-syn)" and
simultaneously annotates that promise as not-yet-built (pointing at
B-030). The motivating gap is stated one section up:

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:97-98` —
  `##GAP-CONFORMANCE-IS-SILENT`: "Interface conformance is silent.
  Structural satisfaction means a cell can drift off its seam without a
  compile error naming the seam. Conformance is made loud (§2)."

And the envelope row names the property at the top of the guide:

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:27-28` —
  `##ENVELOPE-LOUD-CONFORMANCE`: "loud interface conformance".

The codemod that scaffolds a new cell emits the assertion as part of its
checked skeleton:

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:302-303` —
  "`go-ai-native codemod add-cell` emits a cell skeleton (package,
  conformance assertion, directive tags, registry arm, Example stub) as
  ONE checked operation."

### A near-miss the Go rule's pattern must not match

The Go codemod writes two blank-identifier var lines into generated
test scaffolds that are NOT conformance assertions (they keep a symbol
referenced from tests), so a syntactic scanner must be precise about the
`= (*Type)(nil)` form:

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-cli/src/codemod.rs:61` —
  `var _ = New // the constructor is the surface (GUIDE §2)`
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-cli/src/codemod.rs:63` —
  `var _ *{type_name} // keep the type name referenced from tests`

### The extractor's own fixtures do not carry the assertion

A fact worth recording: the go-extract clean and dirty fixtures — the
extractor's own test corpus — contain no `var _` assertion, so the
pattern is unexercised by the corpus that would regression-test a new
emit:

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/clean/internal/cells/greet/greet.go:1-41` —
  no `var _`; the only interface is a consumer-side narrow one (`type clock interface`, :11).
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:1-52` —
  no `var _`.

## Q2 — go-extract under the task: facts today, the var-_ skip, the ts_env_read wire, the Go analog, the minimal missing record

### Fact kinds go-extract emits today

`extract.go` emits exactly two fact tags (the struct at
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/extract.go:46-51`):

- `go_unsafe` — `fact{Fact: "go_unsafe", Kind, Line}` at
  `extract.go:412`; `Kind` is one of the ban-census kinds
  (`init_decl`, `blank_import`, `ambient_call`, `naked_go`,
  `error_string_match`, `t_skip`, `reasonless_suppression`,
  `seam_error_missing_req` — per the engine docstring at
  `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/facts.rs:119-128`).
- `item` — at `extract.go:431` (func/method), `:449` (type), `:467`
  (var/const); carries `Kind` ∈ {`func`,`method`,`type`,`var`,`const`},
  `Symbol`, `IsExported`, `HasDocExample`, and for `kind=type` a
  `Underlying` brand field.

There is no third tag.

### go-extract sees var-declarations but actively SKIPS the blank-identifier one

The `ValueSpec` (var/const) branch in `genItems` skips every name `_`
with the comment that names the very pattern at issue:

- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/extract.go:461-463` —
  ```
  if name.Name == "_" {
      continue // conformance assertions et al.
  }
  ```

So today go-extract does not see `var _ seams.X = (*Impl)(nil)` as a
fact at all — it discards it before emitting. (The brand half of the
type story IS seen: `primitiveUnderlying` at `extract.go:479-495`
records `type AccountID string` → `"string"` on the `item`/`type` fact;
a type-shape fact can therefore flow through — but no fact is emitted
for the blank-var conformance assertion.)

### The minimal missing record, modeled on the ts_env_read wire

The packet's template is the TS `ts_env_read` pathway. It exists
end-to-end today and is the shape a Go conformance wire would copy:

1. **Extract** — the TS extractor emits the fact:
   `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/tools/ts-extract/extract.ts:89` (type def `fact: "ts_env_read"`) and `:391` (the emit); exercised by the test at
   `tools/ts-extract/test/extract.test.ts:104-107`.
2. **Bridge / parse** — the TS bridge has a `RawFact::TsEnvRead { source, line }` variant (`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-extract-bridge/src/lib.rs:94`) mapped to the engine fact at `:231`.
3. **Fact** — engine `Fact::TsEnvRead { source, line, in_test }` at
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/facts.rs:146-150`.
4. **Rule** — `TsFlagSites` consumes it: the `let Fact::TsEnvRead { .. } = fact else { continue }` match at
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/typescript.rs:310`; the rule is mounted only when `[typescript] composition_root` is set.

The Go analog that already exists (the wire to model a Go conformance
fact on) is `go_unsafe`:

1. **Extract** — `extract.go:412` emits `go_unsafe`.
2. **Bridge** — Go `RawFact::GoUnsafe { kind, line, reason }` at
   `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-extract-bridge/src/lib.rs:70-75`, mapped to the engine fact at `:260`.
3. **Fact** — engine `Fact::GoUnsafe { kind, line, in_test, reason }` at
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/facts.rs:129-134`.
4. **Rule** — `GoUnsafeInDomain` consumes it (`rules/go.rs`).

**What is absent for a Go conformance wire (the gap):**

- The engine `Fact` enum has no conformance-assertion variant — full
  enum enumerated at
  `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/facts.rs:25-151`;
  the variants are `Item`, `Import`, `Ctor`, `UnsafeUse`,
  `ErrorVariant`, `FileMetrics`, `UnwrapUse`, `EnvRead`, `TsUnsafe`,
  `GoUnsafe`, `TsEnvRead`. None carries a "type asserts it satisfies a
  seam" signal for any language.
- The Go bridge `RawFact` enum has no such variant — full enum at
  `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-extract-bridge/src/lib.rs:69-96`;
  variants are `GoUnsafe`, `Import`, `Item`, `FileMetrics`.
- `extract.go:461-463` discards the assertion before emitting (above).

**Minimal new record kind (by analogy, the asked-for shape):** a new
extract emit from the `ValueSpec` branch (before/around the `_` skip)
recognising `var _ <seam> = (*<Impl>)(nil)` and carrying at minimum the
seam interface name and the impl type name plus line — flowing through a
new `RawFact` variant in the Go bridge into a new engine `Fact` variant,
consumed by a new Go rule. This is a four-site change (extract → bridge
→ enum → rule), exactly the `ts_env_read` / `go_unsafe` shape; no
shorter path exists because no existing fact carries the seam↔impl
relationship. (Note: the engine docstring at `facts.rs:5-8` states that
adding a variant bumps the frontend version and retires old cache slots
— a side-effect a new variant carries.)

## Q3 — Rust promises vs reality; host-crate compile-time assertion patterns

### What the Rust guide promises about registration / assertion / seams

- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:51` —
  `##ONE-CELL-ONE-REGISTRATION-POINT`: "One cell, one registration point.
  Cells import seams + core only, never sibling cells (R-002)."
- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:53` —
  `##OWNERSHIP-ALIGNS-WITH-FILE-BOUNDARIES`: "one cell = one file-set
  with a single registration point."
- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:68` —
  `##SCAFFOLD-B-TYPED-BUILDERS`: "seam protocols are encoded in types,
  not docstrings; the wrong call fails `cargo check`, not a runtime
  assert (R3-008; 94% of compile errors are type-level)."
  This is the Rust answer to "conformance is made loud": the compiler is
  the assertion — a cell type that does not satisfy its seam trait fails
  `cargo check` at the use site.
- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:69` —
  `##SCAFFOLD-C-RUNNABLE-CONTRACTS`: "every load-bearing invariant is
  witnessed by a runnable assertion or proof where it is relied upon."

The Rust guide has NO analogue of the Go `##CONFORMANCE-IS-MADE-LOUD`
row that promises a gate rule checks a written assertion — a grep for
`loud`/`silent`/`drift`/`cargo check`/`compile error` across the guide
returns only `:68` (cargo check is the verifier), `:113` (drift
detectable via spec-rev bumps), `:123` (goldens fail loudly), `:148`
(the oracle is the complement to the `cargo check` loop). None promises
a conform rule checks trait-satisfaction.

### What the Rust gate already checks, and what nobody checks

The Rust roster is
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:53-93`
(11 rules). The three the packet names:

- `R-002` `CellIsolation` —
  `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/structure.rs:87-146`.
  It keys on `Fact::Import` (the import graph) and flags a cell module
  that imports a sibling cell module (`:108-141`). This enforces the
  STRUCTURE of the registration point (cells import seams + core only);
  it does NOT verify the cell type implements its seam.
- `cell-has-oracle` `CellHasOracle` —
  `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/structure.rs:148-180`.
  It keys on `cell_types` (items with `#[cell(...)]` attrs, i.e.
  `Fact::Item`) and checks each is referenced from an integration test
  (`:177`). It does NOT verify seam-satisfaction.
- `R-001` `FlagSites` —
  `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/structure.rs:26-75`.
  It keys on `Fact::Ctor` (`:49`) — construction sites — not seam
  satisfaction.

Not found: no Rust rule keys on "this cell type implements/satisfies its
seam". A grep for `conformance`/`assert`/`satisf`/`seam.*impl` over
`rules/structure.rs` and `rules/mod.rs` returns only doctest
`assert_eq!` lines and a `tests.rs` comment — no conformance check. The
parity pass records the same: `e10-b035-parity-pass.md` row 7 — Rust =
"none" for conformance-assertion presence.

The Rust verdict is therefore unlike Go's: the "type satisfies seam"
question is answered by `cargo check` (guide `:68`), not by a gate rule,
and the gate rules enforce the registration-point STRUCTURE (`R-002`)
and the oracle (`cell-has-oracle`) instead.

### Host-crate compile-time assertion patterns (the `var _` analogues)

A grep for `const _:`/`static_assertions`/`let _: &dyn`/`assert_impl`
over `crates/` and `xtask/` finds exactly one site:

- `crates/vibe-mcp/src/transport.rs:141-147` —
  ```
  const _: () = {
      fn _assert_read<R: Read>() {}
      fn _check() {
          _assert_read::<std::io::Cursor<Vec<u8>>>();
      }
  };
  ```
  This is a Rust compile-time trait-bound assertion (asserts
  `std::io::Cursor<Vec<u8>>: Read`) — the genuine `var _` analogue — but
  it is transport-internal plumbing, not a cell/seam assertion.

Not found: no `static_assertions` crate use anywhere in the host tree;
no `let _: &dyn <Trait> = &<Impl>` pattern; no cell-level
`const _: ()` asserting a cell type satisfies its seam trait. So the
idiom exists in the host tree exactly once and is unrelated to cells.

## Q4 — TS promises vs reality; ts-demo live patterns; what would need detecting

### What the TS guide promises about branded seams / registration / conformance

The TS "conformance made loud" is a suite of mechanisms, not one idiom:

- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:130` —
  `##STRUCTURAL-TYPING-TRAP`: "TypeScript must recover [nominal safety]
  manually through branding … identifiers and other meaning-bearing
  primitives crossing a seam are branded
  (`type UserId = string & { readonly __brand: 'UserId' }`, or a
  branding helper) so the wrong same-shaped value fails `tsc`."
- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:137` —
  `##SCAFFOLD-B-TYPED-SURFACES`: "Branded types for nominal safety …
  `satisfies` for exhaustiveness; sealed unions … seam protocols are
  encoded in types, not docstrings (R3-008)."
- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:154` —
  `##EXHAUSTIVENESS-OVER-E-IS-ENFORCED`: "Exhaustiveness over `E` is
  enforced by a `satisfies never` / `assertNever` check in the default
  branch."
- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:175` —
  `##FLAG-REGISTRY-IS-TYPED-DATA-WITH-PROVENANCE`: the registry is "a
  branded or `as const` table, not stringly-typed ambient lookup" —
  built (B-039), demo at `research/ts-demo/src/main.ts`.
- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:233` —
  `##TYPE-LEVEL-TESTING-IS-TYPESCRIPT-UNIQUE`: "TypeScript can assert
  type relationships at compile time".
- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:235` —
  `##TYPE-LEVEL-TEST-TOOLING`: "`expectTypeOf<X>().toEqualTypeOf<Y>()`
  (vitest), `tsd`'s `expectType`, and `@ts-expect-error` as a negative
  assertion".
- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:237` —
  `##RULE-PUBLIC-SURFACES-CARRY-TYPE-LEVEL-TESTS`: "public
  generic/branded/union surfaces carry type-level tests asserting their
  key relationships; these run in the Class E loop (a type-level test
  that regresses fails `tsc`)."

### What the TS gate checks today

The TS roster is exactly four rules at
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-conform/src/lib.rs:50-69`:
`TsUnsafeInDomain` (always), `TsCellIsolation` (conditional on
`cells_dir`), `TsFlagSites` (conditional on `composition_root`), and
`FileLength` (always). None checks branding presence,
`satisfies`-exhaustiveness, or type-level-test presence.

The TS extractor emits five fact kinds — full `RawFact` enum at
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-extract-bridge/src/lib.rs:67-99`:
`TsUnsafe`, `Import`, `Item`, `FileMetrics`, `TsEnvRead`. The `Item`
fact (`:80-85`) carries no brand field at all (contrast the Go `Item`
which carries `underlying` at `go-ai-native-extract-bridge/src/lib.rs:86-91`).
There is no fact for a branded type, a `satisfies` check, or a
type-level test.

The TS gate DOC is honest in a way the Go doc was not: a grep of
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/tools/conform-frontend-typescript.md`
for `conformance`/`assert`/`brand`/`satisfies`/`type-level` returns only
`:25` (bans-as-facts) and `:73`
(`##NATIVE-TYPE-TOOLING-IS-REAL-TODAY`: "the native type tooling (the
Class-E `tsc` loop, the type-level test tools) is real and usable today;
what waits is the *structural* gate"). There is NO row promising the
gate checks branding/satisfies/type-tests. The TS slack (per B-035
first-pass) is in the engine, not the doc.

### ts-demo live assertion patterns

The branded-type idiom is live in the demo:

- `research/ts-demo/src/cells/greeting/index.ts:9` —
  `export type GuestName = string & { readonly __brand: "GuestName" };`
  (the brand), with the brand constructor at the erasure boundary
  `research/ts-demo/src/cells/greeting/index.ts:64` —
  `return { ok: true, value: cleaned as GuestName };`.
- `research/ts-demo/src/main.ts:19-22` — the typed `as const` registry
  dispatch table (the provenance `##FLAG-REGISTRY-IS-TYPED-DATA-WITH-PROVENANCE` names).

Not found in ts-demo code: no `satisfies`, no `expectTypeOf`, no `tsd`,
no `assertNever`, no `@ts-expect-error` (grep of `research/ts-demo/src`
empty). The type-level-test tooling the guide names at `:235`
(`expectTypeOf`/`tsd`) is not even installed —
`research/ts-demo/package.json` devDependencies are `@types/node`,
`eslint`, `prettier`, `typescript`, `typescript-eslint` only (no
`vitest`, no `tsd`, no `fast-check`); the runner is `node --test`.

### What would need detecting (the TS gap surface)

The guide promises the gate-relevant conformance mechanisms; the gate
checks none of them, and the extractor emits no fact for any:

- branded-type presence on a seam-crossing primitive (the §4 `:130`
  promise) — no fact, no rule;
- `satisfies never` / `assertNever` exhaustiveness over the error union
  (§6 `:154` promise) — no fact, no rule;
- type-level tests (`expectTypeOf`/`tsd`/`@ts-expect-error`) on public
  branded/union surfaces (§12 `:237` promise) — no fact, no rule, and
  not demonstrated in the demo.

## Q5 — Rule form: id class, rules/go.rs mounting, and the warning-not-gate (severity) mechanics

### id class

Go-native rule ids are kebab-case, language-prefixed:

- `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/go.rs:72` —
  `"go-unsafe-in-domain"`
- `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/go.rs:232` —
  `"go-cell-isolation"`

Shared/legacy engine ids mix legacy codes and kebab (`R-001`, `R-002`,
`cell-has-oracle`, `file-length`, `ts-flag-sites`). A new Go
conformance rule would by this convention take a `go-*` kebab id.

### rules/go.rs shape and mounting

A rule is a plain struct `impl Rule` (the trait at
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/finding.rs:53-57`
— methods `id`, `why`, `check` only). Go rules live in
`rules/go.rs`, are re-exported at
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/mod.rs:23`
(`pub use go::{GoCellIsolation, GoUnsafeInDomain};`), and are mounted in
`build_rules` at
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-conform/src/lib.rs:51-60`:

- `GoUnsafeInDomain` pushed unconditionally (`:53`),
- `GoCellIsolation` pushed only when `config.go.cells_dir` is set
  (`:56-58`),
- `FileLength` pushed unconditionally (`:59`).

The mounting site for a new rule is therefore `build_rules` (`lib.rs:51-60`);
the conditional-on-a-config-field pattern (`:56-58`) is the existing
template for a rule that should stay off until its config knob exists
(the same pattern `TsFlagSites` uses at
`typescript-ai-native-conform/src/lib.rs:62-64`).

### Severity / advisory mechanics — there is no per-finding severity

The engine has NO per-finding severity. The `Finding` struct at
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/finding.rs:27-36`
has fields `rule`, `file`, `line`, `message`, `why`, `fingerprint` — no
`severity`, no `level`, no advisory flag. The `Rule` trait
(`finding.rs:53-57`) has `id`/`why`/`check` — no severity method. A grep
for `severity`/`advisory`/`level:`/`warning_only`/`non-block`/`soft_rule`
over the engine `src/` returns one comment only:
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/config.rs:350`
— "(nothing gated, everything advisory)" — which is about the
gated/exempt lists, not a per-finding severity.

"Warning, not blocking gate, at start" (the B-021 lesson) is therefore
expressed through two mechanisms that DO exist, not through severity:

1. **Conditional mounting** — a rule is pushed in `build_rules` only
   when its config field is present (the `cells_dir` / `composition_root`
   pattern above), so a project without the idiom never runs the rule.
2. **The ratchet baseline** —
   `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-conform/src/lib.rs:112-128`
   (`run_check`): `baseline::load` then `baseline::diff(&base, &findings)`
   splits findings into `new` (fingerprint NOT in baseline — these fail
   the gate; the `run_check` doc at `:109` says "any new finding fails")
   and `stale` (baseline fingerprint no longer firing — printed as
   "prune it"). The baseline file is
   `conform-baseline.json` (`{"findings":[fingerprints],"schema":1}`,
   currently `findings: []`); its contract is stated at
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/baseline.rs:11`
   — "The file only shrinks" (the ratchet only tightens). So a new rule
   can land soft by writing its current findings into the baseline: the
   gate stays green, and the ratchet forbids regression while the
   baseline is tightened over time (each cell that gains the assertion
   drops out; a new cell added without the assertion is a `new` finding
   and blocks).

### The B-021 lesson, located

The packet's "урок B-021" reference resolves two places:

- `BACKLOG.md:871` (`##B036-SUT`): "предупреждение — не блокирующий
  гейт на старте (урок B-021)."
- `BACKLOG.md:658-670` (`B-021` itself): `##B021-BUILD` — "Оба порога
  **конфигурируемые**, стартовые значения (3 связи; 120 строки) — честные
  плейсхолдеры до реальной статистики … Оба — предупреждения, не
  блокирующие гейты (по крайней мере на старте)." And `##B021-RULING`
  (`BACKLOG.md:666`): zero hits on our own corpus is not an argument
  against building — "Мы пишем систему для всех, а не только для нас."

`BACKLOG.md` `##B030-FORM` states the intended mechanism for this very
rule: "findings через ratchet-baseline, предупреждение до стабилизации."
