# E-V3-FORMAT-CENSUS — census of the specmap map, the package manifest, and the schema-bump route

Read-only census of the facts the one-time `specmap.json` format change must stand
on — taken on branch `wt/E-V3-FORMAT-CENSUS` (HEAD `7708bdee`). Every factual claim
carries a `path:line`, relative to the worktree root. "Не существует" is recorded
explicitly as a fact about the perimeter, never silently omitted. This is an
evidence file for the three-dodges-at-once design — **(а)** a code-item hash +
end-of-range, **(б)** the map riding inside an installed package, **(в)** privacy
profiles — not a spec: nothing here is proposed, only measured.

The canonical specmap engine is `core-ai-native-specmap` v0.8.0 at
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/`.
Unless a copy is named otherwise, every `index.rs` / `mdspec.rs` / `scanner.rs` /
`rscan.rs` / `ledger.rs` line citation below refers to that canonical v0.8.0 tree.
The `v0.7.0` slot and every `crates/vendor/core-ai-native-specmap/` tree are
byte-identical mirrors produced by `cargo xtask sync-engines` (self-check.sh:332);
they are cited separately only where their existence matters, and are not
re-counted as distinct mechanisms.

Perimeter: the whole tree **except** `vibedeps/`, `.vibe/`, `target/`, `.wt/`
(regenerated copies). No fact below was found only inside those directories.

---

## Headline — what the design must know before its first line

1. **The package manifest parser is strict to unknown keys.** `Manifest`
   (`crates/vibe-core/src/manifest/document.rs:67`) carries
   `#[serde(deny_unknown_fields)]` (`document.rs:66`), and a test pins it
   (`rejects_unknown_top_level_section`, `document/tests.rs:343`). Today's `vibe`
   **fails** on a `vibe.toml` that adds a new key (e.g. `[metamodel]`). So any
   map/profile data that rides *in the manifest* is a manifest-schema bump, not
   an additive key — and `vibe.toml` carries **no** map-related key today (§3.2.4).
2. **A generated map dropped into a package feeds its identity hash.**
   `compute_content_hash` (`crates/vibe-index/src/content_hash.rs:40`) hashes the
   **entire** shippable file tree — every regular file except five build-output
   names (`content_hash.rs:28`) — and the manifest declares **no** file list
   (§3.3.1). A `specmap.json` is not excludable by that mechanism, so placing one
   in a package re-pins the lockfile whenever its bytes change (i.e. on essentially
   every scan-affecting edit). Precedent exists: the fractality package already
   ships a `specmap.json` inside its tree (§3.3.3).
3. **The schema-bump route is alive and *partly* gated.** `SCHEMA = 2`
   (`index.rs:29`), source-of-truth JTD at `schemas/specmap.jtd.json` →
   `jtd-codegen` → `generated/specmap/mod.rs`; `cargo xtask check-codegen`
   (self-check.sh:342) gates the **generated wire types**. But the committed
   **host `specmap.json` artefact** is *not* panel-gated — there is no
   `cargo xtask specmap --check` step in self-check.sh (§3.1, Discrepancy D1).
4. **The "explain a foreign installed package" seam does not exist today.** Nothing
   reads a prebuilt map for a foreign package: `scan_external_units` re-parses
   foreign markdown for *resolution only* (§3.4); `trace explain` builds a fresh
   in-memory map over the project's *own* tree (§3.4). Dodge (б) builds a seam that
   is absent today.
5. **A package's spec namespace is consumer-assigned, not self-declared.** The
   `<namespace>` segment of `spec://<namespace>/…` comes from the *consumer's*
   `specmap.toml` / `[[external_specs]]`, never from the package itself (§3.5). A
   self-shipped map's URI namespace is therefore **not** unambiguous today; the
   rule must be added.
6. **`CodeItem` has neither a hash nor an end-of-range today**
   (`generated/specmap/mod.rs:36`). Dodge (а) is two net-new fields → a real schema
   bump, not a tweak.

---

## §3.1 — Who produces, gates, and reads the map

### §3.1.1 Production points (build / write `specmap.json`)

