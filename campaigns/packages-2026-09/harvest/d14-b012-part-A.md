# D14 · B-012 part A — PROP-014 runtime/distribution family: feasibility evidence

**Date:** 2026-08-01
**HEAD:** `ed0abbab docs(campaign): волна 10 closes the D13 seal tail in the LOG`
**Subject:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml` (read in full)
**Owner directive:** B-012 — «провести исследование, можно ли реализовать». This document is **evidence only**: facts with `file:line`, no verdicts, no build/skip recommendation. The recommendation stays with the boss.

**Default search perimeter** (used for every absence claim below unless a section widens it):

- `crates/` (host crates)
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/` (engine crates)
- `xtask/`
- `tools/`
- `schemas/`

Excluded always: `legacy-spec/**`. Not treated as evidence: `campaigns/**`, `refs/**`. Vendored copies under `vibedeps/**` are cited only as *mirrors* of an engine crate, never as an independent second implementation.

**Scope of part A** — the runtime/distribution family, four mechanisms plus one rider:

| id | mechanism | PROP-014 anchor |
|---|---|---|
| A1 | package-shipped `specmap.json` index + fetch-by-content-hash | `#DISTRIBUTION-RIDES-THE-EXISTING-REGISTRY` (:58), `#RUNTIME-DISTRIBUTION` (:240) |
| A2 | error-rendering index lookup (revision + `run: vibe explain <uri>`) | `#QUERY-ERROR-PROVENANCE` (:226) |
| A3 | `[metamodel] profile` runtime profiles open/contract/none | `#RUNTIME-PROFILES` (:241), `#CLOSED-SOURCE-PROJECTS-SHIP-A-REDACTED-PROFILE` (:60) |
| A4 | `specmap_query` / `specmap_source` MCP tools + runtime channel | `#RUNTIME-TRANSPORT` (:239), `#RUNTIME-EXPOSES-THE-METAMODEL-TO-CONSUMERS` (:56) |
| A5 | trust layer — signing, MCP framing, imperative-phrasing lint (gates A4) | `#RUNTIME-SECURITY-IS-NON-OPTIONAL` (:242), open question `#OPEN-SIGNING-SCHEME` (:415) |

---

## A1 — Package-shipped `specmap.json` index + fetch-by-content-hash

Annotations under test: `#DISTRIBUTION-RIDES-THE-EXISTING-REGISTRY` (:58) — *"no package ships an index (no `vibe.toml` lists `specmap.json` in a payload), and there is no fetch-by-content-hash path — `content_hash` hashes, it does not retrieve"*; and `#RUNTIME-DISTRIBUTION` (:240), which adds that the only `specmap.json` files under `packages/` are extract-test fixtures plus one project's own working index.

### 1. What exists today

**The index producer is complete and gated.**

- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/index.rs:25` — `pub const INDEX_REL_PATH: &str = "specmap.json"`; `:27` `pub const SCHEMA: u32 = 2`.
- Build / serialise / write / gate: `index.rs:55` `build`, `:64` `build_with_scanner`, `:178` `to_canonical_bytes`, `:184` `index_path`, `:297` `write`, `:324` `check`.
- Wire types are codegen'd from a JTD schema: `schemas/specmap.jtd.json` → `core-ai-native-specmap/src/generated/specmap/mod.rs` (`content_hash` field at `generated/specmap/mod.rs:156`).
- Policy is a per-project file, not a per-package one: `core-ai-native-specmap/src/config.rs:127` `Config::REL_PATH = "specmap.toml"`, `:134` `Config::load`, and the root `specmap.toml` declares `namespace`, `scan_roots`, `spec_roots`, `root_spec_docs`, `exempt`, `dispositioned`, `[[external_specs]]`.

**The host index exists and is non-trivial.** `C:\Users\olegc\git\v\vibevm\specmap.json`, read with `python -json`: exactly six top-level keys — `code_items` (898), `edges` (912), `schema` (`2`), `spec_units` (5266), `suspects` (0), `warnings` (265). A `spec_unit` carries `anchor`, `content_hash` (`sha256:…`), `doc_path`, `file`, `heading`, `line`, `uri`. A `code_item` carries `crate_name`, `file`, `item_kind`, `line`, `symbol` — **no** hash, **no** doc text.

**Spec-unit content hashing is real, and it is a hash of unit *text*.**

- `core-ai-native-specmap/src/lib.rs:54` `pub fn content_hash(text: &str) -> String` — `sha256:` prefix, CRLF-invariant (asserted `lib.rs:83-88`), 7+64 chars.
- Call sites that populate the index: `mdspec.rs:287` and `mdspec.rs:450`, both `contentHash: content_hash(&span_text)`.
- It is consumed as a *staleness witness*, never as an address: surfaced in `explain.rs:99` (`hash {…}` line) and `explain.rs:268` (`"content_hash": u.contentHash` in the JSON subgraph).

**A second, unrelated `content_hash` exists at package granularity — this is the "fetch integrity" PROP-002 already has.**

- `crates/vibe-core/src/content_hash.rs:34` `pub struct ContentHash(String)` — the `(group, name, version, content_hash)` identity component, `PREFIX = "sha256:"`.
- Computed over a whole package *tree*: `crates/vibe-index/src/content_hash.rs:40` `compute_content_hash(pkg_dir)`, walking every shippable file, sorted, `rel-path\0bytes\0` into one SHA-256.
- Gated at fetch: `crates/vibe-registry/src/git_package_registry/fetch.rs:274` `fetch_with_expected_hash`; its contract (doc comment `:236-265`) is *"accept the first source whose computed hash equals `h`"*, with mirror-disagreement warnings. `fetch.rs:96-99` states plainly that mirror integrity is not checked at the clone layer and the hash gate is layered on top.
- **Granularity is the whole package tree.** There is no blob-addressed store and no hash→content lookup: `grep -rn --include='*.rs' -E 'resolve_fragment|fn fragment|Fragment'` over `crates/ packages/…/core-ai-native/v0.8.0/crates/ xtask/ tools/ schemas/` returns only `crates/progress-core/src/model.rs:96`, `parse/markers.rs:130`, `rollup.rs:42` — the PROP-043 progress-markup `Granularity::Fragment`, an unrelated concept. The annotation's *"`content_hash` hashes, it does not retrieve"* is confirmed on both hashes.

**The "no `vibe.toml` lists it in a payload" claim is true but the reason is stronger than stated: there is no payload list to list it in.**

- `find . -name 'vibe.toml' -not -path './target/*' -not -path './legacy-spec/*'` → 343 manifests; piping all of them through `grep -n 'specmap\.json\|metamodel'` returns **zero** hits.
- The manifest type has no file-selection surface at all: `crates/vibe-core/src/manifest/document.rs:67-185` — `Manifest` fields are `project`, `package`, `workspace`, `origin`, `requires`, `requires_any`, `provides`, `obsoletes`, `conflicts`, `recommends`, `suggests`, `skill`, `binary`, `mcp_server`, `hooks`, `compatibility`, `boot_snippet`, `features`, `target`, `active`, `llm`, `registry`, `mirror`, `override`, `i18n`, `boot`. No `files` / `payload` / `include`.
- What ships is the whole tree minus a fixed exclusion list: `crates/vibe-index/src/content_hash.rs:28` — `const SHIPPABLE_EXCLUDES: &[&str] = &[".git", ".vibe", "target", "node_modules", ".vibeignore"]`, applied per-entry at `:36`, `:43`.
- Consequence, verified on disk: a `specmap.json` at a package root already travels. `vibevm/vibepacks/org.vibevm.fractality/fractality/v0.1.0/specmap.json` sits beside `vibevm/vibepacks/org.vibevm.fractality/fractality/v0.1.0/vibe.toml` (`[package] name = "fractality"`, `kind = "tool"`, `version = "0.1.0"`). No `.gitignore` in the tree names `specmap` (`grep -rn 'specmap' --include='.gitignore' .` → zero hits outside `target/`).
- The other `specmap.json` files under `packages/` are indeed fixtures: `…/go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/{clean,dirty}/specmap.json`, `…/go-ai-native-mcp/v0.1.0/…`, `…/typescript-ai-native-{lang,mcp}/…/tools/ts-extract/test/fixtures/{clean,dirty}/specmap.json`.

**A cross-package consumption path already exists — and it deliberately does *not* use a shipped index.** Root `specmap.toml`, the `[[external_specs]]` table: `namespace = "core-ai-native"`, `root = "vibedeps/flow-core-ai-native/0.7.0/spec"`, with the comment *"Installed packages' spec trees, read for URI RESOLUTION only (PROP-014 §7.1) … Units found here never enter the committed specmap.json."* Typed at `config.rs:99` `ExternalSpec { namespace, root }`. So the host reads an installed package's **markdown source** out of the materialised slot and re-derives units, rather than reading an index the package shipped.

### 2. What would have to be built

Two independent halves; the annotation treats them as one sentence.

**Half 1 — ship the index (mechanically small).**

- Nothing in the manifest layer needs a new key for the *file* to travel; the missing pieces are (a) a decision on where in a package tree the index lives and under whose namespace its URIs are minted, (b) a producer step so package-owned indices are generated and gated the way the host's is, and (c) whether the index enters the package's `content_hash` (it would, automatically, via `compute_content_hash` — which makes the index self-invalidating on every code edit inside the package, a new churn surface for `vibe.lock` pins).
- Surfaces touched: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/index.rs` (a "package mode" root), each shipping package's own CI wiring, `specmap.toml` per package.
- A *consumer-side reader* is the real new surface: today the only reader of a foreign package's traceability data is `[[external_specs]]`, which re-derives from markdown. Reading a shipped index instead means a new resolution path that trusts bytes it did not produce — which is where A5 attaches.

