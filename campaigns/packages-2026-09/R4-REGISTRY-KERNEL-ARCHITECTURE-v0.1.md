# R4 registry kernel and staged transforms — implementation architecture v0.1

Status: central implementation design, 2026-08-27; R4.0 extraction implemented
and accepted 2026-08-29 at `6af1b86f` with map `8531cf82`; R4.1 owner controls
`52a59dcc` and unit transaction `91142777` / `ab68d145` are landed, and the
TransformPlan T1 config/digest substrate `b65f9958`, T3 selector substrate
`48d7dc75` and exact T2 frame freeze `d5fcd92d` are landed (combined map
`5f6bca62`). T2 implementation `49e944f0`, borrowed hash validation
`87ef2df6` and map `b768bcb8` are landed; T4 ArtifactPlan carriage
`a252fcc8` / map `5aa44611` and T5 private behavior registry `0eb46c82` /
map `0f73cdfe` are now landed. T6a fallible discovery is current. Semantic
authority remains PROP-054 §§3 and 7 plus the accepted lifecycle spec debt.
This document fixes the crate boundary and execution dataflow.

Execution status and dependency order live in
[`LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md`](LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md).

## 1. The one-machine boundary

Lifecycle contributions and compiler transforms share one declaration,
activation, ordering, selector and provider-attribution machine. There is one
pure collector. Workspace may adapt a lock/materialised world into its input;
`vibe-spec` may execute a compiled transform plan; neither may collect the
manifest rows again.

The kernel is the landed crate `vibe-extension-registry`; its exact runtime
dependency and ambient-access fences are part of R4.0. `vibe-ext` is reserved
for the R5 plugin SDK. `vibe-core` remains manifest grammar, not a world-aware
registry.

Required dependency DAG:

```text
vibe-extension-registry → vibe-core
vibe-spec               → vibe-extension-registry
vibe-workspace          → vibe-extension-registry + vibe-spec
vibe-lifecycle          → vibe-extension-registry + vibe-workspace
```

Forbidden edges:

- `vibe-workspace → vibe-lifecycle` (direct cycle);
- `vibe-spec → vibe-lifecycle` (`spec → lifecycle → workspace → spec`);
- a second workspace/compiler collector hidden to avoid either cycle.

The kernel needs only `vibe-core`, `glob`, `specmark` and `thiserror`. It is
filesystem/env/CLI-free; provider roots are already-resolved `PathBuf` data.

## 2. What moves and what stays

Move from `vibe-lifecycle::registry` without semantic edits:

- dependency/host provider identities and metadata;
- `ExtensionWorld`, dependency and host source rows;
- registry rows, natural/effective tiers, notices and states;
- both collection entry points and errors;
- compiled selector and `SelectorSubject`;
- registry views, exhaustive view and point planning;
- synthetic preset source.

Keep above the kernel:

- `vibe-lifecycle`: `ExecutablePlan`, `HandlerExecution`, dispatch/envelope,
  lifecycle state and filesystem-aware fingerprints;
- `vibe-install`: provisional-resolution adapter, legacy hook sugar and
  slot-target orchestration;
- `vibe-workspace`: root/lock/slot discovery and compiler invocation;
- `vibe-cli`: narration and machine report adapters;
- `vibe-spec`: typed pass manager, backend behavior registry, builtins/native
  implementations and IR mutation.

`EffectiveManifestKind` remains report/workspace metadata unless extraction
proves collection behavior reads it. `vibe-lifecycle` initially re-exports moved
public names so existing callers migrate without a flag day.

## 3. Identity and ordering inputs

The kernel accepts typed identities only:

- installed package: validated group/name, exact lock version/kind/content hash,
  resolved slot root;
- grouped host/package-role host: group/name coordinate;
- ungrouped project: shared `HostOwner` percent-coded printable identity over
  the raw project name;
- virtual workspace: control-only, never a declaration provider.

Display strings are output, never parse input for ordering or selector identity.
The R8 `ProviderPin` and extension `ExtensionKey` use the same HostOwner codec.

Effective order remains exactly:

1. synthetic/effective-stack presets;
2. dependencies in **root lock order**, declaration order inside each manifest;
3. host declarations in their array order;
4. host activations in `[[extensions.use]]` order.

Activation replaces config as one complete value and records its ordinal.
Disable/inactive rows stay in exhaustive views but do not enter execution plans.

R4.1 will install one ordered-world adapter in `vibe-workspace`: durable
`from_lock`, provisional `from_lock_and_resolution`, and one typed host-source
projection. It iterates the absolute-root lock, substitutes resolved rows in
place and appends genuinely new rows in resolver order. `fs::read_dir` may
remain only to report orphan slots; an orphan is outside the extension world
and never becomes ordering input.

## 4. Three honest world epochs

Adapters, not the kernel, own when the world is observed:

1. pre-install provisional resolution for slot hooks;
2. post-install durable lock/materialised world for default phases and compile;
3. pre-wipe old world for clean hooks.

All three call the same collector. Each command states which lock value is its
epoch authority: ready apply overlays its resolution on pre-apply lock order;
post-install/default compilation reads the durable root lock; clean plans from
the pre-wipe old-lock intersection. Post-removal uninstall regeneration is not
clean: it compiles the remaining **future** world from the in-memory lock after
the removed entry is dropped and before that lock is published. The adapter
orders the epoch a command owns; it never chooses or invents one. A post-barrier
equality test feeds equivalent provisional/durable owner views and requires one
registry/order.

## 5. Compiler plan translation

Workspace takes one filesystem-backed world snapshot per lifecycle/install run
— every participating manifest parsed once, dependency order from the root
lock — then invokes the one pure collector once per **lane owner** over an
owner-scoped view and translates that owner's effective `compile:*` rows into a
`vibe-spec`-native `TransformPlan`. One collector means one implementation and
one snapshot, not one effective plan: the selected node, every member node and
every package-owned unit may have distinct plans. An owner with no compile rows
shares the empty plan. The plan contains no manifest objects and no filesystem
resolver:

- stable extension/provider key, version/content hash and effective order;
- stage (`source|document|lane|emitted`);
- handler implementation identity (`builtin` now; `native` after R5);
- exact config value and config digest;
- optional precompiled selector material needed at document stages.

`vibe-spec` owns behavior lookup. Manifest/registry rows never carry an
`Arc<dyn Pass>` across crate boundaries. `BackendRegistry` remains a behavior
registry and is not a second declaration collector.

Exact in-process types, semantic TOML lowering, digest framing, selector
boundary and behavior-registry ABI are frozen in
[`R4-TRANSFORM-PLAN-ABI-v0.1.md`](R4-TRANSFORM-PLAN-ABI-v0.1.md); implementation
may not replace that contract with generic JSON or display-string identity.

### 5.1 Owner-scoped activation

PROP-054 `##COMPILE-ACTIVATION` is literal: activation authority follows the
artifact being written. A node lane uses that node's manifest; a per-unit lane
uses that package's own manifest. The world snapshot therefore retains every
package's `ExtensionsControl`, but dependency controls remain inert in another
owner's view. For package P's lane the collector sees, in the same four tiers:
presets applicable to P; P's dependency closure in root-lock order; P's own
declarations; P's own activations. Host controls cannot leak into P or a
sibling, and P's controls cannot leak into host/sibling views. Package controls
are data lost by today's `DependencyExtensionSource`; R4.1 adds them plus a
pure dependency-seat→owner-seat projection before plan construction.

### 5.2 Selector subject

Source/document transforms require the provider plus forward-slashed declared
path of the addressed document. That identity is carried into `SourceIr` /
`DocumentAddress` from worklist discovery; it is not reconstructed by parsing
`SpecAddress::to_string()` or `ContributionMeta.origin`.

Lane/emitted transforms are one complete artifact and use an artifact subject,
not a fake document path. `applies_to` is currently legal only where the
manifest grammar can express a real subject; adding artifact selectors is a
separate grammar act.

### 5.3 T10 adapter seam and lowering authority — decision record

§5 says workspace translates owner-scoped rows into a plan but not which crate
owns each half of the translation, how the registry-owned epoch reaches the
seed, or where typed document providers are born. Settled centrally before T10
implementation; T10 lands as three gated slices (T10a world adapter, T10b
lowering + typed subjects, T10c fingerprint + header), each its own commit and
exact affected-set gate.

