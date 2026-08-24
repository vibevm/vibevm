# E11-R1-SEAM-ERRORS — census of the "seam error cites REQ" detection (B-033)

Read-only census for `BACKLOG.md {#b-033}` (Go seam-error REQ-citation: a
dedicated rule by the Rust paradigm, plus the message half) and its TS twin
(the guide promises an `E` union that cites `spec://` REQs; today there is
nothing). The point of the census is to measure, before any build, exactly
what the extractors already see and what inputs are missing for the two
twins. No design recommendations — measurements and missing inputs only.

All paths are relative to the worktree root, on the worktree's own
non-vendored copies under `vibevm/vibepacks/org.vibevm.ai-native/...` (the
`vibedeps/**` and `*-mcp/**` / `*-lang/**` vendor copies are regenerated
mirrors and are not cited). The Rust paradigm lives in the v0.8.0 core
crate (the line ranges the packet and the parity table cite match v0.8.0,
not v0.7.0). "Not found" is recorded as an explicit fact, never silent.

The headline up front: the Rust paradigm is **one fact shape, two rules,
two halves** — `Fact::ErrorVariant` carries both the Display `message` and
the owning-enum `enum_attrs`, and the two rules each examine a different
field. Go today has **only the structure half** (the error type has a field
literally named `Spec`), and it rides a kind inside the `go-unsafe-in-domain`
umbrella with no id of its own; the message half (does `Error()` render the
REQ) is not checked because the Go extractor never reads an `Error()` body.
TS has **neither half** — no error fact, no error rule — and the extractor
emits no fact that carries union shape, variant members, or a `spec://`
reference on an error type.

---

## Q1 — The Rust paradigm (two rules, two halves, one fact)

File: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/diagnostics.rs`.

Both halves consume the **same** fact variant,
`Fact::ErrorVariant { enum_symbol, variant, message, line, enum_attrs }`,
defined at `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/facts.rs:66-73`
(`enum_attrs` documented as "Attributes of the OWNING enum (where the REQ
edge lives)", facts.rs:71).

**Message half — `ErrorMessageCitesReq`** (diagnostics.rs:229-282):

- struct: `diagnostics.rs:229-231`; `id() = "error-message-cites-req"` (`:234-236`); `why()` (`:237-242`); `check()` (`:243-281`).
- matches `Fact::ErrorVariant { enum_symbol, variant, message, line, .. }` (`:250-259`).
- the gate: `if message.is_empty() || message.contains("spec://") { continue; }` (`:260`) — fires when the Display/`#[error("…")]` template text carries no literal `spec://` substring.
- finding emitted: `rule: "error-message-cites-req"`, message text "``{name}::{variant}`` display text cites no spec:// REQ" (`:264-276`).
- REQ-URI (the first arg to `req_message`): `discipline://rust-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops` (`:269`).
- fingerprint: `error-message-cites-req|{file}|{name}::{variant}` (`:275`).

**Attribute half — `ErrorEnumCitesReq`** (diagnostics.rs:308-366):

- struct: `:308-310`; `id() = "error-enum-cites-req"` (`:313-315`); `why()` (`:316-321`); `check()` (`:322-365`).
- matches `Fact::ErrorVariant { enum_symbol, line, enum_attrs, .. }` (`:330-338`).
- the gate: `if enum_attrs.iter().any(|a| a.starts_with("spec(")) { continue; }` (`:339`) — fires when the owning enum carries no `#[spec(...)]` attribute.
- dedup: one finding per enum, via `BTreeSet<String> flagged` (`:324`) and `if !flagged.insert(enum_symbol.clone()) { continue; }` (`:342-344`).
- finding emitted: `rule: "error-enum-cites-req"`, message text "thiserror enum ``{name}`` carries no #[spec] REQ edge" (`:346-360`).
- REQ-URI: `discipline://rust-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops` (`:351`) — the **same** URI as the message half (both halves cite the one Class-F card).
- fingerprint: `error-enum-cites-req|{file}|{enum_symbol}` (`:359`).

