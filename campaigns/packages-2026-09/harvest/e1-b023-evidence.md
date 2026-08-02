# E1 · B-023 — TS/JS syntactic tier + Python frontend: feasibility evidence

**Date:** 2026-08-03
**HEAD:** `779b3aaa docs(campaign): коэффициент параллельности — до 5 на запускалку, 10 всего`
**Subject path:** `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/ENGINE-CONFORM-v0.1.md`
**Genre:** evidence only. Every in-tree claim carries `file:line`; every
absence claim names its perimeter and search terms; every model-knowledge
line is marked «model knowledge — verify at build time». **No verdicts, no
build/skip recommendation** — that stays with the boss. The prior
measurement F-146 (`campaigns/packages-2026-09/harvest/d7a-core-sync-reverify.md:922`)
is a starting point only and is re-measured here at `779b3aaa`; it is cited
as a pointer, never as evidence.

**Default search perimeter** (used for every absence claim below unless a
section widens it):

- ENGINE: `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/`
- DRIVERS (incl. `tools/` sidecars and `tests/`):
  - `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/`
  - `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/`
  - `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/`
- HOST: `crates/`, `xtask/`, `tools/`, `schemas/`

Excluded always: `legacy-spec/**`, `target/**`, `node_modules/**`,
`.vibe/cache/**`, `vibedeps/**` (vendored copies are mirrors only). Never
treated as evidence: `campaigns/**`, `refs/**`.

---

## Q1 — the engine's frontend seam

### What the spec promises

The §2 frontend table (`ENGINE-CONFORM-v0.1.md:44-49`) draws the trait as:

```rust
trait Frontend {
    fn lang(&self) -> Lang;
    fn tier(&self) -> Tier;
    fn extract(&self, files: &[SourceFile]) -> Result<Vec<Fact>, FrontendError>;
}
```

and the fact store's cache key is named at `##KEY-IS-FILE-HASH-PLUS-FRONTEND-VERSION`
(`ENGINE-CONFORM-v0.1.md:81`): *"(file content-hash, frontend id+version)"*.
The tier vocabulary `##TIER-VOCABULARY` is at `:59`.

### What a frontend must implement — the real `Frontend` trait

The trait that actually binds lives in the engine crate, and **its signature
is not the one the §2 table draws** (the spec's own annotation
`##BALANCE-IS-EXPLICIT-THROUGH-ESCALATION-TIERS` at `:11` already concedes the
tier half of this):

- `core-ai-native-conform/src/facts.rs:176-189` — `pub trait Frontend`:
  - `:177` `fn id(&self) -> &'static str;`
  - `:178` `fn version(&self) -> &'static str;`
  - `:181` `fn extract(&self, file: &str, crate_name: &str, module: &str, text: &str) -> Vec<Fact>;`
  - `:188` `fn warm(&self, _pending_files: &[String]) {}` — default no-op; the doc
    at `:182-187` names its purpose: *«the store calls this ONCE per run with
    every repo-relative file whose facts are not already cached … A frontend
    with per-invocation process overhead (`ts-tsc` spawns node) extracts the
    whole pending set here and serves `extract` from memory; in-process
    frontends (rust-syn) keep the no-op default.»*

**Divergence from the spec block, measured:** the real trait has **no
`lang()`** and **no `tier()`**. `grep -nE 'fn lang|fn tier|enum Tier|enum Lang'`
over `core-ai-native-conform/src/` returns **zero** matches. The extract
signature is also narrower than drawn: it takes **one file's bytes plus a
computed module path**, not a `&[SourceFile]` slice, and returns `Vec<Fact>`,
not `Result<Vec<Fact>, FrontendError>` (there is no `FrontendError` type — a
per-file extraction failure is an empty `Vec`, per `##FRONTEND-CRASH-…` at
`:63`). The module path is computed by the engine, not the frontend
(`store.rs:454-470` `module_path`).

### The `Fact` type a frontend produces

- `core-ai-native-conform/src/facts.rs:23-135` — `pub enum Fact` (`#[serde(tag = "fact", rename_all = "snake_case")]`),
  ten variants, each `file:line`:
  - `:27` `Item { kind, symbol, line, attrs: Vec<String>, is_pub: bool, has_doctest: bool }`
  - `:42` `Import { from_module, to_path, line }`
  - `:48` `Ctor { type_name, line }`
  - `:58` `UnsafeUse { context, line, in_test, in_deviation }`
  - `:66` `ErrorVariant { enum_symbol, variant, message, line, enum_attrs }`
  - `:76` `FileMetrics { lines }`
  - `:85` `UnwrapUse { method, line, in_test, in_deviation }`
  - `:97` `EnvRead { method, line, in_test, in_deviation }`
  - `:113` `TsUnsafe { kind, line, in_test, reason: Option<String> }` — TS-specific
  - `:129` `GoUnsafe { kind, line, in_test, reason: Option<String> }` — Go-specific
- `facts.rs:149-155` — `pub struct SourceFacts { file, crate_name, facts: Vec<Fact> }`
  is the per-file carrier that `extract`/`warm` results are folded into.

**There is no Python variant.** The enum closes at `GoUnsafe`; an
extraction of Python source today has no fact shape to emit. (Absence
perimeter: `core-ai-native-conform/src/facts.rs`; search terms `python`,
`Py` — no variant is named for the language, unlike `TsUnsafe`/`GoUnsafe`.)

### How the engine consumes it — `store.rs` entry points

`core-ai-native-conform/src/store.rs` exposes one workspace-extraction entry
point **per language**, each taking a caller-supplied `&dyn Frontend`
(the language choice is made at the call site, outside the engine — the
spec's `##ENGINE-RUNS-THE-CHEAPEST-ADEQUATE-FRONTEND` at `:26` already
records this):

- `:45-49` `pub struct Store { root, roots, exclude }` — the cache root is
  `<repo>/target/conform/facts/`.
- `:52` `Store::at_repo(repo, config) -> Store` — Rust view (roots from `[roots]`).
- `:64` `Store::for_typescript(repo, config) -> Store` — TS view (roots from `[typescript]`); doc `:62-63`: *«the cache directory is shared (slots are keyed by frontend id+version, so the two languages never collide).»*
- `:75` `Store::for_go(repo, config) -> Store` — Go view (roots from `[go]`).
- `:92` `extract_workspace(repo, &dyn Frontend, &mut ExtractionLog) -> Result<Vec<SourceFacts>>` — Rust.
- `:105` `extract_typescript(...)` — flat walk over `.ts/.tsx/.mts/.cts`, `.d.ts` and `node_modules`-style trees skipped (`TS_SKIP_DIRS` at `:289-297`).
- `:118` `extract_go(...)` — flat walk over `.go`, `vendor`/`testdata`/`_test.go` walked (`GO_SKIP_DIRS` at `:372-379`).
- `:132-179` `extract_sources(...)` — the **shared cache loop** every entry point funnels through.

**There is no Python entry point.** `grep -inE 'python|extract_python|for_python|Py'`
over `core-ai-native-conform/src/` returns **zero** matches — no
`Store::for_python`, no `extract_python`, no `python_sources` walker. The
three language views exhaust the store's surface. (Absence perimeter:
`core-ai-native-conform/src/store.rs`; same terms.)