**Half 2 — fetch-by-content-hash (genuinely new).**

- New types: a fragment address (today `content_hash` is a *field on a unit*, not a key), and a store mapping `sha256:…` → source text. Nothing of the kind exists.
- New protocol surface: a retrieval verb. The registry layer's vocabulary is package-granular (`fetch`, `fetch_with_expected_hash`, `lookup`) — `crates/vibe-registry/src/git_package_registry/fetch.rs`.
- Backward compatibility: an index that carries fragment addresses is a **schema bump** — `schemas/specmap.jtd.json` + regenerated `generated/specmap/mod.rs` + `index.rs:27 SCHEMA` from `2` to `3`, and every committed `specmap.json` (host root, `research/*/specmap.json`, `vibevm/vibepacks/org.vibevm.fractality/fractality/v0.1.0/specmap.json`) is regenerated. `code_item` gaining a content hash — which §2.2's and §2.5's own annotations flag as missing — would be the same bump.
- Open design question the spec does not answer: what a "source fragment" *is* on the code side. `CodeItem` has `file`+`line` and no span end and no body, so a code fragment cannot be addressed from the index as it stands.

### 3. Dependencies

- **On A3:** half 1 is what a `contract`/`none` profile would redact. Shipping an index before the profile exists ships the `open` profile unconditionally, for every package, with no opt-out key (`[metamodel]` does not parse — see A3).
- **On A5:** half 1 puts authored prose from a foreign package into a consumer's tree. Whether that requires the signing decision depends on whether the index is treated as reachable-by-an-agent (A4) or only as a build input; `#INVARIANT-RUNTIME-CHANNEL-SHIPS-SIGNED` (:297) binds the *channel*, and `#RUNTIME-DISTRIBUTION` (:240) is listed under §2.8 "Runtime exposure", i.e. inside the channel.
- **On external decisions:** `#OPEN-CROSS-PACKAGE-URIS` (:410) — group-qualified `spec://org.vibevm.world/…` resolution is explicitly deferred "after PROP-008 settles live", and a shipped index is exactly a bag of foreign URIs. `[[external_specs]]` today sidesteps this by letting the consumer declare the namespace→root mapping by hand.
- Half 2 depends on half 1 only loosely (a fragment store could exist without a shipped index), but is useless without it.

### 4. Effort class

**Half 1: S.** The generator, the schema, the canonical serialiser and the gate all exist; packages already ship whole trees, so the file travels for free — what is left is policy plumbing per package.

**Half 2: L.** A content-addressed fragment store, a retrieval verb on the registry, a schema bump with a regeneration of every committed index, and a prior decision on what a code-side fragment is — none of which has a starting half in the tree.

### 5. Observations on warrant

- **A consumer already exists for the data, in a different shape.** `[[external_specs]]` (root `specmap.toml`, `config.rs:99`) proves the host wants a foreign package's spec units at build time and gets them by re-deriving from shipped markdown. Whether a shipped index beats re-derivation is an efficiency question with a measurable answer: the host derives 5266 units per run.
- **The index is already shipped by accident in at least one package** (`vibevm/vibepacks/org.vibevm.fractality/fractality/v0.1.0/specmap.json`), with no manifest key, no namespace declaration, and nothing that reads it. That file is evidence the "no payload key" framing is not the obstacle.
- **The engine ships in a package; the policy stays with the consumer.** Root `specmap.toml`'s own header states this posture explicitly ("A consumer of the rust-ai-native stack writes its own specmap.toml for its own layout — the same posture conform.toml takes (PROP-024)"). A package-shipped index inverts that for one artefact.
- **Nothing in the tree currently asks a package "what do you trace".** Both `#DISTRIBUTION-…` consumers named in §1.2 and §2.8 are the runtime channel (A4), which is unbuilt for consumers.

---

## A2 — Error-rendering index lookup (revision + `run: vibe explain <uri>` hint)

Annotation under test: `#QUERY-ERROR-PROVENANCE` (:226) — *"error renderings cite `violates spec://…` from compile-time constants … with no revision and no `run: vibe explain` hint. The doorway is real; the lookup is not."*

### 1. What exists today

**The compile-time half is real and enforced, at scale.**