**The paradigm in one line.** One fact variant (`Fact::ErrorVariant`),
two rules, each gating on a *different field* of that variant for a
*different signal*: the message half scans the Display `message` for the
`spec://` substring (`:260`); the attribute half scans `enum_attrs` for a
`spec(`-prefixed attribute (`:339`). Both are mounted unconditionally in
the Rust driver with `gated_crates = config.rust.gated.clone()` — see Q6.
What the rules match on are serde facts (attribute text verbatim and the
Display template string), not the AST; the fact does the abstraction.

---

## Q2 — Go today: the `seam_error_missing_req` kind and what go-extract emits

### The rule side — a kind inside the umbrella, no id of its own

File: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/go.rs`.

- `GoUnsafeInDomain` struct (`:52-54`); constructor `new(cells_dir: Option<&str>)` (`:56-61`); `in_cells(file)` (`:63-67`).
- `Rule::id() = "go-unsafe-in-domain"` (`:70-73`). **Every finding this struct emits carries this umbrella id** as its `rule` field (`:153`), including the seam-error one — there is no dedicated id.
- `check()` (`:79-163`) matches `Fact::GoUnsafe { kind, line, in_test, reason }` (`:84-92`).
- deviation honour: `if reason.is_some() && kind != "reasonless_suppression" { continue; }` (`:96-98`) — a reasoned `//spec:deviates` covering the site is testimony and suppresses the seam-error finding too.
- cell-scoped filter (`:102-108`): only `init_decl | blank_import | ambient_call | naked_go` are cell-scoped; `seam_error_missing_req` is **not** in that set, so it fires everywhere (not only under `cells_dir`).
- the arm (`:145-149`):
  ```
  "seam_error_missing_req" if !in_test => (
      GO_GUIDE_ERRORS,
      "a seam error type without a Spec field cannot cite its REQ",
      "carry the violated spec:// URI (Code + Spec + Err) and render it",
  ),
  ```
- REQ-URI: `GO_GUIDE_ERRORS = "discipline://go-ai-native-lang/guide#errors"` (`:15`, used at `:146`).
- fingerprint: `{id}|{file}|{kind}#{line}` (`:158`).

So Go today detects **only the structure half** — "the error type has no
field named `Spec`". The fix text (`:148`) already names the message half
("and render it"), but no arm checks that `Error()` actually renders the
REQ. The finding rides `go-unsafe-in-domain`, with no rule id of its own.

### The extractor side — what go-extract sees and emits

File: `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/extract.go`.

- `seamErrorShape(s *ast.TypeSpec, errOwners map[string]bool)` (`:519-535`) is the producer:
  - fires only when `strings.HasSuffix(s.Name.Name, "Error") && errOwners[s.Name.Name]` (`:520`) — the type name ends in `Error` **and** it owns an `Error()` method;
  - requires the type to be a `*ast.StructType` with fields (`:523-525`);
  - loops the fields and `return`s (no finding) the moment it finds one named `Spec` (`:527-532`);
  - otherwise emits `ex.unsafeAt("seam_error_missing_req", ex.line(s.Pos()))` (`:534`).
- `errorMethodOwners()` (`:499-515`) builds the `errOwners` set — type names that carry an `Error() string` method. Comment (`:497-498`): "the seam-error shape's other half". **It reads only the method's receiver (which type owns it), not the body** (`:503-513`).
- `unsafeAt(kind, line)` (`:411-421`) builds `fact{Fact:"go_unsafe", Kind:kind, Line:line}` and attaches a deviation `Reason` if the line falls inside a `//spec:deviates` range (`:413-419`).
- The NDJSON `fact` struct (`:48-67`) carries exactly: `Fact, Kind, Symbol, IsExported, HasDocExample, Underlying, ToPath, Reason, Lines, Line`. **No field for a method body, a format string, or any string literal.**

### Does go-extract see `Error()` method bodies? — No (explicit fact)

