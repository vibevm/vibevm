# D14 · B-012 feasibility harvest — part B (graph / check / explain family)

**Date:** 2026-08-01
**HEAD:** `ed0abbab docs(campaign): волна 10 closes the D13 seal tail in the LOG`
**Subject spec:** `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md` (read in full)

**Nature of this document.** Evidence only, for the owner's B-012 feasibility
question ("can it be built"). No verdicts, no build/don't-build recommendation,
no spec edits. Every claim carries a `file:line` or the exact command +
perimeter that produced it.

**Default search perimeter** (used for every absence claim unless a section
widens it):

- `crates/` (host crates)
- `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/` (engine crates)
- `xtask/`
- `tools/`
- `schemas/`

Excluded always: `legacy-spec/**`. Not evidence: `campaigns/**`, `refs/**`.
`vibedeps/flow-core-ai-native/0.8.0/crates/` is the vendored copy of the engine
crates and is cited only to show what the *host build* actually links.

**Sections:** B1 multiplicity lint · B2 `content_hash` + derived node views ·
B3 LLM prose producer · B4 spec-unit length warning · B5 rustdoc composition in
`explain` · B6 (rider) `decides` verb.

---

## B1 — Per-item edge-multiplicity lint in `vibe check`

Anchor: `##RULE-MULTIPLICITY-LINT`, PROP-014 line 190. Annotation: *"Specified,
not built: no checker in any layer counts edges per item; `vibe check`'s checks
do not include a multiplicity lint."*

### 1. What exists today

**The roster the annotation names.** `crates/vibe-check/src/lib.rs`:

- `CheckId` enum — `:75`–`:110`, twelve ids: `ManifestValidity`, `WalFreshness`,
  `WalWellformed`, `BootDirectory`, `LockfileFiles`, `ReviewAging`,
  `FeaturesGraph`, `SubskillStructure`, `I18nCoverage`, `ActivationConflict`,
  `RedirectBlock`, `BootGraphIntegrity`. No multiplicity/edge id among them.
- `CheckId::all()` — `:134`–`:148`, eleven **cell-backed** ids;
  `BootGraphIntegrity` is excluded by design (`:130`–`:133`) because it is wired
  outside the cell registry, at `crates/vibe-cli/src/commands/check.rs:195`.
- `all_checks()` — `:358`–`:372`, the single registration point; its own
  doctest pins the count: `assert_eq!(checks.len(), 11)` (`:355`).
- `check_project()` — `:395`, iterates that list verbatim; a test pins the
  dispatch order as observable output (`:509`–`:529`).
- Cell layout: one file per check under `crates/vibe-check/src/checks/`
  (`activation_conflict.rs`, `boot_directory.rs`, `features_graph.rs`,
  `i18n_coverage.rs`, `lockfile_files.rs`, `manifest_validity.rs`,
  `redirect_block.rs`, `review_aging.rs`, `subskill_structure.rs`,
  `wal_freshness.rs`, `wal_wellformed.rs`), registered once in
  `crates/vibe-check/src/checks/mod.rs:11`–`:21`, re-exported `:23`–`:33`.
  A cell "imports the seam and shared core … never a sibling cell"
  (`checks/mod.rs:5`–`:7`).

**`vibe check` cannot see the graph today.** `crates/vibe-check/Cargo.toml`
depends on `specmark`, `vibe-core`, `vibe-resolver`, `anyhow`, `thiserror`,
`walkdir` — **no `specmap-core`, no `specmap.json` reader**. The `Check` seam
signature is `fn run(&self, project_root: &Path, opts: &CheckOptions, report:
&mut CheckReport)` (`lib.rs:343`) — a project path only; nothing in it carries an
index.

**The other candidate home already runs.** The specmap driver
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-specmap/src/lib.rs:22`
(`run_specmap`) builds the index and then runs `run_ratchet_gate` (`:64`), which
already walks the built `Specmap` and emits per-symbol findings to stderr
(`:74`–`:86`), blocking under `--check` (`:94`). The host reaches it through the
two-line shim `xtask/src/specmap.rs:11`–`:13`.

**The data the lint needs is already in the index and already grouped once.**
`Edge.fromSymbol` — `…/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated/specmap/mod.rs:91`–`:92`;
`ratchet.rs:54` already builds exactly the grouping key
(`let tagged: HashSet<&str> = map.edges.iter().map(|e| e.fromSymbol.as_str())`)
— a `HashSet` where the lint wants a counting map.

**Absence check.** `rg -n -i "multiplicity|edges_per_item|edge_count|fan.?out"`
over `crates/ packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/ xtask/
tools/ schemas/` returns **no traceability hit**: the only matches are
`xtask/src/mirror.rs` (git mirror fan-out), `crates/vibe-settings/src/events/mod.rs:238`
(event fan-out), `crates/vibe-resolver/src/lib.rs:213`, `features.rs:262`,
`crates/vibe-cli/src/commands/workspace/mod.rs:30`, `aiui/cdp.rs:26`. Confirmed:
nothing counts edges per item anywhere in the perimeter.

**Measured state of the graph the lint would police** (committed
`specmap.json` at HEAD, 898 code items / 912 edges):

| edges on one item | items |
|---|---|
| 1 | 880 |
| 2 | 16 |
| ≥3 | **0** |

Max fan-out **2**. Verbs: 677 `implements`, 223 `verifies`, 12 `deviates`;
provenance 912/912 `authored`. Zero code items carry no edge (the index only
admits tagged items — `rscan.rs:128`–`:131` records an item only when its edge
list is non-empty; the schema says so at `generated/specmap/mod.rs:31`–`:32`).
*(These are properties of the committed artefact; see the freshness caveat under
B4 §1 — the code half of the index checks out against the tree, the spec half
has drifted.)*

### 2. What would have to be built

Two mutually exclusive placements, and the choice is the real decision:

**(a) In the specmap engine, beside the orphan ratchet** (the cheap path).
Touch `…/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/ratchet.rs`
(or a new sibling `lint.rs`): count `map.edges` by `fromSymbol`, compare against
a threshold, emit findings the way `Orphan` is emitted. New surface: one
threshold field on `Config` (`…/src/config.rs`, the `specmap.toml` parser — PROP-014
§7.5 calls 3 a placeholder, so it must be configurable), one struct mirroring
`Orphan` (`ratchet.rs:38`–`:48`), one reporting loop in `run_specmap`'s
`run_ratchet_gate`. No schema change: the finding is computed at gate time, the
same posture the orphan table already takes ("computed at gate time and
deliberately not serialised", `ratchet.rs:22`–`:24`, echoed at `index.rs:11`).
Edit propagation: authored copy under `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/`,
then `cargo xtask sync-engines` writes through the vendored slots
(`xtask/src/sync_engines.rs:293`; the host builds the vendored
`rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-specmap`, per
`Cargo.toml:102`, and that copy is byte-identical to the authored one today —
verified `diff -rq`, empty).

**(b) In `vibe check`, where the anchor literally says.** New `CheckId` variant
+ `as_str` arm + `all()` entry (`lib.rs:75`, `:113`, `:134`), a new cell file
under `checks/`, registration in `checks/mod.rs` and `all_checks()`, and two
pinned tests to update (`lib.rs:355` doctest count 11→12, `:509` order test).
The load-bearing cost is not the boilerplate: `vibe-check` would need a new
dependency on `specmap-core` (or on a `specmap.json` reader), which crosses the
separability seam the codebase states out loud — "specmap owns its own data
model in `specmap-core`, so the traceability engine … can relocate without a
`specmap-core → vibe-wire` edge" (`xtask/src/codegen.rs:45`–`:47`;
`…/specmap/src/lib.rs:24`–`:27`). Alternatively `vibe check` reads the committed
`specmap.json` as opaque JSON — cheap, but then the host lint drifts from the
engine's schema with no compiler to catch it.

### 3. Dependencies

- **Independent of B2–B6.** Needs only `Edge.fromSymbol`, which ships.
- Depends on a **threshold decision** — PROP-014 `#OPEN-THRESHOLDS` (line 414)
  calls 3 edges/item a placeholder "until Phase 3 metrics", and
  `#PHASE-3-INSTRUMENT-THE-ECONOMICS` (line 363) is the phase that would set it.
  That phase has not run.
