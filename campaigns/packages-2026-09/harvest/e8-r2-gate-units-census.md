# E8-R2-GATE-UNITS — census of gate units and the FlagSites rule

Read-only census for the B-034 / B-039 / B-035 fork: *what is a gate unit
per language, where the gate lists live today, and what the FlagSites rule
(R-001) actually checks and mounts.* Every claim is pinned to `path:line`
relative to the worktree root. All paths use the worktree's own copies
(`packages/org.vibevm.ai-native/...`); `vibedeps/**` and `*/vendor/**`
copies are the same files and are not cited.

**Perimeter read.** Engine `core-ai-native-conform` (the crate dependents
import as `conform_core`; package name `core-ai-native-conform`,
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/Cargo.toml:2`,
lib name `conform_core` as used by every frontend, e.g.
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:16`).
The three gate drivers
(`rust-`/`go-`/`typescript-ai-native-conform/src/lib.rs`), their three
frontend bridges (`*-conform-frontend/src/lib.rs`), the two extractors
(`tools/go-extract/extract.go`, `tools/ts-extract/extract.ts`), the host
`conform.toml` plus the three `research/*-demo/conform.toml` and the four
extractor-fixture `conform.toml` files, and the three
`GUIDE-AI-NATIVE-{RUST,TYPESCRIPT,GO}.md`.

**Headline.** Only Rust has a gate-unit notion and a gate list today: the
engine's `validate_against_tree` classifies *cargo-workspace members*
against `gated_crates`/`exempt` (`config.rs:266`), and the Rust driver is
the only one that calls it (`rust …/lib.rs:119`). Go and TypeScript have
**no** gate-unit classification at all — they scan `roots` into a flat
file list (`store.rs:388`, `store.rs:305`), derive a "cell" per file from
a path prefix (`go.rs:207`, `typescript.rs:144`), and run three rules with
no gated/exempt list and no tree invariant. FlagSites (R-001) is a
Rust-shaped rule (it keys on `Fact::Ctor`, `facts.rs:48`, emitted only by
the Rust frontend at `rust-ai-native-conform-frontend/src/lib.rs:365`);
neither the TS nor the Go driver mounts it, the Go guide promises it
anyway (`GUIDE-AI-NATIVE-GO.md:366`), and the TS guide records its absence
as `→ B-039` (`GUIDE-AI-NATIVE-TYPESCRIPT.md:175`).

---

## Q1 — The gate unit per language, by the trees on disk

