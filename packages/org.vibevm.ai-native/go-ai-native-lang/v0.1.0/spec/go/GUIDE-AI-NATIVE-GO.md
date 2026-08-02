# AI-Native Go — The Guide {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · status: BETA · T2 · supersedes the legacy projection GUIDE-GO-v0.1 (which stays, untouched, in `flow:org.vibevm.ai-native/core-ai-native/spec/legacy-projections/`) · third supported language, after Rust (pilot) and TypeScript** @impl/done

##projection-onto-go *The projection of the Discipline onto Go.* @impl/done

##READ-THE-T1-CORE-FIRST *Read `00-MANIFESTO.md` and
`02-EXECUTABLE-SCAFFOLDS.md` (the T1 core) first; this guide assumes the central law and
the nine scaffold classes.* @impl/done

##STRUCTURALLY-PARALLEL-TO-THE-SIBLING-GUIDES *Structurally parallel to `rust/GUIDE-AI-NATIVE-RUST.md` and
`typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` — cross-language diffing of the guides is a
feature of the discipline.* @impl/done

##CROSS-REFERENCE-NOTATION *Cross-references are marked `(≈ Rust §N)` / `(≈ TS §N)`;
sections with no sibling analogue are marked `[Go-specific]`.* @impl/done

##A-HUMAN-CAN-READ-AI-NATIVE-GO *A human CAN read and modify AI-Native Go; it is ordinary idiomatic Go at the token
level — arguably the least surprising projection of the three, because Go's own culture
already runs half the discipline.* @spec/done

##what-differs-is-the-envelope-lead *What differs is the envelope:* @impl/done

- ##ENVELOPE-CLOSED-ERROR-SETS *closed error sets,* @impl/done
- ##ENVELOPE-LOUD-CONFORMANCE *loud
  interface conformance,* @impl/done
- ##ENVELOPE-OWNED-GOROUTINES *owned goroutines,* @impl/done
- ##ENVELOPE-SPEC-TRACEABILITY *`//spec:` traceability,* @impl/done
- ##ENVELOPE-EXECUTABLE-SCAFFOLDS *executable scaffolds,* @impl/done
- ##ENVELOPE-FAST-PER-CELL-LOOP *and a fast per-cell verification loop.* @impl/done

---

## 0. Why Go is special — and the law applied to Go {#law}

> ##LAW-IDIOMATIC-INSIDE-ENGINEERED-AROUND **Idiomatic inside the file; engineered around the file.** *(≈ Rust §0, TS §0)* @impl/done

##typology-one-line-each-lead The typology, one line each: @spec/done

- ##TYPOLOGY-RUST-ENFORCES Rust *enforces*; @spec/done
- ##TYPOLOGY-TYPESCRIPT-PERMITS-BUT-COMPILES TypeScript *permits but compiles*; @spec/done
- ##TYPOLOGY-PYTHON-TRUSTS Python
  *trusts*; @spec/done
- ##TYPOLOGY-CPP-DEMANDS-A-SUBSET C++ *demands a subset to survive*; @spec/done
- ##TYPOLOGY-GO-PRESCRIBES **Go prescribes**. @spec/done

##discipline-pre-installed-lead The language ships with
its discipline pre-installed: @spec/done

- ##PRE-INSTALLED-GOFMT gofmt ended the formatting war, @spec/done
- ##PRE-INSTALLED-UNUSED-IS-REJECTED the compiler rejects unused
  imports and variables, @spec/done
- ##PRE-INSTALLED-ERRORS-ARE-VALUES errors are values by culture, @spec/done
- ##PRE-INSTALLED-NO-INHERITANCE inheritance does not exist, @spec/done
- ##PRE-INSTALLED-INTERNAL-IS-ENCAPSULATION `internal/` is compiler-enforced encapsulation, @spec/done
- ##PRE-INSTALLED-SINGLE-IDIOM-CULTURE and "idiomatic Go" is the strongest
  single-idiom culture of any mainstream language. @spec/done

##GO-IS-MASSIVELY-IN-DISTRIBUTION Go is also massively in-distribution —
ordinary Go is among the safest surfaces a model can read or write. @spec/done

##prescription-cuts-three-ways-lead That prescription cuts three ways for the Discipline: @impl/done

##ADVANTAGE-1-HALF-THE-ENVELOPE-IS-FREE **Advantage 1 — half the envelope is free.** Uniformity (R3-006) is largely enforced
upstream: one formatter, one vocabulary of idioms, a stdlib that models have seen
millions of times. @spec/done

##GUIDE-SPENDS-NO-BUDGET-ON-STYLE The guide spends almost no budget on style — the language already won
those arguments. @impl/done

##ADVANTAGE-2-VERIFICATION-IS-THE-FASTEST **Advantage 2 — verification is the fastest of the three stacks.** `go build` and
per-package `go test` are famously quick; the Class E loop needs no project-reference
machinery (TS) and no cold `target/` pain (Rust). @spec/done

##GO-SHIPS-THE-SCAFFOLD-MACHINERY `go test -json` is a native
machine-readable stream; `Example` functions are compiled AND executed doctests;
`go test -fuzz` is a built-in differential engine; `httptest` is a stdlib simulator. @spec/done

##GO-HANDS-THE-CATALOG-MORE-MACHINERY Go hands the scaffold catalog more standard machinery than either sibling. @spec/done

##HAZARD-EXPRESSIVENESS-IS-THE-LOWEST **The hazard — prescriptions stop one step short of contract, and expressiveness is the
lowest of the three.** Go has no sum types, no exhaustive `switch`, no typestate culture,
late and deliberately modest generics. @spec/done