- Engine: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-mcp/src/error.rs:25`, `:37`, `:48` — three `#[error("… (violates spec://…/MCP-CORE-v0.1#wire; fix surface: …)")]` variants, pinned by the doctest at `error.rs:14-20` (`assert!(e.to_string().contains("MCP-CORE-v0.1#wire"))`).
- Host: `crates/vibe-core/src/error.rs` — the enum carries `#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#package-identity")]` at `:30`, and its variants embed the URI in the Display template: `:34` emits `violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref` for the `BadPackageRef` variant declared at `:37`. Fifteen `violates spec://` occurrences in this file alone.
- **Scale, measured:** `grep -rn --include='*.rs' -o 'violates spec://' crates/` → **232** occurrences across **48** files. The engine crates add 3 more (`packages/…/core-ai-native/v0.8.0/crates/`).
- The conform grammar is the second, *separate* citation channel: `core-ai-native-conform/src/rules/mod.rs:49` `req_message(uri, why, fix_surface) -> "violates REQ {uri}: {why}; fix surface: {fix_surface}"`, with its acceptor beside it at `rules/mod.rs:62` `matches_req_grammar` (accepts `spec://`, `discipline://`, `misra://`).
- Both are *gated*, not merely conventional: `core-ai-native-conform/src/rules/diagnostics.rs:235` rule id `error-message-cites-req` (the Display text must contain `spec://`; check at `:243-280`) and `:314` `error-enum-cites-req` (the owning enum must carry a `#[spec]` edge). The underlying fact type is `core-ai-native-conform/src/facts.rs:66` `Fact::ErrorVariant { enum_symbol, variant, message, line, enum_attrs }`.

**A single rendering chokepoint already exists in the host CLI, and it already interposes structure.**

- `crates/vibe-cli/src/main.rs:229-233` — the whole dispatch funnels into `match result { … Err(err) => { ctx.error(&err); as_exit_code(&err) } }`.
- `crates/vibe-cli/src/output.rs:212` `pub fn error(&self, err: &anyhow::Error)` — human mode prints `error: {err:#}` (`:215`); JSON mode builds a payload and then calls `stamp_structured_error` (`:222`).
- `crates/vibe-cli/src/output.rs:238` `fn stamp_structured_error(&self, payload: &mut Value, err: &anyhow::Error)` — walks the anyhow chain for a known variant and *adds machine-readable fields alongside the stringified error*. Its own doc comment (`:230-237`) frames it as the extension point: *"future structured variants extend this match"*. This is precisely the shape A2 needs, already load-bearing.

**Loading the index from the CLI is already done, tolerantly.**

- `crates/vibe-cli/src/commands/progress_evidence.rs:39` `SpecmapEvidence::load(root) -> Result<Option<…>>`, reading `root.join(INDEX_REL_PATH)`; `:43` returns `Ok(None)` on `NotFound`, `:48` errors on a malformed index. The doc comment at `:35-38` states the posture: *"most consuming projects have none, and that is not an error … a corrupt index must never read as an absent one."* Its test at `:186` asserts `no specmap.json ⇒ no provider, no error`.

**Lookup feasibility, measured against the real index.** Extracting every distinct `spec://<path>#<anchor>` that appears after `violates ` in `crates/**/*.rs` yields **81 distinct URIs**; intersecting with the 5266 `spec_units[].uri` in the committed root `specmap.json`: **81 present, 0 missing**. The lookup would resolve for every error citation in the host today.

**The revision half has no data on the host.** Same index, same measurement: of 5266 `spec_units`, **0 carry `revision`** and **0 carry `kind`**. The machinery is not broken — it is unexercised here:

- The parser is complete: `core-ai-native-specmap/src/mdspec.rs:63` `parse_kind_line` accepts `` `prop|req|design|guide r<N> [planned|disputed(#anchor)]` `` and warns (non-fatally) on a malformed revision (`:88-96`).
- Other projects' committed indices do populate it: `research/rust-demo/specmap.json` — 6 units, 5 with revision and kind; `research/go-demo/specmap.json` — 7 units, 6 with revision and kind, and 16/16 edges pinned.
- Host `specmap.json` edges: 912 total, **201 carry `pinned_r`** (the code-side `r = N`); verbs `implements` 677 / `verifies` 223 / `deviates` 12; `suspects` 0.
- `vibevm/vibepacks/org.vibevm.fractality/fractality/v0.1.0/specmap.json`: 171 units, 0 with revision/kind; 75 edges, 0 pinned.

**No `vibe explain` hint exists anywhere.** `grep -rn --include='*.rs' -E 'run: vibe explain|vibe explain|trace explain' crates/` returns exactly one line: `crates/vibe-cli/src/cli.rs:220`, the doc-comment example on the `Trace` subcommand. And that subcommand is a **delegating alias** — `cli.rs:215-218` documents it as *"arguments pass through verbatim to the installed `rust-ai-native trace`"*, dispatched at `main.rs:217` via `commands::trace::run(&args)`, which returns a child exit code, not a `Result<()>` through `ctx.error`.

### 2. What would have to be built

- **Extract the URI from the rendered message.** The URI lives inside the `#[error("…")]` Display template, so there is no typed accessor; the renderer would parse `{err:#}` for `violates (spec|discipline)://…#anchor`. Two grammars are in play and neither acceptor covers the other: `matches_req_grammar` (`rules/mod.rs:62`) accepts the *conform finding* shape (`violates REQ <uri>: <why>; fix surface: <where>`), while thiserror templates use `(violates <uri>; fix: …)` / `(violates <uri>; fix surface: …)`. A shared acceptor is a new artefact, or the two grammars converge.
- **Load the index at render time.** `SpecmapEvidence::load` is the precedent; a lookup-oriented sibling is needed (unit by URI → `revision`, `kind`, `heading`, `file:line`) plus a decision on failure posture: an absent/stale index must degrade to today's output silently, and error rendering is the one path that must never itself fail.
- **Surfaces touched:** `crates/vibe-cli/src/output.rs` (`error`, `stamp_structured_error`), `crates/vibe-cli/src/main.rs` (only if the non-`ctx.error` exits at `:205-217` must also carry the hint), and — if the hint text is to be produced by the engine rather than the host — a new render helper in `core-ai-native-specmap`.
- **Backward compatibility:** none of this changes the index schema. It changes *stderr text* and the JSON error payload — both are user-visible contracts, and the JSON payload has downstream consumers per `output.rs:235-237` ("CI, monitoring pipelines can branch on `error_kind`").
- **The engine-crate half is different.** `core-ai-native-mcp/src/error.rs` is a transport crate that by contract knows nothing of vibe (`core-ai-native-mcp/src/lib.rs:20-22`: *"Nothing here knows any language, any discipline rule, or vibe"*). Its errors cannot gain a `run: vibe explain` hint without breaking that separability seam — the hint would have to be added by whoever renders, not by the enum.
- **What the hint should name is undecided.** `vibe trace` is an alias to an installed stack binary (`cli.rs:215`), so `run: vibe explain <uri>` names a command that does not exist; `vibe trace explain <uri>` exists only when `rust-ai-native` is installed. §2.6's `#QUERIES-LIVE-BEHIND-XTASK-DURING-THE-PILOT` (:227) explicitly defers the promotion to `vibe explain` to "a Phase 4 decision".

