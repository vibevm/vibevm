# R4 registry kernel and staged transforms — implementation architecture v0.1

Status: central implementation design, 2026-08-27. Semantic authority remains
PROP-054 §§3 and 7 plus the accepted lifecycle spec debt. This document fixes
the crate boundary and execution dataflow before R4.0 extraction starts.

## 1. The one-machine boundary

Lifecycle contributions and compiler transforms share one declaration,
activation, ordering, selector and provider-attribution machine. There is one
pure collector. Workspace may adapt a lock/materialised world into its input;
`vibe-spec` may execute a compiled transform plan; neither may collect the
manifest rows again.

The kernel is a new crate (working name `vibe-extension-registry`; final name is
chosen once and then stable). `vibe-ext` is reserved for the R5 plugin SDK.
`vibe-core` remains manifest grammar, not a world-aware registry.

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

Standalone materialised-world reconstruction currently enumerates directories
without lock ordering. That vector is never accepted as ordering authority.
Workspace must read the absolute-root lock or consume the already lock-ordered
install resolution.

## 4. Three honest world epochs

Adapters, not the kernel, own when the world is observed:

1. pre-install provisional resolution for slot hooks;
2. post-install durable lock/materialised world for default phases and compile;
3. pre-wipe old world for clean hooks.

All three call the same collector. A post-barrier equality test feeds equivalent
provisional/durable rows and requires one registry/order. Clean may retain only
the already-specified old/future control intersection; it does not create a
fourth collector.

## 5. Compiler plan translation

Workspace collects once per lifecycle/install run, then translates effective
`compile:*` rows into a `vibe-spec`-native `TransformPlan`. The plan contains no
manifest objects and no filesystem resolver:

- stable extension/provider key, version/content hash and effective order;
- stage (`source|document|lane|emitted`);
- handler implementation identity (`builtin` now; `native` after R5);
- exact config value and config digest;
- optional precompiled selector material needed at document stages.

`vibe-spec` owns behavior lookup. Manifest/registry rows never carry an
`Arc<dyn Pass>` across crate boundaries. `BackendRegistry` remains a behavior
registry and is not a second declaration collector.

### 5.1 Selector subject

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
lanes use the same plan digest. A changed plan cannot leave fresh untransformed
unit lanes beside regenerated transformed node lanes.

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

1. Add the kernel crate and move the pure registry/tests byte-for-byte.
2. Make lifecycle consume/re-export it; all existing registry/e2e tests green.
3. Add workspace/spec dependencies and prove empty-registry byte identity.
4. Build one strict durable-world adapter from selected manifest + root lock.
5. Thread one collected registry/plan through unit and node compilation.
6. Add four positions with builtin identity transforms first.
7. Add plan digest, active-only header and unit transaction parity.
8. Bind `xml-minify` plus semantic/delta RED corpus.
9. Add JTD analyzer/report CLI.

Each slice is a separate commit and exact affected-set gate. One full panel
closes the integrated R4 batch, not each mechanical move.

## 11. Acceptance matrix

1. Existing ordering/control/selector/view suite moves without behavior change.
2. Provisional and durable equivalent worlds collect one identical registry.
3. Empty compile registry preserves schedule, bytes, errors and mtimes.
4. Source/document run per address; lane/emitted once per artifact.
5. Document selector receives typed provider/path, including host identities.
6. Disabled/inactive rows remain queryable and never execute.
7. Plan fingerprint changes with order/config/provider version/hash.
8. Same plan preserves per-unit/node freshness; changed plan invalidates both.
9. Active header lists exact effective order; empty plan emits no header.
10. Emitted transform cannot forge stale provenance/digest.
11. Unit and node artifacts share crash-safe publication behavior.
12. XML-minify REDs and both owner byte scenarios stay green.
13. Analyzer totals reconcile exactly to artifact bytes and stage deltas.
14. `cargo metadata` proves the intended acyclic dependency DAG.