##DISCIPLINE-CARRIES-MORE-WEIGHT-IN-LINTERS-AND-CONTRACTS Where Rust encodes an invariant in a type and TS
in a branded union, Go often CANNOT put it in the type system at all — so the Discipline
carries proportionally more weight in **linter-borne rules, runnable contracts, fuzz
oracles, and conventions with checkers**. @impl/done

##four-prescriptions-lead And four specific prescriptions stop exactly
one step short of contract grade; closing those gaps is this guide's whole job: @impl/done

1. ##GAP-ERROR-SETS-ARE-OPEN **Errors are values — but their SETS are open.** Culture says return `error`; nothing
   says WHICH errors. The seam's failure set becomes part of the checked contract (§5). @impl/done
2. ##GAP-CONFORMANCE-IS-SILENT **Interface conformance is silent.** Structural satisfaction means a cell can drift
   off its seam without a compile error naming the seam. Conformance is made loud (§2). @impl/done
3. ##GAP-GOROUTINES-ARE-UNOWNED **Goroutines are unowned by design.** `go f()` has no owner, no join handle, no
   cancellation unless you build them. Ownership is made structural (§5). @impl/done
4. ##GAP-INIT-BLESSES-SIDE-EFFECTFUL-IMPORT **`init()` blesses the side-effectful import.** The stdlib itself registers drivers
   at import (`database/sql`, `image/*`, `_ "net/http/pprof"`) — which is exactly why
   cells must ban it explicitly (§2). @impl/done

##LAW-PROJECTED-ORDINARY-IDIOMATIC-GO **The law, projected.** Go source under this discipline reads as *ordinary idiomatic
Go*. @impl/done

##NO-INVENTED-NOTATION No invented notation — that would incur the out-of-distribution penalty (EsoLang:
0–11% on unfamiliar surface; in-context learning cannot teach it). @spec/done

##GO-OOD-TAIL-IS-QUARANTINED Go's own OOD tail is
named and quarantined: reflection-driven frameworks, struct-tag DSLs, `unsafe`, cgo, and
clever channel topologies are the constructs models handle worst and humans debug
longest — they are boundary-only (§7). @impl/done

##STRICTNESS-LIVES-IN-THE-ENVELOPE The strictness we add lives in the envelope:
closed error sets, conformance assertions, ownership discipline, `//spec:` metadata,
linter evidence, and the per-cell loop — never in exotic surface. @impl/done

## 1. The prescriptive baseline — take everything the language gives {#baseline}

##kind-line-baseline `req r1` — the toolchain floor below is MUST; policy-gated rows are named as such. @impl/done

##cross-ref-baseline *(≈ TS §1: TS must OPT IN to its compiler's strictness flag by flag; Go must simply
not opt OUT of its culture. This section is the free-lever twin.)* @spec/done

- ##BASELINE-VERSION-FLOOR **Version floor: go 1.24; target the latest stable.** Modules with committed `go.sum`;
  `GOFLAGS=-mod=readonly` in CI (the lockfile is native — A2 by default). `go.work` for
  multi-module workspaces. @impl/done
- ##BASELINE-GOFMT **gofmt is non-negotiable and free** — the one language where the style war was won
  upstream; the floor's first step is a formatting check, and it costs the Discipline
  zero attention budget. @impl/done
- ##BASELINE-LINTER-TIER **`go vet` MUST** (floor step); **staticcheck MUST** (MIT; policy-gated floor step —
  a `DISABLED by policy` line prints with its reason and is re-questioned weekly);
  **`exhaustive` linter** (BSD-2) for closed-set switches (§5; policy-gated with the
  same printed-line rule); **`govulncheck`** (BSD-3) in CI, not the floor (it touches
  the network). **golangci-lint is GPL-3.0: never vendored, never linked, never in the
  floor** — at most a personal separate-process dev tool, per the licensing flow. @impl/done
- ##BASELINE-RACE-DETECTOR-GATES-TESTS **The race detector gates tests:** `go test -race` is the MUST configuration for any
  package that starts a goroutine; findings are failures, not warnings. @impl/done
- ##BASELINE-SUPPRESSION-POLICY **Suppression policy (xfail-strict posture).** The blessed forms carry a reason by
  construction: staticcheck's `//lint:ignore <Check> <reason>` and the exhaustive
  linter's `//exhaustive:ignore <reason>`. A bare `//nolint` (any linter), a reasonless
  ignore directive, or a `t.Skip` on a known-failing test (§10) is a conform finding.
  The suppression census only shrinks (BROWNFIELD §4 at the lint level). @impl/done
- ##BASELINE-GENERICS-LEGAL-AND-BOUNDED **Generics: legal and bounded.** Type parameters for containers/algorithms in infra
  packages; domain seams stay interface-based unless a measured hot path says otherwise.
  No type-parameter theater (R-021) — Go's generics are deliberately modest; code that
  fights that modesty is OOD. @impl/done