| who | site | what |
|---|---|---|
| host xtask | `xtask/src/specmap.rs:11` (`run_specmap`) → `rust_ai_native_specmap::run_specmap(&repo_root()?, check)`; dispatch `xtask/src/main.rs:325` (`Cmd::Specmap { check } => run_specmap(check)`) | host root `specmap.json` over `crates/*` + `xtask` under namespace `org.vibevm.core/vibevm` (`specmap.toml`) |
| core write | `index.rs:297` `write` / `index.rs:302` `write_with_scanner`; the physical `std::fs::write` at `index.rs:316` | the actual byte emitter (canonical pretty JSON + trailing newline, `index.rs:178`) |
| Rust stack binary | `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-specmap/src/main.rs` (`--check` / `--gate`) | standalone `rust-ai-native-specmap` over Rust trees |
| TS stack binary | `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-specmap/src/main.rs` | standalone `typescript-ai-native-specmap` (injects the TS scanner via the D3 seam) |
| Go stack driver | `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-specmap/src/lib.rs` | `run_specmap_go` (injects the Go scanner) |
| MCP servers | `specmap_write` tool in `rust-ai-native-mcp/…/tools_discipline.rs:199`, `typescript-ai-native-mcp/…/tools_discipline.rs:209`, `go-ai-native-mcp/…/tools_discipline.rs:200` | agent-facing regenerate-and-write, thin wrappers over the same library fns |

The D3 seam that lets a non-Rust scanner build the index is
`index::build_with_scanner` (`index.rs:64`); the TS/Go drivers inject
`RecordsScanner` through it (`typescript-ai-native-cli/src/trace.rs:18`,
`go-ai-native-cli/src/trace.rs:19`).

### §3.1.2 Gate points (byte-compare / freshness)

| gate | site | exact string / note |
|---|---|---|
| core `--check` | `index.rs:324` `check` / `index.rs:329` `check_with_scanner`; byte-compare at `index.rs:344` (`fresh == committed`) | regenerate-in-memory vs committed `specmap.json`; drift report on mismatch |
| core `--gate` | the binary `--gate` flag → `run_gate` (orphan-ratchet / coverage only) | **not** a full index byte-compare (self-check.sh:405-408: "Orphan-coverage gate only (`--gate`) … coverage is what matters") |
| codegen drift gate | `xtask/src/codegen.rs:237` `run_check_codegen`; self-check.sh:342 | `cargo xtask check-codegen` — gates the **generated wire types** (vibe-wire + specmap `generated/`), not the committed artefact |
| MCP `specmap_check` | `rust-ai-native-mcp/…/tools_discipline.rs:192` (+ TS/Go analogues) | byte-compare via the library `--check` |
| fixture gate tests | `typescript-ai-native-specmap/tests/gate.rs:16` (`clean_fixture_check_is_byte_stable_and_gate_green`), `:24` | `--check` over committed fixture golden files |

**The host root `specmap.json` freshness is NOT a panel gate.** `grep "xtask specmap" tools/`
returns nothing; self-check.sh runs `cargo xtask conform check` (:325),
`sync-engines --check` (:332), `check-codegen` (:342), and the package `--gate`
self-traces (:415-442) — but no `cargo xtask specmap --check` over the host root.
No CI workflow files exist in the perimeter (`.github/workflows/` etc. absent), so
self-check.sh is the gate panel. → Discrepancy D1.

### §3.1.3 Reader points (read a finished `specmap.json`)

| reader | site | what it does |
|---|---|---|
| `load_committed` | `index.rs:289` | reads the committed artefact to classify drift during `write` (`index.rs:309`) |
| explain | `explain.rs:89` `explain_unit`, `:199` `explain_text`, `:209` `explain_json` | renders the traceability subgraph around a `spec://` URI — operates on an **in-memory** `Specmap`, not disk |
| intent ledger | `ledger.rs:217` `prose_explain` → `explain::explain_json` | prose render, epoch-keyed cache under `.ledger/` (the map is passed in, not read from disk) |
| `trace explain` CLI | `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/trace.rs:8` | **builds a fresh in-memory map** (`index::build`), does **not** read the committed `specmap.json` |
| MCP `trace_explain` | `rust-ai-native-mcp/…/tools_discipline.rs:207` (+ TS/Go) | same fresh-build explain path |

