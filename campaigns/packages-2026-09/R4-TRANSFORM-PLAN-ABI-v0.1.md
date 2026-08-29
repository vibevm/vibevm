# R4.1 TransformPlan ABI and digest — implementation design v0.1

Status: accepted central design, 2026-08-29. Semantic authority remains
PROP-054 §§3.4, 7.1–7.3 and the R4 architecture. This document freezes the
in-process ABI and digest before implementation; execution status stays in the
implementation ledger.

## 1. Boundary

`TransformPlan` is a `vibe-spec`-native, owned semantic value. It contains no
manifest object, filesystem resolver, display-string parser or `Arc<dyn Pass>`.
`vibe-workspace` is the only declaration→plan adapter: it receives an
owner-scoped `ExtensionRegistry`, lowers exact effective rows/config/selectors
and attaches the plan to `ArtifactPlan`. `vibe-spec` owns behavior lookup,
schedule insertion, mutation and verification.

The plan is **per lane owner**. PROP-054 `##COMPILE-ACTIVATION` already closes
the question: each node manifest activates its node lane; each package manifest
activates that package's own unit lane. One collector implementation and one
filesystem snapshot do not imply one effective plan.

Required new DAG edges are acyclic:

```text
vibe-spec      → vibe-extension-registry + vibe-core
vibe-workspace → vibe-extension-registry + vibe-spec + vibe-core
```

## 2. Rust ABI

All fields are private. `TransformPlan::build` is the only nonempty constructor;
it assigns dense zero-based order from the supplied effective-row sequence.

```rust
pub struct TransformPlan {
    entries: Vec<TransformEntry>,
    digest: Option<PlanDigest>,          // None iff entries is empty
}

pub struct TransformSeed {              // workspace adapter input
    key: String,                         // canonical ExtensionKey spelling
    provider: TransformProvider,
    stage: TransformStage,
    implementation: TransformImplementation,
    config: Option<TransformConfig>,     // None != Some(empty)
    selector: Option<CompiledSelector>,  // source/document only
}

pub struct TransformEntry {
    seed: TransformSeed,
    order: u32,                          // assigned, never caller-authored
    config_digest: Option<ConfigDigest>,
    implementation_digest: ImplementationDigest,
}

pub enum TransformStage { Source, Document, Lane, Emitted }

pub enum TransformProvider {
    Dependency {
        id: DependencyProviderId,
        version: String,
        kind: PackageKind,
        content_hash: ContentHash,
    },
    Host {
        id: HostIdentity,
        version: String,
        kind: Option<PackageKind>,
        content_hash: Option<ContentHash>,
    },
}

pub enum TransformImplementation {
    Builtin { name: String, epoch: u32 },
    // R5 adds Native { name, abi, artifact_digest } as another discriminant.
}

pub struct PlanDigest([u8; 32]);         // sha256:<64 lowercase hex>
pub struct ConfigDigest([u8; 32]);
pub struct ImplementationDigest([u8; 32]);
```

`build(Vec<TransformSeed>)` refuses duplicate keys, more than `u32::MAX`
entries, selector presence on lane/emitted, selector absence/presence that
contradicts the stage grammar, blank/unbounded implementation identity and an
invalid provider/version/hash. It enumerates order itself; a sparse/reordered
caller ordinal is unrepresentable.

`ArtifactPlan` gains a transform plan; `ArtifactPlan::compatibility` always
pins `TransformPlan::empty()`, so compatibility fragment APIs run no tier-1
transform without a special branch in execution.

## 3. Lossless effective configuration

Generic JSON is forbidden: TOML datetime and the TOML number tower are not JSON
values. Workspace lowers `ExtensionConfig` into this neutral owned tree:

```rust
pub type ConfigTable = BTreeMap<String, ConfigValue>;
pub enum ConfigValue {
    String(String), Integer(i64), Float(ConfigFloat), Boolean(bool),
    Datetime(ConfigDatetime), Array(Vec<ConfigValue>), Table(ConfigTable),
}
pub struct ConfigFloat(u64); // f64 bits; every NaN canonicalised like EqTomlTable
pub struct ConfigDatetime {
    date: Option<ConfigDate>, time: Option<ConfigTime>,
    offset: Option<ConfigOffset>,
}
```

Datetime components mirror `toml_datetime::Datetime` field-for-field; no
render/parse round trip enters identity. Table order is semantic-insensitive and
stored sorted; array order is semantic and retained. `None` means no effective
config was authored; `Some(empty)` means an authored activation cleared the
value. They remain distinct in plan identity even though the lifecycle handler
fingerprint deliberately fuses them for its different delivered-envelope law.

## 4. Canonical digests

Promote the existing `vibe-spec` `StableDigest` from the emit cell to one
compiler digest cell; do not copy lifecycle's different labelled/BE framing.
The primitive is: domain as the first length-framed field; byte discriminants;
u32/u64 little-endian; `field = u64_le(len)||bytes`; explicit optional bits;
counts as u64 little-endian.

Domains:

- `vibe-transform-config-v1\0epoch=1\0`
- `vibe-transform-implementation-v1\0epoch=1\0`
- `vibe-transform-plan-v1\0epoch=1\0`

Config recursion frames a closed tag then exact payload. Integer is i64 LE;
float is the canonical bit key; datetime frames presence + numeric components
and signed offset minutes; array frames count/order; table frames sorted
key/value pairs. Never rendered TOML, Rust layout or serde JSON.