- An `Error()` method is emitted as a bare `item` fact — kind `"method"` (because it has a receiver), `Symbol "Error"`, `Line`, `IsExported`, `HasDocExample = false` — by `funcItem` (`:423-440`, method branch at `:425-427`). The body is never read.
- `errorMethodOwners` (`:499-515`) reads only the receiver type (`:506-513`), never the body.
- The `fact` struct (`:48-67`) has no body / format-string / literal field.
- String-literal handling in the extractor is limited to: import paths (`importedPackages`, `:271`), `//spec:` directive URIs and `reason="…"` (`collectMarkers`, `:344-392`), and the `stringsOnError` detector (`:597-617`) — which matches `strings.Contains(err.Error(), …)` *call sites*, i.e. consumers of the error string, **not** the `Error()` definition's format string.

So the structure-half signal that reaches NDJSON is exactly one record,
`{"fact":"go_unsafe","kind":"seam_error_missing_req","line":<struct-decl-line>}`,
confirmed by the bridge's replay fixture
(`vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-extract-bridge/src/lib.rs:304`).
**Nothing in the stream says whether `Error()` renders `violates REQ` /
`spec://`** — that is the missing input for the message half.

---

## Q3 — Go fixtures: the clean and dirty seam-error idioms

Canonical fixtures under `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/`.

**CLEAN — `clean/internal/cells/greet/greet.go`** (both halves satisfied; no finding):

- `GreetError` struct, fields `Code int`, `Spec string`, `Err error` (`:16-20`); the `Spec` field is at `:18`.
- `Error()` renders `"greet: %d: violates REQ %s"` via `fmt.Sprintf(e.Code, e.Spec)` (`:22-24`) — **the message-half idiom** (the format string renders the REQ).
- `Unwrap()` (`:26`).
- (No `//spec:implements` on the error type itself; only `New` carries one at `:35`.)

**DIRTY — `dirty/internal/cells/plan/plan.go`** (structure-half violation; message half absent too):

- `PlanError` struct, fields `Code int`, `Err error` — **no `Spec` field** (`:17-20`); comment at `:16`: "lacks a Spec field on purpose (seam_error_missing_req)".
- `Error()` renders `"plan: %d"` — **no REQ** (`:22`).
- The struct declaration sits at `:17`, which is exactly the line the extractor flags (bridge replay `{"fact":"go_unsafe","kind":"seam_error_missing_req","line":17}`, go-bridge `lib.rs:304`).

**Live demo (not a fixture, the canonical seam error carrying both halves plus a marker)** — `research/go-demo/internal/seams/errors.go`:

- `PlanError` struct, fields `Code PlanErrorCode`, `Spec string`, `Err error` (`:27-31`).
- `Error()` renders `"plan: %s: violates REQ %s; fix surface: hand the planner non-nil states"` (`:35-40`).
- `//spec:implements spec://go-demo/PROP-001#req-errors r=1` on the type (`:26`).

So the dirty fixture is dirty on **both** halves (no `Spec` field **and**
`Error()` renders no REQ); the clean fixture satisfies both. The dirty
fixture's `Error()` body (`plan.go:22`) is precisely the input a future
message-half rule would need to flag — and it is the input the extractor
does not currently read (Q2).

---

## Q4 — TS today: what ts-extract emits, and the guide promise

### The extractor emits no error-union signal at all

File: `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/tools/ts-extract/extract.ts`.

- The fact vocabulary emitted (interfaces): `UnsafeFact` (`ts_unsafe`, kinds `any_type | as_cross | non_null | ts_ignore | ts_expect_error`) (`:56-66`); `ImportFact` (`:68-72`); `ItemFact` (`:74-81`); `MetricsFact` (`:83-86`); `EnvReadFact` (`ts_env_read`) (`:88-92`); union `ExtractFact` (`:94`). **No error / union / variant fact.**
- `declarationInfo` (`:286-321`) classifies declarations; a type alias becomes an `item` of kind `"type"` (`:303-305`) — but it captures only the name/symbol/export flag, **not** the alias's RHS, its union members, or any discriminant.
- The `visit()` walker (`:387-481`) checks: env reads, `AnyKeyword`, `AsExpression`, `NonNullExpression`, import/export declarations, dynamic-import `CallExpression`, and declarations (item + JSDoc markers). It does **not** walk a type alias's RHS for union members, does **not** detect discriminated unions, and does **not** scan arbitrary string literals for `spec://` — string literals are read only as import specifiers (`:430`, `:449`).
- `spec://` references in TS are carried by JSDoc tags (`@implements` / `@documents` / …) and become `markers` (`Marker` interface `:96-102`; `markerFromTag` `:261-278`), **not facts**.