### 3. Dependencies

- **On A1:** none for the host (the host index is at the repo root already). For an *installed package's* error to cite its own revision, the index must travel — that is A1.
- **On A4/A5:** none. This is a local, on-disk lookup in a checkout the operator owns; nothing crosses a trust boundary and nothing is exposed over a wire.
- **On unrelated spec work:** the revision half is blocked on data, not code — host spec units must start declaring `` `req r1` `` kind lines before a lookup has a revision to print. `#REVISION-R-IS-AN-AUTHOR-ASSERTED-SEMANTIC-BUMP` (:135) and `#INVALIDATION-SPEC-BUMP-MAKES-EDGES-SUSPECT` (:140) are the clauses that would come alive with it; `suspects` is 0 today partly because 0 units declare a revision to bump.
- **On a naming decision:** which command the hint names (see above).

### 4. Effort class

**S** for the doorway itself — one chokepoint (`output.rs:212`), one existing structured-extension hook (`output.rs:238`), one existing tolerant loader (`progress_evidence.rs:39`), and a 100%-hit-rate URI set (81/81).

**M** if the revision is required in the rendering, because that is not a code task: it needs kind lines authored across the host spec corpus before it prints anything.

### 5. Observations on warrant

- **The consumer exists and is the stated highest-leverage one.** §2.6 calls it *"the single highest-leverage consumer: every failure becomes a doorway into the metamodel"*, and the tree agrees at 232 citation sites gated by two conform rules whose stated `why` is *"errors are agent food … navigable back to the requirement without source access"* (`diagnostics.rs:240-244`).
- **The last clause of that `why` is the tension.** The gate's own justification is that the Display text is navigable *without source access* — a compile-time constant achieves that; an index lookup adds the revision and a follow-up command, and requires an index on disk, which is *more* coupling than the rule was written to demand.
- **The revision the mechanism would print is empty in this repository.** 0/5266 units carry one. The demos prove the parser works, so this is an authoring gap in the host corpus, not an engine gap.
- **The nearest analogous feature already shipped and is small.** `stamp_structured_error` (`output.rs:238`) is the same idea (enrich a rendered error with structured facts) for a single error variant; the code is a `match` over the anyhow chain.

---

## A3 — `[metamodel] profile` runtime profiles (open / contract / none)

Annotations under test: `#RUNTIME-PROFILES` (:241) — *"`[metamodel]` is in no manifest, no schema and no parser; the three profile values have no representation"*; and `#CLOSED-SOURCE-PROJECTS-SHIP-A-REDACTED-PROFILE` (:60) — *"the profile has no manifest key, no parser and no redaction path … this fact cannot land before `#RUNTIME-EXPOSES-THE-METAMODEL-TO-CONSUMERS` above it does."*

### 1. What exists today

**Absence confirmed, perimeter named.**

- `grep -rn --include='*.rs' --include='*.toml' --include='*.json' -i 'metamodel'` over `crates/ vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/ xtask/ tools/ schemas/` → **one** hit, and it is prose: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/ledger.rs:34` — *"the discipline package in effect, and the metamodel wire schema"* (a doc comment naming an epoch input).
- All 343 `vibe.toml` files (`find . -name 'vibe.toml' -not -path './target/*' -not -path './legacy-spec/*'`) piped through `grep -n 'specmap\.json\|metamodel'` → **zero** hits.
- No `redact`-anything: `grep -i 'redact'` over the same perimeter returns only publish-token hygiene — `crates/vibe-cli/src/commands/registry/publish.rs:182`, `:185`; `crates/vibe-cli/src/commands/registry/redirect/{create.rs:197,update.rs:87}`; `crates/vibe-cli/src/commands/show/config.rs:88`, `:234-239`, `:393`. Nothing redacts spec or code content.

**The manifest is a closed schema — an unknown table is an error, not an ignored key.**

- `crates/vibe-core/src/manifest/document.rs:66-67` — `#[serde(deny_unknown_fields)] pub struct Manifest`. The type itself carries `#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest", r = 1)]` at `:62-65`.
- Consequence: a `vibe.toml` written today with `[metamodel] profile = "open"` **fails to parse**, and would keep failing for every already-released `vibe` binary after the key lands. There is one existing lever for that: `crates/vibe-core/src/manifest/package.rs:332` `Compatibility { min_vibe_version, requires_kinds }` (`[compatibility]`, itself `deny_unknown_fields` at `:331`) — a package declaring the new table would need `min_vibe_version` set, and old binaries would report a parse error rather than a compatibility message.
- The Rust type *is* the schema: there is no JSON-schema artefact for `vibe.toml` anywhere (`find` for `*schema*.json` outside `target/` returns `crates/vibe-cli/resources/package-tree.schema.v1.json`, a fractality manual-test fixture, and `refs/` copies of cargo/uv schemas — none of them the manifest). The prose schema is `VIBEVM-SPEC.md:583` §7.3 "The manifest schema", and `VIBEVM-SPEC.md` is recorded as owner-frozen (`CLAUDE.md:136`; PROP-014 :25 calls it "a 99KB owner-frozen spec"; root `specmap.toml` calls it "the owner-frozen implementation spec").

**Precedents for exactly this shape of key are plentiful and consistent.** Three existing `[package]`-level enums, all with a default that stays out of the serialised form:

- `crates/vibe-core/src/manifest/package.rs:155` `PackageFormat { Simple (default), Normal }`, `#[serde(rename_all = "lowercase")]`, `is_default()` used at `:125-126`.
- `package.rs:219` `PublishPosture { All(bool), Registries(Vec<String>) }`, `#[serde(untagged)]`, default `All(true)`, `is_default()` at `:236`.
- `package.rs:285` `Materialization` (snapshot / hardlink / in-place), skipped when default at `:109-110`.

**The data the profiles would gate does not have the shape the three values assume.** `schemas/specmap.jtd.json`, `code_item` definition: properties are exactly `symbol`, `item_kind`, `crate_name`, `file`, `line`. No signature, no doc text, no body, no span end. Confirmed against the live index (`specmap.json` sample `code_item`). The `explain` renderers emit no more than that (`explain.rs:156-158` text; `explain.rs:242-246` JSON).

- `contract` is specified as *"spec units + signatures of items, no bodies"* — the index has **no signatures**, so today's data is *below* `contract`, not above it.
- `open` is specified as *"full graph + source"* — source retrieval is exactly A1 half 2, which does not exist.
- `none` is the only one of the three that today's tree can express, and it expresses it by shipping nothing.

### 2. What would have to be built