**Rust — the unit is the *crate directory*.** A root entry of `<dir>/*`
scans each subdirectory of `<dir>` that contains a `Cargo.toml` as one
crate; any other root entry is a literal crate dir
(`config.rs:37-39`, expanded identically by the scanner at
`store.rs:231-246` and by the validator at `config.rs:298-310`). The name
a crate is known by is its directory basename, derived through
`crate_dir_name` (`store.rs:212-217`, which resolves `.` to the project
directory's own basename so a bare single-crate layout is nameable).
`gated_crates` is the list of crate names under the Class-F/G gates
(`config.rs:43-44`); `exempt` is the list held out, each with a reason
(`config.rs:61-64`, `config.rs:209-216`).

Measured on the host policy: `roots = ["crates/*", "xtask"]`
(`conform.toml:13`); `gated_crates` lists 13 crates
(`conform.toml:41-55`); `exempt` lists 6 (`conform.toml:106-128`). So the
workspace members are exactly the `crates/*/` dirs plus `xtask`, and the
invariant requires every one of them to appear in `gated_crates ∪ exempt`.
The package demo (`research/rust-demo/conform.toml:8,16`) uses
`roots = ["crates/*"]` with one gated crate `rust-demo` and no `exempt`
(and notably no `registry_file`).

**Existing Rust enumeration that already feeds gated-or-exempt
classification:** `validate_against_tree` (`config.rs:266-326`) expands
`roots` into the on-disk crate set and checks coverage; `workspace_sources`
(`store.rs:225-283`) is the scanner half that attributes each `.rs` file
to its crate name. A host-side test, `every_crate_is_gated_or_exempt`
(`xtask/src/conform.rs:29`), enforces the same invariant on the host
`conform.toml` from outside the engine. So Rust has a complete, working
"gated-or-exempt" enumerator.

**Go — the extractor enumerates *files*, not packages or cells.**
`go-extract` emits one NDJSON record per `.go` file
(`tools/go-extract/extract.go:37-44`, walked by `walkTree`
`extract.go:127-151`, or an explicit `--files` list). Its facts are
`file_metrics` / `item` / `import` / `go_unsafe`
(`extract.go:48-67`); there is **no** package-level fact and **no** cell
notion in the extractor. On the engine side, `go_sources`
(`store.rs:388-449`) walks each `[go].roots` dir and tags every `.go` file
with a `crate_name` equal to the **root directory's basename**
(`store.rs:415`) — *not* the Go `package` the file declares. A "cell" is
derived later, per file, as the path segment immediately under `cells_dir`
(`GoCellIsolation::cell_of_file`, `go.rs:207-212`). There is **no**
function that enumerates "all cells" or "all Go packages", and **no**
gated/exempt list or tree invariant for Go.

Fixture layout (exact dirs):
- clean — `internal/cells/greet/greet.go` only (one cell)
  (`tools/go-extract/test/fixtures/clean/internal/cells/greet/greet.go`).
- dirty — `internal/cells/plan/{plan.go,plan_test.go}` plus
  `internal/registry/registry.go`
  (`tools/go-extract/test/fixtures/dirty/…`).
- demo (`research/go-demo/`) — `internal/cells/{batchplanner,naiveplanner}/`
  (two cells, each with `planner.go`+`planner_test.go`+`doc.go`),
  `internal/registry/registry.go`, `internal/seams/`, `internal/sim/`,
  `cmd/reconcile/main.go`.

**TypeScript — the extractor enumerates *files*, not cells.**
`ts-extract` emits one record per `.ts/.tsx/.mts/.cts` file
(`tools/ts-extract/extract.ts:98-105`, walked by `walkSources`
`extract.ts:496-524`, or `--files`). Its facts are `file_metrics` /
`ts_unsafe` / `import` / `item` (`extract.ts:56-88`); no cell notion in the
extractor. `typescript_sources` (`store.rs:305-367`) walks each
`[typescript].roots` dir and tags every file with `crate_name` = **root
dir basename** (`store.rs:332`). `cells_dir` semantics: "the directory
whose immediate subdirectories are cells" (`config.rs:163-165`); a cell is
derived per file as that immediate subdir (`TsCellIsolation::cell_of`,
`typescript.rs:144-149`). No gated/exempt list, no tree invariant for TS.

Fixture layout (exact dirs):
- clean — `src/cells/greet/index.ts`, `src/cells/parse/index.ts`,
  `src/core/text.ts`.
- dirty — `src/cells/greet/{index.ts,internal.ts}`,
  `src/cells/parse/logic.ts`, `src/rubble.ts`.
- demo (`research/ts-demo/`) — `src/cells/greeting/index.ts` (+`.test.ts`),
  `src/cells/farewell/index.ts` (+`.test.ts`), `src/core/text.ts`.
  `cells_dir = "src/cells"`, `seam = "index"`, `roots = ["src"]`
  (`research/ts-demo/conform.toml:13-16`).

**Existing enumeration code per language that *could* feed a gated-or-exempt
classification.**
- Rust: **yes, complete** — `validate_against_tree` (`config.rs:266`) plus
  `workspace_sources` (`store.rs:225`).
- Go: **partial, file-grain only** — `go_sources` (`store.rs:388`) and
  `go-extract`'s `walkTree` (`extract.go:127`) enumerate files grouped by
  root basename; `GoCellIsolation::cell_of_file` (`go.rs:207`) derives a
  cell from one path. Nothing enumerates the *set* of cells (the immediate
  subdirs of `cells_dir`) or classifies them.
- TS: **partial, file-grain only** — `typescript_sources` (`store.rs:305`)
  and `ts-extract`'s `walkSources` (`extract.ts:496`); `TsCellIsolation::cell_of`
  (`typescript.rs:144`) derives a cell from one path. Nothing enumerates
  the cell set or classifies it.

So a Go/TS gated-or-exempt invariant would need a **new** enumerator (walk
`cells_dir`'s immediate subdirs, optionally walk Go packages under
`roots`); none exists today.

---

## Q2 — Tree classification: what `validate_against_tree` classifies, and what a Go/TS "tree" would be

**Rust.** `validate_against_tree` (`config.rs:266-326`) classifies
**workspace members** — crate directories. Its inputs are the top-level
Rust `roots` (`config.rs:39`). It expands `<dir>/*` into the subdirs that
carry a `Cargo.toml` (`config.rs:299-306`) and literal roots through
`crate_dir_name` (`config.rs:307-309`), builds an `on_disk` set, then
asserts every on-disk crate is in `gated ∪ exempt`
(`config.rs:311-317`, error "`<c>` is neither gated nor exempt — classify
it") and every listed name matches a real dir (`config.rs:318-324`, error
"`<c>` is listed but no crate directory matches it — typo?"). It also
rejects duplicates, both-listed crates, and empty reasons
(`config.rs:271-289`). It returns `Result<()>` and reads **only** the Rust
top-level fields — never `[go]` or `[typescript]`.

Crucially, `validate_against_tree` is invoked **only by the Rust driver**
(`rust-ai-native-conform/src/lib.rs:119` in `run_check`, `:188` in
`run_freeze`). The Go driver (`go-ai-native-conform/src/lib.rs:86-132`) and
the TS driver (`typescript-ai-native-conform/src/lib.rs:84-135`) never call
it. The Rust tree invariant is therefore both *Rust-only in scope* and
*Rust-only in invocation*.

**Go — what a "tree" would be, by existing discovery.** The natural tree
units are (a) Go *packages* under `[go].roots`, and (b) *cells* under
`cells_dir`. No existing function classifies either. The closest
discoveries are file-grain: `go_sources` (`store.rs:388`) returns files
tagged by root basename; `GoCellIsolation::cell_of_file` (`go.rs:207`) and
`cell_of_import` (`go.rs:217`) derive a cell from a single path/import but
never collect the set. `go-extract` reports the file's package only
implicitly (it does not emit a package fact). There is **no** Go equivalent
of `validate_against_tree` — nothing that enumerates packages or cells and
nothing that checks coverage against a list (there is no Go list to check
against).

**TS — what a "tree" would be, by existing discovery.** The natural tree
units are *cells* under `cells_dir`. No existing function classifies them.
The closest discoveries are file-grain: `typescript_sources`
(`store.rs:305`) returns files tagged by root basename;
`TsCellIsolation::cell_of` (`typescript.rs:144`) derives a cell from one
path. There is **no** TS equivalent of `validate_against_tree`.

**Not found (stated explicitly).** No function in the engine or any
frontend enumerates the full set of Go packages, Go cells, or TS cells, and
none classifies any of them as gated or exempt. The gated-or-exempt
invariant exists for Rust crates only.

---

## Q3 — The empty default: what actually runs and where a false green hides

**Rust — protected by two announce mechanisms.** On empty/underscoped
`roots`, `workspace_sources` (`store.rs:225`) returns `[]`, so `run_check`
gets zero facts. But the Rust driver calls `warn_vacuously_gated`
(`rust-ai-native-conform/src/lib.rs:100-108`, invoked at `:131` in
`run_check` and `:193` in `run_freeze`), which prints every gated crate the
scan attributed no sources to (engine: `Config::vacuously_gated`,
`config.rs:349-357`). It also prints the gated/exempt count summary
(`rust …/lib.rs:168-172`) and the `ConfigOrigin::Defaulted` banner when no
`conform.toml` exists (`rust …/lib.rs:33-36`). Residual Rust surface: if
`gated_crates` is *also* empty there is nothing to warn about — but "0
gated, 0 exempt" is still printed (`rust …/lib.rs:168-172`), so the
nothing-gated state announces itself rather than masquerading as a
configured green.

**Go — silent false green on an empty `[go].roots`.** With
`[go].roots = []` (or roots that match nothing), `go_sources`
(`store.rs:388`) returns `[]` → zero facts. `build_rules` still constructs
`GoUnsafeInDomain::new(Some(cells_dir))` (`go-ai-native-conform/src/lib.rs:53`)
and `GoCellIsolation` when `cells_dir` is set (`go …/lib.rs:56-58`); both
iterate zero facts and emit zero findings; `FileLength` likewise
(`go …/lib.rs:59-61`). The gate is **green with no warning**: the Go driver
has **no** `warn_vacuously_gated` and **no** gated/exempt summary. The only
announce is the `ConfigOrigin::Defaulted` message
(`go …/lib.rs:31-35`), which fires *only when `conform.toml` is absent* —
not when `[go].roots` is empty inside a present file. `probe()`
(`go …/lib.rs:70-72`, delegating to the frontend's `probe`
`go-ai-native-conform-frontend/src/lib.rs:81-84`) runs the extractor with
an *empty* file list (`Some(&[])`) — it verifies the **toolchain** (go,
extractor, protocol) is present, **not** that `roots` produced any files.
So: a present `conform.toml` carrying `[go].roots = []` produces a silent,
vacuous green. (The Go default for `roots` is `["."]`, `config.rs:135`, so
the common case scans the whole module; the false green needs an explicit
empty or non-matching root.)

**TypeScript — identical shape, silent false green on an empty
`[typescript].roots`.** With `[typescript].roots = []`,
`typescript_sources` (`store.rs:305`) returns `[]` → zero facts → zero
findings from `TsUnsafeInDomain`, `TsCellIsolation` (if `cells_dir` set,
`typescript-ai-native-conform/src/lib.rs:51-56`), and `FileLength`
(`:57-59`). No `warn_vacuously_gated`, no gated/exempt summary in the TS
driver. The `ConfigOrigin::Defaulted` message (`ts …/lib.rs:28-32`) fires
only on an absent `conform.toml`. `probe()` (`ts …/lib.rs:68-70`, frontend
`typescript-ai-native-conform-frontend/src/lib.rs:78-87`) checks
node+`typescript` presence, not file coverage. Silent vacuous green on an
empty/underscoped `[typescript].roots` inside a present file. (TS default
for `roots` is `["src"]`, `config.rs:190`.)

**Where false green is possible.** Go and TS, on an empty or non-matching
`[go]`/`[typescript]` `roots` within a *present* `conform.toml`. Rust is
covered by `warn_vacuously_gated` + the gated/exempt summary. (Note: the
fixture and demo configs set the *top-level* `roots = []`
— the Rust roots — because those projects are not Rust; their Go/TS roots
are non-empty, e.g. `research/go-demo/conform.toml:13` and
`research/ts-demo/conform.toml:13`. The top-level `roots = []` is itself
harmless here because neither Go nor TS driver reads top-level `roots` —
`Store::for_go`/`for_typescript` read the sub-tables, `store.rs:75-81` and
`store.rs:64-70`.)

---

## Q4 — FlagSites (R-001): engine body, Rust mounting, what TS mounting needs, registry-file precedent, the Go promise

**Engine body** (`rules/structure.rs:26-75`). `FlagSites` carries two
fields: `registry_file: String` (the one legal construction site) and
`gated_crate: String` (the crate whose constructions are gated)
(`structure.rs:26-31`). `id()` returns `"R-001"` (`structure.rs:34-36`).
`check(&[SourceFacts])` (`structure.rs:41-74`): it first collects the
workspace's cell type names via `cell_types(facts)` (`rules/mod.rs:94-109`,
which keys off `Fact::Item` facts whose attribute text starts with
`cell(`); then for each `SourceFacts` whose `crate_name == gated_crate`
**and** whose `file != registry_file`, it flags any `Fact::Ctor` whose
`type_name` is a cell type (`structure.rs:44-71`). Inputs the rule needs:
(i) cell-typed items (`Fact::Item` with a `cell(` attr), and (ii)
`Fact::Ctor` construction sites. `Fact::Ctor` is documented as "the R-001
signal" (`facts.rs:47-48`).