- Placement (a) vs (b) is a **seam decision** (engine gate vs host linter), not
  a technical blocker either way.

### 4. Effort class

**S** — the counting pass is a dozen lines over data already in memory, and one
of the two homes (the ratchet) already has the reporting loop, the config file
and the blocking behaviour; almost all remaining cost is choosing where it lives
and adding a configurable threshold.

### 5. Observations on worth (no recommendation)

- **No consumer today, and nothing to report.** At the measured distribution the
  lint fires on **zero** items; the threshold is 3 and the observed maximum is 2.
  It would ship green and stay green until the corpus changes shape.
- The PROP itself frames the value as *preventive*, not diagnostic —
  `#SYSTEM-REPRESENTS-MANY-TO-MANY-AND-LINTS-ITS-GROWTH` (line 37) says the
  fan-out "can rise without a signal". That is a tripwire argument, and it is
  weakened *and* strengthened by the same number: nothing to clean up today, so
  installing it is cheap; nothing to catch today, so its value is entirely
  future-dated.
- The tree already contains the twin lint this one is said to mirror —
  `ActivationConflictCheck` (`lib.rs:96`–`:100`, "Mirrors Tessl's review-rubric
  'activation distinctiveness' axis"), a threshold-driven pairwise check with a
  configurable figure — so the shape has precedent in `vibe check`.
- Counter-signal on the `vibe check` placement: the host linter has stayed free
  of the traceability engine on purpose (dependency list above), and the specmap
  gate already blocks CI on a per-symbol graph property (orphans). A second
  graph property in a different binary would split the graph gate across two
  tools.

---

## B2 — `CodeItem.content_hash` + derived `Command` / `ErrorVariant` node views

Anchor: `##EDGE-MODEL-NODES`, PROP-014 line 198. Annotation: *"Specified, not
built: `CodeItem` carries no content hash …, and there are no derived `Command`
or `ErrorVariant` node views. (`ErrorVariant` exists as a conform **fact** —
`conform/src/facts.rs:66` — which is a different graph.)"* Three separable
sub-mechanisms; taken in order.

### 1. What exists today

**`CodeItem` is exactly five fields, twice over.**

- Wire type: `…/core-ai-native-specmap/src/generated/specmap/mod.rs:34`–`:52` —
  `crateName`, `file`, `itemKind`, `line`, `symbol`. Struct doc `:31`–`:32`:
  "Only items carrying at least one edge appear; the full-orphan inventory is a
  later-phase table."
- Schema: `schemas/specmap.jtd.json`, definition `code_item` — same five
  properties, no optional block.
- Serialised reality: every one of the 898 `code_items` in the committed
  `specmap.json` carries exactly `['crate_name','file','item_kind','line','symbol']`
  (measured over the committed file at HEAD).
- Construction: `rscan.rs:45`–`:53` (`record_item`) — the only site that mints a
  `CodeItem`, and it is only reached when the item carries at least one specmark
  edge (`rscan.rs:126`–`:135`, `tag_item`: `if edges.is_empty() { return; }`).

**Nothing hashes code.** `content_hash` (`specmap/src/lib.rs:54`) has exactly
four call sites in the engine: `mdspec.rs:287` and `mdspec.rs:450` (the two
markdown span hashes — fact units and heading units), and `ledger.rs:61` /
`:74` / `:136` (epoch inputs and the prose cache key). No call takes Rust source.
The PROP's own §2.2 note (line 141) states the consequence: "a code change is
invisible to the edge and cannot invalidate it".

**A per-file code hash does already exist — in the other engine.**
`core-ai-native-conform` has its own `content_hash` (`conform/src/store.rs:182`,
re-exported `conform/src/lib.rs:35`), applied per source file at
`conform/src/store.rs:145` to key the fact cache slot (`store.rs:83`–`:87`).
File grain, not item grain, and it feeds conform's store, not the specmap index.

**`Command` node view: absent.** `rg -n "Command"` over
`…/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/` and
`schemas/specmap.jtd.json` returns exactly one hit —
`ledger.rs:67 std::process::Command::new("rustc")`. No node kind, no derived
view, no schema definition. The engine has no notion of a CLI command at all.

**`ErrorVariant`: exists, but in the conform graph.**
`…/core-ai-native-conform/src/facts.rs:66`–`:73` —
`Fact::ErrorVariant { enum_symbol, variant, message, line, enum_attrs }`,
documented `:64`–`:65` as "A `#[error(\"...\")]`-carrying enum variant (thiserror)
… the Class-F diagnostics signal", with `enum_attrs` explicitly "Attributes of
the OWNING enum (**where the REQ edge lives**)" (`:71`). Two rules consume it:
`error-message-cites-req` (`conform/src/rules/diagnostics.rs:235`, findings
fingerprinted `:275`) and `error-enum-cites-req` (`:314`, `:359`). So the
extraction a specmap `ErrorVariant` view would need already runs — into a store
keyed by `(file content-hash, producer)` (`specmap/src/ledger.rs:2`–`:5` states
the split), consumed by a different rule engine, never entering `specmap.json`.

**The index's other absent tables, for context** (PROP-014 line 210 already
records this): the committed `specmap.json` has six keys — `code_items`,
`edges`, `schema`, `spec_units`, `suspects`, `warnings` (verified at HEAD:
5266 / 898 / 912 / 0 / 265). No coverage table, no orphan table. Orphans are
deliberately gate-time-only (`ratchet.rs:22`–`:24`).

### 2. What would have to be built

**(a) `CodeItem.content_hash`.** Scanner work is small: `rscan.rs` already holds
the `syn` item at `walk_items` (`:137`) and already reads `.span().start().line`
(`:139`) — the end line is the symmetric call, so slicing the item's source text
and hashing it with the existing `crate::content_hash` is a handful of lines in
`record_item` (`:45`). Two real questions it forces:

  - *What is hashed* — token stream, or source text including doc comments and
    formatting? A text hash makes every `cargo fmt` and every doc-comment typo a
    code-hash change; a token hash needs a canonicalisation decision. Nothing in
    the tree settles this today.
  - *What consumes it.* PROP-014's §2.2 exception (line 141) is the stated
    consumer: `deviates` edges flip to *review* when **either** side changes. To
    implement that, the hash must be compared against a **previous** value, and
    the only place a previous index is read is `index.rs:289` (`load_committed`)
    → `classify_drift` (`:198`), which today compares spec-unit hashes only
    (`:200`–`:223`). So this is: new field → new drift arm in `classify_drift`
    → new report line, in the same shape as the existing `unbumped-hash:` arm
    (`:235`–`:239`).