No reader consumes a **foreign** package's committed map — see §3.4.

### §3.1.4 Physical `specmap.json` files in the perimeter (13)

| path | owner |
|---|---|
| `specmap.json` | host root traceability index (namespace `org.vibevm.core/vibevm`) |
| `packages/org.vibevm.fractality/fractality/v0.1.0/specmap.json` | the fractality package's own self-trace (schema 2; real `code_items`) |
| `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/{clean,dirty}/specmap.json` | go-extract test golden files (2) |
| `packages/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/tools/go-extract/test/fixtures/{clean,dirty}/specmap.json` | go-extract test golden files (2) |
| `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/tools/ts-extract/test/fixtures/{clean,dirty}/specmap.json` | ts-extract test golden files (2) |
| `packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/tools/ts-extract/test/fixtures/{clean,dirty}/specmap.json` | ts-extract test golden files (2) |
| `research/{rust,ts,go}-demo/specmap.json` | research demo artefacts (3) |

---

## §3.2 — The package manifest (decisive for privacy profiles (в))

### §3.2.1 The parser

The manifest is `vibe.toml`; its parser is `crates/vibe-core/src/manifest/document.rs`:
`pub struct Manifest` (`document.rs:67`), `FILENAME = "vibe.toml"` (`document.rs:286`),
loaded by `Manifest::read` (`document.rs:289`) / `Manifest::parse_str`
(`document.rs:298`) (both go through `read_toml` then `validate`). The
`[compatibility]`, `[requires]`, etc. sub-shapes live in `manifest/package.rs` and
`manifest/project.rs`. (Not `vibe-workspace` / `vibe-registry`; those consume the
parsed `Manifest`.)

### §3.2.2 Is it strict to unknown keys? — **YES**

`#[serde(deny_unknown_fields)]` is on the top-level `Manifest` (`document.rs:66`)
**and on every section** (`WorkspaceSection` :202, `OriginSection` :238,
`BootSection` :269, `PackageMeta` `package.rs:76`, `Compatibility` `package.rs:331`,
`ProjectSection` `project.rs:38`, etc.). A test pins the top-level behaviour:
`rejects_unknown_top_level_section` (`document/tests.rs:343`) parses a manifest with
a `[mystery]` section and asserts `is_err()`. Companion tests:
`boot_section_rejects_unknown_field` (`document/tests.rs:447`),
`rejects_unknown_package_field` (`manifest/lockfile/tests.rs:272`).

**Consequence for the design:** today's `vibe` **fails** on a `vibe.toml` carrying
a new key. Map/profile data in the manifest ⇒ manifest-schema bump (and an old
`vibe` rejects the new manifest — see §3.2.3 for the version lever).

### §3.2.3 Minimum-version mechanic — **EXISTS**

Field: `Compatibility.min_vibe_version: Option<String>`
(`crates/vibe-core/src/manifest/package.rs:334`; doctest `package.rs:323`
`min_vibe_version = "0.2"`; struct attr `package.rs:331`). It is propagated to the
registry index entry (`crates/vibe-index/src/scanner/manifest.rs:78`
`min_vibe_version: c.min_vibe_version.clone()`, surface type
`crates/vibe-index/src/types/entry/relations.rs:17`) and to the publish post-hook
(`crates/vibe-publish/src/post_hook.rs:253`). The lockfile carries its own,
separate `schema_version = 5` (`vibe.lock` `[meta]`). So a "this package requires
vibe ≥ X" lever exists and is already wired through publish + registry.

### §3.2.4 Map-related key in the manifest today — **does not exist**

The 26 `Manifest` fields (`document.rs:67-184`) are: `project, package, workspace,
origin, requires, requires_any, provides, obsoletes, conflicts, recommends,
suggests, skill, binary, mcp_server, hooks, compatibility, boot_snippet, features,
target, active, llm, registry, mirror, override, i18n, boot`. **None** is
map/specmap/metamodel/privacy-related. N=0.

---

## §3.3 — What physically rides in a package (decisive for (б))

### §3.3.1 How package content is gathered — **whole directory, no file list**