**Rust mounting — exact mechanics**
(`rust-ai-native-conform/src/lib.rs:53-63`). Inside `build_rules`,
`FlagSites` is pushed **only when both** `config.registry_file` and
`config.registry_gated_crate` are `Some`, and the two are passed straight
through as the struct's `registry_file` / `gated_crate`. On the host both
are set — `registry_file = "crates/vibe-cli/src/registry.rs"` and
`registry_gated_crate = "vibe-cli"` (`conform.toml:17-18`) — so R-001 is
live there. The `Fact::Ctor` facts it consumes are produced by the Rust
frontend, which emits one for every `<Type>::new(...)` call
(`rust-ai-native-conform-frontend/src/lib.rs:356-369`).

**What TS mounting would need.** Three things, none present today:
1. A registry-site config field. `TsConfig` has **no** `registry_file` /
   `registry_pkg` analogue (`config.rs:155-174`); its only path-shaped
   field is `cells_dir`.
2. `Fact::Ctor` (or an equivalent construction-site fact). `ts-extract`
   emits `file_metrics` / `ts_unsafe` / `import` / `item` only
   (`extract.ts:56-88`); it does **not** emit `Ctor`. `Fact::Ctor`
   (`facts.rs:48`) is emitted solely by the Rust frontend
   (`rust-ai-native-conform-frontend/src/lib.rs:365`).
