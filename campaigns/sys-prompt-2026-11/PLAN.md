# Campaign: the cache-stable SYSTEM prompt prefix

**Status:** planned 2026-11-10; written against `main` at `e2508fc1d`. Campaign
node `SYS-PROMPT`: starts after `APP-VERSIONS` closes, before `USER-PROJECT`.
Nothing is implemented. Phase 0 (spikes) may run while `APP-VERSIONS` finishes
— the spikes read code and work in sandbox copies, and S2/S3 touch owners that
`APP-VERSIONS` does not (`crates/vibe-workspace/src/boot_artifacts/`,
`…/install/bootgen/`), so the files are disjoint from its registry, resolver,
lockfile and channel work.

Cold start: read this file, then `LOG.md` in this directory, then
`spec://org.vibevm.core/vibevm/common/PROP-076#root`, then
`HARNESS-DELIVERY.md` in this directory — the owner-adopted design for how the
lane reaches a coding agent, which is the input to the harness phases below and
is **not** restated here.

## 1. Mandate

The owner's request is 2026-11-07 and its research proposal is
`campaigns/next-semantic-transfer-2026-11/architecture/SYSTEM-PROMPT-SPEC.md`
with `system-prompt.json`; the three reviews beside it in
`…/research/system-prompt-{boot-audit,technical-review,design-review}.md` are
its evidence. The design settled against the code on 2026-11-10 is
[`PROP-076`](../../vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml).
This file does not restate either; it is the order of work, the files, the
gates and the risks.

Two sentences carry the whole campaign. **`vibe` generates one byte-stable
prefix — the user's template, the selected entrypoint with its managed boot
block replaced in place, and the already-compiled static lane — whose bytes are
a pure function of its inputs, so a provider that caches on an exact prefix
match can cache it.** **The live part of boot stays live: the INDEX entries are
a plan a runner executes per request, never bytes frozen into the artifact, and
a stale prefix is refused before a request is sent rather than believed.**

The gate on the first sentence is determinism, measured as byte equality across
checkouts and operating systems. The gate on the second is a **differential
boot-parity test**: the prefix plus the executed continuation must deliver the
same instruction text, in the same order, as today's file-reading boot, modulo
exactly the managed-block substitution and nothing else. Neither gate involves
a model's judgement; neither requires a paid API call.

## 2. Baseline — verified facts at writing time (code, not specs)

Verified by a read-only survey of the tree at `e2508fc1d` on 2026-11-10.
**Seven paths and three behavioural claims in the 2026-11-07 research were
wrong**; those corrections are `PROP-076#SYS-BASELINE-CORRECTIONS` and are not
repeated here except where a phase depends on them.

- **The boot layout owner is one private list plus three predicates.**
  `crates/vibe-core/src/layout.rs`: `BOOT_DIR = "boot"` (`:66`),
  `INDEX_MD` (`:72`), `STATIC_XML` (`:74`), `STATIC_MD` (`:76`),
  `BOOT_PUBLICATION_PREFIX = ".vibe-boot-"` (`:85`); path accessors
  `current_boot_dir` (`:200`), `current_boot_index` (`:216`),
  `current_boot_static_xml` (`:222`), `current_boot_static_md` (`:228`).
  `generated_boot_paths()` (`:250`) is **private** and returns the three
  boot-dir-relative *paths*, not bare names; `is_generated_boot_output`
  (`:241`) matches them exactly and additionally any direct boot-dir child
  whose name starts with the publication prefix (`:243-247`);
  `is_reserved_boot_source` (`:268`) does the same case-insensitively
  (`:271-285`) for declaration admission. Callers that inherit an addition
  for free: nine production sites of `is_generated_boot_output`
  (`vibe-cli/src/commands/show/effective.rs:104`, `…/show/effective/read.rs:157`
  and `:178`, `…/tree/build/in_place.rs:44`, `…/in_place/read.rs:122` and
  `:143`, `vibe-spec/src/resolver/declared/validate.rs:59`,
  `vibe-spec/src/source_policy/authored_boot.rs:97`,
  `vibe-spec/src/source_policy/inventory.rs:174` and `:283`,
  `vibe-workspace/src/vibedeps/derived.rs:468`,
  `…/vibedeps/source_policy/inventory.rs:86`) and three of
  `is_reserved_boot_source` (`vibe-core/src/manifest/migration/source.rs:61`,
  `…/manifest/source_descriptor/mod.rs:228`,
  `vibe-workspace/src/vibedeps/derived/declared.rs:107`).
- **The self-inclusion hazard is real today.**
  `crates/vibe-workspace/src/install/bootgen/own_boot.rs:167` `legacy_entries`
  reads the immediate children of the boot directory (`:168`, `:174`, no
  recursion) and admits every one that `vibe_specdoc::is_spec_source` accepts —
  which is **by extension alone**, `Some("md" | "xml")`
  (`crates/vibe-specdoc/src/load.rs:76-81`) — excluding only the three
  generated names (`:185-190`), skipping non-regular files (`:177-179`) and
  hard-erroring on a Markdown/XML name collision (`:212-222`). Both
  `SYSTEM.md` and `SYSTEM-TEMPLATE.md` are therefore enrolled as ordinary boot
  inputs today. The automatic native discovery path is
  `crates/vibe-spec/src/source_policy/authored_boot.rs:74-129`, which already
  skips generated outputs at `:97` and keeps only
  `SourceInterpretation::VibeSpec` + `SourceLoading::Scan` rows (`:125-129`).
- **Composition is pure and the defaults are in code, not comments.**
  `crates/vibe-workspace/src/boot.rs:303` `compute_effective_boot(NodeBootInputs)`
  does inherited foundation (`:310-327`), own (`:331-345`), dependencies in topo
  order (`:350-401`, Kahn with a `<group>/<name>` min-heap tiebreak at `:426`),
  then a stable sort by band (`:407`). Own and inherited authored boot are
  unconditionally `LinkType::Dynamic` + `PackageFormat::Simple`
  (`:314`/`:323`, `:336`/`:341`). `EffectiveBoot` is one `Vec<BootEntry>`
  (`:255-258`) with `dynamic_entries()` = `link != Static` (`:274-276`).
  `LinkType` lives in `crates/vibe-core/src/manifest/package.rs:394` and
  `#[default]` sits on `Dynamic` (`:405`); the precedence chain is
  `declared.or(suggested).or(workspace_default).unwrap_or_default()`
  (`boot.rs:355-359`), so the final fallback is **Dynamic** — the comment at
  `boot.rs:352-354` says `static` and is wrong.
- **INDEX `kind` is read timing, computed from one expression.**
  `crates/vibe-workspace/src/boot_artifacts.rs:187` `prepare_index_body` maps
  `dynamic_entries()` (`:196`) with
  `kind: if e.when.is_some() { "dynamic" } else { "static" }` (`:203-207`) and
  emits the `static` pointer only when static entries exist (`:191`, `:194`).
- **Conditions: `installed:` is generation-time, `os:` is the only runtime
  gate.** `crates/vibe-workspace/src/install/bootgen/conditions.rs:86-96` —
  `installed:` false drops the contribution entirely, true removes the
  condition, and only `WhenCondition::Os` survives as a residual (`:92`).
  Membership comes from `installed_identities` (`:22-27`) over the exact
  resolution. The grammar is closed at
  `crates/vibe-core/src/manifest/package/when.rs:71-112`: exactly
  `os:windows|macos|linux` and `installed:<group>/<name>`; an unknown OS,
  a missing `/` and any other prefix are each a named error (`:80-85`,
  `:90-95`, `:106-110`), and the type is `#[serde(try_from = "String")]`
  (`:35`) so a bad `when` fails deserialisation rather than defaulting.