The manifest declares **no** file list / include / writes field (none among the 26
`Manifest` fields, §3.2.4). The shippable set is the whole package directory minus
five build-output names. `compute_content_hash`
(`crates/vibe-index/src/content_hash.rs:40`) walks `pkg_dir` recursively
(`WalkDir`, `content_hash.rs:41-47`), keeps regular files, prunes only
`SHIPPABLE_EXCLUDES = [".git", ".vibe", "target", "node_modules", ".vibeignore"]`
(`content_hash.rs:28`; the prune is `is_shippable` `content_hash.rs:32`). This is a
byte-for-byte port of `vibe-registry`'s hasher (module doc `content_hash.rs:1-6`,
parity test `crates/vibe-index/tests/content_hash_parity.rs`), so the publish path
hashes the same set. The lockfile `files_written` field is the install *output*
side (what the package projects into the consumer tree), not a content declaration.

### §3.3.2 What the content hash covers — and the design answer

The hash covers **all** shippable files: for each, `hash(rel_path || 0x00 ||
file_bytes || 0x00)`, sorted lexicographically, folded into a final SHA-256
(`content_hash.rs:50-69`). It is **not** a selection; there is no allowlist.

**Design question — "if a generated map file is placed in a package, does every
code edit re-pin its lockfile entry?": YES (and more).**

- Any byte change to *any* shippable file re-pins — code edits already do so today
  (the code file is hashed). Adding a generated `specmap.json` makes the pin
  *additionally* a function of the map's bytes.
- The map is derived from code + spec, so a regeneration that changes map bytes
  (i.e. essentially every scan-affecting edit — new symbol/edge/unit) flips the
  pin, on top of the code-edit churn.
- The only exclusion lever is the five hardcoded names; a file named `specmap.json`
  is **not** excludable. `.vibeignore` is an *excluded entry name*, not a parsed
  ignore-list (`is_shippable` matches file names, `content_hash.rs:32-37`) — so
  there is no per-file opt-out short of renaming the map to an excluded name or
  parking it under `target/` (which is not shipped at all).

Net: a map-in-package makes the package identity pin track a derived artefact.
Depending on policy this is either acceptable (pin churns honestly with the code)
or must be escaped by excluding the map from the hash — which today has no clean
mechanism.

### §3.3.3 Precedent — a generated file already riding inside a package

**Yes — two:**

1. `packages/org.vibevm.fractality/fractality/v0.1.0/specmap.json` — a generated
   specmap (schema 2, real `code_items` over `fractality_backend_claude_code`)
   committed *inside* a published package's tree. It sits at the package root, so
   it is in the shippable set and feeds `compute_content_hash`. This is a direct,
   concrete instance of "a map riding in a package."
2. The `jtd-codegen` output `generated/specmap/mod.rs`
   (`…/core-ai-native-specmap/src/generated/specmap/mod.rs`) — a generated,
   committed file that ships inside the `rust-ai-native` stack package. Same
   policy: generated, committed, hashed.

---

## §3.4 — Consumer of installed-package spec text today (the seam for (б))

Two distinct consumers read the markdown of **installed** packages (under
`vibedeps/`); **neither** reads a prebuilt map.

1. **specmap `scan_external_units`** —
   `mdspec.rs:527`. Walks each `Config::external_specs` tree
   (the consumer's `[[external_specs]]`, an installed package's spec dir),
   re-parses every `.md` via `parse_units` (`mdspec.rs:555`), and mints spec
   **units** (anchor, kind, revision, doc_path, contentHash). These units
   participate in **resolution only** — dangling-edge suppression + suspect
   detection (`index.rs:71-91`) — and are **never serialised** into the project's
   own index (`mdspec.rs:520-526` comment; asserted by the test
   `external_specs_resolve_edges_without_entering_the_index`, `index.rs:537`).
   Extracts: anchors, kind/revision lines, doc paths.
2. **`vibe-spec` file resolver + directive loader** —
   `crates/vibe-spec/src/resolver.rs:57` `FileResolver`, `resolve_file`
   (`resolver.rs:110`), `spec_root` (`resolver.rs:122`); directives at
   `crates/vibe-spec/src/directives.rs:284`. Resolves `spec://<group>/<name>/…`
   document addresses (and INCLUDE directives) to files under `vibedeps/` slots
   during **boot** (PROP-009 / PROP-035). It reads markdown to follow INCLUDEs and
   invert the lossy `PROP-NNN` truncation (`resolver.rs:5-9`), **not** to build a
   traceability map.