3. Cell types keyed the way `cell_types` expects. `cell_types`
   (`rules/mod.rs:94-109`) discovers cells from `Fact::Item` attrs starting
   with `cell(`; TS cells are *directories* (immediate subdirs of
   `cells_dir`), not attribute-carried types, so `cell_types` would find
   none in a TS tree.
How the engine is fed for TS today: `Store::for_typescript`
(`store.rs:64-70`) → `extract_typescript` (`store.rs:105-113`) →
`typescript_sources` (`store.rs:305`) supplies the file list; the `ts-tsc`
frontend's `extract()` (`typescript-ai-native-conform-frontend/src/lib.rs:99-121`)
returns per-file `Fact`s; the rules receive that flat `Vec<SourceFacts>`
(`typescript-ai-native-conform/src/lib.rs:87-91`). So mounting FlagSites
for TS would require a new "construction/dispatch site" fact, a config
field naming the legal site, and a cell-type notion the rule can match —
all absent.

**Precedent of a flag-registry file in TS demo/guide — none.** `grep` for
`registry|flag|dispatch|select` under `research/ts-demo/` hits only the
*package* registry (npm/vibe) in `vibe.toml`, `package-lock.json`,
`vibe.lock`, `README.md` — nothing in `src/`. `research/ts-demo/src`
contains two cells (`cells/greeting`, `cells/farewell`) and one core module
(`core/text.ts`) and **no registry, no dispatch table, no flag**. The TS
guide states this on the record: the runtime-flag tier is "*Specified, not
built: no registry object exists. `research/ts-demo/src` is two cells and
one core module … with no registry, no dispatch table and no flag*"
(`GUIDE-AI-NATIVE-TYPESCRIPT.md:173`), and the typed-registry anchor is
"*Specified, not built (→ B-039) … The nearest built thing is adjacent
rather than this: the vendored conform engine exports a `FlagSites` rule …
that would police WHERE flags become cells … but it is Rust-shaped
(`gated_crate`, a `registry.rs` path) … and it is constructed only in the
engine's own `rules/tests.rs`, never on a check path this stack runs*"
(`GUIDE-AI-NATIVE-TYPESCRIPT.md:175`).