**Serialised-index impact — the load-bearing part.** Adding one field to
`CodeItem`:

  - bumps `pub const SCHEMA: u32 = 2` (`index.rs:29`; its meaning is documented
    `:27`–`:28`) → 3;
  - rewrites **all 898** `code_items` entries in the committed
    `specmap.json`, i.e. the whole artefact churns in one commit, and
    `index::check` byte-compares (`index.rs:344`) so the regeneration must land
    in the same commit as the code;
  - must go through the JTD codegen, and **that route is stale**:
    `xtask/src/codegen.rs:48`–`:55` routes the `specmap` schema's output to
    `packages/org.vibevm.ai-native/rust-ai-native-lang/**v0.5.0**/crates/specmap-core/src/generated`
    — a path that does not exist (the only slot under `rust-ai-native-lang/` is
    `v0.7.0`; `ls` of the v0.5.0 path fails). The same stale path is repeated
    for the drift check at `codegen.rs:215`. Meanwhile the engine's own header
    says the source of truth is "the package's `schemas/specmap.jtd.json`
    (regeneration is a maintainer dev-op in the package's dev repo)"
    (`specmap/src/lib.rs:24`–`:27`) — and **no such file exists in this
    repository**: a glob for `**/specmap.jtd.json` excluding `legacy-spec/`
    returns the single host copy `schemas/specmap.jtd.json`. So the schema-bump
    path itself needs repairing (or replacing with a hand-edit of the checked-in
    generated module) before the field can land cleanly;
  - propagates through `cargo xtask sync-engines` (`xtask/src/sync_engines.rs:293`,
    manifest `sync-engines.toml`) to the vendored copies. Under
    `packages/org.vibevm.ai-native/**` the specmap engine exists in **8** slots
    (authored `core-ai-native/v0.8.0`, a legacy `core-ai-native/v0.7.0`, and
    vendored copies in `rust-ai-native-lang/v0.7.0`, `rust-ai-native-mcp/v0.7.0`,
    `typescript-ai-native-lang/v0.6.0`, `typescript-ai-native-mcp/v0.6.0`,
    `go-ai-native-lang/v0.1.0`, `go-ai-native-mcp/v0.1.0`), plus regenerated
    `vibedeps/**` and `.vibe/cache/**` copies. The host builds the vendored
    `rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-specmap`
    (`Cargo.toml:102`), byte-identical to the authored copy today (`diff -rq`,
    empty).

**(b) Derived `Command` node view.** Greenfield in every layer: the scanner has
no concept of a CLI command; there is no clap-surface extractor, and the host CLI
is `crates/vibe-cli` while the engine scans by `specmap.toml`'s
`scan_roots = ["crates/*", "xtask"]`. Building it means: decide what a command
*is* (a clap `Subcommand` variant? a `Cmd::` arm? a `#[spec]`-tagged handler
fn?), add an extractor, add a node kind + schema definition + generated type,
and teach `explain` to resolve `vibe install` as a target the way it resolves
symbols and URIs today (`explain.rs:199`–`:205` dispatches on the `spec://`
prefix only). This is the one sub-mechanism with **no existing half**.

**(c) Derived `ErrorVariant` node view.** Two shapes, materially different in
cost:

  - *Re-extract inside specmap*: `rscan.rs` learns thiserror `#[error(...)]`
    variants — duplicating what `conform`'s frontend already does, and the two
    engines are deliberately separable (the ledger header states the split,
    `specmap/src/ledger.rs:2`–`:5`; codegen states specmap "owns its own data
    model", `codegen.rs:45`–`:47`).
  - *Join at query time*: leave the fact in conform and have the runtime channel
    compose. That has no home today either — the channel itself is unbuilt
    (PROP-014 line 56).

### 3. Dependencies

- **(a) is a hard prerequisite for the §2.2 `deviates`-exception behaviour**
  (line 141) — that rule cannot exist without a code-side hash. 12 `deviates`
  edges exist in the index today, so the rule would have a live population.
- **(a) is *not* required by B5.** Checked: B5 needs a *doc* field on
  `CodeItem`, an independent addition; both are "add a field, bump the schema",
  and if both are wanted they should share **one** schema bump rather than two.
- **(c) depends on the conform↔specmap seam decision**, and (b)+(c) are both
  named by `#RUNTIME-TRANSPORT` (line 239) / `#RUNTIME-EXPOSES-THE-METAMODEL-TO-CONSUMERS`
  (line 56) as the things a runtime channel would serve — a channel this PROP
  itself blocks on signing (`#OPEN-SIGNING-SCHEME`, line 415;
  `#PHASE-4-MCP-TOOLS-BLOCKED-ON-SIGNING`, line 369).
- **(a) additionally depends on repairing the codegen route** (`codegen.rs:50`–`:52`)
  or on a decision to hand-maintain the generated module.

### 4. Effort class

- **(a) `content_hash` on `CodeItem`: M** — the scanner change is hours, but the
  schema bump drags a stale codegen route, a full-index rewrite of 898 entries,
  an eight-slot vendor propagation, and a "what exactly is hashed" decision that
  determines whether the field is noisy or useful.
- **(b) `Command` view: L** — no existing half in any layer; needs a new
  extractor, a new node kind through schema + codegen + explain resolution, and
  a definition of "command" that does not exist anywhere in the tree.
- **(c) `ErrorVariant` view: M** — the extraction exists in conform and would
  either be duplicated (cheap code, expensive seam) or joined (cheap seam,
  no channel to join in).

### 5. Observations on worth (no recommendation)

- **(a) has a named, currently-dead consumer**: the §2.2 `deviates` exception is
  written as a rule and is unimplementable without it; PROP-014 line 141 says so
  in its own words ("the stated behaviour is what the data model can do rather
  than a decision it enforces"). 12 `deviates` edges would come under it.
- Against (a): the index's other honest asymmetry is that **all 5266 spec units
  in the host tree carry no `kind` and no `revision`** (measured: `units with
  kind: 0`, `units with revision: 0`), so the spec-side hash — which *is* built —
  currently drives nothing either: `suspects` is 0 despite 201 edges carrying a
  pin, because no target unit declares a revision to be stale against. Adding a
  code-side hash extends a mechanism whose spec-side half is not yet exercised in
  this repository.
- **(c) is the one with a live consumer story**: error provenance is called "the
  single highest-leverage consumer" by the PROP (line 226), and it half-works
  today through conform's two rules — but through compile-time constants, not an
  index lookup (line 226's own annotation).
- **(b) has no consumer anywhere in the tree.** Nothing asks the index about a
  command; the only surface that would (`explain`) accepts a symbol or a URI
  (`explain.rs:199`–`:205`), and the runtime channel that motivates it is blocked
  on an undecided signing scheme.

---

## B3 — LLM prose producer behind `vibe explain --prose`

Anchor: `##LLM-AS-RENDERER`, PROP-014 line 234. Annotation: *"Specified, not
built: the prose producer is a deterministic template (`ledger.rs:168`) and the
crate's own header says an LLM producer slots in later."*

### 1. What exists today

**The producer, and the slot it leaves.**
`…/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/ledger.rs`:

- `prose_explain(root, map, target)` — `:131`, the served entry point.
- `const PRODUCER: &str = "explain.item/prose-template-1"` — `:132`. The
  producer id is already a first-class part of the cache key, which is exactly
  the seam a second producer would slot into.
- Cache key: `content_hash(format!("{PRODUCER}\n{epoch}\n{subject}"))` — `:136`,
  where `subject` is the serialised `explain_json` subgraph (`:133`–`:134`).
  Storage `.ledger/objects/<sha[0..2]>/<sha>` (`:119`–`:124`); telemetry
  `.ledger/telemetry.json` (`:90`–`:92`, `Telemetry` `:83`–`:88`).
- The template itself: `render_prose` — `:168`–`:223`; walks the subgraph's
  `edges` and `units` arrays and emits one bullet per edge plus a mandatory
  provenance line (`:213`–`:221`).
- Header, verbatim on the slot (`:9`–`:12`): "One query kind ships:
  `explain.item` … The producer is a deterministic template (the tool MUST be
  fully useful without an LLM; **an LLM prose producer slots in later under its
  own producer id + model id**)."
- Tests already pin the caching contract the LLM producer would inherit:
  `:261` (second identical call is a hit), `:279` (epoch change invalidates),
  `:298` (epoch stable).

**The command surface — and a naming discrepancy worth recording.** The anchor
says `vibe explain --prose`. What ships is `trace explain --prose`:
`xtask/src/main.rs:310`–`:314` declares the flag (doc: "Prose render through the
local ledger (LEDGER §6 query kind 2): template producer, epoch-keyed cache under
`.ledger/`, provenance line on every render"), dispatched at `:335`–`:343` to
`run_trace_explain`
(`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/trace.rs:8`,
prose branch `:13`–`:27`). **`vibe explain` does not exist**: `rg "\bexplain\b"`
over `crates/vibe-cli/src/cli/*.rs` and `crates/vibe-cli/src/main.rs` returns a
single hit — `crates/vibe-cli/src/cli/agentic.rs:23`, which is PROP-018's
"explain this project" relay op, a different mechanism (PROP-014 line 56 makes
the same distinction itself). Promotion to `vibe explain` is PROP-014's own
Phase 4 item (`#PHASE-4-PROMOTE-XTASK-TO-VIBE`, line 367), not done.

**`crates/vibe-llm` — ventilated as asked. It is empty.** The whole crate is two
files:

- `crates/vibe-llm/Cargo.toml` — description "LLM provider abstraction for
  vibevm (M1.5+); **stub in M0**"; dependencies `anyhow`, `thiserror` only.
- `crates/vibe-llm/src/lib.rs` — **9 lines**: a module doc reading "**STATUS: M0
  stub.** Concrete providers (Anthropic, OpenAI, OpenRouter, Ollama) land in the
  v1.5 LLM milestone per `VIBEVM-SPEC.md` §10.4; this crate is a deliberate
  placeholder until then, not forgotten work." (`:3`–`:5`), a spec pointer
  (`:7`), and `#![forbid(unsafe_code)]` (`:9`). **Zero items** — no trait, no
  struct, no function.
- **Nothing depends on it**: `rg "vibe-llm" crates/*/Cargo.toml
  xtask/Cargo.toml` returns only `crates/vibe-llm/Cargo.toml:2` (its own name).
  It is a workspace member and a declared workspace dep only
  (`Cargo.toml:19`, `:40`, `:73`).
- It is exempt from both gates *because* it is empty — `specmap.toml` exempt
  list, comment "`vibe-graph`, `vibe-llm` — M0 stub crates, no code yet".
- Prose references to a future it: `crates/vibe-resolver/src/activation.rs:14`,
  `crates/vibe-mcp/src/tools.rs:9`, `crates/vibe-mcp/src/agentic.rs:13`, `:85`,
  `:237` — all conditional ("once `vibe-llm` is real", "far backlog §6").

**`VIBEVM-SPEC.md` §10.4 (`:1137`–`:1176`) is the design that crate is holding a
place for**: an `LLMProvider` trait with `chat`, `chat_with_tools`, and
`stream_chat` (v2) (`:1145`–`:1148`); four providers — Anthropic Messages API,
OpenAI Chat Completions, OpenRouter, Ollama (`:1140`–`:1143`); a tool-use loop
pseudocode (`:1152`–`:1174`); and a sandboxing requirement, "file operations …
scoped to the project root. No path traversal." (`:1176`). Scoped "for v1.5".
No provider client is a dependency anywhere: `rg -i
"anthropic|openai|openrouter|ollama"` over the root `Cargo.toml`, every
`crates/*/Cargo.toml`, and the engine crates' manifests returns **zero** hits
(the only network dep is generic `reqwest`, used by `vibe-registry`, `vibe-cli`,
`vibe-index`, `vibe-publish`).

**The *other* inference path is built and shipping.** PROP-018's agentic relay
(`crates/vibe-mcp/src/agentic.rs`): "vibevm composes a domain-grounded
instruction and the calling agent executes it" (`:1`–`:2`), explicitly "a
deliberate division of labour, not a fallback" (`:8`). `ActiveBackend`
(`:81`–`:89`) enumerates `Relay` / `Builtin` / `None`, where `Builtin` is
documented "Standalone with the built-in `vibe-llm` engine (**far backlog §6**)"
(`:85`). `Affinity` (`:62`–`:69`) and `check_affinity` route an op to the backend
it needs, with a refusal message that names the missing one (`:100`–`:107`).

### 2. What would have to be built

Two shapes, and the choice decides everything downstream.

**(a) LLM producer as a second ledger producer (relay-shaped).** Touch
`…/core-ai-native-specmap/src/ledger.rs` only: a second `PRODUCER` id (e.g.
`explain.item/llm-1`, plus the model id — `#OPEN-EXPLANATION-CACHING`, line 413,
says renderings are keyed by "(subgraph hash, model id)"; today's key carries
producer + epoch + subgraph at `:136` and has **no model-id component**), a
producer trait or enum so `prose_explain` can dispatch, and a caller-supplied
render function so the engine never links a provider. Surfaces: `ledger.rs`
(dispatch + key), `trace.rs:8` (a flag to select the producer),
`xtask/src/main.rs:310` (the flag), and whatever composes the prompt. The engine
stays LLM-free, which is what `#INVARIANT-TOOL-IS-FUNCTIONAL-WITHOUT-AN-LLM`
(line 296) requires.

**(b) LLM producer via an in-process provider (`vibe-llm`-shaped).** Everything
in (a), plus building `vibe-llm` from zero to the §10.4 design: the
`LLMProvider` trait, at least one provider client, HTTP + auth, API-key
sourcing/secret handling, error surface, retry/timeouts, and removing the crate
from the `specmap.toml` / `conform.toml` exempt lists once it has a public
surface (it is exempt *because* it is empty). The engine crate would then need
either a dependency on a provider (crossing the separability seam) or the same
injected-render-function seam as (a) — i.e. (b) does not remove (a)'s work, it
adds to it.

Common to both: a **non-determinism policy**. Everything else in this engine is
a determinism contract (`index.rs:457` `index_is_deterministic`; the whole
`--check` byte-compare at `index.rs:344`). The prose path already sidesteps it by
caching per epoch, but a live model makes two runs of the same command differ,
so where the render may and may not be invoked has to be stated. The PROP fixes
the direction — `--prose` is "a presentation layer, never the data layer"
(line 225) — but not the mechanics.

### 3. Dependencies

- **Independent of B1, B2, B4, B6.** Consumes only `explain_json`'s output
  (`explain.rs:209`), which ships.
- **B5 is upstream in substance**: the anchor's own sentence says `--prose`
  feeds "spec unit texts + **rustdoc of linked items** + deviation reasons" to
  the provider (line 234). The subgraph it actually gets today has no rustdoc
  (see B5) and no unit *body text* either — `explain_json` emits unit
  `uri`/`heading`/`revision`/`content_hash`/`file`/`line` (`explain.rs:265`–`:269`),
  not the unit's prose. So an LLM producer over today's subgraph would be
  rendering identifiers, not content.
- Depends on `#OPEN-EXPLANATION-CACHING` (line 413) for the model-id key
  component, and on an owner decision about provider credentials (an API key in
  the tool's runtime is a secrets question, outside this document's scope).

### 4. Effort class

- **(a) relay-shaped producer: M** — the producer-id seam, the cache and the
  telemetry all exist; the work is a dispatch point, a model-id key component, a
  prompt, and the non-determinism policy. Days.
- **(b) with a real `vibe-llm`: L** — an empty crate to a working provider
  abstraction is the §10.4 milestone in full (trait, ≥1 client, auth, errors,
  tests), and it does not subsume (a).

### 5. Observations on worth (no recommendation)

- **The consumer exists and is thin.** `--prose` is wired end to end today
  (flag → `run_trace_explain` → ledger → cache → provenance line), so an LLM
  producer would land in a working pipeline rather than build one.
- **The input is currently too thin to render from.** As noted above, the
  subgraph carries no unit body and no rustdoc; the deterministic template
  therefore emits a bullet list of edges (`ledger.rs:189`) and a unit heading
  line (`:210`). An LLM given that same JSON has nothing more to say than the
  template already says — which is a fact about ordering, not about worth.
- **The project's own bet on inference is elsewhere.** `agentic.rs:8` calls the
  relay "a deliberate division of labour, not a fallback", and the built-in
  engine is labelled "far backlog" in the same file (`:85`). A `vibe-llm`-shaped
  answer to this anchor argues against that posture; a relay-shaped answer runs
  with it.
- **The invariant is explicit and would bind either shape**:
  `#INVARIANT-TOOL-IS-FUNCTIONAL-WITHOUT-AN-LLM` (line 296) and line 225's "The
  tool MUST be fully useful without an LLM" — so whatever ships, the template
  producer stays.
- Counter-signal on urgency: `--prose`'s cache telemetry is plumbed but the
  crate notes no re-verification path runs yet ("none do yet — the template
  producer recomputes from scratch, cost ~0", `ledger.rs:79`–`:80`). Nothing
  today is paying the cost the cache exists to avoid.

---

## B4 — Spec-unit length warning (≤ 120 lines) in `vibe check`

Anchor: `##SPEC-PRINCIPLE-UNITS-FIT-A-PAGE`, PROP-014 line 272. Annotation:
*"Specified, not built: no checker warns on spec-unit length; the 120-line
figure is a target with no enforcement."*

### 1. What exists today

**Absence, with the perimeter named.** `rg -n "\b120\b"` over `crates/`,
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/`, `xtask/`,
`tools/`, `schemas/` returns exactly three hits, none of them a rule:
`…/core-ai-native-conform/src/rules/budget.rs:210` (a line number inside a
doctest fixture), `crates/vibe-cli/src/commands/aiui/control.rs:46` (a default
terminal width), `crates/vibe-cli/src/cli/aiui.rs:122` (a CLI arg default).
A second sweep for `too.?long|max_lines|line_budget|unit_length` over the same
perimeter surfaces only conform's `FileLength` rule (below) and one prose line in
a test fixture. Confirmed: nothing measures spec-unit length.

**The length is computed today and thrown away.** In
`…/core-ai-native-specmap/src/mdspec.rs`, `parse_units`:

- the span end is computed at `:404`–`:413` (walk forward to the next
  same-or-higher heading, fences never terminate a span);
- `:414` `let body_lines = &lines[i..end];` — the unit's line count is exactly
  `end - i` at this point;
- `:415` joins it to `span_text`, and the **only** consumer is
  `contentHash: content_hash(&span_text)` at `:450`.

Fact units take the same shape: span `lines[span_lo..span_hi]` at `:280`, hashed
at `:287`. The wire type has nowhere to put a length — `SpecUnit`
(`…/generated/specmap/mod.rs:150`–`:204`) carries `anchor`, `contentHash`,
`docPath`, `file`, `heading`, `line`, `uri`, and optional `disputes`, `kind`,
`revision`, `status`. `line` is the heading line only (`:177`).

**The warning channel already exists, is serialised, and is in daily use** — so
a length warning needs **no schema change**. `Warning { code, file, line,
message }` (`…/generated/specmap/mod.rs:233`–`:246`); codes emitted today:
`invalid-anchor` (`mdspec.rs:386`), `duplicate-anchor` (`mdspec.rs:209` via
`duplicate_anchor_warning`), `malformed-kind-line` (`mdspec.rs:436`),
`unreadable-file` (`mdspec.rs:490`), plus `dangling-edge` (`index.rs:96`),
`pin-into-unmarked-unit` (`index.rs:110`), `pin-ahead-of-unit` (`index.rs:132`).
The committed index carries **265** warnings at HEAD.

**A second, cheaper home exists in the host — and there the length is already a
field.** `crates/vibe-spec` is PROP-035's spec compiler, "a **read-only**
consumer of the spec corpus" (`lib.rs:6`–`:7`). Its IR node
(`crates/vibe-spec/src/doctree.rs:39`–`:64`) carries:

- `pub span: Range<usize>` — documented `:58` as "**Source lines** `[start,
  end)` this node covers, **subtree included**". `span.len()` *is* the unit's
  line count, already materialised;
- `pub kind: NodeKind` — `Heading` vs `Fact` (`:25`–`:32`), so the rule can be
  scoped to heading units;
- `pub children: Vec<NodeId>` (`:63`) and `parent` (`:61`), so a *leaf-scoped*
  length (excluding nested subsections) is derivable without re-parsing.

`vibe-spec` also states the separability constraint explicitly: it "deliberately
does **not** reuse the vendored `specmark-grammar` parser … it is a
sync-engines–gated snapshot that must not be edited from the host tree"
(`lib.rs:9`–`:14`). A length warning implemented there touches no vendored
engine at all.

**The rule shape has precedent, one engine over.**
`…/core-ai-native-conform/src/rules/budget.rs` ships `FileLength { max_lines: u32 }`
(`:137`–`:139`), rule id `"file-length"` (`:143`), threshold-configurable,
consuming a `Fact::FileMetrics { lines }` (`…/conform/src/facts.rs`, "Whole-file
metrics, one per parsed file — the guide §2 'position is a resource' signal
(file-length budget)"), emitting a `Finding` at `:163`–`:177`. Its `why()`
(`:146`–`:149`) is almost the same sentence PROP-014 line 272 gives for units:
"past the budget a file pages badly and its middle third buries invariants".
So a length budget with a configurable threshold is a solved, shipped pattern in
the family — just for code files, in the other engine.

**Measured: what the rule would fire on today.** No shipped code computes this,
so this is an independent reimplementation of `mdspec.rs`'s heading-span rule
(`fence_mask` `:131`, `parse_heading` `:23`, `heading_level` `:44`, the span loop
`:404`–`:413`) run over the corpus `specmap.toml` actually scans
(`spec_roots = ["spec"]` + `root_spec_docs = ["VIBEVM-SPEC.md"]`). Fact units
excluded (their span is one paragraph/list item). Method stated so the numbers
can be rejected or re-run:

| view | units | > 120 lines |
|---|---|---|
| all heading units | 1117 | **72** |
| …of which h1 | 81 | 43 |
| …h2 | 535 | 25 |
| …h3 | 466 | 4 |
| …h4 | 35 | 0 |
| leaf units (no anchored heading nested inside) | 946 | **3** |

Longest under the literal rule:
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:1 #root` — 4406 lines;
`…:713 #log` — 3579; `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md:1 #root`
— 1858; `VIBEVM-SPEC.md:1 #root` — 1588. The three leaf offenders are
`spec/modules/vibe-progress/OWNER-GUIDE.md:1 #root` (176),
`VIBEVM-SPEC.md:702 #manifest-schema-consumer-project-role` (129),
`spec/modules/vibe-registry/PROP-002-decentralized-registry.md:329 #redirect` (125).

**Freshness caveat on index-derived numbers** (measured while doing the above,
recorded once and applying to every count in this document that comes from the
committed artefact): for **599 of 5266** spec units the `line` recorded in
`specmap.json` no longer lands on that unit's anchor in the working tree at HEAD
— e.g. `spec/boot/00-core.md` records `LAYER-HEAD` at line 34 where the tree now
has `##VOID-FOLLOW-HEIRS`; concentrated in `spec/common/PROP-000.md` (137),
`spec/modules/vibe-progress/PROP-043-progress-markup.md` (112),
`spec/common/PROP-018-agentic-standalone-modes.md` (92),
`spec/modules/vibe-workspace/PROP-009-loading-model.md` (91). The **code** side
does not show this: 898 of 912 edges land on a line carrying `#[spec(`,
`#[verifies(` or `scope!` (the 14 others are consistent with multi-line
attributes). So the spec tree has moved since the index was last regenerated;
counts and distributions quoted here are properties of the committed artefact,
not of a freshly-built one.

### 2. What would have to be built

**(a) In the specmap engine** — the smallest diff. In `mdspec.rs::parse_units`,
after `:414`, compare `end - i` against a threshold and push a
`Warning { code: "unit-too-long", … }`. No wire change (the `Warning` table
exists and is serialised), no `SCHEMA` bump. New surface: one threshold on
`Config` (`…/specmap/src/config.rs`, the `specmap.toml` parser), and the same
`cargo xtask sync-engines` propagation to the vendored slots described in B2.
One caveat: `parse_units` currently pushes warnings for *malformed* input only;
this would be the first warning about *authored style*, and every one of them
lands in the committed `specmap.json` (which `--check` byte-compares,
`index.rs:344`), so the corpus must be clean or the threshold set above what
exists before the change can land at all.

**(b) In `vibe-spec` (host)** — nearly free in code, since `Node::span` is
already line-grain: iterate `DocTree` nodes, filter `NodeKind::Heading`, compare
`span.len()`. It then needs a surface to report through — either a new
`vibe check` cell (same registration work as B1(b): `CheckId` variant, `as_str`
arm, `all()` entry, cell file, `checks/mod.rs` registration, `all_checks()`
entry, and the two pinned tests at `crates/vibe-check/src/lib.rs:355` and `:509`)
— and `vibe-check` would gain a `vibe-spec` dependency, which is a host↔host
edge, not an engine seam crossing.

**The decision the numbers force, in either home: what "unit" means for
length.** Under the literal span rule (subtree included — `mdspec.rs:404`–`:413`
and `doctree.rs:58` agree on this), a `#root` anchor spans the whole document, so
**43 of the 72 findings would be h1 document anchors** and the warning would say
"this document is long" while claiming to say "this unit is long". Under a
leaf-scoped reading the same corpus yields **3** findings. The spec text does not
choose (`#SPEC-PRINCIPLE-ONE-UNIT-ONE-DECISION`, line 264, argues for the small
grain; `#SPEC-UNIT-SPAN-AND-FACT-UNIT-GRAIN`, line 82, defines the span as the
nesting one).

### 3. Dependencies

- **Independent of B1, B2, B3, B5, B6.** Needs no new field anywhere: the length
  is already computed in the engine (and already stored in `vibe-spec`).
- Depends on `#OPEN-THRESHOLDS` (line 414) exactly as B1 does — the same clause
  calls both 3-edges/item and 120-lines/unit placeholders "until Phase 3
  metrics".
- Depends on the unit-grain decision above, which is a spec decision, not a
  coding one.
- If placed in the engine, inherits the "corpus must be clean first" ordering
  (warnings are serialised and gated).

### 4. Effort class

**S** — the measurement exists in both candidate homes (computed-and-discarded in
`mdspec.rs:414`, stored as a field in `doctree.rs:59`), the reporting channel
exists in both (`Warning` / `Finding`), and the threshold pattern is already
shipped in `budget.rs`. The cost is the grain decision and, in the engine home,
one index-churning commit.

### 5. Observations on worth (no recommendation)

- **It would fire on real content today** — unlike B1, which fires on nothing.
  72 findings literally, 3 under the leaf reading. That is the difference between
  a tripwire and a backlog generator, and which of the two it is depends entirely
  on the grain decision.
- **The rationale is already load-bearing elsewhere in the family.** conform's
  `file-length` rule states the identical argument for code
  (`budget.rs:146`–`:149`) and is a shipped, baselined gate; PROP-014's own
  wording for units ("Long units page badly and hash-churn often", line 272) is
  the same argument one layer up.
- **The hash-churn half of that argument is currently inert.** All 5266 host
  units carry no revision (measured: `units with revision: 0`), so no unit is
  under revision discipline and "hash-churn" costs nothing yet — the
  `unbumped-hash` diagnostic at `index.rs:235` cannot fire for any of them
  (it requires `new_rev` to be `Some`, `:233`).
- **The biggest offenders are campaign logs, not contracts.** The top four by
  span are two terraform campaign documents' `#root`/`#log` anchors and
  `VIBEVM-SPEC.md#root` — an owner-frozen document (PROP-014 line 25 calls it "a
  99KB owner-frozen spec"). A rule that fires there is reporting on genres it
  cannot ask anyone to change, which argues for scoping (by kind, by directory,
  or by leaf-ness) more than for a threshold number.

---

## B5 — Rustdoc composition in `explain` (spec = contract, rustdoc = detail)

Anchor: `##RUST-PRINCIPLE-RUSTDOC-IS-THE-DETAIL-LAYER`, PROP-014 line 282.
Annotation: *"Specified, not built: `explain` cannot compose rustdoc —
`CodeItem` carries no doc field and the renderer emits symbol, kind, crate, file,
line and edges only."* Re-verified line by line; the annotation is exact, and
understates the gap in one respect (below).

### 1. What exists today

**The renderer, exhaustively.**
`…/core-ai-native-specmap/src/explain.rs`:

- `explain_text` — `:199`–`:205`; dispatches on a `spec://` prefix to
  `explain_unit` (`:91`) or `explain_symbol` (`:123`).
- `explain_symbol`'s item line — `:156`–`:159`, verbatim:
  `format!("code item \`{}\`\n  {} in {} ({}:{})\n", item.symbol, item.itemKind,
  item.crateName, item.file, item.line)`. Five fields. Then edges out
  (`:169`–`:175`), the target unit's line (`:176`–`:178`), and sibling edges
  (`:180`–`:192`). No doc text of any kind.
- `unit_line` — `:23`–`:63`; emits kind, revision, status/disputes, heading,
  file, line. **Also no body text** — so the spec half of the composition is
  identifiers too, not contract prose.
- `explain_json` — `:209`; the items projection at `:241`–`:247` is
  `symbol / item_kind / crate_name / file / line`; the units projection at
  `:264`–`:270` is `uri / heading / revision / content_hash / file / line`.

That last point is the understatement: the anchor asks `explain` to compose
"spec (contract) + rustdoc (detail)". **Neither** half carries prose today —
`explain` composes two sets of identifiers, and `--prose` (B3) renders those.

**`CodeItem` has no doc field.** `…/generated/specmap/mod.rs:34`–`:52`
(`crateName`, `file`, `itemKind`, `line`, `symbol`); `schemas/specmap.jtd.json`
`code_item` — same five, no optional block. Same finding as B2(a) from the other
direction.

**The doc comment is already in the scanner's hand, and is dropped.**
`…/core-ai-native-specmap/src/rscan.rs`:

- `walk_items` (`:137`) passes each item's `&attrs` to `tag_item` for every item
  kind — fn `:141`, struct `:145`, enum `:149`, union `:153`, trait (+ its
  methods) `:157`, const `:168`, static `:172`, type `:176`, impl (+ its
  methods) `:180`, mod `:197`, and `scope!` via `Item::Macro` `:205`;
- `tag_item` (`:126`) forwards them to `edges_from_attrs` (`:79`);
- `edges_from_attrs` matches on the attribute path's **last segment** and
  handles exactly `"spec"` (`:90`) and `"verifies"` (`:105`), discarding
  everything else at `:120` (`_ => {}`).

In `syn`, a `///` doc comment is an ordinary attribute (`#[doc = "…"]`) in that
same `attrs` slice. So the detail layer is parsed, walked past, and thrown away
at zero savings — the marginal cost of capturing it is a match arm, not a new
parse.

**Precedent: the family already reads doc comments — in the other engine.**
`…/core-ai-native-conform/src/facts.rs:36`–`:39`:
`has_doctest: bool`, documented "The item's doc comment carries at least one
fenced code block — a compiled doctest candidate (Class G)". conform extracts a
*predicate over* the doc comment; nobody extracts the text.

**The reverse direction ships.** `…/core-ai-native-specmark/src/lib.rs` — the
proc-macro injects a rendered `Spec:` line **into** rustdoc: `doc_line` (`:26`),
`emit_with_doc` (`:39`, `#[doc = #doc]` at `:43`), applied by `#[spec]` (`:80`)
and `#[verifies]` (`:96`); the crate header states it (`:5`) and `#[cell]` does
the same for its manifest (`:135`–`:144`). This is PROP-014
`#ATTRIBUTE-IS-A-NO-OP-WITH-TWO-CONSUMERS` (line 149) and it is real. So
spec→rustdoc is built; rustdoc→explain is not.

### 2. What would have to be built

**Capture (cheap).** In `rscan.rs`, extract `#[doc]` attribute values in
`tag_item`/`record_item` (`:45`, `:126`) and store them on the `CodeItem`.

**Carry (the expensive half).** A new `doc` field means the same chain B2(a)
described: `schemas/specmap.jtd.json` → jtd-codegen → the checked-in generated
module → `SCHEMA` bump `index.rs:29` → a full rewrite of all 898 `code_items` →
`cargo xtask sync-engines` propagation to the eight
`packages/org.vibevm.ai-native/**` slots — with the same obstacle that the
codegen route is stale (`xtask/src/codegen.rs:50`–`:52` targets a
`rust-ai-native-lang/v0.5.0` path that does not exist). Two size questions this
one raises that B2(a) does not:

- **Volume.** The committed `specmap.json` is already **3.0 MB** (3 141 644
  bytes) for 5266 units + 898 items + 912 edges. Doc comments on 898 tagged
  items — which are, by house style, substantial (every doctest in this
  repository lives in one) — would plausibly multiply that. The index is a
  committed artefact regenerated on every change and byte-compared at the gate
  (`index.rs:344`), so its size is a diff-review cost on every commit that
  touches a tagged item's docs.
- **Truncation policy.** First line only (the rustdoc summary), full text, or
  configurable? Nothing in the tree decides. First-line-only keeps the index
  small and matches what `explain`'s one-line-per-item rendering could use; full
  text is what an LLM producer (B3) would want.

**Render.** `explain_text`'s item block (`explain.rs:156`) and `explain_json`'s
item projection (`:241`) each gain a field. If the *spec* half is to be composed
too (which the anchor's "spec (contract) + rustdoc (detail); neither duplicates
the other" implies), `SpecUnit` would need body text as well — that text is
currently read (`mdspec.rs:415` `span_text`) and hashed away exactly like the
unit length in B4.

**Alternative that avoids the index entirely.** `explain` already rebuilds the
map in memory on every invocation rather than reading the committed artefact —
`…/rust-ai-native-cli/src/trace.rs:9`–`:12` ("Build fresh in-memory: explain
answers for the tree as it is, never for a stale committed artefact"). A doc
lookup could therefore be done **at explain time**, from `file` + `line`, without
ever entering `specmap.json`: no schema bump, no size growth, no vendor
propagation. This is the same posture the orphan table already takes
(`ratchet.rs:22`–`:24`: computed at gate time, deliberately never serialised).

### 3. Dependencies

- **Checked as asked: B5 does *not* depend on B2's `content_hash`.** They are
  independent additions to the same struct. They share one cost centre — if both
  land, they should share **one** schema bump.
- **B3 depends on B5 in substance, not in code**: the `--prose` anchor
  (line 234) specifies the LLM is fed "spec unit texts + rustdoc of linked items
  + deviation reasons". Two of those three are absent from the subgraph today
  (deviation reasons *are* present — `explain.rs:255`, `:83`–`:86`).
- Depends on the truncation/volume decision, and — if the index route is taken —
  on the same stale-codegen repair as B2(a).

### 4. Effort class

- **Explain-time lookup (no schema change): S** — extract the doc text where the
  item is already being walked, render it; nothing leaves the process.
- **Serialised `doc` field on `CodeItem`: M** — the capture is trivial, but it
  drags the full schema-bump chain plus a materially larger committed artefact
  and a truncation policy nobody has set.

### 5. Observations on worth (no recommendation)

- **A consumer is queued behind it.** `--prose` is specified to feed on rustdoc
  (line 234) and today feeds on identifiers (B3). B5 is the input B3 was
  specified to consume.
- **The half-built shape is asymmetric in a way that reads as unfinished**:
  the tag pushes a `Spec:` line *into* rustdoc (`specmark/src/lib.rs:39`–`:43`)
  so a human reading the docs sees the contract, while a machine reading the
  graph cannot see the docs. The round trip is one-directional by omission, not
  by decision — no code or comment states a reason for the asymmetry.
- **The principle it serves is stated twice and enforced nowhere.** Line 268
  (`#SPEC-PRINCIPLE-SPEC-NEVER-RESTATES-HOW`) puts detail in rustdoc "where it
  cannot drift from the code", and line 282 says the metamodel "joins the two
  layers at query time". The join is the missing piece; the split is real and
  already practised.
- Counter-signal: the whole value depends on `explain` being used. Its only
  surfaces are `cargo xtask trace explain` and `rust-ai-native trace explain`
  (`xtask/src/main.rs:335`–`:343`); promotion to `vibe explain` is unbuilt
  (line 367) and the runtime/MCP channel is blocked on signing (line 369). So
  the composition would today serve a CLI a developer must know to run.

---

## B6 (rider) — a `decides` verb for `prop` nodes

Anchor: `##ROW-KIND-PROP`, PROP-014 line 120 (a table cell). Annotation:
*"`decides` is not a verb this system can emit. The `Verb` enum in
`specmark-grammar` is `Implements · Verifies · Documents · Deviates · Informs`,
and its own doctest states the verb set is closed (`Verb::parse("fulfills") ==
None`); `decides` returns zero hits across every crate."*

### 1. What exists today

**The enum and its closedness, with the doctest.**
`…/core-ai-native-specmark-grammar/src/lib.rs`:

- `pub enum Verb` — `:39`–`:46`: `Implements`, `Verifies`, `Documents`,
  `Deviates`, `Informs`. Doc line `:31`: "The closed verb set (PROP-014 §2.3)."
- The closedness doctest — `:33`–`:38`, the load-bearing line being `:37`:
  `assert_eq!(Verb::parse("fulfills"), None); // the verb set is closed`.
- `Verb::as_str` `:49`–`:57`; `Verb::parse` `:59`–`:68`, closed by the catch-all
  `_ => return None` at `:66`.
- The crate header states it as a rule it enforces: `:18` "Rules enforced here:
  **the verb set is closed**".
- A **second** pin, in the unit tests: `spec_args_unknown_verb_and_key`
  (`…/specmark-grammar/src/lib/tests.rs:185`–`:187`) parses
  `fulfills = <uri>` and asserts the error contains "unknown specmark verb".
- The user-facing error messages enumerate the five by name, twice:
  `lib.rs:295`–`:297` ("verbs: implements, verifies, documents, deviates,
  informs") and `:302`–`:306` ("unknown specmark verb `{verb_ident}`; expected
  one of …").

**Absence of `decides`, with the perimeter.** `rg -n '"decides"|Decides|decides
*='` over `crates/`,
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/`, `xtask/`,
`tools/`, `schemas/` returns **zero** hits. (A looser `rg "\bdecides\b"` over the
same perimeter returns 23 hits, all English prose inside doc comments — "the
caller decides…", `crates/vibe-actions/src/gate.rs:12`,
`crates/vibe-publish/src/post_hook.rs:323`, etc. None is a token, identifier or
string literal.)

**The `prop` kind, by contrast, is fully plumbed — and completely unused.**
Parser `mdspec.rs:77`–`:78` (`"prop" => SpecUnitKind::Prop`); wire enum
`…/generated/specmap/mod.rs:129`–`:130`; schema
`schemas/specmap.jtd.json` → `spec_unit.optionalProperties.kind` =
`enum ["prop","req","design","guide"]`; renderer `explain.rs:31`. And in the
committed host index, **0 of 5266** units carry a `kind` at all — so there is
not one `prop` node in this repository's graph for a `decides` edge to touch.

**The verb set's downstream fan-out** (what "opening" the enum actually moves):

| surface | file:line |
|---|---|
| `Verb` enum + `as_str` + `parse` | `specmark-grammar/src/lib.rs:39`, `:49`, `:59` |
| closedness doctest | `specmark-grammar/src/lib.rs:37` |
| closedness unit test | `specmark-grammar/src/lib/tests.rs:185`–`:187` |
| two error messages enumerating the verbs | `specmark-grammar/src/lib.rs:295`, `:302` |
| `deviates`-requires-`reason` / `reason`-only-on-`deviates` | `specmark-grammar/src/lib.rs:352`–`:366` |
| grammar → wire mapping | `specmap/src/rscan.rs:21`–`:29` |
| wire enum `EdgeVerb` | `specmap/src/generated/specmap/mod.rs:66`–`:82` |
| JTD schema enum | `schemas/specmap.jtd.json`, `edge.verb` = `["implements","verifies","documents","deviates","informs"]` |
| rendering | `specmap/src/index.rs:31`–`:40`, `specmap/src/explain.rs:12`–`:21` |
| **canonical ordering key** | `specmap/src/index.rs:42`–`:51` (`verb_key`: Implements 0 … Informs 4), used by the sort at `:151`–`:159` |

The ordering key is the non-obvious one: `verb_key` defines the committed
index's edge order, and the determinism contract is a tested property
(`index.rs:457` `index_is_deterministic`) byte-compared at the gate (`:344`).
Where a sixth verb is inserted in that numbering decides whether existing edge
rows move in the artefact.

**The direction problem — the deepest finding in this section.** The table cell
says a `prop` unit's typical edges are "`decides`, **referenced by REQs**"
(line 120). That describes a **spec→spec** relation. The shipped edge model has
none: `#EDGE-MODEL-EDGES` (line 199) defines edges as
`(CodeItem) --verb--> (SpecUnit @ r)`, and the wire type agrees —
`Edge.fromSymbol` is a code symbol (`…/generated/specmap/mod.rs:91`–`:92`), built
only in `rscan.rs::record_edge` (`:55`–`:66`) from a `#[spec]` / `#[verifies]` /
`scope!` site. The one spec↔spec edge the PROP mentions anywhere,
`conflicts_with` (line 199, brownfield amendment), is **also unbuilt**: what
ships is a *node property* — `SpecUnit.status = disputed` plus
`SpecUnit.disputes = <other anchor>` (`mod.rs:184`–`:203`), parsed from the kind
line's `disputed(#anchor)` form (`mdspec.rs:101`–`:108`) — not an edge. And 0
units carry a status in the host index.

### 2. What "opening the enum" would mean

Three separable readings, and they are not the same project:

**(a) Literal: add `Decides` to the closed verb set.** Add the variant + `as_str`
arm + `parse` arm (`lib.rs:39`, `:49`, `:59`); update the two error strings
(`:295`, `:302`); update the closedness doctest (`:33`–`:38` — note it asserts
about `"fulfills"`, so it keeps passing, but the *doc sentence* "The closed verb
set" now describes a set that grew by owner decision, which is exactly what a
doctest-pinned invariant is for: the pin does not forbid the change, it forces it
to be a deliberate edit rather than a drift); add the wire variant
(`mod.rs:66`–`:82`) and the schema enum value; add the `verb_to_wire` arm
(`rscan.rs:21`); add `verb_str` arms (`index.rs:31`, `explain.rs:12`); pick the
`verb_key` slot (`index.rs:42`) and accept whatever edge reordering follows;
regenerate the index; `cargo xtask sync-engines` to the eight
`packages/org.vibevm.ai-native/**` slots (B2 §2). Mechanically this is the
best-understood change in this document — the enum is small, centralised, and
every consumer is an exhaustive `match` the compiler will point at.

**(b) What (a) does *not* give you: a producer.** After (a), `decides` is
writable as `#[spec(decides = "spec://…#some-prop")]` on a **code item** — i.e.
"this function decides this PROP", which is not what the table row means. To
express "this PROP decides X, and REQ Y references it" the edge model needs a
spec-side tail: a new node type for the edge's `from`, or a second edge table, or
a markdown-side edge syntax that `mdspec.rs` would have to learn (it currently
parses headings, kind lines and `##ID` facts only). That is a change to
`#EDGE-MODEL-EDGES` (line 199), not to the verb list.

**(c) Or: no verb at all.** The row's other half — "referenced by REQs" — is
arguably already expressible: `informs` exists (`Verb::Informs`, `lib.rs:45`),
is the `design`-kind row's own verb (line 122), and has **zero** producers today.
Whether `decides` is a distinct relation or a naming of `informs` in the
prop direction is a spec question the tree cannot answer.

### 3. Dependencies

- **(a) is independent of B1–B5.** It touches the grammar and its fan-out, and
  nothing else in this part.
- **(b) depends on a change to the edge model** (`#EDGE-MODEL-EDGES`, line 199) —
  spec→spec edges have no representation, no producer, and no precedent; the one
  spec-relational feature that exists (`disputes`) chose a node property instead.
- **The `prop` kind must actually be used first** for either to have a
  population: 0 of 5266 host units declare a kind. That is a spec-authoring
  action (`GUIDE-SPEC-AUTHORING §3` kind lines), not a code change.
- Interacts with `#SPEC-PRINCIPLE-NORM-AND-RATIONALE-ARE-SEPARATED` (line 266)
  and `#RUST-PRINCIPLE-TYPED-VERBS-NO-BARE-LINKS` (line 280) — "the verb is what
  makes the graph queryable", the standing argument for adding rather than
  overloading.

### 4. Effort class

- **(a) alone: S** — one variant through a centralised, exhaustively-matched
  chain; the compiler finds every site, and the only judgement calls are the
  `verb_key` slot and the index churn.
- **(b), the relation the row actually describes: L** — spec→spec edges do not
  exist in the model, the scanner, the schema, or the renderer; and PROP-014's
  only other spec-relational feature deliberately went the other way.

### 5. Observations on worth (no recommendation)

- **Two of the five existing verbs already have zero producers.** Measured over
  the committed index: `implements` 677, `verifies` 223, `deviates` 12,
  `documents` **0**, `informs` **0**. The verb set is not currently
  under-expressive in practice; it is under-used.
- **There is nothing for the edge to point at.** Zero `prop`-kind units exist in
  the host index (0 of 5266 carry any kind), so a `decides` edge would have no
  legal target in this repository on the day it shipped.
- **The doctest is not the obstacle it looks like.** `Verb::parse("fulfills") ==
  None` (`lib.rs:37`) asserts that *unknown* verbs are rejected; it does not
  freeze the membership list. What actually resists change is the *sentence*
  ("The closed verb set", `:31`; "the verb set is closed", `:18`) — a design
  commitment, cheap to edit and expensive to edit lightly.
- **The annotation's own framing is the sharpest evidence**: "a decision can be
  a node and never the tail of the edge this cell names" (line 120). The node
  half is fully built (kind `prop` parses, serialises and renders); the edge half
  is not merely unimplemented, it is unrepresentable in the current model.
- Worth noting for scoping: the table cell is documentation of *typical* edges
  per kind, not a normative `req`. The document's own closing clause
  (`#UNEXERCISED-MECHANISM-IS-REMOVED-FROM-THE-SPEC`, line 425) offers two exits
  for anything unexercised — build it, or annotate it in place — and this cell is
  already annotated.