Rules consume facts through a separate, smaller seam:
`core-ai-native-conform/src/finding.rs:53-57` — `pub trait Rule { fn id(&self); fn why(&self); fn check(&self, facts: &[SourceFacts]) -> Vec<Finding>; }`; the engine-wide runner is `:79` `pub fn check(rules: &[&dyn Rule], facts: &[SourceFacts], scope: Option<&str>) -> Vec<Finding>`. A rule never sees which frontend produced the facts — it sees only `SourceFacts`.

### The "facts class" caching wrapper — key composition

The cache key the spec names at `:81` is exactly what the code computes, and
it is the only thing that makes the store incremental:

- `store.rs:83-87` `fn slot(&self, frontend: &dyn Frontend, content_hash: &str) -> PathBuf` —
  `self.root.join("{id}-{version}").join("{content_hash}.json")`. **Key = `(frontend id+version, file content-hash)`.**
- `store.rs:191-203` `pub fn content_hash(text: &str) -> String` — `sha256:`
  over LF-normalised text (CRLF-invariant; asserted `store.rs:186-189`).
- The loop that honours the key: `store.rs:140-151` computes each file's hash,
  builds its slot, and collects cache-misses into `pending`; `store.rs:152-154`
  calls `frontend.warm(&pending)` **once** (the batch hook); `store.rs:156-171`
  serves each file from its slot if present, else calls `frontend.extract(...)`
  and writes the slot. `ExtractionLog` (`store.rs:18-24`) records `extracted`
  (misses) vs `cached` (hits) — the producer log the run-twice-diff test
  asserts on.

So a frontend's identity in the cache is exactly the `(id, version)` pair the
trait exposes (`facts.rs:177-178`); bumping `version` retires every slot for
that frontend wholesale (the doc at `facts.rs:6-8` states facts never
deserialize across schemas). The shared cache directory means a TS frontend
and a (hypothetical) Python frontend coexist without collision — which is the
property `for_typescript`'s doc relies on at `store.rs:62-63`.


---

## Q2 — the Rust T-syn precedent (in-process, `syn`-based)