- ##BASELINE-BOUNDARY-VALIDATION **Boundary validation (parse, don't validate).** JSON decoding is loose by default —
  missing fields become zero values silently, unknown fields are ignored. Boundary
  decode uses `json.Decoder.DisallowUnknownFields` plus explicit validation; boundary
  DTO structs convert explicitly into domain types; absent-vs-zero ambiguity is resolved
  with pointer fields or a validation layer at the boundary, never guessed in cells.
  Struct tags live on boundary DTOs only (§7). *Specified, not built: nothing
  enforces this. No conform rule and no floor step inspects boundary decode, and
  `DisallowUnknownFields` appears in no Go source anywhere in the tree — the rule
  lives only as prose here, in the core Go projection, and as one inventory item
  the brownfield skill looks for (`go-ai-native-terraform`, "loose boundary
  decoding"). Nor is it demonstrated: `research/go-demo`, the one Go consumer,
  decodes no JSON at all, so the rule is untested there rather than broken.* @spec/done

## 2. Cells, closure, ownership {#cells}

##kind-line-cells `req r1` *(≈ Rust §1, TS §3)* @impl/done

##CELL-IS-THE-UNIT-OF-MODIFICATION The **cell** is the unit of modification, closed under paging (R3-001): it declares its
full semantic dependency set so a pager can assemble sufficient context mechanically. @impl/done

- ##CELL-IS-A-PACKAGE-UNDER-INTERNAL-CELLS **A cell is a package under `internal/cells/<name>`.** `internal/` makes non-module
  imports a compile error — the cell ring as language physics; the in-module sibling ban
  (a cell importing a sibling cell, R-002) is checked at T-syn from the import graph.
  Seams live in a neutral package (`internal/seams` by convention; configurable);
  **`internal/registry` is the only package that imports cell packages** (§6). @impl/done
- ##CELL-BANS-INIT-AND-BLANK-IMPORTS **Import-is-execution, Go edition: `init()` and blank imports are banned in cells.**
  So is package-level `var` with a non-constant initializer. The single carve-out is
  boundary adapters wrapping stdlib-style driver registration — registration happens
  there or in the composition root, never as a side effect of importing domain code. @impl/done
- ##CELL-HAS-NO-AMBIENT-STATE **No ambient state.** Cells never touch `os.Getenv`, `time.Now`, `os.Stdin/Stdout`,
  `http.DefaultClient`/`DefaultServeMux`, `flag.CommandLine`, `math/rand`'s global
  source, or the global `log`/`slog` default. Capabilities are injected at construction
  — and Go makes this uniquely cheap: the cell declares the narrow interface it needs
  *privately* (`type clock interface{ Now() time.Time }`) and structural typing does the
  rest. No central capability package, no mocking framework: tests hand in literal
  fakes (§4-H). @impl/done
- ##CONTEXT-IS-THE-CANCELLATION-CAPABILITY **`context.Context` is the cancellation capability:** first parameter of every
  potentially-blocking seam method, never stored in a struct field (vet-checked). @impl/done
- ##EXPORTS-ARE-THE-SURFACE **Exports are the surface.** A cell package exports its constructor (`New(...)`) and
  nothing else beyond seam-required types. Exported-but-unreferenced identifiers are
  findings. @impl/done
- ##CONFORMANCE-IS-MADE-LOUD **Conformance is made loud** — every cell carries the compile-time assertion, and
  conform checks its presence (T-syn): *Specified, not built (→ B-030): the Go gate registers exactly three rules — `GoUnsafeInDomain`, `GoCellIsolation`, `FileLength` (`build_rules`, `lib.rs:53-60`) — and none parses for the assertion's presence; the assertion itself is real, idiomatic Go and the pattern below is correct.* @impl/plan

```go
// internal/cells/batchplanner/planner.go
//
//spec:implements spec://go-demo/PROP-001-reconciler#req-planner-seam r=1
//spec:cell seam=Planner variant=batch replaces=naive flag=planner
package batchplanner

var _ seams.Planner = (*BatchPlanner)(nil) // silent conformance made loud — MUST

func New(store seams.Store, clk clock) *BatchPlanner { /* … */ }
```

- ##PROMOTION-TO-A-SEPARATE-MODULE **Promotion** to a separate module on the usual triggers: heavy optional deps,
  independent release cadence, ~2 kLoC. @impl/done

## 3. Surface form: naming, position, uniformity {#surface}

##kind-line-surface `req r1` *(≈ Rust §2, TS §4)* @impl/done

- ##NAMES-ARE-TOKEN-PROGRAMS **Names are token programs** (R3-004, R-020). Canonical cell type name is computed
  from the manifest: `{Variant}{Seam}` → `BatchPlanner`; the package is the lower-case
  variant (`batchplanner`). One name = one referent across contract surfaces; no
  shadowing, no synonym pairs; structural tokens from a closed vocabulary. Length is
  free; ambiguity is not. (Short closure-local bindings — `i`, `ok`, `ctx` — are
  idiomatic Go and exempt; the rule scopes to contract surfaces.) @impl/done
- ##FAMILY-PREFIX-RULE **The family-prefix rule (owner policy; PROP-028 §2.4).** Every named surface of the
  Go discipline is language-FIRST, carrying the `go-ai-native` stem as a prefix: the
  umbrella binary `go-ai-native` (crate `go-ai-native-cli`), the standalone tools
  `go-ai-native-conform` / `go-ai-native-specmap` / `go-ai-native-tcg`, the libraries
  `go-ai-native-conform-frontend` / `go-ai-native-extract-bridge` /
  `go-ai-native-specmap-scan` / `go-ai-native-tcg-bridge`, the server package/binary
  `go-ai-native-mcp` (agent-visible server name: the family, `go-ai-native`), the skills
  `go-ai-native-sweep` / `go-ai-native-terraform`. Language-NEUTRAL artifacts stay
  outside the stem (the shared engine crates carry `core-ai-native-*`). @impl/done
- ##CONTRACT-FIRST-ORDERING **Contract-first ordering within an item** (R3-002): the doc comment states behavior,
  invariants, and the error contract; the `Example` function shows canonical use; both
  precede or immediately adjoin the declaration. Autoregression makes reading order
  conditioning order; intent goes first. @impl/done
- ##POSITION-IS-A-RESOURCE **Position is a resource** (R3-003): package-level invariants live in the package doc
  block (`doc.go`) or at file top; safety-critical facts never sit in a file's diluted
  middle third. Prefer more, smaller, single-purpose files at equal token mass — Go
  packages are natively multi-file, so splitting costs nothing (§15). A conform check
  warns on files over the length budget. @impl/done
- ##UNIFORMITY-IS-LOAD-BEARING **Uniformity is load-bearing** (R3-006, H6) — and Go's culture already enforces most
  of it. What remains ours: one idiom per operation *within this repository* (one way to
  construct a cell, one error shape per seam, one fake per capability), and legitimate
  exceptions are MARKED (`//spec:deviates … reason="…"`) so they do not propagate as
  false training signal. @impl/done

## 4. The nine scaffolds in Go {#scaffolds}

##kind-line-scaffolds `req r1` *(≈ Rust §3, TS §5)* — each is a card in this package's `cards/`; here is the
Go shape and the rule. @impl/done

- ##SCAFFOLD-A-GENERATORS **A — Generators / codegen** (`scaffold-a-generators`). **`go:generate` is the
  culture's own slot** — the directive names the emitter next to its output's home;
  `stringer`-class tools, `text/template` emitters, schema-to-type generation. Committed
  output is plain idiomatic Go; the generator input is the taggable unit; outputs are
  excluded from orphan checks. *Rule:* where an artifact is mechanically derivable from
  a smaller spec, ship generator + committed output + a CI regenerate-and-diff check,
  not hand-maintained output (A3). @impl/done
- ##SCAFFOLD-B-TYPED-SURFACES **B — Typed surfaces / defined types** (`scaffold-b-typed-builders`). **Go's defined
  types are nominal for free** — `type AccountID string` does not interchange with
  `string` or with `type OrderID string` at call sites: the identity-swap failure TS
  must brand away fails `go build` here by default. Meaning-bearing primitives crossing
  a seam are defined types; required-field protocols are constructor-enforced (`New`
  validates and is the only path — unexported struct fields make bypassing it a compile
  error); call-order protocols use staged builders; option lists use functional options.
  Typestate via phantom type parameters is possible since generics but is NOT idiomatic
  Go — use it only where a protocol genuinely demands compile-time ordering, and mark
  it. *Rule:* seam protocols are encoded in types and constructors, not docstrings; the
  wrong call fails `go build`, not a runtime check (R3-008). @impl/done
- ##SCAFFOLD-C-RUNNABLE-CONTRACTS **C — Runnable contracts** (`scaffold-c-runnable-contracts`). Go has no
  `debug_assert!`; the projection is an explicit `invariant` helper (panics — an
  invariant violation IS the panic case, §5) restated at use sites (R3-009), plus
  `testing/quick` / fuzz properties backing behavioral claims. *Rule:* every
  load-bearing invariant is witnessed by a runnable check where it is relied upon, not
  only documented at definition. @impl/done
- ##SCAFFOLD-D-DIFFERENTIAL-ORACLES **D — Differential / characterization oracles** (`scaffold-d-differential-oracle`).
  **Native fuzzing is the engine:** one `FuzzXxx` target drives old and new cells
  through the seam and asserts agreement; the seed corpus lives in `testdata/` and runs
  deterministically in CI (`go test` runs seeds; `-fuzz` explores locally). Golden files
  live in `testdata/` under the promotion protocol — the conventional `-update` flag
  never runs in CI. *Rule:* no replacement of a non-trivial cell merges without a
  differential or characterization oracle against prior behavior (R-040). @impl/done
- ##SCAFFOLD-E-PER-CELL-FAST-LOOP **E — Per-cell fast loop** (`scaffold-e-fast-loop`). `go test ./internal/cells/<name>/
  -race` answers in seconds with zero setup — the strongest Class E substrate of the
  three stacks. The agent loop is edit → per-package test → read structured error →
  edit; first signal < ~60s (R3-007). *Rule:* whole-repo CI is not an agent loop; the
  per-cell loop is the substrate that makes every other scaffold's signal fast enough. @impl/done
- ##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS **F — Structured, REQ-citing diagnostics** (`scaffold-f-structured-diagnostics`).
  Seam error types render `violates REQ <spec-uri>: <why>; fix surface: <where>` (§5);
  custom checks emit the same grammar; conform emits SARIF. *Rule:* every custom check
  and every seam error is agent-actionable — REQ URI + fix surface, never bare free
  text (R3-011). @impl/done
- ##SCAFFOLD-G-EXECUTABLE-EXAMPLES **G — Executable examples** (`scaffold-g-doctests`). **`Example` functions are real
  doctests, and stronger than Rust's:** `ExampleXxx` with an `// Output:` comment is
  compiled AND executed by `go test`, its stdout diffed against the comment — a
  behavioral guarantee, not just compilation. *Rule:* every public seam item carries ≥1
  `Example` of canonical construction+use with `// Output:` where output is
  deterministic; an example that lies fails the build (R2C-004, H4). @impl/done
- ##SCAFFOLD-H-LOCAL-SIMULATORS **H — Local simulators / reference models** (`scaffold-h-simulators`). Hand-rolled
  in-memory fakes are Go's native test culture (small interfaces make them one-screen
  literals); **`httptest` is a stdlib network simulator**; subsystems with non-obvious
  dynamics (a reconcile loop, a state machine) ship a steppable reference model. *Rule:*
  non-obvious dynamics ship a runnable model or fake, not a prose description (DR2-019). @impl/done