**Does the Go guide promise the same rule — yes; does the Go frontend
mount it — no.** The Go guide carries the R-001 binding:
`##R-001-BINDING-FLAG-AT-THE-SEAM` — "R-001 binding — flag at the seam,
never in the veins:" (`GUIDE-AI-NATIVE-GO.md:366`), with the code comment
"// internal/registry — the only flag reader and the only package permitted
to import cell packages." (`GUIDE-AI-NATIVE-GO.md:369-370`), and the cells
section: "**`internal/registry` is the only package that imports cell
packages** (§6)." (`GUIDE-AI-NATIVE-GO.md:174`). The Go demo instantiates
it: `research/go-demo/internal/registry/registry.go:1-3` opens "Package
registry is the composition root's selector: the ONLY package that imports
cell packages, and the only flag reader (R-001 …)". The Go config even
reserves the field — `GoConfig::registry_pkg` (`config.rs:125`) — but its
doc says it "carries no rule", and `go-ai-native init` merely *writes* it
into `[go]` (`GUIDE-AI-NATIVE-GO.md:580-581`). The Go driver's `build_rules`
(`go-ai-native-conform/src/lib.rs:51-63`) pushes only `GoUnsafeInDomain`,
`GoCellIsolation`, and `FileLength` — **no FlagSites, no registry rule**.
And `GoCellIsolation` deliberately does *not* enforce sole-importer status:
it only fires on files **inside** `cells_dir` (`go.rs:240-243`); a file
outside `cells_dir` (e.g. `cmd/...` or any non-registry package) may import
a cell freely (`go.rs:394-406`, test `files_outside_cells_dir_import_cells_freely`).
So Go has the guide promise, the config field, and a demo instance — and
zero enforcement.