The `trace explain` path (`rust-ai-native-cli/src/trace.rs:8`) builds a **fresh
in-memory** map of the project's *own* tree (`index::build`) — it neither reads the
committed artefact nor touches foreign packages. `progress-core`
(`crates/progress-core/src/scope.rs:13-22`) **excludes** `vibedeps` outright.

**Headline for (б):** there is **no** consumer today that reads a prebuilt map for a
foreign installed package. Every question about a foreign package's specs is
answered either by re-parsing its markdown (resolution only) or not at all. The
"consumer asks about an installed package, rebuilding nothing" seam is greenfield.

---

## §3.5 — Addressing (will URIs collide?)

### §3.5.1 How the unit URI and `doc_path` are computed

- URI: `format!("spec://{namespace}/{doc_path}#{anchor}")` — for headings
  (`mdspec.rs:445`) and for facts (`mdspec.rs:282`).
- `doc_path` = `canonical_doc_path(file)` (`mdspec.rs:305`): relative to `spec/`
  (`mdspec.rs:306`), `.md` stripped (`:311`), and a filename carrying a document id
  truncated to it (`PROP-003-dep-evolution.xml` → `PROP-003`, `:312-321`); files
  without an id keep their full stem.
- `parse_units` (`mdspec.rs:336`) derives `doc_path` once and threads `namespace`
  through.

### §3.5.2 What is the package identity in the URI, and where does it come from?