- ##SCAFFOLD-I-CODEMODS **I — Scaffolded edit operations / codemods** (`scaffold-i-codemods`). `gofmt -r` for
  pattern rewrites; `go/ast` + `go/format` codemods for structural ones; the shipped
  `go-ai-native codemod add-cell` emits a cell skeleton (package, conformance assertion,
  directive tags, registry arm, Example stub) as ONE checked operation. *Rule
  (provisional, [E-hyp]):* a capability-demanding multi-file edit is offered as one
  parameterized checked operation; validate weak-agent parameterization in pilot. @impl/done

## 5. Errors as contract surface — and goroutines as owned resources {#errors}

##kind-line-errors `req r1` *(≈ Rust §4, TS §6)* @impl/done

##errors-sets-are-contract-lead Go made errors values twenty years before it was cool; the Discipline makes their
**sets** part of the contract: @impl/done

- ##SEAM-OWNS-A-CLOSED-ERROR-SET **Each seam owns a closed, enumerated error set:** @impl/done

```go
// PlanError is the Planner seam's closed failure set.
type PlanErrorCode int

const (
	ErrConflict PlanErrorCode = iota + 1 // desired and actual disagree irreconcilably
	ErrUnknownKind                       // a resource kind outside the seam's vocabulary
)

type PlanError struct {
	Code PlanErrorCode
	Spec string // the violated REQ URI: "spec://go-demo/PROP-001-reconciler#req-plan-total"
	Err  error  // wrapped cause, if any
}

func (e *PlanError) Error() string {
	return fmt.Sprintf("plan: %v: violates REQ %s", e.Code, e.Spec)
}
func (e *PlanError) Unwrap() error { return e.Err }
```

  ##CONSUMERS-USE-ERRORS-AS Consumers use `errors.As` against the published type and switch on `Code`; boundary
  rendering appends the REQ URI and a fix surface (PROP-014 §2.6; Class F). @impl/done