This is the in-process precedent the spec names at `##ROW-TIER-T-SYN`
(`ENGINE-CONFORM-v0.1.md:31`) (*«the `syn` half is real and running
(`rust-ai-native-conform-frontend`, whose own module doc calls itself "the
Rust T-syn frontend")»*) — and the shape RustPython would imitate.

### Crate, file, size

- Crate `rust-ai-native-conform-frontend` — `Cargo.toml:2`; description
  (`Cargo.toml:14`): *«The conform engine's Rust T-syn frontend
  (ENGINE-CONFORM §2): syn-precision facts — items, attributes, imports,
  ctor sites, unsafe»*.
- **One extractor file**, `src/lib.rs` (**405 LOC**); unit tests out-of-line
  in `src/lib/tests.rs` (**268 LOC**, pulled in via `#[cfg(test)] #[path = "lib/tests.rs"] mod tests;` at `lib.rs:403-405`); integration test
  `tests/engine.rs` (**152 LOC**). (`wc -l`.)
- Dependencies (`Cargo.toml:16-20`): `conform-core.workspace = true`,
  `specmark.workspace = true`, `proc-macro2.workspace = true`,
  `quote.workspace = true`, `syn.workspace = true` — the workspace pins the
  `syn` version (not redeclared here; see Q6).

### The frontend surface

- `lib.rs:40` `pub struct RustFrontend;` — a zero-sized seam (doc `:19`: *«construct it directly and call `extract`»*).
- `lib.rs:42-89` `impl Frontend for RustFrontend`:
  - `:43-45` `id() = "rust-syn"`.
  - `:47-58` `version() = "6"`, with an inline bump log (`:48-57`) recording each shape change and the rule *«Bump when extraction changes shape — the store key includes it, so old cached facts are simply never read again.»*
  - `:60-89` `extract(...)` — `syn::parse_file(text)` (`:61`); an unparseable file returns `Vec::new()` (`:61-63`, B5: zero facts, no error); the fact list is opened with `FileMetrics { lines }` (`:66-68`) and then sorted by line (`:73-86`).
- **No `warm` override** — `RustFrontend` takes the trait's no-op default (`facts.rs:188`), because parsing is in-process and per-file with no startup cost. (This is the contrast the default's own doc draws at `facts.rs:182-187`.)

### What it extracts — every extraction, `file:line`

The extraction is a `syn::visit::Visit` walk (`lib.rs:14` `use syn::visit::Visit`; `lib.rs:189` `impl<'ast> Visit<'ast> for Extractor`). The visitor state is `struct Extractor { module, facts, test_depth, deviating_depth }` (`lib.rs:91-104`) — the two depth counters carry `#[cfg(test)]`/`#[test]`/`#[spec(deviates=…)]` scope *as data* on the facts inside.

| Visit method | `lib.rs` | Fact emitted |
|---|---|---|
| `visit_item_fn` | `:190` | `Item{kind:"fn"}` (`:191-198`); `UnsafeUse{context:"fn …"}` if `unsafe fn` (`:209-216`) |
| `visit_impl_item_fn` | `:226` | `UnsafeUse` if `unsafe fn` in an impl (`:233-240`) — v5 fix, were invisible before |
| `visit_item_mod` | `:247` | no fact; toggles `test_depth` on `#[cfg(test)]` (`:248-255`) |
| `visit_expr_method_call` | `:258` | `UnwrapUse` for `.unwrap()`/`.expect(...)` (`:260-267`) |
| `visit_item_struct` | `:271` | `Item{kind:"struct"}` (`:272-279`) |
| `visit_item_enum` | `:283` | `Item{kind:"enum"}` (`:284-291`); `ErrorVariant` per `#[error("…")]` variant (`:294-325`) |
| `visit_item_trait` | `:329` | `Item{kind:"trait"}` (`:330-337`) |
| `visit_item_use` | `:341` | `Import{from_module, to_path}` (`:348-352`, path re-rendered via `ToTokens`) |
| `visit_expr_call` | `:356` | `Ctor{type_name}` for `<Type>::new(...)` (`:364-369`); `EnvRead` for `env::{var,var_os,set_var,remove_var}` (`:374-387`) |
| `visit_expr_unsafe` | `:392` | `UnsafeUse{context:"block"}` (`:393-398`) |

**Item kinds produced:** `fn`, `struct`, `enum`, `trait` (no `mod`, `const`, `static`, `type`, `impl`-block facts — the visitor has no `visit_item_const`/`_static`/`_type`/`_impl`). **Attributes carried:** `attr_text` (`lib.rs:146-159`) emits only `spec(...)` / `cell(...)` / `verifies(...)` attribute *text* (verbatim token stream), nothing else. **Spans:** `line_of` (`lib.rs:185-187`) via `syn::spanned::Spanned` — a start line only, no end, no byte span. **Visibility + doctest:** `is_pub` (`:181-183`) and `has_doc_fence` (`:166-179`, a ``` fence in a doc comment) ride on `Item`.

**Deliberately not produced here:** `Fact::TsUnsafe` / `Fact::GoUnsafe` — the sort arm at `lib.rs:84-85` covers them with the comment (`:82-83`) *«Never produced by rust-syn — the ts-tsc and go frontends own these.»* The fact model is shared; each frontend owns its language's variants.

### Tests

- **Unit (`src/lib/tests.rs`, 9 `#[test]`s):** `extracts_items_with_cell_and_spec_attrs` (`:15`), `extracts_imports_ctors_and_unsafe` (`:34`), `unparseable_source_yields_no_facts` (`:63`), `emits_file_metrics_for_parsed_files` (`:68`), `unwrap_in_domain_vs_test_scopes` (`:79`), `unwrap_in_deviation_scopes_fn_grain_only` (`:114`), `unsafe_scoping_sees_tests_testimony_and_impl_methods` (`:153`), `extracts_visibility_and_doctest_presence` (`:200`), `extracts_thiserror_variants_with_enum_attrs` (`:236`).
- **Integration (`tests/engine.rs`, 2 `#[test]`s over a synthetic mini-workspace `mini_workspace` at `:17-38`):**
  - `:54` `incremental_one_file_diff_reextracts_one_file` — cold run extracts 4 (`:64`), warm run all-cache (`:71-72`), a 1-file edit re-extracts exactly that 1 file (`:83-87`), and a `Cargo.lock` touch invalidates nothing (`:93-99`, asserting the store key excludes toolchain context).
  - `:103` `findings_and_sarif_are_deterministic_and_baseline_gates` — **the run-twice-diff** (`:121-124`, `assert_eq!(findings_a, findings_b)` / `assert_eq!(sarif_a, sarif_b, "same inputs — byte-identical SARIF")`) plus baseline freeze/thaw (`:145-151`). This is the test F-146 (`d7a-…:982-991`) identified as the whole-pipeline determinism proof.

### What the precedent is, as a shape

A single in-process file: a zero-sized `Frontend` impl whose `extract` parses one file's text with a real parser, walks the AST with a `Visit`-style visitor, pushes `Fact`s (opened with `FileMetrics`, scoped by test/deviation depth counters), returns them sorted by line, and tolerates unparseable input as an empty vec. Identity in the cache is `(id, version)`; nothing else crosses a process boundary. **No `warm`, no sidecar, no node/python child process.** This is the shape the spec's `##ROW-FRONTEND-PYTHON` T-syn cell (`ENGINE-CONFORM-v0.1.md:57`, *«RustPython parser (MIT) in-process»*) names, and the shape RustPython would slot into directly.


---

## Q3 — the TS semantic sidecar precedent (`ts-extract`)

This is the deep-semantic frontend the spec's `##ROW-FRONTEND-TS-JS`
T-sem cell names (`ENGINE-CONFORM-v0.1.md:55`: *«TypeScript compiler API
via a Node **sidecar process**»*). The packet asks for the wire protocol,
the facts, the size, the tests, the runtime prerequisite, and the
sidecar-generic vs TS-specific split.

### Three pieces, three files

| Piece | File | LOC | Role |
|---|---|---|---|
| Frontend adapter | `typescript-ai-native-conform-frontend/src/lib.rs` | 138 | `impl Frontend` (`TsTscFrontend`), `warm`/`extract` over an in-memory cache |
| Sidecar bridge (Rust) | `typescript-ai-native-extract-bridge/src/lib.rs` | 281 | process spawn, NDJSON parse, error taxonomy, lowering |
| Sidecar (node) | `tools/ts-extract/extract.ts` | 541 | the TypeScript Compiler-API extractor |
| Runtime manifest | `tools/ts-extract/package.json` | 12 | node/typescript prereq |

(`wc -l`.) The frontend's own doc (`frontend/src/lib.rs:1-15`) states the
split: *«a [`conform_core::Frontend`] whose facts come from the TypeScript
Compiler API, via the packaged `tools/ts-extract` extractor and the
`typescript-ai-native-extract-bridge` protocol … `warm()` runs ONE node
process for every cache-missed file and parks the lowered facts in memory;
`extract()` then serves per-file from that cache.»*

### Wire protocol — how invoked, what format comes back

**Rust → node invocation** (bridge `src/lib.rs:160-185` `extract_tree`):
- `:165` `Command::new("node")`.
- `:166-172` args: `<extractor script> --root <project_root> [--files <a> <b> …]` — the `--files` list is the store's `pending` set (the batch hook), so **one node process serves the whole cache-miss set**.
- `:173-175` `cmd.output()`; a spawn failure → `BridgeError::NodeMissing` (`:175`, fix surface *«install node >= 22.6»* at `:34`).
- `:177-183` exit-code → error mapping: success continues; **exit 3 → `TypescriptUnresolvable`** (`:179-181`); any other non-zero → `ExtractorFailed` (`:182`).
- `:184` stdout parsed by `parse_ndjson`.

**node → Rust format** (NDJSON, one record per source file):
- `extract.ts:534` `process.stdout.write(JSON.stringify(record) + "\n")` — newline-delimited JSON, one `FileRecord` per line. This is exactly the spec's `##SIDECAR-PROTOCOL-IS-NDJSON-OVER-STDIO` (`ENGINE-CONFORM-v0.1.md:61`: *«newline-delimited JSON over stdio, versioned; sidecars emit Facts, nothing else»*).
- `FileRecord` shape (bridge `src/lib.rs:103-111`; mirrored `extract.ts:98-105`): `{ protocol: u64, file: String, in_test: bool, degraded: bool, facts: Vec<RawFact>, markers: Vec<RawMarker> }`.
- `RawFact` enum (bridge `src/lib.rs:68-90`, `#[serde(tag = "fact", rename_all = "snake_case")]`): `TsUnsafe { kind, line, reason }`, `Import { to_path, line }`, `Item { kind, symbol, line, is_exported, has_doc_example }`, `FileMetrics { lines }`.
- `RawMarker` (bridge `src/lib.rs:93-100`): the §9 JSDoc spec tags (`@implements`/…), separate from conform facts.
- **Protocol versioning:** `pub const PROTOCOL: u64 = 1` (bridge `src/lib.rs:24`); `const PROTOCOL = 1` (`extract.ts:33`). A record whose `protocol != 1` → `BridgeError::Protocol` (bridge `src/lib.rs:124-128`); the doc at `:21-23` ties a protocol bump to the `ts-tsc` frontend `version`, retiring conform cache slots. `parse_ndjson` (`:115-132`) is **pure** over the recorded stream — testable without node.

### What facts it extracts (TS-specific, in `extract.ts`)

Per file (`extractFile`, `extract.ts:315-494`), opened with `file_metrics` (`:326`):
- **Items** (`declarationInfo` `:278-313`): `function`, `class`, `interface`, `type`, `enum`, `const`, `module` — each with `is_exported` (an `export` modifier) and `has_doc_example` (a fence or `@example` in the first 2000 chars of the node text, `:414`).
- **Imports** (`:373-403`): import/export declarations' module specifiers **and** dynamic `import("…")`/`require("…")` — the `to_path` is the specifier text.
- **`ts_unsafe` kinds** (`GUIDE-AI-NATIVE-TYPESCRIPT §8`): `any_type` (the `any` keyword, `:342-349`), `as_cross` (an `as` cast that is not `as const`, `:350-364`), `non_null` (the `!` operator, `:365-372`), `ts_ignore`/`ts_expect_error` (from comment **trivia** via a separate `createScanner` walk, `:438-483` — these live in comments, not the AST).
- **Spec markers** (not conform facts): `@implements`/`@verifies`/`@documents`/`@deviates`/`@informs`/`@scope` (`SPEC_TAGS` `:36-43`), parsed from raw comment text (`:251-270`, the Phase-0 finding that `@implements`' class-expression slot eats the `spec` scheme).
- **Degradation (B5):** an unparseable file (`createSourceFile` throws, or parses to zero statements, or `visit` throws) yields `degraded: true` with only `file_metrics` (`:329-339`, `:428-435`) — one broken file never blinds the gate.

The bridge lowers these into the engine model via `conform_facts` (`bridge/src/lib.rs:191-225`): `RawFact::Item.is_exported → Fact::Item.is_pub`, `has_doc_example → has_doctest`, **`attrs` is always empty** for TS items (`:218`, the cell/import/spec attribute machinery is Rust-specific), `RawFact::Import` takes the file as its `from_module` (TS modules ARE paths, `:203-207`).

### Process economics — `warm` / `extract` / `probe`

- `TsTscFrontend` (`frontend/src/lib.rs:25-29`) holds `warmed: Mutex<HashMap<String, Vec<Fact>>>`.
- `Frontend::warm` (`frontend/src/lib.rs:99-101`) → `warm_batch(Some(pending))` (`:51-73`): **one node run for the whole pending set**, lowered facts parked in the map by file. Extraction failure prints to stderr and yields an empty per-file set — the gate keeps running (B5).
- `Frontend::extract` (`frontend/src/lib.rs:102-120`): serves from the map; if a file was never warmed (defensive path), it runs a single-file batch (`:114`).
- `probe` (`frontend/src/lib.rs:78-87`): runs the extractor with `Some(&[])` so drivers fail **hard** with the taxonomy's message before a gate run silently yields zero facts — this is the mechanism behind the spec's `##FRONTEND-CRASH-DEGRADES-VISIBLY-NEVER-SILENTLY` (`ENGINE-CONFORM-v0.1.md:63`), cited at `typescript-ai-native-conform/src/lib.rs:66-70`.
- `id() = "ts-tsc"`, `version() = "1"` (`frontend/src/lib.rs:91-98`).

### Fixtures / tests

- **Node side** (`tools/ts-extract/test/extract.test.ts`, 8 `node:test` cases, run by `package.json:7` `node --test`): `one protocol-1 record per source file, sorted` (`:57`), `the unsafe set is AST-classified; string literals never fire` (`:71`), `imports carry the specifier, including sibling-internal paths` (`:93`), `spec markers surface with raw-text URIs` (`:103`), `exported items carry symbol, kind, and export visibility` (`:123`), `file metrics are always present, even for rubble` (`:133`), `a syntactically hopeless file degrades to zero facts, not an error (B5)` (`:140`), `missing typescript resolution exits 3 with the recipe` (`:149`). Fixtures under `tools/ts-extract/test/fixtures/{clean,dirty}/` carry `src/cells/{greet,parse}/…`, `conform.toml`, `specmap.toml`, `specmap.json`, `spec/PROP-001.md`.
- **Bridge side** (`bridge/src/lib.rs:227-281`, 3 tests, pure — no node): `replay_parses_and_lowers_into_engine_facts` (`:248`, drives a recorded `REPLAY` const at `:234-245`), `protocol_mismatch_is_its_own_error_class` (`:269`), `garbage_line_is_a_protocol_error_naming_the_parse` (`:277`).
- **Frontend side** (`frontend/src/lib.rs:123-138`, 1 test): `extractor_materialises_content_addressed_and_idempotent` (`:127`).

### Runtime prerequisite

- **node ≥ 22.6** — the script is erasable-syntax-only TypeScript run under type-stripping, no build step (`extract.ts:21-24`; `package.json:5`; `BridgeError::NodeMissing` fix surface at `bridge/src/lib.rs:34`).
- **The consumer project must resolve `typescript` at runtime** — it is **not bundled**. `devDependencies.typescript = "^6.0.0"` (`package.json:9-11`); resolved from `--root` via `createRequire(pathToFileURL(join(root, "package.json")))` (`extract.ts:143-162`); absence → `exit(3)` (`:156`). The header at `extract.ts:16-19` notes the tsc floor step needs the same install, so *«the structural gate adds no new dependency»*.

### Sidecar-generic vs TS-specific — the module boundary

The split is clean and lives along the crate/script seam:

- **Sidecar-generic (in the Rust `typescript-ai-native-extract-bridge` crate):**
  - process spawn — `Command::new("node")` + arg assembly (`bridge/src/lib.rs:160-172`);
  - NDJSON stream parse — `parse_ndjson` (`:115-132`);
  - error taxonomy — `BridgeError` with four named fix surfaces (`:29-62`);
  - protocol constant + version check (`:24`, `:124-128`);
  - content-addressed extractor materialisation — `materialise_extractor` (`:143-156`) + `EXTRACTOR_SOURCE: &str = include_str!("../../../tools/ts-extract/extract.ts")` (`:138`, so extractor/bridge version skew is impossible — they compile from one tree);
  - exit-code → error mapping (`:177-183`);
  - generic lowering `conform_facts` (`:191-225`).
  - The `warm`-batch orchestration + `Mutex<HashMap>` cache is in the frontend adapter (`frontend/src/lib.rs:51-73`), equally language-neutral.
- **TS-specific (entirely in `extract.ts`):** the `TsModule` structural interface (`:170-210`, the slice of the `typescript` surface used), Compiler-API parsing (`createSourceFile` `:330`), the unsafe-set classification, the JSDoc-marker extraction, the comment-trivia scanner. The bridge's `RawFact`/`RawMarker`/`FileRecord` wire types mirror the TS record shape (they *are* the protocol), but the bridge code around them is plumbing that names no TS concept beyond those wire types.

The `devDependencies` of the bridge crate (`typescript-ai-native-extract-bridge/Cargo.toml:16-20`): `conform-core`, `serde`, `serde_json`, `thiserror` — i.e. exactly the four a generic spawn-and-parse bridge needs, no `typescript`, no node binding.


---

## Q4 — the Go sidecar precedent (`go-extract`), placed beside `ts-extract`

The Go stack mirrors the TS stack crate-for-crate. The boss's question —
*does a third sidecar have a ready-made shape?* — is answered by laying the
two side by side: the protocol is shared, the bridge skeleton is copied,
and the divergences are small and named.

### Three pieces, three files (same shape as Q3)

| Piece | File | LOC | Role |
|---|---|---|---|
| Frontend adapter | `go-ai-native-conform-frontend/src/lib.rs` | 135 | `impl Frontend` (`GoExtractFrontend`), `warm`/`extract`/`probe` |
| Sidecar bridge (Rust) | `go-ai-native-extract-bridge/src/lib.rs` | 364 | process spawn, NDJSON parse, error taxonomy, lowering |
| Sidecar (go) | `tools/go-extract/extract.go` | 674 | the stdlib-only `go/parser`+`go/ast` extractor |

(`wc -l`.) No `package.json`-equivalent — the Go extractor needs no manifest.

### Wire protocol

- **Rust → go** (`go-ai-native-extract-bridge/src/lib.rs:223-248` `extract_tree`): `Command::new(go_binary())` (`:228`), args `run <extractor> --root <project_root> [--files <a> <b> …]` (`:229-238`). `go_binary()` (`:151-153`) returns the `GO_AI_NATIVE_GO` env override (`:32`) or `go` from PATH — the doc at `:29-32` notes this project's own dev box keeps go at `C:/opt/go` off PATH.
- **Overlay form** (`extract_content` `:184-219`, Go-only): `go run extract.go --stdin-file <rel>` with the file's content on **stdin** — used by the oracle relay (TCG-PROTOCOL-GO §3). **TS has no overlay form.**
- **node-equivalent format:** NDJSON, one `record` per file — `extract.go:118-124` `out.Encode(rec)` via `json.NewEncoder(os.Stdout)`. The `record` struct (`extract.go:37-44`) is the same shape as TS's `FileRecord`: `{ Protocol, File, InTest, Degraded, Facts, Markers }`.
- `const protocol = 1` (`extract.go:33`); `pub const PROTOCOL: u64 = 1` (bridge `:27`).
- Exit-code mapping (bridge `:211-213`, `:243-245`): success continues; any non-zero → `ExtractorFailed`. **There is no "Unresolvable" error class** — Go's parser is in the stdlib, so there is no "resolve the language's compiler package" failure mode (contrast TS exit-3 → `TypescriptUnresolvable`).
- `parse_ndjson` (bridge `:124-141`) is byte-for-byte the same logic as the TS bridge's.

### What facts it extracts (Go-specific, in `extract.go`)

Per file (`extractSource` `extract.go:168-194`), opened with `file_metrics` (`:176`):
- **Items** (`funcItem` `:423-440`, `genItems` `:442-474`): `func`, `method` (recv ≠ nil), `type`, `const`, `var`; exported = `ast.IsExported`. **`type` items carry `underlying`** (`:452`, `primitiveUnderlying` `:479-495`) — the primitive underlying of a defined type (`type AccountID string → "string"`), the Go brand signal; aliases and non-primitive underlyings yield `""`.
- **Imports** (`importedPackages` `:268-290`): every import emits an `import` fact; a blank import (`_`) additionally emits `go_unsafe blank_import` (`:283-285`).
- **`go_unsafe` kinds** (the Fact doc `core-ai-native-conform/src/facts.rs:120-127` lists them): `init_decl` (`:437-439`), `blank_import` (`:284`), `ambient_call` (`:566-582`, the `ambientDefaults` package→selector map `:539-556`), `naked_go` (`ast.GoStmt`, `:250-251`), `error_string_match` (`:586-617`, `err.Error()==` / `strings.*` over `.Error()`), `t_skip` (`:629-641`, `_test.go` only), `reasonless_suppression` (`:645-674`, `//nolint`/`//lint:ignore`/`//exhaustive:ignore` without a reason), `seam_error_missing_req` (`:519-535`, an `XxxError` struct with an `Error()` method but no `Spec` field).
- **Markers** (`collectMarkers` `:344-392`): `//spec:` directives (`implements`/`verifies`/`documents`/`deviates`/`informs`/`scope`, `markerTags` `:339-342`), parsed from raw comment text. Unlike TS, **Go markers carry the author-asserted revision `r`** (`:367-374`; bridge `RawMarker.r` `:105`).
- **Degradation (B5):** `parser.ParseFile` returning `nil` → `degraded: true` metrics-only (`:180-183`); a partial parse (`perr != nil`) keeps going with `degraded: true` (`:184-187`).

The bridge lowers via `conform_facts` (`:254-289`): `RawFact::GoUnsafe → Fact::GoUnsafe`, `Import` takes the file as `from_module` (`:266-270`), `Item.is_exported → is_pub`, `has_doc_example → has_doctest`, **`attrs` always empty** (`:282`), and the `underlying` field is dropped on lowering (it has no engine `Fact` home — it lives only in the RawFact/markers layer for the oracle).

### Process economics — `warm` / `extract` / `probe`

Identical pattern to TS: `GoExtractFrontend` holds `warmed: Mutex<HashMap<String, Vec<Fact>>>` (`:32`); `warm` (`:96-98`) → `warm_batch` (`:54-76`) runs **one `go run`** for the pending set; `extract` (`:99-117`) serves from the map with a defensive single-file fallback; `probe` (`:81-84`) runs `extract_tree(…, Some(&[]))` for a hard pre-gate failure. `id() = "go-extract"`, `version() = "1"` (`:88-95`).

### Fixtures / tests

- **Bridge replay tests** (`go-ai-native-extract-bridge/src/lib.rs:291-364`, 4 tests, pure — no go): `replay_parses_and_lowers_into_engine_facts` (`:316`, `REPLAY` const `:299-313` recorded from a live `go run` on the `dirty` fixture 2026-07-17), `protocol_mismatch_is_its_own_error_class` (`:341`), `garbage_line_is_a_protocol_error_naming_the_parse` (`:349`), `extractor_materialises_content_addressed_and_idempotent` (`:355`).
- **Frontend test** (`go-ai-native-conform-frontend/src/lib.rs:124-135`): the same single materialise-idempotent test as TS.
- **No `go test` suite for `extract.go` itself** — unlike the TS extractor's 8 `node:test` cases, the Go extractor has no in-language test of its own. The only `*_test.go` under `tools/go-extract/` is a **fixture** (`test/fixtures/dirty/internal/cells/plan/plan_test.go`). Go-extractor behaviour is pinned solely by the bridge's recorded `REPLAY`.
- Fixtures under `tools/go-extract/test/fixtures/{clean,dirty}/` carry `internal/cells/…/*.go`, `conform.toml`, `specmap.toml`, `specmap.json`, `spec/PROP-001.md`.

### Runtime prerequisite

- **go ≥ 1.24** — `BridgeError::GoMissing` fix surface at `bridge/src/lib.rs:41`; `materialise_extractor` writes `go 1.24` into the synthetic `go.mod` (`:176`).
- **Stdlib-only, no external resolution** — `go/parser`, `go/ast`, `go/token` (`extract.go:21-23`); the header at `:5-8` states *«`go run extract.go` must work with no module context, no go.mod, no network»*. This is the structural difference from TS: **Go's parser ships with the `go` binary itself**, so there is no equivalent of "the consumer project must resolve `typescript`" — only the `go` binary is required.

### Side-by-side — does a third sidecar have a ready-made shape?

**Yes. The protocol is shared and the bridge skeleton is copied, not re-derived.** Mapping the two bridges cell by cell (TS bridge `typescript-ai-native-extract-bridge/src/lib.rs` ↔ Go bridge `go-ai-native-extract-bridge/src/lib.rs`):

| Concern | TS bridge | Go bridge | Reusable by a 3rd sidecar? |
|---|---|---|---|
| Module doc | `:1-13` "spawn-and-parse bridge … One `<X>` per batch … `parse_ndjson` is pure" | `:1-13`, same text with `go`/`node` swapped | yes — text template |
| `pub const PROTOCOL: u64 = 1` | `:24` | `:27` | yes — verbatim |
| `BridgeError` taxonomy | **4** variants (`:29-62`): NodeMissing, TypescriptUnresolvable, ExtractorFailed, Protocol | **3** variants (`:37-61`): GoMissing, ExtractorFailed, Protocol | mostly — the `*Unresolvable` variant exists **only** when the runtime must resolve a separate compiler package; a stdlib-parser language (Go, and CPython's own `ast`) omits it |
| `RawFact` enum (tagged `fact`) | `:68-90` `TsUnsafe/Import/Item/FileMetrics` | `:67-96` `GoUnsafe/Import/Item/FileMetrics` | shape yes — swap the language variant; Go `Item` has an extra `underlying` field |
| `RawMarker` | `:93-100` (no revision) | `:101-109` (carries `r: Option<u32>`) | shape yes — extra fields are additive |
| `FileRecord` | `:103-111` | `:112-120` | yes — identical fields |
| `parse_ndjson` | `:115-132` | `:124-141` | yes — byte-identical logic |
| `EXTRACTOR_SOURCE = include_str!` | `:138` (→ `extract.ts`) | `:147` (→ `extract.go`) | yes — verbatim pattern |
| `materialise_extractor` | `:143-156` (writes `extract-<hash>.ts`) | `:158-179` (writes `extract-<hash>.go` **+ a `go.mod`** `:170-177`) | yes — Go adds a module-cut-off file; a Python sidecar adds nothing of the kind |
| `extract_tree` | `:160-185` (`Command::new("node")`) | `:223-248` (`Command::new(go_binary()).arg("run")`) | yes — swap the spawn command |
| overlay `extract_content` | absent | `:184-219` (`--stdin-file`) | Go-only (oracle relay) |
| `conform_facts` lowering | `:191-225` | `:254-289` | yes — swap the language variant, same field-mapping idiom |
| `warm`/`extract`/`probe` adapter | `frontend lib.rs:51-120` | `frontend lib.rs:54-117` | yes — structurally identical (`Mutex<HashMap>`, single test) |
| deps | `conform-core, serde, serde_json, thiserror` (`Cargo.toml:16-19`) | `conform-core, specmark, serde, serde_json, thiserror` (`Cargo.toml:16-20`) | yes — a generic spawn-and-parse bridge's full dep set |

**Divergences a third sidecar must decide on (named, not concluded):**
1. the spawn command — `node <script>` / `go run <script>` / `python3 <script>`;
2. the `BridgeError` taxonomy — whether a `*Unresolvable` variant is needed (TS yes: `typescript` resolved from the consumer; Go/CPython no: parser in the runtime);
3. the language-specific `RawFact` variant + the `RawMarker` extra fields (revision, underlying);
4. whether the overlay `--stdin-file` form is needed (oracle-relay concern);
5. materialisation extras (Go's `go.mod`; TS/Python none).

The shared spine — protocol versioning, NDJSON over stdio, `FileRecord` shape, `parse_ndjson`, content-addressed materialisation, `extract_tree`'s warm-batch flow, `conform_facts` lowering, the `Frontend` adapter with its `Mutex<HashMap>` cache and `probe` — is copied, not invented, between the two that exist.


---

## Q5 — which gate rules need which depth (rule → fact inventory; roster parity)

Two parts: the full rule roster with the fact fields each reads, and the
roster count (the B-035 parity gap, re-measured at this HEAD). Per the
packet, this section inventories **only** — it does not conclude whether
T-syn suffices.

### The full engine roster — 15 rule ids (re-counted at `779b3aaa`)

`grep -rnE 'fn id(&self)'` over `core-ai-native-conform/src/rules/*.rs`
returns **15** `Rule` impls (re-exports at `rules/mod.rs:21-25`). The
engine-conform spec annotation `##RULE-RECORD-DECLARES-ITS-TIER`
(`ENGINE-CONFORM-v0.1.md:24`) says *«All fifteen shipped rules»* — that
count is now accurate. **The F-146 prior measurement
(`d7a-core-sync-reverify.md:340`) said «thirteen ids»; that is stale — the
roster grew by two (the re-count finds `ambient-env` at `budget.rs:331` and
`pub-doctest` at `diagnostics.rs:143` that the thirteen-list omitted).**

| # | rule id | source `file:line` | family |
|---|---|---|---|
| 1 | `unsafe-gate` | `budget.rs:59` | budget (Rust) |
| 2 | `file-length` | `budget.rs:143` | budget (neutral) |
| 3 | `no-unwrap-in-domain` | `budget.rs:222` | budget (Rust) |
| 4 | `ambient-env` | `budget.rs:331` | budget (Rust) |
| 5 | `seam-has-doctest` | `diagnostics.rs:44` | diagnostics (Rust) |
| 6 | `pub-doctest` | `diagnostics.rs:143` | diagnostics (Rust) |
| 7 | `error-message-cites-req` | `diagnostics.rs:235` | diagnostics (Rust) |
| 8 | `error-enum-cites-req` | `diagnostics.rs:314` | diagnostics (Rust) |
| 9 | `R-001` (FlagSites) | `structure.rs:35` | structure (Rust) |
| 10 | `R-002` (CellIsolation) | `structure.rs:91` | structure (Rust) |
| 11 | `cell-has-oracle` | `structure.rs:175` | structure (Rust) |
| 12 | `go-unsafe-in-domain` | `go.rs:72` | go |
| 13 | `go-cell-isolation` | `go.rs:232` | go |
| 14 | `ts-unsafe-in-domain` | `typescript.rs:48` | typescript |
| 15 | `ts-cell-isolation` | `typescript.rs:189` | typescript |

**Roster parity (B-035, re-measured):** Rust-relevant rules = **11** (#1, #3–#11); Go = **2** (#12, #13); TS = **2** (#14, #15). The TS roster (2) is the same size as Go's (2) and roughly one-fifth of Rust's (11). The parity gap B-035 measured is present at this HEAD.

### Which rules a TS project actually runs

The TS gate driver assembles its rule set in **one** place — `typescript-ai-native-conform/src/lib.rs:48-61` `build_rules` — so the count is exact:

- `:50` `TsUnsafeInDomain` — always;
- `:51-56` `TsCellIsolation::new(cells_dir, seam)` — **only if `config.typescript.cells_dir` is set** (else the cells gate is off);
- `:57-59` `FileLength { max_lines }` — always.

**So a TS project runs 2 rules unconditionally (`ts-unsafe-in-domain`, `file-length`) plus `ts-cell-isolation` when cells are configured — at most 3.** `FileLength` is the one engine-neutral rule that runs for TS; it reads `Fact::FileMetrics { lines }` (`budget.rs:157`), which the TS frontend produces (`extract.ts:326`), and gates via `in_src`/`max_file_lines`. None of the other nine engine-neutral rules is in the TS set — each reads a Rust-only fact (`UnsafeUse`/`UnwrapUse`/`EnvRead`/`ErrorVariant`/`Ctor`) or Rust `#[cell]` attrs the TS frontend never emits (TS `Item.attrs` is always empty, `conform_facts` at `typescript-ai-native-extract-bridge/src/lib.rs:218`).

### Per-rule fact inventory — what each rule reads

| rule id | `file:line` of `check` | fact variant(s) read | fields read |
|---|---|---|---|
| `unsafe-gate` | `budget.rs:65-115` | `UnsafeUse` | `context, line, in_deviation` (+ `crate_name` from `SourceFacts`) |
| `file-length` | `budget.rs:150-183` | `FileMetrics` | `lines` (+ `file` via `in_src`) |
| `no-unwrap-in-domain` | `budget.rs:230-276` | `UnwrapUse` | `method, line, in_test, in_deviation` |
| `ambient-env` | `budget.rs:339-392` | `EnvRead` | `method, line, in_test, in_deviation` |
| `seam-has-doctest` | `diagnostics.rs:51-102` | `Item` | `kind, symbol, line, is_pub, has_doctest` |
| `pub-doctest` | `diagnostics.rs:150-199` | `Item` | `kind, symbol, line, attrs, is_pub, has_doctest` |
| `error-message-cites-req` | `diagnostics.rs:243-281` | `ErrorVariant` | `enum_symbol, variant, message, line` |
| `error-enum-cites-req` | `diagnostics.rs:322-365` | `ErrorVariant` | `enum_symbol, line, enum_attrs` |
| `R-001` | `structure.rs:41-74` | `Ctor` (+ `cell_types`→`Item`) | `type_name, line` (+ `symbol, attrs` for cell discovery) |
| `R-002` | `structure.rs:97-145` | `Import` (+ `cell_types`→`Item`) | `to_path, line` (+ `symbol, attrs, file, crate_name`) |
| `cell-has-oracle` | `structure.rs:183-234` | `Item`, `Import`, `Ctor` | `symbol, attrs, line` / `to_path` / `type_name` |
| `go-unsafe-in-domain` | `go.rs:79-163` | `GoUnsafe` | `kind, line, in_test, reason` |
| `go-cell-isolation` | `go.rs:238-273` | `Import` | `to_path, line` (+ `file`) |
| `ts-unsafe-in-domain` | `typescript.rs:54-101` | `TsUnsafe` | `kind, line, in_test, reason` |
| `ts-cell-isolation` | `typescript.rs:195-236` | `Import` | `to_path, line` (+ `file`) |

### T-syn vs T-sem — what the TS facts actually require

The packet asks, per rule, whether the facts it consumes require the type
checker (T-sem) or are derivable from a parse tree (T-syn). The three rules
a TS project runs read three fact variants — `TsUnsafe`, `Import`,
`FileMetrics`. What each is, by the extractor that produces it:

- **`Import { to_path }`** (`ts-cell-isolation`) — the import specifier
  *text*, taken straight from the AST's `moduleSpecifier` (`extract.ts:373-403`). Specifier text is a syntactic property; it does not require resolving the module or its types.
- **`FileMetrics { lines }`** (`file-length`) — a line count (`extract.ts:325-326`); lexical, no parse at all.
- **`TsUnsafe { kind, line, in_test, reason }`** (`ts-unsafe-in-domain`) — five kinds, each produced by a syntactic test in `extract.ts`:
  - `any_type` — the `any` keyword node (`extract.ts:342-349`, `node.kind === ts.SyntaxKind.AnyKeyword`);
  - `as_cross` — an `as` expression that is **not** `as const` (`extract.ts:350-364`); the test compares only the asserted type's *name* against `"const"` — it does **not** compare the source and target types (it cannot; it has no checker);
  - `non_null` — the `!` operator (`ts.isNonNullExpression`, `extract.ts:365-372`);
  - `ts_ignore` / `ts_expect_error` — comment-trivia matches (`extract.ts:438-483`, a `createScanner` walk over comments).

**What the extractor actually calls on the `typescript` module** (the `TsModule` interface at `extract.ts:170-210` and its uses): `createSourceFile`, `forEachChild`, `createScanner`, `getJSDocTags`, `getTextOfJSDocComment`, and the `is*` predicate predicates. **Absent from that interface and from every call site:** `createProgram`, `getPreEmitFileEmitOutput`, `getTypeChecker`, `getSymbolAtLocation`, and any checker/symbol-resolution surface. `grep -nE 'createProgram|getTypeChecker|getSymbolAtLocation|Checker|Program'` over `tools/ts-extract/extract.ts` returns **zero** matches. So the shipped TS frontend — labelled T-sem by the spec's `##ROW-FRONTEND-TS-JS` (`ENGINE-CONFORM-v0.1.md:55`, *«TypeScript compiler API»*) — exercises the Compiler API's **parser** half (`createSourceFile`/`createScanner`), not its type-checker half.

Consequence for the inventory (stated as fact, not as a sufficiency verdict): every fact field the three TS-gate rules read is produced by a parse-tree / lexical / trivia test in `extract.ts`; none of those productions invokes the type checker. The `as_cross` fact in particular is named "cross-type" but is produced by a purely syntactic `as`-that-isn't-`as-const` test (`extract.ts:350-364`) — the extractor does not, and cannot with the surface it uses, determine whether a cast actually crosses types.

(The nine engine-neutral rules that TS does *not* run read `UnsafeUse`/`UnwrapUse`/`EnvRead`/`ErrorVariant`/`Ctor`/`Item`-with-`cell(`-attrs. Whether *those* facts are T-syn-derivable is a Rust-side question and is not part of the TS inventory; the rust-syn frontend that produces them is itself an in-process parser visitor, Q2.)


---

## Q6 — the dependency surface for the four candidate technologies

Two halves: in-tree first (does any candidate already ride in a dependency
tree?), then the model-knowledge block on crate names and licence families
(each line marked «model knowledge — verify at build time»).

### In-tree — none of the four is present in any manifest

Perimeter searched: every `Cargo.toml`, `Cargo.lock`, and `package.json`
under the repo, excluding `target/`, `node_modules/`, `vibedeps/`,
`.vibe/cache/`, `legacy-spec/`. Eleven `Cargo.lock` files fall inside the
perimeter (host root + each package workspace + `research/rust-demo`):
`./Cargo.lock`, `packages/…/core-ai-native/v0.7.0/Cargo.lock`,
`…/core-ai-native/v0.8.0/Cargo.lock`, `…/go-ai-native-lang/v0.1.0/Cargo.lock`,
`…/go-ai-native-mcp/v0.1.0/Cargo.lock`, `…/rust-ai-native-lang/v0.7.0/Cargo.lock`,
`…/rust-ai-native-mcp/v0.7.0/Cargo.lock`, `…/typescript-ai-native-lang/v0.6.0/Cargo.lock`,
`…/typescript-ai-native-mcp/v0.6.0/Cargo.lock`, `…/fractality/fractality/v0.1.0/Cargo.lock`,
`research/rust-demo/Cargo.lock`.

Terms run: `tree[ -]?sitter`, `tree_sitter`, `\bswc\b`, `rustpython`, `\bruff\b`
(over `*.toml`/`*.lock`); and `name = "(tree-sitter|tree_sitter|swc|rustpython|ruff)"` over the lock files. **Result: zero matches** — in the perimeter manifests, and even in `vibedeps/` (the broader un-excluded search for `tree.sitter|rustpython|swc` over every `Cargo.lock` returns nothing). This confirms and extends the spec's own annotation `##ROW-TIER-T-SYN` (`ENGINE-CONFORM-v0.1.md:31`: *«`tree-sitter` / `tree_sitter` return no hit in any crate or manifest»*) to **all four** candidates.

**What IS present as a language-runtime dependency** — exactly one, and it is not one of the four:
- `tools/ts-extract/package.json:9-11` — `"devDependencies": { "typescript": "^6.0.0" }`. This is the TS T-sem sidecar's runtime prerequisite (Q3); it is **not** a Cargo dependency (the conform engine never links it) — `grep 'typescript =' --include='Cargo.toml'` over `packages/ crates/` returns nothing. It is resolved by the node sidecar from the consumer project at run time (`extract.ts:143-162`).

So: bringing any of the four candidates in is a **new dependency edge** for the engine — none rides in transitively today, and none has a vendored mirror under `vibedeps/`. (Absence perimeter: the 11 perimeter `Cargo.lock` files + every `Cargo.toml`/`package.json` under the repo with the standard exclusions; search terms as above.)

### Model knowledge — crate names and licence families

Marked per the packet. **No version numbers are asserted** (the packet forbids invented versions); licence **family** only, per the licensing flow's permissive-only rule. The spec's own license-posture column (`ENGINE-CONFORM-v0.1.md:51-57`) labels each row "clean" — these are the underlying facts.

**tree-sitter (+ TS/JS grammars)** — *the universal T-syn backend the spec names at `:31` (`tree-sitter (MIT) universal`) and `:55` (`tree-sitter / SWC (Apache-2.0)`):*
- «model knowledge — verify at build time» core Rust binding crate: `tree-sitter` (MIT) — the library is C with Rust bindings generated by `tree-sitter generate`; consumed in Rust via the `tree-sitter` crate.
- «model knowledge — verify at build time» TS grammar: `tree-sitter-typescript` (MIT) — parses `.ts`/`.tsx` (and `.cts`/`.mts`); exposes `language_typescript()` / `language_tsx()`.
- «model knowledge — verify at build time» JS grammar: `tree-sitter-javascript` (MIT) — plain `.js`/`.jsx`.
- «model knowledge — verify at build time» licence family: **MIT** for the core and both grammars. (Grammar licences vary across the tree-sitter-org catalogue; the official TS and JS grammars are MIT — verify at pin time.)

**SWC parser** — *the JS/TS alternative the spec names at `:55`:*
- «model knowledge — verify at build time» parser crate: `swc_ecma_parser` (the ECMAScript/TypeScript parser; TypeScript is a `Syntax::Typescript`/`TsConfig` mode of the same parser, not a separate crate). The umbrella facade is the `swc` crate; finer crates include `swc_common` (span/source-map), `swc_ecma_ast`.
- «model knowledge — verify at build time» licence family: **Apache-2.0** (the spec's `:55` label "SWC (Apache-2.0)" matches).

**RustPython parser** — *the in-process Python T-syn backend the spec names at `:57` (`RustPython parser (MIT) in-process`):*
- «model knowledge — verify at build time» parser crate: `rustpython-parser` (produces a Python AST); sibling crates `rustpython-ast`, `rustpython-compiler-core`. RustPython is itself a Python interpreter written in Rust; the parser crate is the in-process, no-sidecar half (the Q2 `syn`-visitor shape, but over Python).
- «model knowledge — verify at build time» licence family: **MIT** (the spec's `:57` label "RustPython parser (MIT)" matches).

**CPython `ast` / `symtable`** — *the Python T-sem route the spec names at `:57` (`CPython ast/symtable via sidecar`):*
- «model knowledge — verify at build time» runtime prerequisite: a **`python3` interpreter on PATH** — the route is a stdlib-only sidecar (`python3 extract.py`), the Python twin of `tools/ts-extract/extract.ts` / `tools/go-extract/extract.go`. The `ast` and `symtable` modules ship with the interpreter (no separate package to resolve) — structurally like the Go sidecar (parser in the runtime), not the TS one (compiler resolved from the consumer project).
- «model knowledge — verify at build time» licence family: **PSF-class** (Python Software Foundation License, a permissive BSD-style licence; the spec's `:57` label "PSF / MIT — clean" matches).

**Permissiveness, in summary (model knowledge — verify at build time):** all four candidates and their named sub-crates/grammars are permissively licensed (MIT / Apache-2.0 / PSF-class) — the licensing flow's permissive-only bar is met by each. The spec's license-posture column already records this posture for every frontend row.

---

## Method and freshness

**Re-measured at this HEAD (`779b3aaa`), not carried:**
- the `Frontend` trait signature, the `Fact` enum's ten variants and fields, the `Rule`/`Finding` types, the `Store` entry points and cache-key composition — all read directly from `core-ai-native-conform/src/{facts,store,finding}.rs`;
- the rust-syn frontend's extractions, size (405/268/152 LOC), and tests — read from `rust-ai-native-conform-frontend/src/lib.rs` + `src/lib/tests.rs` + `tests/engine.rs`;
- both sidecars' wire protocol, facts, sizes, tests, and runtime prerequisites — read from the three TS files and the three Go files;
- the rule roster re-count (**15** at this HEAD; F-146's "thirteen" is stale), the TS driver's 3-rule assembly, and the per-rule fact inventory — read from all five rule-family files + the TS gate driver;
- the in-tree dependency absence (zero hits for all four candidates across 11 perimeter `Cargo.lock` files + all `Cargo.toml`/`package.json`).

**Carried as pointers, re-verified:** the spec's `##ROW-*` / `##TIER-VOCABULARY` / `##SIDECAR-…` anchors (`ENGINE-CONFORM-v0.1.md`) were read in full and re-quoted; F-146 (`d7a-core-sync-reverify.md:922`) is cited only as the prior measurement whose "thirteen ids" tally is corrected here.

**Model-knowledge lines** (crate names, grammar coverage, licence families, runtime prerequisites not verifiable from the tree) are each marked «model knowledge — verify at build time»; no version numbers are asserted.

**No build, no test run, no `vibe`/`conform` command, no writes outside this one evidence file.** All commands run were read-only (`grep`, `find`, `wc -l`, `cat`, `sed -n` on source).