The `<namespace>` segment. For the project's own tree it is `Config::namespace`
(the project's `specmap.toml` `namespace =`, used at `mdspec.rs:485` and `:505`).
For an installed package's tree it is `ExternalSpec.namespace`
(`config.rs:99-105`), used at `mdspec.rs:555`. It is **not** derived from the
package's `(group, name)` identity; it is a short string hand-assigned by the
**consumer** in its `specmap.toml` `[[external_specs]]`.

### §3.5.3 Design question — "if every package ships its own map, whose namespace are its URIs in?"

**Not unambiguous today; the rule must be added.** A package's spec namespace is
presently *consumer-assigned* (`ExternalSpec.namespace`, `config.rs:100`) — the
package itself declares no spec namespace (the manifest has no such field, §3.2.4,
and identity is `(group, name, version)` only). A self-shipped map's URIs need an
authoritative namespace that the package must self-declare; nothing in the package
or the engine today provides it. Evidence: `mdspec.rs:555` mints under
`ext.namespace` (consumer side); `ExternalSpec` (`config.rs:98-105`) is the only
source; no self-declared namespace exists in `Manifest` (`document.rs:67-184`).

---

## §3.6 — The schema-bump route (recently repaired — is it alive?)

### §3.6.1 Current schema number

`pub const SCHEMA: u32 = 2;` (`index.rs:29`); its single live use is
`schema: SCHEMA,` (`index.rs:168`).

### §3.6.2 Files a 2 → 3 bump must touch

1. JTD source of truth: `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/specmap.jtd.json`.
2. Generated wire types (regenerated, not hand-edited): `…/crates/core-ai-native-specmap/src/generated/specmap/mod.rs` + `…/src/generated/mod.rs` (`// Generated by cargo xtask codegen. DO NOT EDIT.`, `generated/mod.rs:1`).
3. The `SCHEMA` const + its doc comment: `index.rs:27-29` (and the use at `:168`).
4. Canonical example: `…/schemas/specmap.example.json` (`"schema": 2,` `:38`).
5. Engine consumers of the schema literal in tests: `ledger.rs:340`, `explain.rs:306` (see §3.6.4).
6. Every committed `specmap.json` artefact carries `"schema": 2` (the 13 files in §3.1.4) — they are regenerated, not hand-edited, but each must be re-emitted.
7. Vendored mirrors regenerated by `sync-engines` (`v0.7.0`, `crates/vendor/core-ai-native-specmap/`) — regenerated from the authored source, not edited in place.

### §3.6.3 The codegen / schema-check command

- Regenerate: `cargo xtask codegen` — `run_codegen` (`xtask/src/codegen.rs:98`); outputs to `crates/vibe-wire/src/generated` and `…/core-ai-native-specmap/src/generated` (`xtask/src/codegen.rs:243-246`); dispatch `xtask/src/main.rs:323` `Cmd::Codegen => run_codegen()`.
- Check: `cargo xtask check-codegen` — `run_check_codegen` (`xtask/src/codegen.rs:237`); dispatch `xtask/src/main.rs:324` `Cmd::CheckCodegen => run_check_codegen()`; **exact gate string in the panel: `run_step "cargo xtask check-codegen" cargo xtask check-codegen` (self-check.sh:342)**. This is the gate a schema-bump regen must satisfy.

### §3.6.4 Second hardcoded occurrences of the schema number

The literal `2` (or `"schema": 2`) appears, beyond the live const/use:

- test fixtures in the engine: `ledger.rs:340` (`mini_map`, `schema: 2`),
  `explain.rs:306` (`fixture`, `schema: 2`);
- canonical example: `specmap.example.json:38`;
- research demos: `research/rust-demo/specmap.json:186`, `research/ts-demo/specmap.json:128`, `research/go-demo/specmap.json:262`;
- ts-extract fixture: `typescript-ai-native-mcp/…/ts-extract/test/fixtures/clean/specmap.json:51` (and its dirty twin);
- every one of the 13 committed `specmap.json` files (§3.1.4) carries `"schema": 2`;
- vendored `v0.7.0` mirrors carry the same.

**Literal collision (not a specmap schema site):** `crates/progress-core/src/cache/tests.rs:89` and `crates/vibe-cli/src/commands/progress_evidence.rs:119` carry `"schema": 2` but in **progress-evidence** JSON — a different schema. Flagged so a blind `grep` does not mistake them for specmap sites.

---

## §3.7 — The code element today

### §3.7.1 `CodeItem` — exact shape

`pub struct CodeItem` (`generated/specmap/mod.rs:36`):

| field | type | serde rename | line |
|---|---|---|---|
| `crateName` | `String` | `crate_name` | `mod.rs:37-38` |
| `file` | `String` | `file` | `mod.rs:41-42` |
| `itemKind` | `String` | `item_kind` | `mod.rs:44-45` |
| `line` | `u32` | `line` | `mod.rs:47-48` |
| `symbol` | `String` | `symbol` | `mod.rs:52-53` |

Comment (`mod.rs:33-34`): "A tagged code item (fn, struct, enum, trait, impl, mod). Only items carrying at least one edge appear."

### §3.7.2 End-of-range on a code item? — **NO**

`CodeItem` has only `line` (the start). There is **no** end / length / span / hash
field. The index dedups on `(symbol, file, line)` (`index.rs:150`), confirming
identity is start-line only. (Contrast: `SpecUnit` *does* carry `contentHash`,
`generated/specmap/mod.rs:158-159` — but only over the markdown unit span, not
code.) Dodge (а) adds two fields that do not exist today.

### §3.7.3 Counts in the root host `specmap.json` (counted, not estimated)

```
jq '{schema, codeItems:(.code_items|length), edges:(.edges|length),
     specUnits:(.spec_units|length), suspects:(.suspects|length),
     warnings:(.warnings|length)}' specmap.json
→ { "schema": 2, "codeItems": 903, "edges": 920, "specUnits": 6025,
   "suspects": 0, "warnings": 211 }
```

**`codeItems` = 903, `edges` = 920, `specUnits` = 6025.**

### §3.7.4 `CodeScanner` implementations in the tree

Trait: `pub trait CodeScanner { fn id(); fn scan(root, cfg) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>); }` (`scanner.rs:23-27`).

| impl | language | site (authored) | parser | parses |
|---|---|---|---|---|
| `RustScanner` | Rust | `scanner.rs:33-42` (→ `rscan::scan_workspace`) | `syn` — `syn::parse_file` at `rscan.rs:243` (`use syn` `rscan.rs:15`) | specmark `#[spec]`/`#[verifies]`/`scope!` tags |
| `CompositeScanner<'a>` | (compose) | `scanner.rs:45-71` | delegates to `Vec<&dyn CodeScanner>` | mixed-tree concatenation; sort/dedup downstream (`index.rs:148-159`) |
| `RecordsScanner<'_>` | TypeScript | `typescript-ai-native-specmap-scan/src/lib.rs:180` | TS compiler API via the `ts-extract` Node bridge (`typescript-ai-native-extract-bridge`) | JSDoc markers (`@implements`/`@verifies`/…) |
| `RecordsScanner<'_>` | Go | `go-ai-native-specmap-scan/src/lib.rs:159` | `go/ast` via the `go-extract` Go bridge (`go-ai-native-extract-bridge`) | `//spec:` directives |

The TS/Go scanners are also vendored into their `-mcp` packages
(`typescript-ai-native-mcp/…/typescript-ai-native-specmap-scan/src/lib.rs:180`,
`go-ai-native-mcp/…/go-ai-native-specmap-scan/src/lib.rs:159`). Every other
`impl CodeScanner` hit in the tree (`v0.7.0`, `crates/vendor/…`) is a
`sync-engines` byte-mirror of the four above — not a distinct implementation.

---

## Discrepancies (documentation vs. code)

**D1 — the host `specmap.json` artefact is not panel-gated, despite the docs saying it is.**
PROP-014 (the index module doc, `index.rs:1-4`; the JTD schema description,
`schemas/specmap.jtd.json:3`; the decision `##INDEX-IS-CANONICAL-JSON-WITH-A-SCHEMA`,
`PROP-014-…md:209`) describes the index as "regenerated by `cargo xtask specmap` and
gated by `cargo xtask specmap --check` in CI." Measured reality: the `--check` path
exists in code (`index.rs:324`) but is **not** wired into the panel — self-check.sh
runs `check-codegen` (which gates the *generated wire types*, a different object)
and the package `--gate` self-traces (orphan-coverage only, self-check.sh:405-408),
with **no** `cargo xtask specmap --check` over the host root (`grep "xtask specmap"
tools/` → empty; no CI workflows in the perimeter). The committed host index can
therefore drift past the panel undetected. The design should not assume a live host
freshness gate.

**D2 — "identity is the source, never artifacts" is in tension with shipping a derived map.**
PROP-024 §2.2 (per `content_hash.rs:23-27`) excludes build output so "identity is
the source." A shipped `specmap.json` is a *derived* artefact (a function of code +
spec), not source. The hash mechanism does not distinguish the two — both feed it
(§3.3.2) — so the design must decide explicitly whether a derived map is "source"
for identity purposes. This is a policy tension to resolve, not a code bug.