- **New manifest surface.** A `MetamodelSection { profile: MetamodelProfile }` in `crates/vibe-core/src/manifest/` plus a field on `Manifest` (`document.rs`), following the `PackageFormat` shape (lowercase rename, `Default`, `is_default`, `skip_serializing_if`). Mechanically this is the smallest possible addition; the type work is ~40 lines with doctests.
- **A schema decision that is not a Rust decision.** Whether the key is package-role or project-role (or both) — `Manifest` mixes the two and `Manifest::validate` enforces the XOR (`document.rs:335-344`). A closed-source project redacting *its own* published package is package-role; a consumer capping what it will *accept* from installed packages would be project-role. §2.8.3 only describes the former.
- **A redaction path, which is where the work actually is.** With no consumer surface (A4) and no shipped index (A1), there is no place to apply a profile. Once either exists, redaction must be applied at the *producer* (`core-ai-native-specmap::index::build` / `write`) or at the *server* (A4's tool handlers) — and those give different guarantees: producer-side redaction means the bytes never leave the closed-source machine; server-side means they do and are filtered.
- **`contract` needs new index data before it can mean anything.** Item signatures are not in the scanner's output today (`core-ai-native-specmap/src/rscan.rs` feeds `CodeItem`); adding them is a **schema bump** — `schemas/specmap.jtd.json` + regenerated `generated/specmap/mod.rs` + `index.rs:27 SCHEMA` 2→3 + regeneration of every committed index. (Note in passing: that schema file's `metadata.rustOptions.package` still says `specmap_core::specmap` and its description still names `crates/specmap-core/src/generated/specmap/`, a path that no longer exists after the engine relocated into the package — anyone bumping the schema meets this first.)
- **Documentation surface:** `VIBEVM-SPEC.md` §7.3 (`:583`) enumerates the manifest tables in prose and is owner-frozen — an addition there is an owner decision, not a coder one.

### 3. Dependencies

- **Hard-gated by A4** by the PROP's own words at :60 — *"Nothing is redacted because nothing is exposed at runtime to redact — this fact cannot land before `#RUNTIME-EXPOSES-THE-METAMODEL-TO-CONSUMERS` above it does."* The manifest key can land earlier; it would be inert.
- **`open` is gated by A1 half 2** (source-by-hash) — the value cannot be honoured without a fragment retrieval path.
- **`contract` is gated by a schema bump** (signatures in `code_item`) — see above.
- **On external decisions:** `#OPEN-NON-OSS-CONTRACT-PROFILE` (:416) is explicitly unresolved — *"Exactly which item metadata (signatures? doc comments?) is safe to ship; needs a real closed-source consumer to decide."* No closed-source consumer is present in this tree.
- **Not gated by A5.** Redaction is a confidentiality control; signing (A5) is an integrity control. They are independent, though both are named in §2.8.

### 4. Effort class

**S for the key, L for the mechanism.** Adding `[metamodel] profile` to the manifest is a well-precedented ~40-line type addition with three sibling patterns to copy. Making the three values *mean* anything requires a redaction point that does not exist (A4), a fragment path for `open` (A1), and an index schema bump for `contract` — and the spec itself defers the content question to a consumer that does not exist.

### 5. Observations on warrant

- **No consumer exists today, by construction.** The mechanism's purpose is to let a closed-source project ship less than everything over a channel that is not built. Its own annotation says so.
- **The tree's default posture is already the conservative one.** Nothing is exposed; nothing ships an index that a consumer reads. Adding the key without the channel changes nothing observable.
- **The one closed-source-adjacent signal in the tree points the other way.** `PublishPosture::All(false)` (`package.rs:222-224`, "`publish = false` — never published (workspace-internal)") is how this repository expresses "do not ship this", and `.ledger/` is documented as *"Local per checkout; never shipped, never signed, never exposed"* (`core-ai-native-specmap/src/ledger.rs:18`). Both are all-or-nothing postures; neither is a graded profile.
- **`contract` is under-determined, not merely unbuilt.** §7.7 (:416) asks the question and the tree contains no answer, so building it would mean deciding on the spec's behalf.

---

## A4 — `specmap_query` / `specmap_source` MCP tools + the runtime channel

Annotations under test: `#RUNTIME-TRANSPORT` (:239) — *"there is no `specmap_source(content_hash) -> fragment` tool and no general `specmap_query`"*; and `#RUNTIME-EXPOSES-THE-METAMODEL-TO-CONSUMERS` (:56) — *"there is no runtime channel … the MCP servers that would carry it do not offer it: `core-ai-native-mcp` is a transport whose only shipped tool is `echo` … An agent driving `vibe` cannot ask the running tool anything."*

### 1. What exists today

**Two corrections to the :56 annotation, both verified.**

*(a) `core-ai-native-mcp` ships **zero** tools, not one.* `echo` is a test fixture: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-mcp/src/server.rs:187` opens `#[cfg(test)] mod tests`, and `struct Echo` is at `:193` with `name: "echo"` at `:197`. Nothing in `core-ai-native-mcp`'s public surface (`lib.rs:31-37`: `capture`, `Server`, `StdioTransport`, `Transport`, `testing`, `Tool`, `ToolDescriptor`, `ToolOutput`, `ToolSet`, wire types) is a tool. Its own header (`lib.rs:20-22`) states the seam: *"Nothing here knows any language, any discipline rule, or vibe."*

*(b) `trace_explain` is a **shipped MCP tool** in all three stack servers.* The narrower :239 annotation is accurate; the blanket ":56" sentence is not.

- `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-mcp/src/lib.rs:48-67` — `pub const TOOL_NAMES: [&str; 18]`, containing `specmap_check` (`:57`), `specmap_write` (`:58`), `trace_explain` (`:65`). Pinned by `lib.rs:76-80` (`tools/list` is exactly the declared inventory) and by the doctest at `:27-33`.
- Handlers: `rust-ai-native-mcp/src/tools_discipline.rs:192` (`specmap_check`), `:199` (`specmap_write`), `:206` (`trace_explain`, with `target` required at `:216-218`, `json`/`prose` booleans at `:212-213`, dispatching to `rust_ai_native_cli::run_trace_explain` at `:219`).
- `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/crates/typescript-ai-native-mcp/src/lib.rs:50-68` — 17 tools, `trace_explain` at `:66`.
- `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/crates/go-ai-native-mcp/src/lib.rs:49-67` — 17 tools, `trace_explain` at `:65`; handler at `tools_discipline.rs:207`.
- Declared to agent hosts: `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/vibe.toml:27-31` — `[[mcp_server]] name = "rust-ai-native"`, `binary = "rust-ai-native-mcp"`, `args = ["--path", "{project_root}"]`.
- Exercised end-to-end: `rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-mcp/tests/server_replay.rs:85-90` drives `specmap_write` then `specmap_check` over the replay transport.
- Documented as parity rows: `…/rust-ai-native-mcp/v0.7.0/spec/tools/discipline-mcp-rust.md:41-43`; `…/go-ai-native-mcp/v0.1.0/spec/tools/discipline-mcp-go.md:46-48`.

**What that shipped channel is *not*: a consumer channel.** The gap is precise and structural.