- ##BANNED-AT-SEAMS **Banned at seams:** matching on error strings; `fmt.Errorf` without `%w` (breaks the
  chain); anonymous `errors.New` for expected failures; `error` returns that are
  sometimes nil-with-meaning. @impl/done
- ##EXHAUSTIVENESS-IS-CARRIED-BY-A-LINTER **Exhaustiveness — the deepest gap, carried by a linter and named honestly.** Go has
  no sum types and no exhaustive `switch`; closed sets are const-enums, and the
  `exhaustive` linter supplies what the compiler won't. This is the one Discipline rule
  in the Go projection enforced entirely by an external evidence provider; the honest
  degradation is stated rather than papered over. A `default:` arm on a closed-set
  switch is the graveyard move (it silences the linter) — banned; where a trap arm is
  genuinely needed it panics, and the linter still checks the named cases. @impl/done
- ##PANIC-IS-THE-INVARIANT-VIOLATION-CHANNEL **panic = invariant violation** — the analog is native, same word. `recover` is legal
  only at goroutine/boundary top level (middleware, `main`), never as control flow in
  cells; panicking on an expected failure is banned. @impl/done
- ##STRUCTURED-CONCURRENCY-BY-OWNERSHIP **Structured concurrency by ownership.** Every goroutine a cell starts has an owner:
  `errgroup.Group` (BSD-3) or `sync.WaitGroup` + context cancellation; a naked `go`
  whose goroutine can outlive its cell is banned; channels are owned and closed by
  their spawner; channel topologies are implementation, never API (§7). The unowned
  goroutine is Go's unreferenced `create_task` — with no GC to even cancel it. @impl/done
- ##RELEASE-MAP-IS-FREE **The release map is free.** Every Go binary embeds `runtime/debug.ReadBuildInfo`
  (VCS revision, dirty flag, module versions), readable from the artifact (`go version
  -m`). The A1 chain *binary → build info → specmap@commit → REQ* needs zero extra
  machinery; the only rule is not to strip what the runtime gave you. @impl/done

## 6. Registry, flags & the composition root {#registry}

##kind-line-registry `req r1` *(≈ Rust §5, TS §7)* @impl/done

##R-001-BINDING-FLAG-AT-THE-SEAM R-001 binding — flag at the seam, never in the veins: @impl/done

```go
// internal/registry — the only flag reader and the only package
// permitted to import cell packages.
func Planner(cfg Config, store seams.Store, clk seams.Clock) seams.Planner {
	switch cfg.Planner { // provenance: default | env | cli | lockfile
	case PlannerBatch:
		return batchplanner.New(store, clk)
	default:
		return naiveplanner.New(store)
	}
}
```

- ##TWO-TIERS-NEVER-CONFUSED **Two tiers, never confused:** build tags (`//go:build`) answer *"is the code in the
  binary"* — the cargo-feature analog, per-file granularity — and are confined to
  registry/adapter files, never inside cell bodies (T-lex); runtime flags answer *"is
  the cell selected"*, read once into a config struct in `main` and passed down. @impl/done
- ##DELIVERY-MODE-HONESTY **Delivery-mode honesty:** Go has no credible lazy in-process loading (the `plugin`
  package is platform- and version-locked); eager is the only mode; presence is the
  build tier's job. @impl/done
- ##NO-DISCOVERY-NO-REFLECTION-WIRING No `ServiceLoader`-style discovery (§2's `init()` ban already killed it), no
  reflection-based wiring, no DI frameworks. The registry `switch` is the system's
  table of contents. @impl/done