---

## Q5 — The three rosters, verbatim

Each `build_rules`, in push order, with the rule id and one line on what
it checks. (The engine renders ids in the SARIF driver order; the rosters
are built in one place each so `run_check`/`run_freeze`/the TCG oracle
cannot drift — `rust …/lib.rs:41-52`, `go …/lib.rs:40-50`,
`ts …/lib.rs:37-47`.)

**Rust** — `rust-ai-native-conform/src/lib.rs:53-93`:
- `R-001` (`FlagSites`, conditional on `registry_file`+`registry_gated_crate`, `:55-63`) — cell constructors appear only in the selection registry file.
- `R-002` (`CellIsolation`, `:64`) — a cell module imports seams and core only, never a sibling cell.
- `UnsafeGate` (`:65`, gated by `audit_crates`) — `unsafe` lives only in designated audit crates or fn-grain `#[spec(deviates)]` sites.
- `SeamHasDoctest` (`:68`, gated by `gated_crates`) — a seam carries a compiled doctest.
- `PubDoctest` (`:71`, gated by `gated_pub_doctest`) — the whole public type surface carries compiled doctests.
- `ErrorEnumCitesReq` (`:74`, gated by `gated_crates`) — an error enum cites its REQ edge.
- `cell-has-oracle` (`CellHasOracle`, `:77`) — every `#[cell]` type is referenced by an integration test (the differential oracle).
- `ErrorMessageCitesReq` (`:78`, gated by `gated_crates`) — error messages speak the REQ grammar.
- `file-length` (`FileLength`, `:81`) — per-file line budget.
- `no-unwrap-in-domain` (`NoUnwrapInDomain`, `:84`, gated by `gated_crates`) — no `unwrap`/`expect` in domain logic.
- `ambient-env` (`AmbientEnv`, `:87`, gated by `gated_crates`/`audit_crates`, scoped by `env_roots`) — `std::env` reads live only in sanctioned files.

**TypeScript** — `typescript-ai-native-conform/src/lib.rs:48-61`:
- `ts-unsafe-in-domain` (`TsUnsafeInDomain`, `:50`) — the §8 ban set (`any`, cross-type `as`, non-null `!`, `@ts-ignore`, unreasoned `@ts-expect-error`).
- `ts-cell-isolation` (`TsCellIsolation`, conditional on `cells_dir`, `:51-56`) — a cell file imports a sibling only through its seam module.
- `file-length` (`FileLength`, `:57`) — per-file line budget.