### Engine side — no error fact, no error rule (explicit fact)

- `Fact` variants in `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/facts.rs`: the TS-relevant ones are `TsUnsafe` (`:113-118`) and `TsEnvRead` (`:146-150`). **There is no `Fact::ErrorVariant` twin for TS** (the Rust `ErrorVariant` at `:66-73` is fed only by the Rust frontend).
- The TS rules file `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/typescript.rs` defines `TsUnsafeInDomain` (`:46-104`), `TsCellIsolation` (`:131-239`), `TsFlagSites` (`:285-346`). **None consumes an error fact; none touches error unions.**

### The guide promise — verbatim search

The literal phrase **"the E union cites spec:// REQs" is not in the guide**
(it is the parity table's paraphrase — `campaigns/packages-2026-09/harvest/e10-b035-parity-pass.md` row 1, attributed to "census Q6"). The governing clauses in `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/vibevm/vibespecs/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.xml` are:

- `:152` `##FAILURE-IS-A-VALUE-ON-THE-CONTRACT-SURFACE` — "…a discriminated union `Result<T, E> = { ok: true; value: T } | { ok: false; error: E }` …, with `E` a discriminated union of named error variants carrying `spec://` REQ references."
- `:154` `##EXHAUSTIVENESS-OVER-E-IS-ENFORCED` (neighbour clause).
- `:157` `##TYPESCRIPT-PROJECTION-OF-THISERROR` — "the discriminated-union `E` is the thiserror enum, and the untyped `throw` is the panic."
- `:159` `##RULE-FAILURE-ON-A-SEAM-IS-A-TYPED-VALUE` — "failure on a seam is a typed value with REQ-citing variants; the exhaustive `switch` over the error union is checked at compile time (R-010, projected)."
- `:141` `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` — "every custom check emits `violates REQ <uri>: <why>; fix surface: <where>`, never bare free text (R3-011)."

### Minimal new facts a TS twin would need (measurement only)

The extractor today emits **no** fact that carries any of: (a) whether a
type alias is a union / its `E`-ness; (b) the union's variant members; (c)
any `spec://` REQ reference attached to the error type. JSDoc
`@implements`/`@documents` markers exist (`extract.ts:261-278`) but no rule
joins them to an error-union signal. None of the existing TS facts
(`ts_unsafe`, `import`, `item`, `file_metrics`, `ts_env_read`) carries any
of those three — all three are missing inputs.

---

## Q5 — Bridges: how NDJSON becomes engine facts (the `ts_env_read` map)

The `ts_env_read` record is the recent end-to-end addition (B-039) and is
the map for any new record kind — including a Go message-half signal.

### The full `ts_env_read` path (the template)

1. **extract.ts** — `EnvReadFact` interface (`:88-92`); detector `envReadSource()` (`:334-359`); emission in `visit()` (`:388-395`).
2. **bridge** — `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-extract-bridge/src/lib.rs`: `RawFact::TsEnvRead { source, line }` (`:94-97`); the `conform_facts` arm lowers it to `Fact::TsEnvRead { source, line, in_test }`, stamping `in_test` from the record (`:231-235`).
3. **engine** — `Fact::TsEnvRead` (`facts.rs:146-150`).
4. **rule** — `TsFlagSites` (`typescript.rs:285-346`), matching `Fact::TsEnvRead` (`:310-314`); mounted **conditionally** in the TS driver only when `[typescript] composition_root` is set (`typescript-ai-native-conform/src/lib.rs:62-64`).

### The Go equivalent path (where a new Go record kind / field inserts)

1. **extract.go** — emission via `unsafeAt(kind, line)` (`:411-421`), called by `seamErrorShape` (`:534`); the `fact` struct (`:48-67`) is where a new field or variant is added.
2. **bridge** — `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-extract-bridge/src/lib.rs`: `RawFact::GoUnsafe { kind, line, reason }` (`:70-75`); the `conform_facts` arm lowers it to `Fact::GoUnsafe { kind, line, in_test, reason }`, stamping `in_test` from the record (`:260-265`).
3. **engine** — `Fact::GoUnsafe` (`facts.rs:129-134`).
4. **rule** — `GoUnsafeInDomain` (`go.rs:52-164`), the `seam_error_missing_req` arm at `:145-149`; mounted in the Go driver (`go-ai-native-conform/src/lib.rs:53-55`).

So a Go message-half signal extends the same four-hop chain
(extract.go `fact` → go-bridge `RawFact` → `Fact::GoUnsafe` (or a new
variant) → a rule arm/id → driver mount) — the `ts_env_read` shape exactly.
The bridges are protocol-versioned (TS `PROTOCOL = 1`,
`typescript-ai-native-extract-bridge/src/lib.rs:24`; Go `PROTOCOL = 1`,
`go-ai-native-extract-bridge/src/lib.rs:27`); each extractor stamps
`protocol` on every record (`extract.ts:33`, `extract.go:33`), and a
record-shape bump surfaces as `BridgeError::Protocol` (TS
`typescript-ai-native-extract-bridge/src/lib.rs:55-61`; Go
`go-ai-native-extract-bridge/src/lib.rs:54-60`), retiring conform cache
slots via the frontend version.

---

## Q6 — Roster form: id conventions, rule files, driver mounting

### id conventions (from the three `build_rules` rosters)

- **Rust** rules carry **no language prefix** — `error-enum-cites-req`, `error-message-cites-req`, `seam-has-doctest`, `pub-doctest`, `flag-sites`, `cell-isolation`, `unsafe-gate`, `cell-has-oracle`, `no-unwrap-in-domain`, `ambient-env`, `file-length` (`vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:53-93`).
- **Go** rules carry the **`go-`** prefix — `go-unsafe-in-domain`, `go-cell-isolation`, `file-length` (`vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-conform/src/lib.rs:51-63`).
- **TypeScript** rules carry the **`ts-`** prefix — `ts-unsafe-in-domain`, `ts-cell-isolation`, `ts-flag-sites`, `file-length` (`vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-conform/src/lib.rs:48-69`).

### Rule home files (defined ONCE in the neutral core)

- Go rules: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/go.rs`.
- TS rules: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/typescript.rs`.
- Rust diagnostics rules: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/diagnostics.rs`.
- The neutrality argument (rules defined once so they cannot drift between language projections) is stated at `go.rs:1-7` and `typescript.rs:1-7`.

### Mounting exemplars

- **Rust** mounts **both** error rules unconditionally with `gated_crates = config.rust.gated.clone()`: `ErrorEnumCitesReq` (`rust-ai-native-conform/src/lib.rs:74-76`), `ErrorMessageCitesReq` (`:78-80`).
- **Go** mounts `GoUnsafeInDomain::new(config.go.cells_dir.as_deref())` (`go-ai-native-conform/src/lib.rs:53-55`); `GoCellIsolation` is conditional on `cells_dir` (`:56-58`); `FileLength` always (`:59-61`). A Go seam-error rule with its own id would add a `Box::new(rules::Go…)` line in this same `build_rules` (`:51-63`), parallel to `GoUnsafeInDomain`.
- **TS** mounts `TsUnsafeInDomain` unconditionally (`typescript-ai-native-conform/src/lib.rs:50`); `TsCellIsolation` conditional on `cells_dir` (`:51-56`); `TsFlagSites` conditional on `composition_root` (`:62-64`); `FileLength` always (`:65-67`). A TS twin would add a line in this `build_rules` (`:48-69`), parallel to `TsFlagSites`.

An always-on new rule takes its config from the existing `[go]` / `[typescript]`
tables (`GoConfig` / `TsConfig`) with no new field; a conditional mount (the
`TsFlagSites` shape) would need a new optional field on `TsConfig` / `GoConfig`
(the `composition_root` field that gates `TsFlagSites` lives on `TsConfig`;
`config.typescript.composition_root` is read at
`typescript-ai-native-conform/src/lib.rs:62`).