## 7. Bans and their escape hatches — the Go theater list {#bans}

##kind-line-bans `req r1` *(≈ Rust §6, TS §8)* @impl/done

##BANS-ARE-DEVIATION-GATED Forbidden by default in domain cells; legal only with `//spec:deviates <uri> r=<N>
reason="…"` and the required machinery. @impl/done

##go-ood-tail-lead These are Go's OOD tail and its
action-at-a-distance set: @impl/done

- ##BAN-INIT-AND-BLANK-IMPORTS **`init()` and blank imports** (§2) — the import-time registration culture stops at
  the boundary ring. @impl/done
- ##BAN-REFLECTION-IN-DOMAIN-CODE **Reflection in domain code** (`reflect`, `Type.Implements`, struct-walking) — the
  second language Go bifurcates into at boundaries (encoding, ORMs) stays there. @impl/done
- ##BAN-STRUCT-TAG-DSLS **Struct-tag DSLs outside boundary DTOs** — tags are stringly programs interpreted by
  reflection at runtime; domain types carry none. @impl/done
- ##BAN-ANY-IN-DOMAIN-SIGNATURES **`interface{}` / `any` in domain signatures** where a type or a small interface
  fits. @impl/done
- ##BAN-CHANNELS-AS-API **Channels as API** — a seam exposes methods; channels are implementation. Clever
  fan-in/fan-out topologies as public surface are hidden control flow (R-021). @impl/done
- ##BAN-RECOVER-AS-CONTROL-FLOW **`recover` as control flow;** panic-driven non-local exits inside cells. @impl/done
- ##BAN-PACKAGE-LEVEL-MUTABLE-STATE **Package-level mutable state** — the module-level singleton in its Go form
  (`http.DefaultClient` is the stdlib's own disguise). @impl/done
- ##BAN-BEHAVIOR-BEARING-STRUCT-EMBEDDING **Behavior-bearing struct embedding** — embedding to inherit method sets across
  domain types is inheritance cosplay; compose via fields and explicit delegation.
  (Interface embedding in interface declarations is fine — that is composition of
  contracts.) @impl/done
- ##BAN-UNSAFE-AND-CGO **`unsafe` and cgo outside designated boundary files** — cells are pure checkable Go. @impl/done
- ##BAN-SPLIT-METHOD-SETS **Method sets split across files to obscure a type;** a type's methods live with it. @impl/done

##BAN-WITHOUT-A-HATCH-IS-A-DISCIPLINE-BUG A ban with no escape hatch is a discipline bug; a deviation with no reason is a code
bug. @impl/done

## 8. Metadata layer (specmap in Go) {#specmap}

##kind-line-specmap `req r1` *(≈ Rust §7, TS §9)* @impl/done

##DIRECTIVE-COMMENTS-ARE-FORCED-BY-THE-TOOLCHAIN **Directive comments** — a deliberate divergence from doc-comment tags, forced by the
toolchain: since Go 1.19 gofmt *reformats doc comments* (would re-wrap prose tags) but
preserves `//name:value` directive lines verbatim, and godoc hides them. @spec/done

##DISCIPLINE-TAKES-THE-SPEC-DIRECTIVE-SLOT Go already owns
the cultural slot (`//go:generate`, `//go:embed`); the Discipline takes `//spec:`: @impl/done

```
//spec:implements <uri> r=<N>                 one edge per line; lines repeat
//spec:deviates <uri> r=<N> reason="..."      reason mandatory
//spec:verifies <uri> r=<N>                   above Test/Fuzz/Example functions
//spec:scope <uri> r=<N>                      in the package doc block (doc.go) —
                                              package-level inheritance
```

##EDGE-MODEL-MIRRORS-PROP-014 Edge kinds mirror PROP-014 (`implements | verifies | documents | deviates | informs`);
≤3 edges per item or split; two-tier revisions (author-asserted `r` + content hash) with
asymmetric invalidation (spec bump → edges suspect; code change → edges stay valid); a
derived deterministic committed index (`specmap.json`); an orphan ratchet over exported
identifiers. @impl/done

##GENERATED-CODE-IS-EXCLUDED Generated code is excluded; the `go:generate` input is the taggable unit. @impl/done