(Cross-check: `scan_external_units`'s "resolution only, never serialised" doc
(`mdspec.rs:520-526`) **matches** the code (`index.rs:71-91`) — no discrepancy
there.)

---

## Caveats — what this census does NOT measure

- **Enforcement of `min_vibe_version`** at install/resolve time — only the field's
  existence and its propagation to the registry index + publish hook were measured
  (§3.2.3), not whether an installing `vibe` actually refuses an older self.
- **CI outside the perimeter** — no `.github/workflows/`, `.gitlab-ci.yml`, etc.
  were found in the perimeter; self-check.sh is treated as the gate panel. A host
  `specmap --check` run by some unmeasured harness cannot be ruled out, only
  reported absent where measured.
- **The `v0.7.0` and `crates/vendor/` mirrors** were treated as `sync-engines`
  duplicates and not audited field-by-field (§3.7.4 counts only authored impls).
- **§3.7.3 counts are for the host root `specmap.json` only** — the artefact the
  format change lands on first. The 12 other `specmap.json` files (package,
  fixtures, demos) have their own counts, not measured here.
- **The lockfile's relationship to a packaged map** beyond content-hash scope
  (e.g. whether `vibe.lock` would record a packaged map as a written file) was not
  traced end-to-end.
- **`progress-core` / `vibe-cli` `"schema": 2` literals** belong to the
  progress-evidence schema, not specmap (§3.6.4) — flagged as a collision, not a
  specmap site.
