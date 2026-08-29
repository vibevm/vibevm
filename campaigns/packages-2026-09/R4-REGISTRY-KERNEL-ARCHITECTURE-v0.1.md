# R4 registry kernel and staged transforms — implementation architecture v0.1

Status: central implementation design, 2026-08-27; R4.0 extraction implemented
and accepted 2026-08-29 at `6af1b86f` with map `8531cf82`; R4.1 owner controls
`52a59dcc` and unit transaction `91142777` / `ab68d145` are landed, and the
TransformPlan T1 config/digest substrate `b65f9958`, T3 selector substrate
`48d7dc75` and exact T2 frame freeze `d5fcd92d` are landed (combined map
`5f6bca62`). T2 implementation `49e944f0`, borrowed hash validation
`87ef2df6` and map `b768bcb8` are landed; T4 ArtifactPlan carriage
`a252fcc8` / map `5aa44611` is now landed. T5 private behavior registry is
current. Semantic authority remains PROP-054 §§3 and 7 plus the accepted
lifecycle spec debt. This document fixes the crate boundary and execution
dataflow.

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