##GODOC-PROVENANCE-TRADE-OFF The trade-off is named honestly: provenance disappears from rendered godoc and lives in
`trace`/the ledger instead (the deliberate opposite of Java's `@Documented` choice). @impl/done

## 9. Prose discipline (the asymmetric hazard) {#prose}

##kind-line-prose `req r1` *(≈ Rust §8, TS §10)* @impl/done

##WRONG-PROSE-IS-WORSE-THAN-NO-PROSE Wrong prose is worse than no prose (R2C-004, H4): models condition on in-repo text with
high trust, so a lying comment is adversarial input, and the harm exceeds absence. @spec/done

##GODOC-IS-UNCHECKED Go-specific sharp edge: godoc comments are the language's celebrated documentation
surface — and nothing checks them. @spec/done

##RULE-BEHAVIORAL-CLAIMS-ARE-CHECKED-OR-LABELLED *Rule:* behavioral claims near code are
**machine-checked** — backed by an `Example` with `// Output:` (which executes) or a
test — or **explicitly trust-labeled** (verified / unverified / aspirational). @impl/done

##SIGNATURE-RESTATING-GODOC-IS-A-DEFECT A godoc line that merely restates the signature is duplication (a defect); misleading log
strings count too (the harm is the false claim, not the syntax). @impl/done

##GODOC-IS-THE-HUMAN-DETAIL-LAYER Godoc remains the human
detail layer; duplication with the spec is a spec defect. @impl/done

## 10. Replacement protocol {#replacement}

##kind-line-replacement `req r1` *(≈ Rust §9, TS §11)* @impl/done

##REPLACEMENT-SHIPS-A-DIFFERENTIAL-ORACLE Replacing a cell ships a **differential oracle** (Class D): a fuzz target driving old
and new cells through the seam, asserting agreement modulo a documented divergence list,
`//spec:verifies`-tagged, run with `-race`, its seed corpus committed under `testdata/`. @impl/done

##CHARACTERIZATION-GOLDENS-FOLLOW-THE-PROMOTION-PROTOCOL Characterization goldens live in `testdata/` and follow the promotion protocol — CI
never regenerates; a local update carries a debt/intent reference in the commit body. @impl/done

##XFAIL-HONESTY-GO-SPECIFIC **xfail honesty [Go-specific]:** Go has no native strict-xfail and this guide bans
`t.Skip` on known-failing tests (a skip hides both regressions and healings) — known
failures live ONLY in `discipline/registry/tests-baseline.json`, which carries full
weight here: Go is the one stack of the three without an in-source xfail twin, stated
rather than hidden. @impl/done

## 11. Test matrices {#matrices}

##kind-line-matrices `req r1` *(≈ Rust §10, TS §12)* @impl/done

##TABLE-DRIVEN-TESTS-ARE-THE-DECLARED-MATRIX **Table-driven tests are Go's native idiom and the Discipline's declared matrix in one**
— a named, bounded case slice with `t.Run` subtests, never an implicit `2^n`: @impl/done

```go
cases := []struct {
	name string
	in   State
	want []Action
}{ /* … the matrix is authored data … */ }
for _, tc := range cases {
	t.Run(tc.name, func(t *testing.T) { /* … */ })
}
```

##PROPERTY-AND-FUZZ-SURFACES `testing/quick` covers simple property surfaces (stdlib; adequate for the demo class);
fuzz targets cover differential and parser-shaped surfaces; the differential oracle
(§10) covers replacement; per-cell tests run in the fast loop. @impl/done

##THIRD-PARTY-PROPERTY-FRAMEWORKS Third-party property frameworks are admitted case-by-case under the licensing flow when `quick` runs out. @impl/done

## 12. How a weak reader actually uses this guide {#weak-reader}

##cross-ref-weak-reader *(≈ Rust §11, TS §13)* @impl/done

##WEAK-SWARM-DOES-NOT-READ-THIS-GUIDE The weak swarm does **not** read this guide. @impl/done

##WEAK-SWARM-RECEIVES-THE-BAND-3-EXTRACT It receives, per edit, the Band-3 ops
extract of whichever cards' triggers fire — a small, activation-matched set (lazy-push,
R3-014; minimal sufficiency, AGENTbench). @impl/done

##GUIDE-AND-CARDS-ARE-THE-AUTHORING-ARTIFACT This guide and the cards are the
authoring/review artifact for the strong author and the human; the runtime surface for a
`.go` edit is a card from *this package's* `cards/`, never another language's. @impl/done

##CROSS-CUTTING-CONCERNS-ARE-SWEPT-BY-RAIDS Cross-cutting concerns the per-edit loop cannot hold are swept by raids
(`03-RAID-PLAYBOOK.md`). @impl/done

## 13. Tooling roadmap pointer (the tcg line) {#tooling}

##cross-ref-tooling *(≈ Rust §12, TS §14)* @impl/done

##tcg-line-two-briefs-lead The tcg line has two briefs, split by where the intervention happens: @impl/done