**Go** — `go-ai-native-conform/src/lib.rs:51-63`:
- `go-unsafe-in-domain` (`GoUnsafeInDomain`, `:53`, scoped by `cells_dir`) — the §2/§5/§7 ban census (`init()`, blank imports, ambient defaults, naked `go`, error-string matching, `t.Skip`, reasonless suppression, seam-error-without-REQ).
- `go-cell-isolation` (`GoCellIsolation`, conditional on `cells_dir`, `:56-58`) — a cell file may not import a sibling cell at all.
- `file-length` (`FileLength`, `:59`) — per-file line budget.

(The Go guide itself documents its three-rule roster:
`GUIDE-AI-NATIVE-GO.md:192`, "the Go gate registers exactly three rules —
`GoUnsafeInDomain`, `GoCellIsolation`, `FileLength`". It cites
`lib.rs:53-60`; the live range is `lib.rs:51-63`.)

---

## Q6 — Guide promises about flags, verbatim

**TypeScript — the two packet anchors and their surrounding registry
clauses** (`GUIDE-AI-NATIVE-TYPESCRIPT.md`, §7 "Registry, flags & the
composition root", `:161`):

> `:163` `##SAME-RULE-AS-RUST-SHARPENED-BY-ERASURE` The Rust guide forbids `if flag` in domain logic; the same rule holds in TypeScript, and the erasure boundary (§2) sharpens it.
>
> `:165` `##FLAGS-READ-ONCE-AT-THE-COMPOSITION-ROOT` Flags and external configuration are read **once, at the composition root** (the app/entry cell), narrowed there through a schema (so `process.env` — pure untyped exterior — is validated and typed exactly once), and a **registry** (a typed `as const` map, or a discriminated-union selector) chooses the cell/strategy.
>
> `:167` `##NO-IF-FLAG-IN-DOMAIN-CELLS` **No `if (flag)` scattered through domain cells** (R-001).
>
> `:173` `##TIER-RUNTIME` … *Specified, not built: no registry object exists. `research/ts-demo/src` is two cells and one core module (`cells/greeting`, `cells/farewell`, `core/text.ts`) with no registry, no dispatch table and no flag; nothing in the stack or the host constructs one either. The tier is described, never instantiated.*
>
> `:175` `##FLAG-REGISTRY-IS-TYPED-DATA-WITH-PROVENANCE` The flag/registry is **typed data with provenance, birth, and sunset** … *Specified, not built (→ B-039): no such table exists in the stack, in the host or in `research/ts-demo` … The nearest built thing is adjacent rather than this: the vendored conform engine exports a `FlagSites` rule (`crates/vendor/core-ai-native-conform/src/rules/structure.rs`) that would police WHERE flags become cells — one legal construction site — but it is Rust-shaped (`gated_crate`, a `registry.rs` path), it says nothing about the table's provenance, and it is constructed only in the engine's own `rules/tests.rs`, never on a check path this stack runs.*
>
> `:177` `##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED` *Rule:* flags are read at the composition root and dispatched through a typed registry; `if (flag)` in a domain cell, or reading config outside the root, requires `deviates` + reason.

**Rust — the registry & flags section** (`GUIDE-AI-NATIVE-RUST.md`, §5
"Registry & flags", `:83`):

> `:85` `##FLAGS-READ-ONCE-AT-THE-COMPOSITION-ROOT` *(From GUIDE-RUST-v0.1, retained.)* Flags read once at the composition root; a registry selects cells; **no `if flag` in domain logic** (R-001).
>
> `:87` `##EXPLICIT-MATCH-OVER-LINK-TIME-MAGIC` Explicit `match` at the composition root over link-time magic — "one match is the system's table of contents."
>
> `:89` `##TWO-TIERS-OF-FLAGS` Two tiers: cargo features (code in binary) vs runtime flags (cell selected).
>
> `:91` `##FLAG-REGISTRY-IS-DATA-WITH-PROVENANCE` The flag registry is data with provenance, birth, and sunset.

`registry_file` on practice — what it points at: in the **host**
`conform.toml` it is `registry_file = "crates/vibe-cli/src/registry.rs"`
paired with `registry_gated_crate = "vibe-cli"` (`conform.toml:17-18`) —
the one file in the `vibe-cli` crate where selection flags may become
cells. In the **package** policies it is absent: `research/rust-demo/conform.toml`
has no `registry_file` (and no `registry_gated_crate`); the TS and Go demos
carry no Rust-side keys at all. So `registry_file` is set only in the host
policy.

**Go — the registry & flags section** (`GUIDE-AI-NATIVE-GO.md`, §6
"Registry, flags & the composition root", `:362`):

> `:364` `##kind-line-registry` `req r1` *(≈ Rust §5, TS §7)*
>
> `:366` `##R-001-BINDING-FLAG-AT-THE-SEAM` R-001 binding — flag at the seam, never in the veins:
>
> `:368-379` (code block) `// internal/registry — the only flag reader and the only package // permitted to import cell packages.` followed by a `Planner(cfg Config, …) seams.Planner` switch over `cfg.Planner`.
>
> `:174` (cells section) **`internal/registry` is the only package that imports cell packages** (§6).

`registry_pkg` in practice — what it points at: the Go demo sets
`registry_pkg = "internal/registry"` (`research/go-demo/conform.toml:17`),
alongside `cells_dir = "internal/cells"` (`:15`) and
`seams_pkg = "internal/seams"` (`:16`); `go-ai-native init` writes all
three from `go.mod` topology (`GUIDE-AI-NATIVE-GO.md:580-581`). The field
is documented as carrying **no rule** (`config.rs:122-125`), and indeed no
Go rule consumes it.

---

## Cross-cutting findings (for the B-034 / B-039 / B-035 fork)

1. **Gate-unit classification exists for Rust only.** `validate_against_tree`
   (`config.rs:266`) + `gated_crates`/`exempt` cover cargo-workspace
   members; the Go/TS drivers never call it and have no analogue
   (`go …/lib.rs:86`, `ts …/lib.rs:84`). Extending the invariant to Go/TS
   means inventing the unit (Go package or cell; TS cell), the enumerator
   (walk `cells_dir` subdirs / Go packages under `roots`), and the list
   fields — none of which exist today (Q1, Q2).
2. **FlagSites is structurally Rust-bound.** It keys on `Fact::Ctor`
   (`facts.rs:48`, `structure.rs:49`), emitted only by the Rust frontend
   (`rust-ai-native-conform-frontend/src/lib.rs:365`), and on `cell(`-attr
   items (`rules/mod.rs:100`); TS/Go extractors emit neither. Mounting it
   on TS (B-039) needs a new construction-site fact + a registry config
   field + a cell-type match; mounting the Go §6 promise needs at least a
   "registry is the sole cell importer / sole flag reader" rule, which
   `GoCellIsolation` explicitly does *not* provide (`go.rs:394-406`) (Q4).
3. **False-green locus is Go and TS.** Both scan `roots` into a flat file
   list with no vacuous-green warning; only Rust has `warn_vacuously_gated`
   (`rust …/lib.rs:100`). A present `conform.toml` with empty
   `[go]`/`[typescript]` `roots` is silently green (Q3).
4. **The roster asymmetry is recorded in the guides themselves.** The TS
   guide flags the missing registry as `→ B-039`
   (`GUIDE-AI-NATIVE-TYPESCRIPT.md:175`) and the runtime-flag tier as
   "described, never instantiated" (`:173`); the Go guide documents its
   own three-rule roster (`GUIDE-AI-NATIVE-GO.md:192`) while still
   promising R-001 at the seam (`:366`).