**Decision — the split of the translation.** `vibe-workspace` owns the WORLD:
the durable `from_lock` ordered-world adapter (§3's shape), the owner-scoped
collector invocation per lane owner (node manifest for a node lane, package
manifest for that package's unit lane, via the kernel's existing
dependency-seat→owner-seat projection), and the choice of epoch authority per
§4. `vibe-spec` owns the LOWERING: one public entry
(`TransformPlan::from_effective_rows` shape — final name free) that consumes
borrowed kernel rows already filtered to compile points in effective order,
and inside the crate maps stage from `CompilePoint`, provider through the
existing `From<&ExtensionProvider>`, config through the one
`toml::Table → ConfigTable` lowering, selector by clone, and implementation by
resolving `ExtensionHandler::Builtin { name }` against
`TransformRegistry::builtins()` for the registry-owned epoch — an off-catalog
name is the existing bounded `UnknownBuiltin` refusal at lowering time, and
workspace never sees an epoch. A non-compile row reaching the lowering is a
typed caller error, never skipped. The kernel gains one view,
`enabled_compile_rows()` — every enabled compile-point row in ONE global
effective order — because concatenating four per-point views would fabricate
an order §3.4 never authored. No manifest object, resolver or display string
crosses into the plan; `TransformSeed`/`TransformPlan` construction stays
crate-private to `vibe-spec`, and only the lowering entry, the plan value and
`ArtifactPlan::with_transforms` widen to `pub`.

**Decision — typed subjects at birth.** `ArtifactInput`'s constructors gain
typed-provider forms; the boot adapter names each contribution's
`DocumentProvider` from the same typed components the world adapter already
holds (lock `Group`/`PackageName` for dependencies; the host arms from the
node's own coordinate), so `Undetermined` becomes unreachable for every
workspace-built input. A component the install model still carries as a bare
`String` is parsed at the adapter seam through the one existing grammar with a
typed refusal — never a panic and never a silent fallback to `Undetermined`;
retyping the install model itself is named follow-up hygiene, not smuggled
into T10. The `[shared by …]` display suffix stays display: the typed
provider is threaded beside `origin`, never parsed out of it. The T8
reached-verdict test upgrades to the whole-compile assertion the ABI §5.1
revisit trigger promises, and a test pins `Undetermined` unreachable for
declared documents.

**Considered and rejected.** Workspace-side seed construction with a public
epoch parameter — hands workspace an identity §2.1 of the ABI forbids it to
author. Lowering in workspace over pub-widened `ConfigTable` — duplicates the
TOML semantic tree at the boundary and makes two crates own one canonical
form. Per-point view concatenation — deterministic but fabricates a cross-
stage order; the digest would bless an order no manifest declared. Reusing
`vibe-orchestrator`'s world builder from workspace — the dependency arrow
points the other way; orchestrator migrates onto the workspace adapter later
instead (named follow-up, not T10).

**When to revisit.** When R5 adds `Native` implementations the lowering entry
gains its second arm under the same registry authority; when R6 adds pass-tier
rows the non-compile refusal splits into its own routing.

**Ratified at T10A acceptance (central, 2026-08-29).** Two spec-silent calls
the adapter surfaced are ruled as implemented: each lane owner's
`[active].stack` resolves against that owner's OWN closure (the node's in the
node's lane, a package's in the package's — a host fact must not cross the
scoping seam, and dropping the preset tier would be invented silence), under
the one durable-mode strictness (absent refuses, ambiguous refuses); and both
owner views carry the owner's dependency CLOSURE — reachability walked over
lock edges, order taken from projecting the reached set back onto the
lock-ordered snapshot, so reachability is never an ordering input.
`compile:pass` rows are inside `enabled_compile_rows()` (the whole compile
family); the T10B lowering refuses them typed until R6 owns the pass tier.

**Ratified at T10B acceptance (central, 2026-08-29).** Three rulings the
lowering landing surfaced, each now pinned in code:

1. **Boot regeneration owns no epoch, so the durable lock it can read is
   evidence, never authority.** §4's sentence — the adapter "orders the epoch
   a command owns; it never chooses or invents one" — decides the seam:
   during `vibe install` the boot lane is written before the resolution's
   lock is published, so the on-disk lock is the PRE-install epoch and a
   world observed against it never existed. Two rules follow
   (`bootgen/owner_plans.rs`): a world that cannot be observed — no lock, an
   unreadable lock, a lock that disagrees with the tree — is NOT a fault at
   this seam and the lane takes `TransformPlan::empty()` (the exact
   historical bytes); a world that IS observed is judged strictly — a
   collection refusal or a lowering refusal propagates (pinned including the
   collection half). Consequence, accepted: an install-time compile-point
   extension is not observed on the install path itself until the
   orchestrator migration (§5.3 follow-up 2) threads the in-memory lock
   value; every post-install path observes. R4.2's activation e2e must
   therefore drive a post-install regeneration, and should include a MEMBER
   node (its own re-seating of the same lock is byte-invisible today).
2. **Activation authority follows the artifact being written.** Two call
   sites, not one: the node path lowers the node's own view; the per-unit
   path lowers THAT package's view through the kernel's dependency-seat →
   owner-seat projection; an uninstalled unit is outside the world (§3's
   orphan rule) and takes the empty plan. Pinned behaviourally on one world
   with two disjoint declarations, plus a call-site fence.
3. **Exactly four names crossed the `vibe-spec` boundary** — `TransformPlan`
   (+ `empty`/`len`/`is_empty`), `from_effective_rows`,
   `TransformLoweringError`, `DocumentProvider` (re-exported;
   `DocumentSubject` deliberately not) — and `ArtifactPlan::with_transforms`
   went `pub` as T4 promised. A visibility fence pins each cell's public set
   exactly. `BootEntry` carries `BootProvenance` beside its display
   `origin`; `UnitInput` gained no second copy — the unit table's key
   already IS the typed pair, and provenance is read off the key it is
   filed under, never parsed from a rendering.

## 6. Four positions in the declared schedule

The accepted schedule stays one list:

```text
source transforms → parse → document transforms → gather
→ close → merge → embed → qualify → absorb → link → assemble
→ lane transforms → emit:<backend> → emitted transforms
```

Source/document calls are per addressed document. Lane/emitted calls are once
per artifact. Gather is not a pass. Compatibility fragments do not run tier-1
transforms.

An empty plan instantiates the exact historical schedule, bytes and errors.
R4 uses append/boundary construction; general before/after/replace positioning
belongs to R6 pass grammar.

### 6.1 Honest emitted mutation

`EmittedArtifact` bytes and manager provenance/digest are inseparable. An
emitted transform runs only after the untransformed emitter/tape oracle succeeds,
then returns bytes through a manager-owned constructor that recomputes digest
and transformed provenance. Direct `bytes_mut` is forbidden.

Lane transforms run the intrinsic/verifier seam after mutation. They cannot
claim closure equivalence merely because the lane shape parses; the manager
retains the immutable pre-transform witness needed by the accepted transition
law.

## 7. Fingerprint, header and publication

The active ordered transform plan enters every artifact identity:

- provider key/version/content hash;
- stage/order/id;
- exact config digest and implementation digest.

Inactive/disabled observable rows do not invalidate bytes. Per-unit and node
lanes use the same plan-digest **algorithm and carrier**, never necessarily the
same value: every artifact binds its lane owner's plan. A per-unit lane hashes
its owner-plan digest into the existing boot-graph fingerprint; the empty plan
adds no plan frame, preserving historical bytes. A node lane deliberately has
no freshness fingerprint and is always recompiled; its plan identity rides the
header/provenance, and the crash-safe transaction preserves bytes/mtime when
the recomputed artifact is equal. Thus a changed owner plan cannot leave that
owner's skippable unit fresh, while distinct owners may legitimately transform
the same authored document differently.

The transforms header is emitted only for a nonempty active plan, after the
reference oracle and by engine framing—not plugin bytes. Markdown/XML tape
validators and decompile know the header. Empty plans keep committed artifacts
byte-identical.

### 7.1 Header grammar and fingerprint frame — decision record

**Decision — header.** One comment line, engine-framed after the reference
oracle, only for a nonempty active plan:
`<!-- vibe:transforms <entry> <entry> … -->` with one token per plan entry in
dense effective order, each token the entry's canonical `ExtensionKey`
spelling encoded by the boot lane's existing label codec (`%` → `%25`,
`-` → `%2D`) — the payload then cannot contain `-` at all, so it is XML-
comment-safe unconditionally, reversible by one rule, and spelled by the same
codec the generated lane already uses for qualified anchors. The codec
implementation is extracted to one shared cell if it is currently local to
the label writer; a second spelling of it may not appear. The static
decompiler already classifies such a comment as skippable non-provenance C1;
tape validators learn the exact grammar. The header records the ACTIVE list —
identity attribution beyond it stays in provenance/IR, and nothing ever
parses the header back (the analyzer law in §9).