- `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/trace.rs:11-12` — `Config::load(root)?` then `index::build(root, &cfg)`. Its own comment at `:9-10`: *"Build fresh in-memory: explain answers for the tree as it is, never for a stale committed artefact."* The tool therefore needs a **source checkout with a `specmap.toml`**, not an installed package.
- The server is bound to the consuming project's own root (`rust-ai-native-mcp/src/main.rs:16-21`, `--path` defaulting to `.`, registrations passing `{project_root}`).
- Foreign packages' units are deliberately excluded from the map the explainer sees: `core-ai-native-specmap/src/index.rs:77` `scan_external_units` feeds only the revision map (`:87-92`), and the comment at `:70-75` states *"their units feed the revision map below … but are never serialised into this project's index."* `Specmap.specUnits` at `:165` is the project's own units only.
- Consequence, from the explainer's own code: `explain.rs:92-94` — `explain_unit` filters `map.specUnits` by URI and `bail!("no spec unit with URI `{uri}` in the index")` when empty. So `trace_explain "spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#runtime"` fails **even in a project whose `specmap.toml` declares that package under `[[external_specs]]`** (root `specmap.toml` does declare it: `namespace = "core-ai-native"`, `root = "vibedeps/flow-core-ai-native/0.7.0/spec"`).

**`specmap_query` and `specmap_source` do not exist.** `grep -rn -E 'specmap_query|specmap_source|specmap_explain' crates/ packages/ xtask/ tools/ schemas/ docs/` (excluding `/target/`) returns only two files, both spec prose: `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/spec/mechanisms/PROP-014-…md:178` and the v0.8.0 subject document at `:56` and `:239`.

**The host `vibe-mcp` has a consumer-facing tool surface — four tools, none of them traceability.** `crates/vibe-mcp/src/tools.rs`:

- `:71` `query_package` — *"Look up an installed package in the project's lockfile and return its full lockfile entry: kind, name, version, content_hash, registry, source_url, source_ref, resolved_commit, files_written, features, subskills_active, describes (PURL), language."* Implementation reads `ctx.load_lockfile()` (`:95-100`). This is the closest existing analogue of "an agent asks the running tool about an installed package" — and it answers from the *consumer's lockfile*, never from anything the package shipped.
- `:158` `read_subskill` — returns *"the concatenated text of every file the subskill's `[content].files_written` recorded"* from an installed package.
- `:289` `materialise_subskill`.
- `:427` `agentic_explain` — the PROP-018 relay the :56 annotation correctly identifies as unrelated (`#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#transports")]` at `:421`).

**The host `vibe trace` is a pass-through alias, not a channel.** `crates/vibe-cli/src/cli.rs:215-222` documents `Trace { args }` as *"a delegating alias: arguments pass through verbatim to the installed `rust-ai-native trace`"*; dispatch at `crates/vibe-cli/src/main.rs:217` returns a child exit code.

### 2. What would have to be built

Three separable pieces, wrongly bundled by the annotation into one.

**(i) Parity: bring `trace_explain` to the host `vibe-mcp`.** New `McpTool` impl in `crates/vibe-mcp/src/tools.rs` following the existing `#[cell(seam = "McpTool", variant = …)]` + `#[spec(implements = …)]` pattern (`:64-66`), loading the index the way `crates/vibe-cli/src/commands/progress_evidence.rs:39` already does. No protocol change, no schema change.

**(ii) `specmap_query`.** The spec says *"a general `specmap_query`"* and never defines the query language. Today's surface is single-target: `explain_text(map, target)` / `explain_json(map, target)` (`explain.rs:199`, `:209`), where `target` is a symbol or a `spec://` URI, with suffix-matching and an ambiguity error (`explain.rs:132-151`). A general query needs a grammar (by kind? by crate? by verb? by coverage state?), a result schema, and a bound on result size — none of which exists in any form. The §2.6 CLI surface (`vibe trace coverage|impact|orphans|stale`) is the closest sketch, and two of those tables are themselves unbuilt per `#INDEX-CONTENTS` (:210) (coverage and orphans are absent from the index).

**(iii) `specmap_source(content_hash) -> fragment`.** Gated wholly by A1 half 2 — there is no hash→content retrieval anywhere (see A1 §1). On the spec side a fragment is derivable (`spec_unit` has `file` + `line` + `content_hash`, so the unit span could be re-read and hash-verified); on the code side it is **not** (`code_item` has `file` + `line` and no span end, no body — `schemas/specmap.jtd.json`, `code_item` block).

**(iv) The consumer-facing channel proper** — what §1.2 actually asks for ("an agent driving `vibe` can ask … why does `vibe install` behave this way"). This needs the map to contain *foreign* packages' units, which `index.rs:70-75` deliberately refuses. Either the exclusion is relaxed (breaking the byte-stability contract the comment protects and the `--check` gate depends on), or a second, non-committed "resolution map" is assembled at query time. Plus A1 for where those units come from.

**Backward compatibility.** Adding tools is additive on the wire, but each server's `TOOL_NAMES` array is a pinned contract with an enumeration test (`rust-ai-native-mcp/src/lib.rs:76-80`, and the `[&str; 18]` / `[&str; 17]` array lengths), and the parity tables in each package's `spec/tools/discipline-mcp-*.md` are part of the released package. Adding one tool touches the array, its length, the test, the doctest at `lib.rs:27-33` (`assert_eq!(names.len(), 18)`), and the shipped spec doc — in three stacks.

### 3. Dependencies

- **A4 (iii) depends on A1 half 2** — hard.
- **A4 (iv) depends on A1 half 1** — hard: without a shipped index there is nothing for a consumer to be told about.
- **A4 is gated by A5 by the PROP's own position**, twice: `#RUNTIME-SECURITY-IS-NON-OPTIONAL` (:242) — *"the trust layer ships **with** the runtime channel, not after it"*; `#INVARIANT-RUNTIME-CHANNEL-SHIPS-SIGNED` (:297); and `#PHASE-4-MCP-TOOLS-BLOCKED-ON-SIGNING` (:369) — *"`vibe-mcp` tools per §2.8 — **blocked on the signing decision (§7.6)**; ships signed or not at all."*
- **A5 is itself blocked on `#OPEN-SIGNING-SCHEME` (:415)**, an undecided open question (*"sigstore vs. minisign-class vs. registry-native git signatures; decide before Phase 4's MCP exposure; blocking for §2.8"*).
- **A3 would gate what any of these return** for a package that declared a profile — but A3 is inert until A4 exists.
- **Unresolved by the spec:** the query language for (ii); what a code-side fragment is for (iii).

### 4. Effort class

- **(i) parity in `vibe-mcp`: S.** Every seam exists — transport, `ToolSet`, the tool trait, the tolerant index loader, and three worked examples of the exact tool shape.
- **(ii) `specmap_query`: M.** The code is small; the undefined query grammar and result-size bound are the work, and they are design decisions the spec does not make.
- **(iii) `specmap_source`: L.** Blocked behind A1 half 2 and an unanswered question about code-side fragments.
- **(iv) consumer-facing channel: L.** It contradicts a deliberate exclusion in `index.rs:70-75` that the determinism gate depends on, and needs A1 first.

### 5. Observations on warrant