- **The static spelling has one owner with three answers.**
  `crates/vibe-workspace/src/boot_artifacts/format.rs:42` `resolve_static_path`
  returns the one present path (`:47-48`), `None` when neither exists (`:49`),
  and a refusal when both do (`:50-55`, "the generator owns one; delete the
  stray").
- **The managed redirect block has exact byte boundaries and a careful
  writer.** `crates/vibe-workspace/src/boot_artifacts/redirect.rs:128`
  `locate_block` matches only an exactly standalone trimmed `<vibevm>` /
  `</vibevm>` line (`:134-141`), refuses a close-before-open and any count
  other than 0/0 or 1/1 (`:160-171`) as `BlockLocation::Malformed`, and returns
  `Present { start, end }` (`:114`, `:155-158`). The writer splices around
  those offsets, preserving every co-tenant byte (`:238-244`), and skips a
  byte-equal write (`:267-269`). `REDIRECT_FILES = ["CLAUDE.md", "AGENTS.md",
  "GEMINI.md"]` (`:29`).
- **The boot transaction's production-role set is closed and validated.**
  `crates/vibe-workspace/src/boot_artifacts/transaction/journal.rs`:
  `INDEX_TARGET` (`:21`), `STATIC_MD_TARGET` (`:22`), `STATIC_XML_TARGET`
  (`:23`), validation `:62-86` requiring the selected/stale pair to be exactly
  the distinct `STATIC` pair (`:75-78`); `transaction.rs:259-282` `common_parent`
  requires all targets to share one directory and be distinct. The durable
  record is **not** named `Transaction`: `journal.rs:7-10` aliases the wire type
  `BootArtifactTransaction` as `Journal`, with `JournalEntry` / `RecordedState`
  / `JournalMode`. The write input is `ArtifactWrite { index_path, index_bytes,
  static_path, static_bytes: Option<&[u8]>, stale_path }`
  (`transaction.rs:61-67`). Entry points: `write_production_with_selectors`
  (`:95`), `write_production_detailed` (`:103`),
  `transaction/control.rs:24 write_controlled_with_selectors`,
  `preflight_artifact_roles` (`:109`), `with_boot_lock` (`:122`),
  `recover_pending_locked` (`:456`); serialised by
  `transaction/lock.rs:17-25` on `layout::BOOT_ARTIFACTS_LOCK`.
- **Three seams sit above that transaction, and one of them writes no
  redirects.** Node builtin: `boot_artifacts.rs:365 write_boot_artifacts_inner`
  → `:405`, selector closure `:413-415`. Node owner-managed/native:
  `boot_artifacts/native_managed/write.rs:91` → `:140`, selector `:150`. Unit
  lane: `boot_artifacts/publication.rs:171 publish_unit_artifacts` → `:186`,
  selectors `|_| Ok(())` at `:196` (package slots write no redirect blocks).
- **Compile-first, then commit, then redirects.** `write_boot_artifacts_inner`
  renders the INDEX body (`:379`) and compiles the static artifact (`:380-391`)
  before touching a path (`boot_dir` at `:392`), rechecks declared sources
  immediately before the write (`:402-404`), and the selector closure runs
  **after** the INDEX/STATIC roll-forward commits
  (`transaction.rs:245`). Artifact roles actually written today: `INDEX.md`
  (`boot_artifacts.rs:396`), the selected `STATIC.*` (`:400`) when static bytes
  exist, **removal** of the stale spelling (`:401` with `after: absent_state()`
  at `transaction/prepare.rs:90`), the three redirect blocks
  (`transaction/selector.rs:15`), and the transient journal / stage files
  (`transaction.rs:26-27`, `transaction/journal.rs:28-39`).
- **There are three independent lane prepare-and-compile sites, not one.**
  `boot_artifacts/compile.rs:86 compile_static_artifact_with_plans` (empty input
  → `None` at `:100-103`, providers at `:106`, the one compiler call at
  `:140-144`); `boot_artifacts/native_managed.rs:232
  compile_static_owner_managed_using` (its own `BootSectionSource::new` at
  `:268`, its own `build_with_providers_bound` at `:279`, its own plan at
  `:294-305`, empty input `:260-263`); and
  `boot_artifacts/native_managed/explicit.rs:22` (custom backend, empty input
  `:35-38`, own providers `:48`, refuses a `Pending` outcome at `:82-85`). A
  change to lane preparation must move all three.
- **A read-only in-memory lane analysis already exists and is provider-free in
  its base form.** `crates/vibe-workspace/src/install/bootgen/analyze.rs`:
  `analyze_node_lane` (`:80`) takes **no** provider, delegates to
  `analyze_node_lane_using` (`:102`), runs the same cells as regeneration —
  `DeclaredBootSources::prepare_with_nodes` (`:126`), `build_unit_table_bound`
  (`:131`), `hoist::soft_static_pulls` (`:133`), `compute_effective_boot`
  (`:187`), `append_hoisted` (`:194`), `desubstitute_covered_units` (`:196`),
  `generated_inputs::admit` (`:197`) — and ends at `compile_static_analyzed`
  (`:216`, i.e. `analyzed.rs:36`), which contains no `transaction::` call.
  Returns `Option<AnalyzedLane>` (`:44-56`: `artifact`, `providers`,
  `identities`); `None` means no static lane. The **bound/native** variants
  take an `OwnerNativeCompileProvider`: `analyze_node_lane_bound_native_using`
  (`:270`, optional), `analyze_effective_bound_native_with_sources` (`:554`,
  optional), `compile_node_backend_using` (`:415`, **mandatory**) — each hands
  control to a caller-supplied trait that may spawn a process
  (`native_managed/explicit.rs:71-73`, `native_managed.rs:332`).
- **Equal bytes are already not rewritten, in two places.**
  `transaction.rs:306-310` `apply_forward` compares `current_state` (present +
  sha, `:522-533`) with the recorded `after` and returns early, so a byte-equal
  artifact keeps its mtime; and `install/bootgen/hybrid_emit.rs:297-312` skips a
  whole unit whose recorded fingerprint matches and has no pending frame.
- **Freshness evidence is strict.**
  `boot_artifacts/publication.rs:70 read_unit_index_freshness` requires an
  exact lowercase 64-hex fingerprint (`:79`, `:116-121`), an optional
  **immediately adjacent** pending frame (`:102-106`), and exactly one current
  recipe marker in the next slot (`:95`, `:107-109`).
- **The unit fingerprint hashes no file bytes.**
  `crates/vibe-workspace/src/boot/hybrid/fingerprint.rs:133` `compute` hashes
  identity (`:149-150`), own path + resolved version (`:152-155`), condition
  (`:156-163`), fragments (`:164-176`), edge modes and targets (`:181-186`),
  child fingerprints for static edges (`:188-203`), identity only for dynamic
  edges (`:204-207`), optional owner-plan digests (`:215-218`) and
  native-pending frames (`:219-222`) — and performs no read. Owner-plan frames
  come from `install/bootgen/owner_plans.rs:15 plan_digest_frames_for`, whose
  `filter_map` omits a unit with no activation. **The node lane carries no
  fingerprint and recomputes** (`boot_artifacts.rs:379` passes `None`;
  comment `:394-395`).
- **Generated-input admission is already a before/after witness.**
  `install/bootgen/generated_inputs.rs:10 admit` captures before (`:22`),
  verifies (`:35-42`), refuses stale unit freshness (`:50-54`), captures after
  and refuses a change during validation (`:57-63`), then retains the admitted
  bytes (`:64` → `declared_sources/generated.rs:94-98`), which later re-reads
  are checked against (`generated.rs:118-136`).
- **Declared sources already hold everything capture needs.**
  `boot_artifacts/declared_sources.rs:24-31` — a held `Project` (`:99-100`,
  slots as held descendants `:130-132`), selected versions (`:118`),
  independently selected source hashes with a three-way equality check
  (`:43-48`, `:62-70`), slot read bindings, prepared nodes
  (`declared_sources/nodes.rs:8-33`) and generated observations
  (`declared_sources/generated.rs:22`, `:94`). Refusals: ordinary resource boot
  promotion (`:254-259`), missing binding (`:56-60`, `:383-387`), lost logical
  native address (`:280-287`, `:302-307`).
- **`PreparedNodeBoot` is the authored-input carrier.**
  `install/bootgen/own_boot.rs:9-16`: `entries` (the only public field), `root`,
  `relative`, `policy: PackagePolicyObservation`,
  `captured: Option<CapturedAuthoredBoot>`; built by
  `prepare(node_dir, node_rel, expected: &Manifest)` (`:18-22`), with
  `recheck()` (`:84`) and `simple(entry, overlay)` (`:98-102`). Callers:
  `install/source_admission.rs:68` and
  `boot_artifacts/declared_sources/nodes.rs:21`.
- **`publish_node_boot` is the source-ownership gate, and it is in `bootgen`.**
  `crates/vibe-workspace/src/install/bootgen/publication.rs:13`: root always
  published (`:18-20`), a member with no `[package]` always published
  (`:21-23`), and a member whose `(kind, group, name)` matches a selected
  dependency is protected with **version absent from the predicate**
  (`:24-28`); its own test asserts exactly that (`:40-61`).
- **The name `SYSTEM` is unoccupied.** `grep -rn 'SYSTEM'` and
  `grep -rni 'prompt'` over `crates/vibe-workspace/src/boot_artifacts/` and
  `…/install/bootgen/` are both empty; every `prefix` hit there is unrelated
  (`str::strip_prefix`, `STAGE_PREFIX`, `BLOCK_BODY_PREFIX`). `prompt` is also
  free as a top-level CLI command: the `Command` enum
  (`crates/vibe-cli/src/cli/command.rs`) has 63 variants and none is `Prompt`.
- **The sizes this feature is about, measured.** `vibevm/vibespecs/boot/STATIC.xml`
  251,817 bytes; `AGENTS.md` 26,121; the eight files `INDEX.md` names 57,894
  (00-core.xml 9,239 · 90-user.xml 22,719 · core-ai-native 3,366 ·
  rust-ai-native-lang 4,875 · typescript-ai-native-lang 7,224 ·
  delegation-rules 2,990 · delegation-first 3,565 · omnichannel 3,916).
  **Every one of the eight INDEX rows is `kind = "static"` with no `when`** —
  this repository's band G is empty today, which is why the OS matrix must be
  proved on a fixture rather than on the host project.
- **Line endings are already pinned, which the determinism claim needs.**
  `.gitattributes:7` is `* text=auto eol=lf`, and `core.autocrlf` is unset on
  this machine. Without that attribute a Windows checkout would produce a
  different prefix.
- **There is no spec-XML validation gate.** `vibe check` does not parse the
  dialect; no `cargo xtask` subcommand and no test walks `vibevm/vibespecs/**`.
  The authoritative validator is the PROP-045 projector
  `vibe_specdoc::project_spec_text` (`crates/vibe-specdoc/src/load.rs:140`),
  reachable corpus-wide via `vibe facts check` (read-only unless
  `--write-state`) or per file via
  `vibe refactor convert-source --from xml --to md --dry-run <path>`, **which
  parses but always exits 0** (`crates/vibe-cli/src/commands/refactor/convert_source.rs:124-131`)
  — read its `dry-run …` line. Without `--dry-run` that command **rewrites and
  renames files**. PROP-076 was validated this way and reports
  `dry-run byte-stable`.
- **Format discipline.** `formats/EPOCHS.toml:35-36` is `public = false`,
  `break_window_open = true`: breaking is free and unmigrated, no note under
  `formats/breaks/` required. A format must exist in `formats/REGISTRY.toml`
  (~190 records, shape documented at `:13-16`: `epoch`, `schema`,
  `recoverable`, `foreign_parsers`, `corpus`, `sunset`) or `FormatId` cannot
  express it — enforced by `xtask/src/codegen/format_id.rs:52` plus
  `crates/vibe-wire/tests/format_registry_completeness.rs:32`, and in the
  other direction by `xtask/src/codegen/strictness.rs:131-146`, which refuses a
  `schemas/**` file no record names. **The only boot-related record today is
  `[format.boot-artifact-transaction]`** (`formats/REGISTRY.toml:1202`); the
  generated lanes themselves are unregistered spec documents. Protocol tags
  like `vibe-app-env/1` have no central registry — they are a constant at the
  emit site plus an untyped `protocol` member in the JTD schema.

## 3. Phase 0 — spikes (no commits)

Each spike answers one question that would change the shape of a later phase.
Run them with the repository's isolated runner
(`python3 /fast/vibevm-test-tmp/claude-int/run_test.py <label> …`). **Every
spike that creates a `SYSTEM*` file does so in a sandbox copy of the tree,
never in the live `vibevm/vibespecs/boot/`** — PROP-076 forbids putting those
names in a working boot directory before the protections of P1 exist.

- **S1 — the prefix, by hand.** Assemble the three segments from the current
  tree with a throwaway script: the template (a short hand-written one), then
  `AGENTS.md` with the bytes between `locate_block`'s offsets replaced by a
  draft transport contract, then `STATIC.xml` verbatim. Questions: what is the
  exact byte count, and is it `26,121 + 251,817` minus the managed block's own
  length plus the template and the frames? Does replacing the block in place
  leave the authored preamble's "read the boot lane" sentence answered at a
  sensible distance, or does the contract need to move? Does anything in the
  current `AGENTS.md` read *worse* with the block replaced? *If the in-place
  substitution reads badly, `PROP-076#SYS-REDIRECT-IN-PLACE` is renegotiated
  before P3, not after.*
- **S2 — the closed transaction role set.** In a sandbox, add two dummy
  production targets and two `StageRole`s and see exactly what refuses. Walk
  `transaction/journal.rs:62-86`, `transaction.rs:259-282` `common_parent`,
  `transaction/prepare.rs`, `transaction/selector.rs:29-34`,
  `boot_artifacts/inverse.rs` (rollback), and the three callers. Question: is
  the extension three files or six, and does the rollback path need a new
  inverse arm per role? *This is the single biggest unknown in the campaign's
  cost; if it is six files plus rollback arms, P4 goes from M to L.*
- **S3 — self-inclusion, demonstrated.** In a sandbox copy, drop
  `SYSTEM.md` and `SYSTEM-TEMPLATE.md` with marker text into the boot directory
  and run the node regeneration twice. Questions: are they enrolled by
  `legacy_entries` (predicted yes); does the marker text appear in the
  regenerated lane or INDEX; and does a **second** run grow the artifact again
  (the recursive case)? *This is the falsifiable form of the hazard P1 exists to
  close, and it is cheap. If it does not reproduce, P1's gates are smaller than
  planned and the plan says so.*
- **S4 — which of the three compile seams this node uses.** Instrument the
  three lane prepare/compile sites with a tracing span and run a normal
  regeneration of this repository, then one with an extension world present.
  Question: does the host node reach `compile.rs:86` only, and is a node with
  extensions guaranteed to reach `native_managed.rs:232` instead? *If v1 can
  support one seam and refuse the others by name, P3 shrinks; if a real node
  can reach two in one run, P3 must handle all three from the start.*
- **S5 — the read-only preflight, measured.** Call provider-free
  `analyze_node_lane` on this repository and time it. Questions: does it
  succeed with no provider on the host node; how long does it take warm (it
  sits in an API caller's request path); and which concrete conditions make it
  impossible to answer without an effectful provider — i.e. what exactly
  produces `freshness_unverifiable`? *Supplies the real numbers for P5's budget
  and the exact refusal list for `PROP-076#SYS-READ-ONLY-CHECK`.*
- **S6 — is there a fake provider to reuse?** Search `crates/vibe-test-support`,
  the `vibe-net` test fixtures and the CLI test commons for an existing local
  HTTP capture fixture (the app-versions campaign used a loopback `http_dir`
  and a counting server). Question: can P6's request capture reuse one, or must
  it be built? *Decides whether P6 is M or L.*

### Spikes — answers (run 2026-11-10 against `c04bda8a5`)

All six ran; every piece of instrumentation was reverted and the tree was
verified clean before P1 began. Three of them changed the design
(`PROP-076#SYS-SIZE-TODAY`, `#SYS-SELF-INCLUSION`, `#SYS-LANE-VERBATIM` +
`#SYS-BASELINE-CORRECTIONS` (7)) and one changed a phase's size (P4, M → L).

- **S1 — the prefix, by hand. Answered; it falsified one number and confirmed
  the in-place substitution.** Assembled from the live tree with a throwaway
  script that re-implemented `locate_block`'s rule (an exactly standalone
  trimmed marker line): `AGENTS.md` 26,121 bytes, `STATIC.xml` 251,817, the
  managed block **830 bytes at offsets 25,291..26,121**. The prefix measured
  **278,092 bytes** — exactly `277,938 − 830 + 146 (template) + 681 (draft
  contract) + 157 (three frame lines)`, so the plan's "within a few hundred
  bytes of 277,938" held at **+154**.
  **The block's end offset equals the file length**: the managed block *is*
  `AGENTS.md`'s tail. So substituting in place puts the transport contract in
  the one position the authored preamble already points at — its first sentence
  says to read "the boot lane in the order the generated block at the end of
  this file prescribes" — and the 600 bytes before it are a clean section
  boundary. Nothing reads worse; `PROP-076#SYS-REDIRECT-IN-PLACE` stands and
  needs no renegotiation, and the no-block case is byte-identical in shape to
  the with-block case here. `AGENTS.md`, `CLAUDE.md` and `GEMINI.md` are
  md5-identical, confirming `#SYS-ENTRYPOINT-ONE`'s premise.
  **Falsified:** the prefix holds **82.8%** of the 335,832 bytes of boot
  material, not 92%. `(251,817 + 26,121) / 335,832 = 82.76%`; the 75% figure
  for the lane alone is right. Corrected in `PROP-076#SYS-SIZE-TODAY`,
  `FORMAT-CHANGES.md` `SP-FMT-003`, `LOG.md` and §5 below.
- **S2 — the closed transaction role set. Answered: neither three files nor
  six — twelve refuse to compile, and five more edits are silent. P4 is L.**
  Two optional `entry` members (`system`, `systemMeta`) added to
  `schemas/boot_artifact_transaction.jtd.json` regenerated cleanly through
  `cargo xtask codegen` (no cascade elsewhere), and `cargo test -p vibe-wire`
  stayed green — the existing corpus document
  `formats/corpora/boot-artifact-transaction/e1/commit.toml` remains valid, so
  `[format.boot-artifact-transaction]` moves **1 → 1** and `SP-FMT-005`'s
  epoch question is settled. Adding the two `StageRole`s, their wire strings,
  two target constants and four payload fields on `ArtifactWrite` made these
  **twelve** files refuse: `transaction/prepare.rs` (the `Journal` literal),
  `transaction.rs`, `transaction/journal.rs`, `transaction/control.rs`,
  `boot_artifacts.rs`, `boot_artifacts/native_managed/write.rs`,
  `boot_artifacts/publication.rs` (×2),
  `install/bootgen/replay_publish.rs`, plus the test files
  `transaction/tests.rs` (×2), `transaction/safety_tests.rs` (×2),
  `transaction/lock_tests.rs`, `transaction/detailed_tests.rs`,
  `redirect/interruption_tests.rs` and `replay_publish/tests.rs`.
  **The real finding is what the compiler does *not* say.** Five further edits
  are required and are not compile-forced, because each enumerates journal
  entries *by name*: `transaction.rs::roll_forward_core` and
  `::rollback_all`, `transaction/durable_io.rs::discard_assets` and
  `::preflight_journal_paths`, and `boot_artifacts/inverse.rs:67`'s
  `lane = [INDEX_FILE, STATIC_FILE, STATIC_XML_FILE]` undo image. An
  extension that compiles can therefore still never roll the new roles
  forward, never roll them back, leak their stage files on failure, and leave
  them out of the operation-level undo. **Rollback needs no new inverse arm
  per role** — `apply_rollback` is data-driven over a `JournalEntry` — but it
  does need the two names in every by-name list. P4's risk is silence, not
  volume, so **P4 is L**.
- **S3 — self-inclusion, demonstrated. Answered: enrolment reproduces, the
  recursive growth does not.** A fixture node with no `[[project.sources]]`
  (so `legacy_entries` is the path taken), a boot directory holding
  `00-core.md`, `20-extra.md`, `SYSTEM-TEMPLATE.md` and `SYSTEM.md`, run
  through `apply_resolution_with_spec_format` twice.
  **Enrolled: yes, exactly as predicted** — `INDEX.md` listed **four** rows,
  each `kind = "static"`, including both new names. Every session would read
  the generated prefix a second time and the template's text twice.
  **Falsified: no recursive growth.** `lane_len = 0` on both runs and
  `index_len = 883` on both: a node's own authored boot is forced to
  `LinkType::Dynamic` (`boot.rs:314`/`:336`), so its bytes never enter the
  compiled lane — the files are only *named*. The marker text appeared in no
  generated artifact. A template written as PROP-076 says a template is
  written — literal text in no spec dialect — was admitted **without any
  error**, because an INDEX row is never parsed at generation time. So the
  hazard is silent doubled delivery, not a growing artifact: quieter than
  predicted and no less worth closing. `PROP-076#SYS-SELF-INCLUSION` amended;
  P1's gates are unchanged in scope.
- **S4 — which of the three compile seams this node uses. Answered: the host
  reaches the builtin seam only, and only two of the three ever publish a
  lane.** With the three `entries.is_empty()` gates instrumented: the real
  host node reached `compile.rs:86` only, with 26 static inputs and a 251,817
  byte artifact byte-equal to the committed `STATIC.xml`. Across
  vibe-workspace's whole lib suite the builtin seam fired 206 times and the
  owner-managed seam 208, in **disjoint** tests (a `native_managed`-filtered
  run: owner-managed 23, builtin 0); the S3 fixture reached **no** seam,
  because own dynamic boot leaves `static_entries()` empty.
  **Exclusivity is structural, not statistical:** the seam is chosen once per
  regeneration, not per node — `install/bootgen.rs:242` drives every node
  through the builtin loop and `:262` drives every node through
  `install/bootgen/native_managed.rs:39` instead — so no node mixes them in
  one run. And the custom-backend seam is **not a lane publisher at all**: it
  compiles `artifacts/<backend>` and is reached only through the
  `compile_node_backend*` analyser queries (`analyze.rs:415`, exported from
  `install.rs:98`), firing exactly once in the suite. **P3/P4 hook two
  publication seams, not three** — `boot_artifacts.rs:365` and
  `native_managed/write.rs:91`, as `SP-FMT-005` already names.
- **S5 — the read-only preflight, measured. Answered, and it is a real
  budget.** Provider-free `analyze_node_lane(workspace, ".", None)` on the
  complete host checkout succeeds with **no provider**: `Ok(Some(_))`, 251,817
  bytes, 26 inputs, **0 unattributable**. Warm timing over three consecutive
  runs: **341.8 ms / 328.2 ms / 338.9 ms**, after a 1.08 ms `Workspace::load`.
  That is too slow to sit unconditionally in an API caller's request path, so
  **P5's stated risk is live**: `prepare` and `check` must answer from the
  sidecar's witnesses and escalate to analysis only on a mismatch, and that is
  a design decision for P5 rather than a later performance fix.
  The spike also produced the first concrete non-freshness refusal for free:
  on an incomplete checkout (this worktree, whose `vibedeps/org.vibevm.doc.web`
  slot is absent) it refuses in **1.7 ms** with `selected slot no-follow
  admission refused`, before any compile. That is a world-integrity failure,
  so it belongs to `unavailable` with its path, never to "no lane" and never
  to `freshness_unverifiable`. Recorded in
  `PROP-076#SYS-READ-ONLY-CHECK`.
- **S6 — is there a fake provider to reuse? Answered: yes, and it is already
  better than what P6 planned to build. P6 stays M.**
  `crates/vibe-cli/tests/common/agent_provider.rs` `MockProvider` is a
  loopback OpenAI-compatible endpoint that counts hits **and retains the exact
  request bodies** (`bodies: Arc<Mutex<Vec<String>>>`, kept precisely so a
  test can prove which prompt bytes were sent), answers with a canned `usage`
  block, and stops and joins its listener on drop. The double-injection
  assertion of `PROP-076#SYS-NO-DOUBLE-INJECTION` is therefore a substring
  count over `bodies()`, and the worker-profile assertion is the same call.
  Also available and not needed here: `tests/common/http_dir.rs`
  `DirectoryServer` (a contact-counting file server) and
  `tests/common/net_fixture.rs` `FileServer` / `HttpServer`. P6 writes no new
  provider fixture; it adds the envelope and the assertions.

## 4. Phases and gates

Every phase after 0 ends with the same gate: targeted tests for the touched
owners, `cargo fmt --check`, strict clippy, `cargo xtask conform check`
(0 new), `cargo xtask check-codegen`, `cargo xtask wire-diff`,
`cargo check --workspace --all-targets --target x86_64-pc-windows-gnu`; the
final phase runs the full workspace suite plus `tools/self-check.sh`. Each
phase is one or a few atomic commits, and every user-visible change gets an
`SP-FMT-NNN` entry in this directory's `FORMAT-CHANGES.md`.

### P1 — the contract: roles, reservations, refusals — **M** — **DONE 2026-11-10**

**Goal.** The three paths exist as *roles* with no generation yet: the two
generated outputs and the one authored input are known to the layout owner, are
excluded from every boot-enrolment path, and are refused as declared sources;
`[system_prompt]` parses and validates. Nothing is written to a boot directory.

**Files.** `crates/vibe-core/src/layout.rs` (the three names, the three
accessors, the two generated paths into the private `generated_boot_paths`, and
the new `is_reserved_boot_authored_input` predicate for the template);
`crates/vibe-workspace/src/install/bootgen/own_boot.rs` (`legacy_entries`
excludes all three, the authored one through the new predicate);
`crates/vibe-spec/src/source_policy/authored_boot.rs` (the explicit arm for the
authored template; the two generated paths are already covered by `:97`);
`crates/vibe-core/src/manifest/document.rs`,
`…/manifest/package/visibility.rs`, `…/visibility/conversion.rs` (the
`[system_prompt]` table in `Manifest`, `ManifestWire` and both `TryFrom` arms);
a new `…/manifest/package/system_prompt.rs` (+ `system_prompt/validation.rs`,
`system_prompt/tests.rs`) for the typed section;
`…/manifest/document/validation.rs` + a new
`…/document/validation/system_prompt.rs` (the refusals, each naming its TOML
path).

**New types.** `SystemPromptSection { entrypoint: EntrypointChoice, lane_form:
LaneForm, recipe: Option<u32> }`; `EntrypointChoice::{Default, File(RedirectFile),
None}` over the existing closed `REDIRECT_FILES` set; `LaneForm::Verbatim`
(one variant, reserved for `Projected`).

**Commands.** None new.

**Formats.** `SP-FMT-001` (the authored template's reserved role),
`SP-FMT-002` (`[system_prompt]`).

**Tests.** Unit, `vibe-core`: the table round-trips; `[system_prompt]` with no
template beside it refuses naming the TOML path; an `entrypoint` outside the
closed set refuses naming the three legal spellings; `lane_form = "projected"`
refuses as not-yet-supported rather than silently behaving as verbatim; every
refusal names a TOML path. Unit, `layout`: each of the three paths is
classified in exactly one role, and an ASCII case alias of each generated path
is refused as a source while the authored template keeps its own predicate.
Unit, `own_boot`: a boot directory containing all three plus two real snippets
enrols exactly the two snippets — **the regression test for S3's demonstrated
hazard**. Unit, `authored_boot`: a declared source row naming any of the three,
in any case spelling, refuses before admission.

**Acceptance.**
```sh
cargo test -p vibe-core -p vibe-spec -p vibe-workspace --offline --locked
cargo test -p vibe-workspace own_boot -- --nocapture
```

**Risks.** The template's "authored but not enrolled" role is the one that can
be got wrong by convenience: labelling it generated would suppress the scan in
one line and then delete the user's file on the next format change. The test
that an opt-out removes only the two generated outputs lands in P4, so P1's own
guard is the three-way classification unit test.

**Parallel.** First. Nothing else may start.

### P2 — capture: witnessed inputs and the prospective entrypoint — **M** — **DONE 2026-11-10**

**Goal.** For an opted-in node, produce an in-memory, fully witnessed input set
for the prefix — template bytes, entrypoint bytes with the *prospective*
managed-block projection, the admitted lane artifact — plus the classified
continuation plan. Still nothing written.

**Files.** New `crates/vibe-workspace/src/system_prompt.rs` with
`system_prompt/capture.rs`; `crates/vibe-workspace/src/boot_artifacts/redirect.rs`
(expose the prospective projection as a value instead of only writing it — the
block body is already computed before the write);
`crates/vibe-workspace/src/boot_artifacts/declared_sources.rs` and
`…/declared_sources/nodes.rs` (bind the two new authored reads under the held
workspace, through the existing guard/recheck path);
`crates/vibe-workspace/src/install/bootgen/analyze.rs` (one accessor for the
admitted lane bytes beside the existing `AnalyzedLane`).

**New types.** `CapturedPrefixInputs { node_rel, entrypoint: Option<CapturedInput>,
template: CapturedInput, lane: Option<CapturedLane>, plan: ContinuationPlan }`;
`CapturedInput { role, relative, bytes, sha256 }`;
`ContinuationPlan { rows: Vec<ContinuationRow> }` with
`ContinuationRow { relative, band: Unconditional | Gated(WhenCondition) }`
derived from `EffectiveBoot::dynamic_entries()` in INDEX order.

**Commands.** None new.

**Formats.** `SP-FMT-101` (the `when = "target:<os>"` gate spelling, from the
owner decision of 2026-11-10). A third variant in the closed condition set of
`crates/vibe-core/src/manifest/package/when.rs:71-112`, resolved at generation
time beside `installed:` in `install/bootgen/conditions.rs:86-96` against
`[system_prompt] target`, and therefore **never** a residual `when` — so it
cannot reach band G and `prepare_index_body`'s `kind` expression keeps its
present meaning. It lands here because this phase already reads
`conditions.rs` to classify the continuation plan. Default (absent `target`)
selects every OS, so no existing row changes.

**Tests.** Unit: the plan's order equals `prepare_index_body`'s row order for
the same `EffectiveBoot`, asserted against the function rather than against a
rendered string; a gated entry lands in `Gated` with its `when` verbatim and an
unconditional one in `Unconditional`; an `installed:` condition never appears
in the plan, because `conditions.rs` resolved it. Unit: the entrypoint capture
equals the *post-write* bytes for a node whose redirect this generation would
change — **the T3 regression test**: witnessing the pre-write file must fail
this test. Unit: a node with no static lane captures `lane: None` and still
produces a valid input set; a node with both `STATIC` spellings propagates
`resolve_static_path`'s refusal unchanged. Unit, sentinels: a boot directory
whose authored text mentions `~/.vibe/steward/...`, a token path and a plan
file leaves all three unopened — asserted by a read-counting filesystem
observer, not by inspecting output.

**Acceptance.**
```sh
cargo test -p vibe-workspace --offline --locked system_prompt::capture
cargo test -p vibe-workspace --offline --locked redirect
```

**Risks.** Exposing the prospective redirect projection must not become a
second renderer: the value and the written bytes come from one function or they
will drift, and a test asserts they are equal for the same inputs.

**Parallel.** Needs P1.

**As delivered.** Three commits, in this order: the `target:` gate spelling in
`vibe-core`; its generation-time resolution in `install/bootgen/conditions.rs`,
with `installed:` and `target:` travelling together as one condition world; the
managed redirect block exposed as a value; and
`crates/vibe-workspace/src/system_prompt/` — `capture.rs`, `plan.rs`,
`reader.rs` and the module root — plus one `DeclaredBootSources::node_input`
that observes a node-relative authored input under the held workspace with the
prepared-node guard rechecked before and after. `AnalyzedLane` gained
`lane_bytes()`, and `analyze.rs` was split along its result/behaviour seam
(`analyze/result.rs`) to stay inside the file budget. Sixteen capture tests,
six condition tests, three end-to-end `target:` tests and three projection
tests.

**Four decisions the plan did not settle, each recorded in `FORMAT-CHANGES.md`
under `SP-FMT-101` and in `LOG.md`.** (1) A continuation row carries the path
exactly as the `INDEX.md` row spells it — **workspace-root-relative**, not
node-relative as `PLAN.md` wrote — because a dependency row names a path under
`vibedeps/` that lies beneath no member node, so a node-relative spelling could
not express it and re-basing would make the plan disagree with the artifact a
session reads. (2) The `target:` selection is resolved **once per
regeneration** from the project's own manifest rather than per node, because
the unit table that carries each contribution's residual `when` is built once
before the node loop and a package unit's lane is published once for the whole
resolution. (3) The generated `INDEX.md` header prose was **left untouched**,
so this repository's `INDEX.md` is byte-identical rather than merely
row-identical: the header describes the grammar a reader may meet *in that
file*, and a `target:` row is never in it. (4) Capture is reached through a
`PrefixReader` seam rather than by calling the filesystem directly, because
`#SYS-NO-HOME-EXPANSION` is a claim about which files are opened and a claim
about reads is proved by counting reads.

**One fact moved further than planned.** `PROP-076#SYS-SELF-INCLUSION` is now
`impl/done`: its last clause — an output listed among its own inputs — had no
generator to bind in P1 and now has one, so capture refuses a continuation row
that resolves to any reserved path inside *this* node's own boot directory, in
any ASCII case spelling. The test is a file identity, not a string: a
dependency slot's own generated lane is untouched.

### P2b — the managed block becomes a router — **L** — *re-planned 2026-11-10*

*(New. `HARNESS-DELIVERY.md` §12 assigns three items to "P2 capture" — the
router block as the prospective entrypoint's managed block, the expected
`STATIC` hash in it, and managed-block neutralisation in `vibedeps/` copies.
They arrived after P2's scope was fixed and they are **not** capture: each one
rewrites the body of every `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` in every
project — 19 tracked files in this repository — and the expected hash must be
the same digest the lane markers use, which the composer owns. So they are
their own atom, placed after P3 where that grammar is settled, and nothing was
silently dropped.)*

**Goal.** The managed block stops being "read these files in order" and becomes
the router of `PROP-076#SYS-NO-RECURSION`: scoped to its own project, keyed on
marker presence anywhere in context rather than on position, carrying the
expected lane hash, and byte-identical between `read` and `launch` delivery.

**Files.** `crates/vibe-workspace/src/boot_artifacts/redirect.rs` (the new body,
its node-id and its expected-hash parameter — one renderer, which P2 already
made the single source of both the written bytes and the witness);
`crates/vibe-workspace/src/boot_artifacts.rs` and
`…/boot_artifacts/native_managed/write.rs` (thread the compiled lane's digest
to the selector that writes the block); `crates/vibe-workspace/src/vibedeps/`
(neutralise the block in materialised copies);
`crates/vibe-check/src/checks/` (the nested-managed-block warning); and the
regenerated instruction files of this repository.

**Formats.** `SP-FMT-105` (the block shape half).

**Tests.** The block's exact bytes, per harness mode. Codex's limit: the whole
block well under 32 KiB, and a refusal when it would fall outside
`project_doc_max_bytes`. Idempotence: two regenerations leave all three copies
byte-identical, and the three stay byte-identical to each other. The expected
hash follows `STATIC.*` and nothing else — editing `90-user.xml` changes no
instruction file. A `vibedeps/` copy carries a neutralised block. `vibe check`
warns about a nested managed block reachable by downward discovery.

**Risks.** This is the phase that edits 19 tracked files and every downstream
project's instruction files in one commit, and the diff is unreviewable by
reading. The guard is that the body is generated from one function with a
golden test over its bytes, so the review is of that function and that golden.
Second risk: the router's wording depends on an unanswered question — what a
harness does with a **missing** imported file — which is why the import tier is
reserved (`#SYS-IMPORT-INERTNESS`) and the router alone ships first.

**Parallel.** Needs P3's marker grammar. Independent of P4.

### P3 — compose: deterministic assembly, segments, sidecar — **L**

**Goal.** A pure function from `CapturedPrefixInputs` to the prefix bytes, the
segment map and the sidecar — byte-identical for identical inputs, with no
clock, no absolute path and no host observation anywhere in it.

**Files.** New `crates/vibe-workspace/src/system_prompt/compose.rs` (the pure
assembly) and `…/system_prompt/contract.rs` (the versioned transport-contract
recipe, its text and its recipe number); `schemas/system_prompt_meta.jtd.json`
(new); `formats/REGISTRY.toml` (`[format.boot-system-prompt]`,
`[format.boot-system-template]`, `[format.boot-system-meta]`);
`crates/vibe-wire/src/generated/**` via `cargo xtask codegen`.

**New types.** `ComposedPrefix { bytes: Vec<u8>, sha256, segments: Vec<Segment> }`;
`Segment { id: SegmentId, offset, len, cache_boundary: bool }` with
layer-qualified ids (`project.template`, `project.entrypoint`,
`project.lane`); `SystemPromptMeta` (the sidecar, generated from the JTD).

**Commands.** None new.

**Formats.** `SP-FMT-003` (`SYSTEM.md`), `SP-FMT-004` (the sidecar,
`vibe-system-prompt/1`), `SP-FMT-009` (the transport-contract recipe as a
versioned surface), and — added 2026-11-10 by the owner decisions —
`SP-FMT-103` (`AGENT-LANE.md`, the second composed artifact) and `SP-FMT-104`
(the lane segment markers).

**Added 2026-11-10 — this phase composes two artifacts, not one.** Beside the
API prefix it composes the **agent lane** of `PROP-076#SYS-AGENT-LANE`:
`STATIC.*` then every unconditional INDEX file in INDEX order, each wrapped in
the begin/end marker pair of `#SYS-LANE-MARKERS` (`<node-id>`, project-relative
path, 12 hex of the segment's own SHA-256), with no template, no entrypoint, no
router and no import line. The marker grammar is settled here because P2b's
expected hash must be the same digest, and the composer additionally
**verifies** that the composed `SYSTEM.md` contains neither an `@` import line
nor a router — so even a mistaken import of the prefix cannot recurse
(`#SYS-NO-RECURSION`). The fixed-host baking of `os:` rows is shared between
the two composers. Extra tests: every begin has its end, pairs are unique,
order matches INDEX, each hash matches its own bytes, and no marker-shaped line
occurs inside a segment's own bytes. Also from the owner decision of 2026-11-10: under a fixed
`[system_prompt] host` the composer resolves `os:` rows at build time and
bakes the matching ones into the prefix, leaving band G empty; the sidecar
records the declared `host` and `target`, which are authored manifest values
and **not** environment detection, so `#SYS-NO-VOLATILE` and
`#SYS-DETERMINISM` are untouched.

**Tests.** Unit, purity: `compose.rs`'s own sources and dependency surface
reach no filesystem, socket, clock, child process or environment variable —
pinned by a test over the module's imports, the way
`crates/vibe-template/tests/purity.rs` pins its crate. Unit, determinism: a
property test over many synthetic input sets asserts the same inputs give
byte-identical bytes and that no two distinct input sets collide on
`prefix_sha256`. Unit, framing: a CRLF template keeps CRLF inside its own
segment while the frames stay LF; the output ends with exactly one LF; a
template whose last byte is not a newline gets exactly one inserted; UTF-8 with
no BOM. Unit, volatility: a golden assertion that the bytes contain no
timestamp, no absolute path, no `HEAD`, no digest and no OS name — and the
dual, that changing the clock, the cwd and the host OS between two composes
changes nothing. Unit, offsets: every segment's `offset`/`len` slices exactly
its own body out of the composed bytes. Unit, layering: the composed bytes of
the project layer are a byte-identical **prefix** of
`project_bytes + arbitrary_following_bytes` — the trivial but load-bearing
assertion `PROP-076#SYS-LAYER-CONCATENATION` makes. Golden: the host
repository's own three inputs compose to a recorded digest and a recorded
length, so a later change to the recipe cannot pass unnoticed.

**Acceptance.**
```sh
cargo test -p vibe-workspace --offline --locked system_prompt::compose
cargo test -p vibe-workspace --offline --locked system_prompt::compose::determinism -- --exact
cargo xtask check-codegen && cargo xtask wire-diff
```

**Risks.** The sidecar's "no continuation content digest" rule is the one a
future contributor will want to break, because a digest there would make
`check` strictly stronger — and would also make every edit of `90-user.xml`
dirty a committed file. The rule gets its own named test and a comment pointing
at `PROP-076#SYS-CONTINUATION-WITNESS-IS-RUNTIME`. Second risk: the
transport-contract text is prose, and prose drifts; pinning it behind a recipe
number with a golden test is what keeps a wording change from silently moving
every prefix digest in the world.

**Parallel.** Needs P2.

### P4 — publish: generated roles, the ignore file, removal — **M** — *re-planned 2026-11-10*

*(Sized M while planning; spike S2 moved it to L; the owner decision
`PROP-076#SYS-ARTIFACTS-NOT-TRACKED` moves it back to **M**, for a reason worth
stating. S2's finding stands in full — twelve files refuse to compile and five
more need by-name edits the compiler never asks for — but the decision that the
generated artifacts are **not git-tracked** removes the most expensive half of
this phase's semantics. An untracked, always-derivable artifact needs no
committed-artifact ownership-conflict machinery: there is no user's hand-edited
`SYSTEM.md` to protect in a tracked file, and the honest answer to a file whose
bytes do not match the sidecar is to rebuild it. What stays is exactly the
transaction safety, which is not negotiable, plus the `.gitignore` writer.)*

**Goal.** The two outputs are written inside the existing boot transaction as
one coherent generation with INDEX, STATIC and the redirects; a failure leaves
the previous set intact; equal bytes are not rewritten; an unowned `SYSTEM.md`
is a refusal.

**Files.** `crates/vibe-workspace/src/boot_artifacts/transaction/journal.rs`
(two `StageRole`s, two admitted production targets, the validation and the
`common_parent` set); `…/transaction/prepare.rs` (staging the two payloads and
the removal case); `…/transaction.rs` (`ArtifactWrite` gains the two optional
payloads; `apply_forward` needs no change — its equal-bytes early return
already covers them); `crates/vibe-workspace/src/boot_artifacts/inverse.rs` (the
rollback arms, if S2 says they are per-role);
`crates/vibe-workspace/src/boot_artifacts.rs` (`write_boot_artifacts_inner`
composes and stages the pair before the commit);
`crates/vibe-workspace/src/boot_artifacts/native_managed/write.rs` (the same,
for the owner-managed seam); new
`crates/vibe-workspace/src/system_prompt/publish.rs` (ownership proof,
conflict and removal).

**Commands.** None new.

**Formats.** `SP-FMT-005` (the transaction roles),
`SP-FMT-010` (removal and ownership semantics, re-scoped below),
`SP-FMT-102` (the untracked artifacts and the vibe-written `.gitignore`),
`SP-FMT-103` (`AGENT-LANE.md` as a generated role).

**Re-planned 2026-11-10 by `PROP-076#SYS-ARTIFACTS-NOT-TRACKED`.** Three
changes to this phase. **(1) Three generated roles, not two:** `SYSTEM.md`,
`SYSTEM.meta.json` and `AGENT-LANE.md`. **(2) A `.gitignore` writer**, owned by
this phase: `vibe` writes the ignore file into the boot directory it already
owns and never touches the repository root's, which is the developer's. It is
written inside the same transaction as the artifacts it covers, because an
artifact that exists before its ignore line is an artifact a `git add -A`
commits. **(3) The ownership-conflict machinery shrinks to a rebuild.**
`SP-FMT-010`'s named refusal existed to protect a *tracked* file that might be
a user's own work; an untracked, derivable artifact whose bytes disagree with
its sidecar is simply stale, and the answer is to rebuild it. What survives
unchanged: the removal case (opting out removes exactly the three outputs the
sidecar proves vibe owns), the stale-spelling removal, and every interruption
and rollback test. A `vibe check` **error** when git tracks a generated prefix
is new here, and is the one place the decision is enforced rather than
documented.

**Tests.** Unit: the role validation admits exactly the five production targets
and refuses a sixth; `common_parent` still holds. CLI/integration with an
isolated HOME over a fixture workspace: an opted-in node publishes all five
artifacts in one commit; a compile failure after the prefix is composed leaves
**all** previous artifacts untouched; a crash between the INDEX commit and the
selector phase recovers to a coherent set through `recover_pending_locked`; a
second run with unchanged inputs writes nothing and leaves five mtimes
unchanged; removing the template removes exactly the two generated outputs and
nothing else; a hand-edited `SYSTEM.md` refuses with the named conflict and is
neither overwritten nor deleted; a `SYSTEM.md` present with no sidecar refuses
the same way; a nonroot authored package member selected as a dependency gets
**no** `SYSTEM*` written into it, at a drifted authored version — the
`publish_node_boot` protection asserted for the new roles.

**Acceptance.**
```sh
cargo test -p vibe-workspace --offline --locked boot_artifacts::transaction
cargo test -p vibe-workspace --offline --locked system_prompt::publish
cargo test -p vibe-workspace --offline --locked safety_tests
```

**Risks.** This is the phase that can corrupt a user's boot directory, so the
existing `safety_tests.rs` (450 lines) and `interruption_tests.rs` are extended
rather than bypassed, and the interruption matrix must cover a failure in each
of the five roles. S2 exists to find out, before the phase starts, whether the
rollback path needs a new arm per role; if it does, the phase is L.

**Parallel.** Needs P3.

### P5 — operations: install-time generation and `vibe prompt` — **L**

**Goal.** Generation happens inside the paths that already publish boot
artifacts, and three explicit verbs exist: `build`, `check`, `prepare`. `check`
and `prepare` are provably read-only and effect-free.

**Files.** `crates/vibe-workspace/src/install/bootgen.rs` (the per-node branch
for an opted-in node, after `publish_node_boot`'s decision);
new `crates/vibe-workspace/src/system_prompt/check.rs` and
`…/system_prompt/prepare.rs`; new `crates/vibe-cli/src/cli/prompt.rs` and
`crates/vibe-cli/src/commands/prompt.rs` (+ `prompt/{build,check,prepare}.rs`);
`crates/vibe-cli/src/cli/command.rs` (the `Prompt` variant),
`crates/vibe-cli/src/commands/mod.rs` (the dispatch arm),
`crates/vibe-cli/src/command_progress.rs` (the progress classification — a
non-exhaustive match, so a forgotten arm is a compile error),
`crates/vibe-cli/src/output.rs` (the JSON envelope);
`schemas/prompt_check.jtd.json` and `schemas/prompt_prepare.jtd.json` (new);
`formats/REGISTRY.toml`; `crates/vibe-check/src/checks/system_prompt.rs` (new)
plus `…/checks/mod.rs` and `…/lib.rs` (the `CheckId`).

**New types / functions.** `CheckVerdict::{Equal, Stale(Vec<Reason>), Missing,
Conflict(Reason), Unavailable(Reason), FreshnessUnverifiable(Reason)}`;
`PreparedRequest { prefix: PreparedBand, unconditional: Vec<PreparedBand>,
gated: Vec<PreparedBand>, unresolved: Vec<UnresolvedRow>, cache_boundaries:
Vec<usize> }` where each band carries its relative path, its observed bytes'
digest and length — and `prepare` returns the bands **separately**, never
concatenated.

**Commands.** `vibe prompt build [--node <rel>]`;
`vibe prompt check [--node <rel>] [--json]`;
`vibe prompt prepare --json [--node <rel>] [--session-target os=<name>]`.

**Formats.** `SP-FMT-006` (`vibe-prompt-check/1`),
`SP-FMT-007` (`vibe-prompt-prepare/1`),
`SP-FMT-008` (the `vibe check` check id), and — added 2026-11-10 —
`SP-FMT-106` (`vibe prompt path`, `vibe prompt flags`, the build summary line)
and the parsing half of `SP-FMT-105` (`[system_prompt.harness.<name>]`).

**Added 2026-11-10 by the owner decisions.** Two more verbs and one line.
**`vibe prompt path [--part lane|system] [--json]`** prints the absolute path of
a generated artifact — two parts because there are two artifacts — and reports a
stale or missing one *as* stale or missing with the exact `vibe prompt build`
that fixes it, **never as an empty path**, which a shell alias would turn
silently into a broken launch. **`vibe prompt flags --harness <name>`** prints
ready-to-paste launch flags. **Every build ends with one summary line** naming
what happened and where — `system prompt: rebuilt → <path> (<size>), lane →
<path> (<size>)` or `system prompt: unchanged → <path>` — from `vibe install`
as well as `vibe prompt build`, with a failed or opted-out build saying so
explicitly, and the same facts in the `--json` report. Also here: the
`[system_prompt.harness.*]` table and constraints C1–C4 of
`HARNESS-DELIVERY.md` §4 (including the refusal of `import` in v1), the
host-mismatch warning, and the `vibe prompt check` additions of its §8.5 —
marker integrity, the block's expected hash against the current lane, `@`
tokens in lane bytes with their count and locations, entrypoint size against
Codex's 32 KiB limit, nested and un-neutralised managed blocks, entrypoint
copies that are not byte-identical, and lane size against a stated context
size.

**Tests.** Unit: the verdict for each of the six cases, each naming its reason;
a changed template, entrypoint or lane input yields `Stale` naming which;
changed bytes of an unconditional continuation row under an unchanged plan
yields `Equal` — the single most important assertion in the phase; a changed
plan (a row added, removed, reordered, or a condition changed) does **not**
yield `Equal`. Unit: a pending native unit yields `FreshnessUnverifiable` with
the exact `vibe prompt build` to run, and the effectful provider is proved
never called by a provider sentinel that panics on invocation. CLI with an
isolated HOME: `vibe prompt check` makes zero writes and zero connections,
asserted by a write-counting filesystem wrapper and a network sentinel;
`vibe prompt prepare` opens exactly the prefix inputs plus the applicable
continuation rows and nothing else, asserted by a read-recording observer;
`prepare` with a non-empty band G and no `--session-target` refuses as
unresolved input rather than using the host OS; an unknown session target
refuses; `vibe install` on an opted-in fixture writes the pair, and on a
fixture with no template writes exactly the artifacts it writes today —
asserted by comparing the full boot directory listing and bytes against a
pre-change golden.

**Acceptance.**
```sh
cargo test -p vibe-workspace -p vibe-check --offline --locked system_prompt
cargo test -p vibe-cli --test cli_prompt --offline --locked
cargo test -p vibe-cli --test cli_prompt prepare_reads_only_the_named_inputs -- --exact
cargo test -p vibe-cli --test cli_prompt check_is_read_only -- --exact
```

**Risks.** `prepare` sits in an API caller's request path, so S5's measured
time is a real budget; if provider-free `analyze_node_lane` is slow on this
repository, `prepare` must be able to answer from the sidecar's witnesses alone
and escalate to analysis only on a mismatch, and that is a design change to
make in this phase rather than a performance fix later. Second risk: adding a
branch to `bootgen.rs`'s node loop is adding code to the path that publishes
every project's boot; the golden "a project without the opt-in is byte-identical
to before" test is the guard, and it goes in first.

**Parallel.** Needs P4.

### P6 — adapters: the request envelope and cache diagnostics — **M**

**Goal.** One supported adapter path places the bands in the right roles, in
the right order, exactly once — and reads the provider's own usage counters
back, without this campaign ever calling a paid API.

**Files.** New `crates/vibe-cli/src/commands/prompt/adapter.rs` (the envelope
construction from a `PreparedRequest`); a new test-only fake provider under
`crates/vibe-cli/tests/common/` (or the fixture S6 found);
`schemas/prompt_request_capture.jtd.json` (new); `formats/REGISTRY.toml`.

**New types.** `RequestEnvelope { instruction_blocks: Vec<Block>, tools:
Vec<ToolRef>, messages: Vec<Message>, cache_boundaries: Vec<usize> }`;
`CacheDiagnostics { write_tokens, read_tokens, uncached_tokens, source:
ProviderUsage }` — every field read from a provider response, never computed
locally and never estimated.

**Commands.** `vibe prompt prepare --json` gains the envelope projection; no
command in this campaign sends a request. **Added 2026-11-10:** `vibe agent
<launcher> [--] [args…]` — composes the agent lane, checks the delivery-mode
constraints, starts the launcher's harness with its **file-based** slot (no lane bytes on a
command line, on any operating system: one argument is capped at 128 KiB on
Linux and the whole line at ~32,767 characters on Windows, against a 310 KB
lane), and passes everything after the launcher name through. Claude Code gets
`--append-system-prompt-file <lane>` plus
`--exclude-dynamic-system-prompt-sections`, and a refusal when the user's own
arguments already set a system prompt; Codex gets a launch-time profile file,
which `vibe agent` may write as `$CODEX_HOME/vibe-<node-id>.config.toml` and an
install never writes. A harness in `read` or `import` mode is started unchanged
and told so in one line. `vibe agent` is a thin layer over `vibe prompt flags`,
so there is one code path.

**Added 2026-11-10 — a launcher is a named thing with a kind**
(`PROP-076#SYS-HARNESS-EXECUTABLE`, which amends `HARNESS-DELIVERY.md` §8.4
where the programs are named directly). A **kind** — `claude`, `codex`, …, a
closed set — is the flag dialect and the delivery rules. A **launcher** is a
name bound to a kind and to the argv that starts it, and it lives in the
**user** configuration only:

```toml
[agent.launchers.claude2]
kind = "claude"            # flag dialect and delivery rules; the closed set
command = ["claude2"]      # argv prefix to exec
config_dir = "~/.claude2"  # optional: where its session logs live
```

`vibe agent <launcher-name> [args…]` resolves the launcher **by name**, takes
its flags from its kind, and execs `command` + flags + the user's own
arguments. The built-in launchers `claude` and `codex` always exist, each with
`kind` equal to its name, and a user launcher of the same name overrides the
built-in. Any number of launchers may share a kind — two Claude Code launchers
with different `config_dir`s are ordinary, not an edge case. `vibe agent list`
prints name, kind, command, config directory and **source** (built-in, or the
exact user-config path), because a developer debugging which program ran needs
to know which file decided it. An unknown name is a refusal **listing the
available launchers**; an unknown kind is a refusal naming the field.
`vibe prompt flags` accepts either a kind or a launcher name and keeps printing
**flags only**, which is what lets its output be appended to a wrapper that
already exists. **A project manifest that names a launcher is a refusal** — a
repository must not choose which program runs on a contributor's machine.
`config_dir` exists because a wrapper that sets `CLAUDE_CONFIG_DIR` internally
makes the log directory unknowable from outside; absent it, vibe reads the
environment, then the kind's default, and when it is still unknown the
diagnostics say so rather than guessing a path and reading someone else's logs.
Two notes for whoever implements it: the environment read belongs at the CLI
composition root that `conform.toml`'s `env_roots` already sanctions, not in the
adapter; and the exec test is a shim on `PATH` that records its own argv, which
is the only way to prove an argv prefix was honoured rather than merely stored.

**Formats.** `SP-FMT-011` (the capture envelope and the usage counters).

**Tests.** Against the fake provider only: the instruction role carries the
prefix exactly once and the entrypoint's text appears exactly once in the whole
request — the double-injection test; the band order is P, U, G, T and the tool
block sits where the adapter declares it; a declared cache boundary lands at a
byte offset the segment map named; a worker profile produces a request with
**no** central prefix at all, asserted on the captured bytes. Unit: the
diagnostics type cannot be constructed from a local estimate — there is no
constructor that does not take a provider usage record, which is a type-level
guarantee rather than a comment. **Explicitly not tested here:** a real cache
hit, a TTL, a latency and a cost. The fake-provider capture is not a cache
claim, and the test names say so.

**Acceptance.**
```sh
cargo test -p vibe-cli --test cli_prompt_adapter --offline --locked
cargo test -p vibe-cli --test cli_prompt_adapter no_duplicate_entrypoint_injection -- --exact
cargo test -p vibe-cli --test cli_prompt_adapter worker_profile_carries_no_central_prefix -- --exact
```

**Risks.** The honest one: an envelope that is right against a fake provider can
still be wrong against a real one, because provider wrapper text, tool
serialisation and model-specific framing are outside this repository. The phase
therefore claims only what it proves — the bytes vibe produced — and
`PROP-076#SYS-CACHE-FACTS` states the rest as provider-owned.

**Parallel.** Needs P5. Independent of P7's fixtures but feeds its matrix.

### P7 — qualify: differential boot parity and the matrix — **L**

**Goal.** The campaign's central claim, as a mechanical test: the prefix plus
the executed continuation delivers the same instruction text, in the same
order, as today's file-reading boot — modulo exactly the managed-block
substitution.

**Files.** New `crates/vibe-workspace/tests/system_prompt_parity.rs` (the
differential test over fixture workspaces); new fixture workspaces under
`crates/vibe-workspace/tests/fixtures/system_prompt/` (a node with gated rows
for all three operating systems, a node with no static lane, a node with both
STATIC spellings, a root with members, a member with inherited foundation, a
node with an extension world); new `crates/vibe-cli/tests/cli_prompt_matrix.rs`.

**Commands.** None new.

**Formats.** None.

**Added 2026-11-10.** Offline marker and block tests; the nested-project
fixture (a sub-project with its own managed block, proving the router's
self-scoping and the `vibedeps/` neutralisation); a two-instruction-file
configuration case (`AGENTS.md` plus `QWEN.md`), proving that concatenation
order cannot change the router's verdict because its test is presence anywhere
rather than position; and, where Codex is installed, the free placement test
`codex debug prompt-input` through the profile file, which answers most Codex
placement questions with no provider spend at all.

**Tests.** The differential: for each fixture, build the reference stream the
way the published boot contract says an agent reads it — the entrypoint, then
the static lane, then every applicable INDEX entry in order — and compare it
against the prefix plus `prepare`'s bands, asserting equality after exactly one
declared normalisation (the managed block's bytes versus the transport
contract's), with the normalisation applied to a named byte range rather than by
a regex over the document. The matrix: Linux, macOS and Windows session targets
against a fixture whose rows are gated for each; `installed:`-present and
`installed:`-absent resolution; root, member and specspace scope; a quiet-worker
profile. Stale refusal: each of the three prefix inputs mutated in turn gives a
named `Stale`, and an attempted send of a stale prefix is refused by the
adapter, not merely warned about. Privacy sentinels: the home plan, custody and
token files exist in the fixture and are proved unopened and absent from every
output. No-growth: ten consecutive regenerations leave the prefix length
unchanged. Cross-directory determinism: the same fixture at two paths gives
equal bytes, and the Windows target leg runs under
`cargo check --target x86_64-pc-windows-gnu` plus the existing cross-platform
test discipline.

**Acceptance.**
```sh
cargo test -p vibe-workspace --test system_prompt_parity --offline --locked
cargo test -p vibe-workspace --test system_prompt_parity boot_parity_is_exact -- --exact
cargo test -p vibe-cli --test cli_prompt_matrix --offline --locked
tools/self-check.sh
```

**Risks.** The reference stream is the test's weak point: if it is built from
the same code as the thing under test, the test proves nothing. It must be
built from the *published contract* — the entrypoint, `STATIC`, then the
`INDEX.md` rows as a TOML document — reading the files directly, with no shared
helper. That is a stated construction rule for the test, and a reviewer's first
question about it.

**Parallel.** Needs P5 and P6.

### P8 — docs: the template, the limits, the economics — **S**

**Goal.** A user can write a template, understand exactly what invalidates
their prefix, and compute whether caching saves them anything — without
believing a number this campaign did not measure.

**Files.** A documented example template and a worked prefix under the
documentation package; the invalidation table of
`PROP-076#SYS-INVALIDATION` in user-facing form; the parametric cost model of
`#SYS-CACHE-ECONOMICS` with a worked example using *placeholder* rates and an
explicit "substitute current rates"; a migration note for a project adopting
the opt-in; and the limitations page — incomplete boot, no portable KV cache,
no promised hit, no TTL guarantee, adapter-dependent.

**Commands.** None new.

**Formats.** None.

**Tests.** The documented example template is a fixture that the P3 golden test
actually composes, so a documentation example that stopped working fails a
test. The documented invalidation table is generated from the same enum the
`Stale` reasons come from, so prose and code cannot drift.

**Acceptance.**
```sh
cargo test -p vibe-workspace --offline --locked system_prompt::docs_example
cargo xtask conform check
```

**Risks.** The only real one is overclaiming. Every number in the economics
section is a placeholder until a separately authorised measurement replaces it,
and the page says so in its first paragraph.

**Parallel.** Needs P7. Drafting may start after P3.

**Added 2026-11-10.** The delivery modes of `HARNESS-DELIVERY.md` in
user-facing form — the three tiers, the per-harness matrix and its recommended
defaults, why `import` is refused in version 1, and the risks of its §9 that
`vibe` cannot control: MCP tool-set changes ahead of the slot in the cache
hierarchy, a mid-session model switch, undocumented harness flags, a resumed
conversation's recorded snapshot, user-local instruction files outside vibe's
accounting, and a 310 KB lane that compaction cannot drop the way it drops tool
results.

### P9 — the live check, Claude Code only — **S** — *authorised, scoped*

*(New, 2026-11-10. `HARNESS-DELIVERY.md` §10, its decision D13, as the owner
narrowed it the same day: **authorised for Claude Code only**, to run **before
the release is published** and only once `vibe agent claude` — launch delivery —
exists. Codex, Qwen Code, Gemini CLI and every other harness stay
**unauthorised**, so their rows in §10 remain research and their recommended
default stays `read`. It is the campaign's **final step**, after P7, and it is
run by the **central session**: a worker neither starts an agent session nor
spends the owner's subscription. Nothing earlier depends on it, because every
phase's own acceptance is offline.)*

**Goal.** Answer the questions no offline test can: whether a harness delivers
a ~310 KB file whole or truncates it, what it does with a **missing** imported
file (silent skip or a literal line — the router's wording depends on it),
whether a cache read of the lane actually occurs on a second session, and
whether Claude Code subagents receive `CLAUDE.md` imports — which, if they do,
means a quiet worker would carry the prefix in context and must be recorded as
a known cost or excluded.

**Method.** A sample project in this campaign's fixtures — no personal data —
whose lane is padded to the real size with unique canary words: one in the last
line of `STATIC.xml`, one in each INDEX file, one in the entrypoint outside the
block, and the last `vibe:lane-end` line as the tail canary. One nested
sub-project with its own block and canaries, and `@spec`-style tokens in
`STATIC`. Two or three short sessions, Claude Code only. Delivery is read from
the answer; caching from the harness's own session logs under
`$CLAUDE_CONFIG_DIR/projects/*.jsonl` (`cache_read_input_tokens` /
`cache_creation_input_tokens`) — on this machine that directory is `~/.claude2`,
not `~/.claude`.

**Pass criteria and fallbacks** are the Claude Code rows (C1–C8) of
`HARNESS-DELIVERY.md` §10; the Codex and other-harness rows are not run under
this authorisation. The fallback is safe by construction: a failure of the
launch evidence means `launch` defaults back to `read`, and a failure of the
subagent test means `launch` must not be Claude Code's default at all — the
cost it would carry is a quiet worker holding the prefix in context, against
`PROP-076#SYS-WORKER-EXCLUSION`.

### Dependency order, as a graph

```
P0 spikes (S1..S6)
  |
  +--> P1 --> P2 --> P3 --> P4 --> P5 --> P6 --> P7 --> P8 --> (P9, if authorised)
             done     |      ^                                 ^
             done     +--> P2b                                 |
                      +---------- (P8 drafting) ---------------+
```

The chain is deliberately linear: every phase consumes the previous one's
output, and the three places where that is not strictly true (P8's drafting,
P6's fixture work, and P2b, which needs only P3's marker grammar and is
independent of P4) are not worth the coordination cost of a parallel branch —
except P2b, which is kept separate because it rewrites 19 tracked files and
wants its own review. S1–S6 are independent of each other and of
`APP-VERSIONS`. P9 is outside the chain: it needs the owner's authorisation and
it spends money.

## 5. Predictions (falsifiable)

- **P1** The three reservations are edits to exactly four files
  (`layout.rs`, `install/bootgen/own_boot.rs`,
  `vibe-spec/src/source_policy/authored_boot.rs`, and the manifest table's
  four sites), and **none** of the twelve existing callers of the two layout
  predicates changes, because they inherit the new paths from the private list.
  *If a caller needs editing, the layout owner is less shared than the baseline
  says.*
- **S3/P1** Dropping `SYSTEM.md` into a boot directory today gets it enrolled
  as an ordinary boot input, and a second regeneration grows the artifact
  again. *If this does not reproduce, the central hazard is smaller than
  `PROP-076#SYS-SELF-INCLUSION` claims and the fact is amended.*
  **Half confirmed, half falsified (S3, 2026-11-10).** Enrolment reproduced
  exactly — four INDEX rows where two were authored. The growth did not and
  cannot: own authored boot is unconditionally dynamic, so the bytes never
  reach the lane. The hazard is silent doubled delivery; the fact is amended
  and P1's gates are unchanged.
- **P3** The host repository's prefix is `26,121 + 251,817 = 277,938` bytes,
  minus the length of `AGENTS.md`'s own `<vibevm>` block, plus the template and
  the three frame lines — i.e. the prefix is within a few hundred bytes of
  277,938 and holds **83%** of the 335,832 bytes of boot material. *Exactly
  measurable; a number outside that range means a segment was dropped or
  duplicated.* **Confirmed by hand (S1, 2026-11-10): 278,092 bytes, `+154`,
  82.81%.** The share read "92%" when the plan was written, which was an
  arithmetic slip corrected here and in `PROP-076#SYS-SIZE-TODAY`.
- **P3** Two checkouts of the same commit at different paths produce
  byte-identical outputs, and a second `vibe prompt build` performs zero
  writes. *This is the whole determinism claim, and it is one command.*
- **P4** The transaction role extension touches `journal.rs`, `prepare.rs`,
  `transaction.rs` and — only if S2 says so — `inverse.rs`, and **nothing**
  under `crates/vibe-workspace/src/boot/`. *If it reaches the composer, the
  seam was chosen wrong.* **Understated (S2, 2026-11-10):** twelve files
  refuse to compile and five more need by-name edits the compiler never asks
  for, `inverse.rs` among them. The "nothing under `boot/`" half still holds
  and remains the seam check.
- **P5** Under `host = "portable"` (the default) a `prepare` with a non-empty
  gated band and no `--session-target` refuses as unresolved input; under a
  fixed `host` the gated band is **empty**, so the same command needs no
  session target at all, and a session whose own OS differs from the fixed
  host is **warned about and served**, never refused. *The owner decision of
  2026-11-10, as the pair of commands that distinguishes its two modes.*
- **P2** Adding `when = "target:<os>"` changes **no** row of this
  repository's `INDEX.md`, because its `[system_prompt] target` is absent and
  an absent target selects every OS. *The compatibility half of the owner
  decision, measurable as a byte-identical INDEX.* **Confirmed, and stronger
  than predicted (P2, 2026-11-10): no byte of `INDEX.md` changes at all**,
  because the generated header prose was deliberately left alone — it describes
  the grammar a reader may meet in that file, and a `target:` row is never in
  it. Asserted on fixtures rather than on the host repository, which has no
  `target:` row to drop: a project declaring no `target` includes every such
  row, a project declaring one platform drops the others entirely rather than
  gating them, and no generated entry carries a `when` under any selection.
- **P5** `vibe prompt prepare` on this repository opens exactly **11** files —
  the three prefix inputs and the eight INDEX rows — and performs zero writes
  and zero network connections. *Counted by an observer, not asserted in prose.*
- **P5** Changing the bytes of `vibevm/vibespecs/boot/90-user.xml` leaves
  `vibe prompt check` at `Equal`, while changing one byte of `AGENTS.md` makes
  it `Stale` naming the entrypoint. *The two halves of the band model, in one
  pair of commands.*
- **P6** A fake-provider capture contains `AGENTS.md`'s text **exactly once**,
  and a worker-profile request contains none of it.
- **P2b/P3** The composed `SYSTEM.md` contains no `@` import line and no
  router, asserted by the composer itself and not only by a test, so the
  recursion `#SYS-NO-RECURSION` forbids is impossible by construction rather
  than by review. *If the assertion can be satisfied while an import line
  survives somewhere in the prefix, the segment boundaries are wrong.*
- **P2b** The expected lane hash in the managed block changes when and only
  when `STATIC.*` changes: editing `90-user.xml` leaves all three instruction
  files byte-identical. *The cost of D6 is bounded exactly here, and this is
  the measurement of it.*
- **P4** A second `vibe install` with nothing changed leaves three generated
  artifacts, one `.gitignore` and three instruction files with unchanged
  mtimes, and `git status` is clean — which under
  `#SYS-ARTIFACTS-NOT-TRACKED` means the artifacts do not appear in it at all.
  *The determinism claim and the no-tracking claim in one command.*
- **P7** The differential parity test passes with exactly one declared
  normalisation and no other difference. ***This is the campaign's central
  claim: the prefix delivers what a file-reading boot delivers.*** *If a second
  normalisation is needed, the export is not boot-equivalent and the design is
  wrong, not the test.*

## 6. Quick start (cold)

```sh
cd /fast/git/vibevm
git log --oneline -3
cat campaigns/sys-prompt-2026-11/PLAN.md campaigns/sys-prompt-2026-11/LOG.md
# the design:
./target/debug/vibe refactor convert-source --from xml --to md --dry-run \
  vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml   # must say byte-stable
# the material this feature is about, and its real sizes:
wc -c AGENTS.md vibevm/vibespecs/boot/STATIC.xml vibevm/vibespecs/boot/INDEX.md
cat vibevm/vibespecs/boot/INDEX.md
# the owners, in reading order:
sed -n '240,290p' crates/vibe-core/src/layout.rs
sed -n '160,226p' crates/vibe-workspace/src/install/bootgen/own_boot.rs
sed -n '120,175p' crates/vibe-workspace/src/boot_artifacts/redirect.rs
sed -n '55,95p'  crates/vibe-workspace/src/boot_artifacts/transaction/journal.rs
sed -n '360,420p' crates/vibe-workspace/src/boot_artifacts.rs
# Phase 0 spikes use the isolated runner:
python3 /fast/vibevm-test-tmp/claude-int/run_test.py <label> cargo test -p <crate> --offline --locked
```

**Two standing warnings.** Never create `SYSTEM.md` or `SYSTEM-TEMPLATE.md` in
the live `vibevm/vibespecs/boot/` before P1 lands — they would be enrolled as
boot inputs. Never run `vibe refactor convert-source` without `--dry-run`: it
rewrites and renames files.

## 7. Acceptance script (whole campaign)

To be written in P7 as `campaigns/sys-prompt-2026-11/accept.sh`, following
`campaigns/corporate-env-2026-11/accept.sh`. It runs, in one network-isolated
sandbox with a fresh HOME: create a fixture project and opt in with a template;
`vibe install` and assert five artifacts in one commit; assert
`vibe prompt check` is `Equal` and performs zero writes and zero connections;
edit a dynamic boot snippet and assert `check` is still `Equal`; edit the
entrypoint and assert `Stale` naming it; regenerate and assert byte-identical
output at a second checkout path; run the differential parity test; capture a
request against the fake provider and assert exactly one copy of the
entrypoint; assert a worker profile carries no central prefix; assert the home
sentinel files were never opened; remove the template and assert exactly the two
generated outputs disappear; and finally hand-edit `SYSTEM.md` and assert the
ownership conflict refuses without writing. It exits non-zero on any step's
failure, any non-loopback connection attempt, any filesystem write outside the
sandbox, and any provider call.

**It never calls a paid API.** The live cache measurement is a separate,
separately authorised exercise (`PROP-076#SYS-OWNER-QUESTIONS` Q5) and is not
part of this script.

## 8. Open points

**Q3 and Q6 are answered (owner, 2026-11-10) and are no longer open.** Q6 was
answered **against** its own default: the generated prefix, its sidecar and the
agent lane are **not** git-tracked, under a `.gitignore` vibe writes into the
boot directory it owns — see
`spec://org.vibevm.core/vibevm/common/PROP-076#SYS-ARTIFACTS-NOT-TRACKED`,
which re-planned P4. The whole delivery design was settled the same day and
adopted as decided (D1–D12 of `HARNESS-DELIVERY.md` §11), adding
`#SYS-AGENT-LANE`, `#SYS-LANE-MARKERS`, `#SYS-HARNESS-DELIVERY`,
`#SYS-NO-RECURSION`, `#SYS-IMPORT-INERTNESS`, `#SYS-BOOT-ORDER-IS-PRECEDENCE`
and `#SYS-OPERATIONS-PATH-AND-FLAGS`. Its D13 — the live run — is the one
genuinely open item, and it is P9. For Q3, see
`spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HOST-AND-TARGET` and the
decision entry in `LOG.md`. It was posed as one axis and turned out to be two:
`[system_prompt] host` (default `"portable"`) decides whether the session's-OS
`os:` gates stay in the gated band or are baked into the prefix, and a new
`[system_prompt] target` plus a new `when = "target:<os>"` gate (default: every
OS) carries the product's own platform and is always resolved at build time.
The manifest keys landed in P1; the gate grammar landed in P2, the baking lands
in P3, the host-mismatch warning in P5. The remaining questions are
`spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OWNER-QUESTIONS`, each with
the default it takes unless the owner objects: (Q1) the unconditional INDEX band
as a cacheable continuation rather than baked into the prefix; (Q2) the override
authority of repository, package and user-preference instructions once they sit
in an API instruction role; (Q3) a portable prefix with gated continuation
rather than a target-bound one; (Q4) the entrypoint represented verbatim with
one in-place substitution rather than filtered; (Q5) no measured provider saving
required before the campaign closes. Q6 is answered above.

**Which of them block a phase.** Q1 is the band model itself: a rejection after
P3 lands is rework, not a tweak, so it wants an answer before P3 starts. Q3 was
the other half of that and is now answered. Q4 shapes P2 and P3 the same way;
S1 gave the owner the evidence for it and it stands. Q2 blocks P6's adapter
contract and nothing earlier. Q6 shaped P4 and is answered. Q5 blocks nothing
and closes with P8. D13 blocks only P9, and P9 blocks nothing in the campaign:
every phase's own acceptance is offline.

## 9. Log

See `LOG.md`.

## 10. Report

Empty until close.