**Decision — fingerprint.** The per-unit Merkle body (`fingerprint::compute`)
gains one frame: the owner plan's `PlanDigest` as `transforms:<sha256 hex>`,
appended only when the plan is nonempty, so every historical fingerprint —
and every unit whose owner activates nothing — keeps its exact current value
and bytes. A node lane gains no fingerprint (it recomputes always); its
equal-bytes no-op stays owned by the publication transaction. Changed owner
plan ⇒ changed unit fingerprint ⇒ stale unit, exactly §7's matrix row 8.

**Considered and rejected.** Framing the plan digest even when empty — breaks
every existing recorded fingerprint for zero information. A second header per
stage — the schedule partition is execution detail; the authored order is the
honest record. Raw key spelling in the comment — a key containing `--` (legal
in package names) would corrupt or forbid the comment; encoding only the
dangerous pair — two spellings for one identity depending on neighbours,
irreversible in the corner.

**When to revisit.** When R6's pass tier records pass entries the header
gains their tokens under the same codec (PROP-054 `##COMPILER-INTERNALS-FLAG`
already promises this); if any consumer ever needs to read the active list
machine-side, it reads IR/provenance, and that stays the law.

**Freeze repair, 2026-08-30 (T10C acceptance) — the codec's letter.** The
parenthetical above was authored imprecisely: the shared codec
(`vibe-specdoc`'s `encode_generated_xml_comment`, which turned out to already
BE the "one shared cell" — public, with the kind-free decode entry exposed
beside it at T10C) does not escape every `-`. Its one canonical rule is: `%`
→ `%25` always; `-` → `%2D` only inside a `--` pair and at payload end; a
single interior hyphen stays raw as the canonical spelling. The safety
conclusion holds exactly as stated — an encoded payload contains no `--`
and never ends in `-`, tokens are joined by single spaces so no `--` forms
across a join, and the last token cannot touch `-->` — and the
considered-and-rejected item stands unchanged: what it rejects is an ad-hoc
neighbour-dependent escape invented HERE, i.e. a second spelling; the
incumbent codec's context rule is the single canonical spelling one cell
owns. Implementing the parenthetical literally would have created exactly
the second spelling this section forbids, so the letter is repaired to the
incumbent rather than the code bent to the letter.