- **A working consumer already exists for the *developer* form.** Three stack MCP servers ship `trace_explain` and register it with agent hosts (`vibe.toml:27-31`); the tool is enumerated, tested, and documented. The unbuilt part is the *consumer-of-a-published-package* form, which is a different audience.
- **The nearest host precedent points at feasibility and at the risk simultaneously.** `query_package` (`tools.rs:71`) proves "agent asks the running vibe about an installed package" is an established pattern here; `read_subskill` (`tools.rs:158`) proves package-authored **prose** already flows into an agent's context through that pattern — which is exactly the exposure §2.8.4 was written about (see A5).
- **The spec's own sequencing blocks this mechanism at two points**, and both are recorded inside the document rather than outside it: §4 Phase 4 (`:369`) and §3.3 (`:297`). Neither is a technical obstacle; both are standing positions.
- **The `#RUNTIME-EXPOSES-…` annotation at :56 needs narrowing.** As written it contradicts the tree (`trace_explain` ships in three servers) and contradicts the sibling annotation at :239, which concedes the tool set and scopes the absence to `specmap_source` / `specmap_query`. Also `echo` is a `#[cfg(test)]` fixture, so "whose only shipped tool is `echo`" overstates in the other direction.

---

## A5 — Trust layer (rider; gates A4): signing, MCP framing, imperative-phrasing lint

Annotation under test: `#RUNTIME-SECURITY-IS-NON-OPTIONAL` (:242) — *"Specified, not built — all three clauses"* — plus the open question `#OPEN-SIGNING-SCHEME` (:415) and the invariant `#INVARIANT-RUNTIME-CHANNEL-SHIPS-SIGNED` (:297).

### 1. What exists today

**(a) Signing — absent, confirmed.**

`grep -rn -iE 'sigstore|minisign|cosign|ed25519|gpg|pgp|\bsign(ed|ing|ature)\b'` across `crates/ vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/ xtask/ tools/ schemas/` over `*.rs`, `*.toml`, `*.json`, `*.sh`, after filtering the "function signature" sense, yields **five** lines and none is cryptographic:

- `crates/vibe-actions/src/params.rs:27`, `crates/vibe-cli/src/commands/prefs/tui/form/control.rs:120`, `crates/vibe-settings/src/schema/types.rs:36` — "a 64-bit **signed** integer".
- `crates/vibe-cli/src/commands/tree/tui/flatten.rs:27` — a function signature.
- `packages/…/core-ai-native-specmap/src/ledger.rs:17` — *"Local per checkout; never shipped, never **signed**, never exposed"*, i.e. an explicit statement that the ledger is out of scope.

The only crypto dependency in the workspace is `Cargo.toml:121` `sha2 = "0.10"`, used for content hashing (`crates/vibe-index/src/content_hash.rs:50`, `crates/vibe-registry/src/git_registry.rs:23`, `…/git_package_registry/mod.rs:43`).

What the repository *does* have, and what it is not:

- **Integrity against a pin, not authenticity of a publisher.** `crates/vibe-registry/src/git_package_registry/fetch.rs:274` `fetch_with_expected_hash` accepts the first source whose recomputed tree hash equals the lockfile's `content_hash`, warns on disagreement, and wipes the cache between attempts (doc `:236-260`, citing PROP-002 §2.3). This detects a *mirror disagreeing with the pin*; it says nothing about who produced the bytes, and the first fetch (no pin — `:243-244`) accepts whatever arrives.
- **Publisher-to-host authentication, not artefact-to-consumer.** `crates/vibe-publish/src/token.rs:1-30` — a five-legged publish-token precedence (env per host, then per-host files under the settings dir), pinned to PROP-000 §20.
- The scheme itself is undecided by the subject document: `#OPEN-SIGNING-SCHEME` (:415) — *"sigstore vs. minisign-class vs. registry-native git signatures; decide before Phase 4's MCP exposure; blocking for §2.8."*

**(b) MCP framing — absent, and there is a live counter-example shipping.**

- The phrase and its variants return nothing: `grep -rn -i 'reference data\|not instructions\|prompt.injection\|injection'` over the perimeter hits only bearer-**token** injection in `crates/vibe-registry/src/git_package_registry/auth.rs:2`, `:260`, `lookup.rs:18`, `mod.rs:212`, `multi_registry_resolver/redirect_follow.rs:249`, `sources.rs:173`, `:198`, plus one unrelated "capability injection" comment at `core-ai-native-conform/src/rules/go.rs:30`. No tool description in `crates/vibe-mcp/src/tools.rs` or in the three stack servers' `tools_discipline.rs` carries the framing.
- **The host already ships an MCP tool whose contract is the exact inverse.** `crates/vibe-mcp/src/tools.rs:429` — `agentic_explain`'s description ends: *"Treat the returned `instruction` field as the authoritative description of the task and follow it."* Its result payload adds `"note": "Carry out this instruction yourself on your own model; nothing was written to disk."` (`:466`). That is deliberate (PROP-018 relay, `#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#transports")]` at `:421`), and it means a blanket "MCP tools frame output as reference data" is not the house rule today.
- **The exposure §2.8.4 describes already exists for a different payload, unsigned and unframed.** `crates/vibe-mcp/src/tools.rs:158-161` — `read_subskill` returns *"the concatenated text of every file the subskill's `[content].files_written` recorded"* from an **installed package** into the agent's context, with no framing.
- **And a larger one exists outside MCP entirely.** `crates/vibe-core/src/manifest/document.rs:146-147` `boot_snippet: Option<BootSnippet>` / `crates/vibe-core/src/manifest/package.rs:500-519` `BootSnippet { source, category, link, when }` — an installed package contributes a markdown file that the agent reads **at session boot**, ordered into the computed boot sequence. Package-authored prose reaching an agent's context is already the normal operation of this system.

**(c) Imperative-phrasing lint — absent, confirmed, and lacking the data it would key on.**

- The conform rule roster is thirteen ids (`fn id(&self)` across `packages/…/core-ai-native-conform/src/rules/*.rs`): `ambient-env`, `cell-has-oracle`, `error-enum-cites-req`, `error-message-cites-req`, `file-length`, `go-cell-isolation`, `go-unsafe-in-domain`, `no-unwrap-in-domain`, `pub-doctest`, `seam-has-doctest`, `ts-cell-isolation`, `ts-unsafe-in-domain`, `unsafe-gate`. None concerns prose phrasing; all are code-shape rules.
- `vibe check`'s roster is `crates/vibe-check/src/lib.rs:134-148` — `ManifestValidity`, `WalFreshness`, `WalWellformed`, `BootDirectory`, `LockfileFiles`, `ReviewAging`, `FeaturesGraph`, `SubskillStructure`, `I18nCoverage`, `ActivationConflict`, `RedirectBlock` (plus `BootGraphIntegrity`, wired externally per the comment at `:131-133`). No phrasing lint, and no spec-unit-length warn either — which is the same gap `#SPEC-PRINCIPLE-UNITS-FIT-A-PAGE` (:272) already records.
- `grep -i 'imperative\|second.person\|phrasing'` over `crates/vibe-check/src` and the conform rules returns one unrelated comment (`crates/vibe-check/src/checks/activation_conflict.rs:36`).
- **The carve-out the lint requires has no data.** The clause exempts `guide`-kind units; measured in A2, **0 of 5266** host spec units carry a `kind`. A lint keyed on "outside `guide` kind" would today treat the entire corpus as non-guide.