Implementation digest frames kind, builtin name and explicit epoch. A builtin's
epoch is registry-owned and must bump with observable behavior; an exact
`[(name, epoch)]` golden plus one frozen input/output vector per builtin makes a
silent behavior change red. A package content hash does not cover host builtin
code and never substitutes for this epoch.

Plan digest frames, in effective order: entry count; canonical key; stage;
assigned order; provider discriminant and **typed components** (dependency
group/name or host variant/raw components, never `Display`); version; kind
presence/value; content-hash presence/value; implementation digest; config
presence/digest; selector presence and canonical dimensions. Provider roots are
excluded.

Selector package/path dimensions are OR-sets: digest strings byte-sort and
deduplicate within each dimension, so reordering an equivalent selector does
not make artifacts stale. Dimension absence remains distinct from present
empty. Entry order remains semantic and is never sorted by key.

## 5. Selector and document subject

The kernel remains the single glob compiler. `CompiledSelector` becomes public
with private fields, `PartialEq/Eq`, public `matches`, and read-only canonical
pattern accessors; compile stays private. `ExtensionRegistry` adds an
`enabled_at(point)` view in effective order **without subject filtering**.
Using today's `plan(point, subject)` during translation would incorrectly drop
selector-bearing transforms before any document exists.

`SelectorSubject` widens from package-only to a borrowed typed provider:

```rust
pub enum SelectorProvider<'a> {
    Dependency(&'a DependencyProviderId), Host(&'a HostIdentity),
}
pub struct SelectorSubject<'a> {
    provider: Option<SelectorProvider<'a>>, path: Option<&'a str>,
}
```

Compatibility dependency constructors remain. Package and host spellings route
through their existing typed identities only at match time.

Source/document transforms need a changing subject inside one artifact-wide
plan. Add immutable `DocumentSubject { provider, declared_path }` to
`ArtifactInput` → `SourceIr`; `DocumentIr` reaches it through its source.
`ContributionMeta.origin` stays display/provenance and gains no parsed identity.
The subject is part of compiler IR JTD so full-IR plugins see the same truth;
the inter-pass verifier requires it unchanged across source/document transforms.
Lane/emitted stages carry no selector by grammar.

## 6. Behavior registry and emitted mutation

`TransformRegistry` is a private `vibe-spec` sibling of `BackendRegistry`,
mapping implementation identity to `Arc<dyn TransformBehavior>`; the trait
object never crosses the crate boundary. Schedule construction resolves each
plan entry and wraps it as one level-preserving pass:

```text
source→source before parse; document→document after parse;
lane→lane after assemble; emitted→emitted after emit
```

Pass name is `transform:<stage>:<key>` and is checked against the implementation
identity; duplicate global pass names refuse. Source/document wrappers evaluate
the compiled selector against the carrier subject on every document.

An emitted behavior returns new bytes, never a mutable artifact reference. The
manager alone consumes the old `EmittedArtifact`, recomputes bytes digest and
provenance and appends the transform identity. No `bytes_mut` or
`provenance_mut` exists.

## 7. Empty plan law

`TransformPlan::empty()` owns no entries/allocation and `digest() == None`.
Appending no pass reproduces the exact historical schedule, errors and bytes;
there is no transform header and no per-unit plan frame. Node lanes still
recompute and transactionally no-op on equal bytes. Equality/order are derived
from semantic members; wall clock, filesystem path and registry all-view rows
do not enter.

## 8. Implementation atoms and gates

1. T1 compiler digest primitive + semantic config lowering/digest.
2. T2 TransformPlan/seed/provider/implementation + digest/refusals.
3. T3 kernel selector public ABI, host subject and `enabled_at` view.
4. T4 ArtifactPlan carries plan; compatibility pins empty; empty-plan byte REDs.
5. T5 behavior registry/name/epoch golden with identity behaviors.
6. T6 four positions wired; per-document/per-artifact invocation REDs.
7. T7 DocumentSubject carrier + compiler IR JTD/codegen/wire-diff.
8. T8 selector evaluation and immutable-subject verifier.
9. T9 manager-owned emitted reconstruction/provenance.
10. T10 workspace owner-view→plan adapter; no manifest type crosses upward.

Use exact crate tests/clippy/conform/specmap per atom. T7 runs codegen,
vibe-wire/vibe-spec tests, check-codegen and wire-diff. Full panel only after
coherent R4. No internal TransformPlan wire/schema is created merely because it
crosses Rust crates in one binary.

## 9. Rejected alternatives

- one host plan for every unit — contradicts `##COMPILE-ACTIVATION`;
- `toml::Value` or serde JSON as the public plan ABI — parser/library shape or
  lossy value tower rather than compiler semantics;
- provider `Display` strings in digests — formatting is not typed identity;
- selector filtering during plan construction — no document subject exists yet;
- selector pattern authored order in digest — OR-set order is not behavior;
- caller-authored order — sparse/duplicate ordinals become possible;
- `Arc<dyn Pass>` in the plan — behavior leaks across ownership boundary;
- package hash as builtin implementation digest — it does not cover host code;
- a TransformPlan JTD wire — no external reader exists.

## 10. Blockers

No owner-choice blocker remains. Package-lane activation and uninstall future
world are already ruled. Implementation prerequisites are ordered work: the
landed owner-control carriage (`52a59dcc`), atomic unit publication
(`91142777`/`ab68d145`), then T1–T10 above.