**Ratified at T10C acceptance (central, 2026-08-30).** Five rulings the
landing surfaced, each pinned:

1. **"After the reference oracle" is provenance, not a byte position.** The
   header is engine output beside the reference bytes; its byte position —
   the FOURTH line, after the three provenance lines and before the blank
   separator, in both lanes — was decided at landing and is pinned byte-exact.
2. **The wire tape gate admits the header as OPTIONAL and judges its
   grammar.** The emitted carrier does not carry the plan and nothing parses
   the header back, so the strongest honest wire law is: absent is lawful,
   present must be well-formed (reserved prefix + codec-canonical tokens,
   judged by the shared codec); a raw or re-spelled token refuses with the
   codec's own error. Pinned on both halves at the wire gate itself.
3. **Plans are lowered once per run, BEFORE the fingerprints**, for every
   table unit (a plan digest is a freshness INPUT, and a static parent
   hashes its child's fingerprint) — with the accepted consequence that an
   owner whose declaration cannot be lowered refuses the run even when it
   emits nothing itself. `verify_boot_graph` observes the same world and
   frames the same digests, pinned by an occurrence-COUNTED fence after a
   contains-only fence was proven blind to the check half framing nothing.
4. **One new public name**: `TransformPlan::digest_hex` (scalar; `None` for
   the empty plan IS the no-frame law, pinned in the owning crate);
   `PlanDigest` itself stays crate-private with one hex rendering behind
   two projections.
5. **The static decompiler needs no change** — classification is per-kind
   and `vibe:transforms` falls through as skippable non-provenance — pinned
   from both sides: in-perimeter against the real emitted line, and at the
   decompiler itself (a header-bearing tape decompiles to its header-free
   twin's exact contribution set).

Package-unit lanes must join the crash-safe whole-artifact transaction before
any byte-changing transform ships; bare `fs::write` is not an R4 publication
path.

## 8. XML-minify binding

The existing strict span-deletion kernel is reused; R4.2 is binding and oracles,
not a new XML serializer. Required REDs include:

- hoisted top-level `#use` in an XML lane;
- all-elided/no-element stream;
- comment codec and invalid `--` payload;
- CDATA/fence boundaries;
- leaf versus mixed-content parent shape;
- DTD/entity refusal;
- `vibe_specdoc::from_xml(after) == from_xml(before)` per XML document;
- real artifact strictly smaller, decompiled node set identical, idempotent.

The binding may not make the kernel permissive to bless non-XML text. Until an
engine-owned segmented emitted-tape adapter handles a hoisted line honestly,
that active transform refuses the artifact by name rather than silently skips
or corrupts it.

## 9. Analyzer follows execution evidence

R4.3 consumes Lane/Emission witnesses and transform trace events:

- node bytes and occurrence count;
- package contribution bytes by typed provider identity;
- frame overhead;
- per-pass delta labelled by stage (lane-byte delta is not artifact-byte delta);
- token estimate only with an explicit estimator id; otherwise absent/null.

The JSON report is JTD-first. It never parses generated artifact comments to
reconstruct attribution already present in IR/provenance.

## 10. Extraction and landing sequence

Owner-control carriage is landed at `52a59dcc`; crash-safe per-unit publication
at `91142777` with normative anchor `ab68d145`; their combined generated map is
`3883f15e`.

1. **Done `6af1b86f`:** add the kernel crate and move the pure registry/tests.
2. **Done `6af1b86f`:** lifecycle consumes/re-exports it; boundary and caller
   gates green; generated map `8531cf82`.
3. Add workspace/spec dependencies and prove empty-registry byte identity.
4. Build one strict durable-world adapter from selected manifest + root lock.
5. Thread one collected registry/plan through unit and node compilation.
6. Add four positions with builtin identity transforms first.
7. Add plan digest and active-only header; **unit transaction parity is done**.
8. Bind `xml-minify` plus semantic/delta RED corpus.
9. Add JTD analyzer/report CLI.

Each slice is a separate commit and exact affected-set gate. One full panel
closes the integrated R4 batch, not each mechanical move.

## 11. Acceptance matrix

1. Existing ordering/control/selector/view suite moves without behavior change.
2. Provisional and durable equivalent owner views collect one identical
   registry/order.
3. Empty compile registry preserves schedule, bytes, errors and mtimes.
4. Source/document run per address; lane/emitted once per artifact.
5. Document selector receives typed provider/path, including host identities.
6. Disabled/inactive rows remain queryable and never execute.
7. Plan fingerprint changes with order/config/provider version/hash.
8. Same owner plan preserves per-unit freshness and equal node bytes/mtime;
   changed owner plan invalidates that unit (and existing static parents), while
   node lanes always recompute.
9. Active header lists exact effective order; empty plan emits no header.
10. Emitted transform cannot forge stale provenance/digest.
11. Unit and node artifacts share crash-safe publication behavior.
12. XML-minify REDs and both owner byte scenarios stay green.
13. Analyzer totals reconcile exactly to artifact bytes and stage deltas.
14. `cargo metadata` proves the intended acyclic dependency DAG.