- ##TCG-AGENTIC-ORACLE-SHIPPED **[`go/tools/vibe-agentic-tcg-go.md`](tools/vibe-agentic-tcg-go.md) — SHIPPED (the
  agentic oracle):** a consultation oracle over the CONSUMER's own gopls — validate an
  in-memory overlay / in-scope symbols / type-valid completions / quick info at
  millisecond-class latency, discipline-enriched in-process by the same conform engine
  as the gate — behind the same language-parameterised `tcg_*` MCP tools
  (`language: "go"`) and one-shot CLI forms. Mechanisms:
  [`mechanisms/TCG-ORACLE-GO-v0.1.md`](mechanisms/TCG-ORACLE-GO-v0.1.md),
  [`mechanisms/TCG-PROTOCOL-GO-v0.1.md`](mechanisms/TCG-PROTOCOL-GO-v0.1.md).
  **Fidelity, honestly:** gopls stands on `go/types` — the reference library
  implementation of the language spec — while the gc compiler runs types2, its
  deliberately-synchronized twin. The Go oracle therefore sits BETWEEN the TS oracle
  (which IS tsc's engine) and the Rust one (rust-analyzer is NOT rustc): far tighter
  than an independent reimplementation, still not the compiler itself. The floor stays
  the truth. @impl/done
- ##TCG-TOKEN-LEVEL-IS-VERY-FAR-FUTURE **[`go/tools/go-ai-native-tcg.md`](tools/go-ai-native-tcg.md) — VERY-FAR-FUTURE
  (token-level):** logit masking to type-valid, discipline-conformant continuations.
  Owner-dispositioned 2026-07-17: not until an inference substrate exists; the brief is
  held at stub depth, at parity with the TS stub. @spec/done

## 14. Wiring a consumer (the shipped toolchain) {#wiring}

##kind-line-wiring `req r1` *(≈ Rust §13, TS §15)* @impl/done

##consumer-wires-in-five-moves-lead The stack ships the toolchain as runnable code (PROP-024); a consumer wires it in five
moves: @impl/done

1. ##WIRING-INSTALL-THE-STACK **Install the stack** — `vibe install` with
   `stack:org.vibevm.ai-native/go-ai-native-lang` in `[requires].packages` materialises
   the slot under `vibedeps/` (the neutral engines ride along as vendored copies; the
   slot is its own Cargo workspace and builds standalone). A Rust-hosting consumer keeps
   `[workspace] exclude = ["vibedeps"]`. @impl/done
2. ##WIRING-GET-THE-BINARIES **Get the binaries** — `vibe bin build` then `vibe bin exec go-ai-native -- <args>`
   (PROP-025 lockfile dispatch), or
   `cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli`, or run in
   place via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
   go-ai-native-cli --bin go-ai-native -- <args>`. @impl/done
3. ##WIRING-MACHINE-PREREQUISITES **Machine prerequisites** — go ≥ 1.24 on PATH (or `GOROOT`-resolvable) and **gopls**
   (`go install golang.org/x/tools/gopls@latest`): installing this stack obliges the
   machine to carry both — inside the stack's own suite an absent tool is a
   recipe-carrying FAILURE, never a skip. staticcheck / exhaustive are optional
   evidence providers the floor policy names. @impl/done
4. ##WIRING-BOOTSTRAP **Bootstrap** — `go-ai-native init` writes `conform.toml` (`[go]`: roots,
   `cells_dir`, `seams_pkg`, `registry_pkg` — topology detected from `go.mod`),
   `specmap.toml` (namespace + discovered `[[external_specs]]`), both ratchet baselines,
   and the BROWNFIELD registries; then `go-ai-native specmap` mints the index and
   `go-ai-native floor` runs the seven steps (gofmt → vet → test → staticcheck+
   exhaustive → conform → specmap → test-gate). Brownfield adoption: the
   `/go-ai-native-terraform` skill. @impl/done
5. ##WIRING-GENERATION-TIME-ORACLE **The generation-time oracle (optional but cheap)** — before writing a nontrivial
   `.go` edit, validate the HYPOTHETICAL content:
   `vibe bin exec go-ai-native-tcg -- validate internal/cells/<cell>/<file>.go
   --content-from - --root .` (the edit on stdin; exit 1 = an error-grade diagnostic or
   a non-baselined finding), or the `tcg_validate` / `tcg_scope` / `tcg_complete` /
   `tcg_type` MCP tools with `language: "go"`. The floor stays the truth; the oracle
   exists so the floor stays green on the first try. @impl/done

## 15. Sweep idioms (Go) {#sweep}

##sweep-idioms-lead *(≈ Rust §14, TS §16)* — the recurring posture is the shipped Sweep Playbook driven by
`/go-ai-native-sweep`; the Go-specific idioms: @impl/done

- ##SWEEP-DANGER-BAND-SPLITS **Danger-band splits are the cheapest of the three stacks:** a Go package is natively
  multi-file — move a cohesive slice of an oversized file into a sibling file of the
  SAME package (no module surgery, no re-exports, imports unchanged). The new file
  inherits the package's `//spec:scope` from `doc.go` automatically; a file carrying
  its own `//spec:` item tags keeps them with the moved items. Measure with the rule
  (physical lines), not the eye. @impl/done
- ##SWEEP-FOUR-EXAMPLE-IDIOMS **The four Example idioms** (export doc-example drain): a construct-and-`Error()`
  assert for seam error types (the Class-F message already cites its REQ, so the
  example doubles as a navigability demo); an encode/decode round-trip for boundary DTO
  types; a zero-value/enumerator demo for const-enum sets; a canonical
  construct-and-use for each seam (via the blessed `New`). @impl/done
- ##SWEEP-SUPPRESSION-DRAINS **Suppression drains:** a reasonless `//lint:ignore` / `//exhaustive:ignore` is
  unrecorded testimony — reason it or fix it; a `t.Skip` on a known-failing test moves
  to `tests-baseline.json` (§10) the day it is found. @impl/done
- ##SWEEP-CENSUS-REGRESSIONS **Census regressions** (gated packages must hold zero): `init_in_cell`,
  `ambient_call_in_cell`, `naked_go_in_cell`, `error_string_match`,
  `seam_error_missing_req` — restructure beats testify: encode the invariant in a type
  or constructor rather than recording an excuse. *Where these live in the
  shipped tooling: `go-extract` emits the kinds, the `go-unsafe-in-domain`
  conform rule reports them, and `go-ai-native health` summarises them into the
  snapshot's `ban_census` block. Two names above are the shipped kinds verbatim
  (`error_string_match`, `seam_error_missing_req`); the other three are this
  guide's cell-scoped reading of kinds the extractor names without the suffix —
  `init_in_cell` is `init_decl`, `ambient_call_in_cell` is `ambient_call`,
  `naked_go_in_cell` is `naked_go` — because the engine expresses "in a cell" as
  a scope predicate over `cells_dir` rather than as part of the name.* @impl/done
- ##SWEEP-FLIP-ONLY-AFTER-DRAIN **Flip-only-after-drain:** a package enters `gated_crates` only at zero findings (the key is the shared neutral engine's — one spelling across the language stacks today);
  the collector (`go-ai-native health`) names promotion candidates and ranks the drain
  backlog smallest-gap-first; a flip must never widen a baseline. @impl/done