### 2. What would have to be built

**(a) Signing — the largest single piece in part A.**

- A scheme decision first (`#OPEN-SIGNING-SCHEME`), which determines everything downstream: key custody, trust roots, revocation, offline verification, and whether CI can sign.
- **What** is signed is also undecided in a load-bearing way. §2.8.4(a) says *"the shipped index and fragments"*; but the artefact that actually travels is a whole package tree already identified by `content_hash` (`crates/vibe-core/src/content_hash.rs:34`, computed at `crates/vibe-index/src/content_hash.rs:40`). Signing the existing tree hash is a smaller change with wider effect; signing the index alone requires A1 first and leaves everything else unsigned.
- Surfaces: `crates/vibe-publish/` (produce), `crates/vibe-registry/` + `crates/vibe-install/` (verify at fetch — the natural point is beside `fetch_with_expected_hash`, `fetch.rs:274`), `crates/vibe-core/` (a signature type and probably a lockfile field, i.e. a **lockfile schema change**), plus a new dependency outside the current tree (nothing crypto beyond `sha2` is vendored).
- Rule 4 note: this is CI / signing / secrets territory — non-routine by `vibevm/vibespecs/boot/00-core.xml:24` and `vibevm/vibespecs/boot/STATIC.xml:619`, `:849`, `:1195-1196`. It stops for the owner before it is started, delegated or not.

**(b) Framing — small in code, a policy decision in substance.**

- Mechanically: text in `ToolDescriptor.description` for the traceability tools (the three stack servers' `tools_discipline.rs`, and any new host tool). Each server's descriptions are already long, structured, and agent-directed, so the pattern exists.
- The decision is which tools get which framing, given that `agentic_explain` deliberately says the opposite, and given that MCP descriptions are advisory — nothing in `core-ai-native-mcp`'s loop (`server.rs`) or in `ToolOutput::into_result_value` (`toolset.rs:64`) can make a client honour a framing line.

**(c) Lint — a new rule in an established pattern, blocked on data.**

- Either a conform rule (the Class-F pattern at `core-ai-native-conform/src/rules/diagnostics.rs:228-282` is the template: a `Rule` impl with `id`, `why`, `check`, emitting `Finding`s through `req_message`, ratcheted through `conform-baseline.json`) or a `vibe-check` `CheckId` (`crates/vibe-check/src/lib.rs:75`, `:112-127`, `:134`).
- Needs a detector for imperative second person in prose — a heuristic with a false-positive rate, which is why the ratchet matters — and needs the `kind` carve-out to have data (see above).

### 3. Dependencies

- **A5 gates A4** by the PROP's own three statements: `#RUNTIME-SECURITY-IS-NON-OPTIONAL` (:242, *"the trust layer ships **with** the runtime channel, not after it"*), `#INVARIANT-RUNTIME-CHANNEL-SHIPS-SIGNED` (:297), `#PHASE-4-MCP-TOOLS-BLOCKED-ON-SIGNING` (:369, *"blocked on the signing decision (§7.6); ships signed or not at all"*).
- **(a) is blocked on an undecided external question** — `#OPEN-SIGNING-SCHEME` (:415), which the document itself marks as blocking for §2.8.
- **(a) depends on A1** if signing is scoped to "the index and fragments" — there is nothing to sign until an index ships; it does **not** depend on A1 if scoped to the package tree.
- **(b) depends on A4** for the traceability tools specifically (no tools, no descriptions), but is independent for the tools that already ship (`read_subskill`).
- **(c) depends on kind-line data**, the same corpus gap that blocks A2's revision half.
- The three clauses are independent of each other: framing and the lint need no signing scheme.

### 4. Effort class

- **(a) signing: L.** A scheme decision, a new crypto dependency, produce+verify paths across three crates, a likely lockfile schema change, key custody — and it is a Rule-4 red-line topic that stops for the owner before work starts.
- **(b) framing: S.** Description-string edits in an existing pattern; the cost is the policy call, not the code.
- **(c) lint: M.** One rule in a well-worn pattern, plus a prose heuristic that needs tuning against 5266 real units, plus the `kind` data gap.

### 5. Observations on warrant

- **The threat the clause names is not hypothetical here, and it is not confined to the unbuilt channel.** Installed-package prose already reaches an agent's context by two shipped routes: `read_subskill` (`tools.rs:158`) and `[boot_snippet]` (`package.rs:500`, read at session start). Neither is signed and neither is framed. So the exposure exists whether or not A4 is built; A4 would widen it.
- **The house style is currently the opposite of clause (b) where it has taken a position.** `agentic_explain` (`tools.rs:429`) instructs the consuming agent to follow the returned text as authoritative. Any framing rule would have to say why traceability output differs from that.
- **The integrity guarantee the repository already relies on is pin-based, not identity-based.** `fetch_with_expected_hash` (`fetch.rs:274`) protects against a mirror serving different bytes than the lockfile recorded; it does not answer "is this the publisher's content", which is the question §2.8.4(a) asks.
- **Clause (c)'s carve-out has no data to carve out.** 0/5266 host units carry `kind`, so the lint's scope is presently undefined on the host corpus, even though the parser supports the marker (`mdspec.rs:63-84`) and other projects populate it (`research/go-demo/specmap.json`: 6 of 7 units).
- **The document's marker choice for :242 is itself the evidence.** The annotation moves the marker rather than the text precisely because the position ("ships signed or not at all") is held while the mechanism is unbuilt — so the unbuilt state of A5 is currently doing the work of *preventing* A4, not merely lagging it.

---

## Cross-cutting notes

- **Measurement commands used** (all read-only): `git log --oneline -1`; `find . -name 'vibe.toml' -not -path './target/*' -not -path './legacy-spec/*' | xargs grep -n 'specmap\.json\|metamodel'`; `grep -rn --include='*.rs' -o 'violates spec://' crates/ | wc -l`; a Python read of `specmap.json` / `research/{rust,go}-demo/specmap.json` / `vibevm/vibepacks/org.vibevm.fractality/fractality/v0.1.0/specmap.json` counting units, kinds, revisions, pins, verbs, suspects, warnings; a Python extraction of `violates spec://…#anchor` URIs from `crates/**/*.rs` intersected against `spec_units[].uri`. No build, no test run, no `vibe` command, no writes outside this file.
- **One incidental drift noticed while reading, not acted on:** `schemas/specmap.jtd.json` still names `crates/specmap-core/src/generated/specmap/` in its description and `specmap_core::specmap` in `metadata.rustOptions.package`, a path that no longer exists after the engine relocated into `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/`. Anyone bumping the index schema (A1 half 2, A3's `contract` profile) meets this first.
