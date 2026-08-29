# R4.1 TransformPlan ABI and digest — implementation design v0.1

Status: accepted central design, 2026-08-29; T1 `b65f9958`, T2 `49e944f0`, T3
`48d7dc75`, T4 `a252fcc8`, T5 `0eb46c82` and T6a `01f1522e` implemented.
Borrowed hash validation is `87ef2df6`; current map is `ce3e62bf`. Exact T2
construction/refusal/byte schedule, T4 carriage, T5 registry, T6 execution
split and T6b construction/error surface are frozen after adversarial review.
Semantic authority remains PROP-054 §§3.4,
7.1–7.3 and the R4 architecture. Execution status stays in the implementation
ledger.

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
entries, any selector supplied on lane/emitted, blank/unbounded implementation
identity and an invalid provider/version/hash. Selector absence is legal at
every stage. It enumerates order itself; a sparse/reordered caller ordinal is
unrepresentable.

### 2.1 Construction authority and refusals

The Rust sketch above is semantic, not permission for arbitrary callers to
author its fields. T2 lands the family `pub(crate)` behind private fields. A
seed carries a typed `ExtensionKey`, not a reparsed key string, and a
`TransformConfig(ConfigTable)` wrapper; the printable key enters the digest
only through `ExtensionKey::as_str()`. Public crate-root construction waits for
T10's real workspace consumer.

`TransformImplementation` is opaque. Its builtin name/epoch constructor is
private to the transform module: T2 tests can exercise it, T5's behavior
registry becomes its production authority, and workspace never supplies an
epoch. T2 does **not** advertise an empty or speculative builtin table. T5 adds
the closed name→epoch lookup and its `UnknownBuiltin` refusal together with the
behaviors it can actually return.

T2 validation is deterministic: checked entry count first; then each seed in
input order validates key scalar, duplicate key, provider/version/hash,
implementation, and selector/stage. The scalar law for a key, exact version or
ungrouped host name is nonempty and contains no ASCII control byte; it is not a
new SemVer parser and it does not trim or normalize accepted spelling.
Dependency group/name and coordinate hosts are already typed. Every required
or present `ContentHash` is nevertheless rechecked under the same grammar as
`ContentHash::parse`, through its borrowed `is_valid_spelling` predicate: the
type intentionally exposes `from_validated` to trusted hash producers, so
invalid Rust-constructed values remain reachable, while a multi-megabyte
refusal must not clone a parser error merely to discard it. `parse` and the
predicate share one grammar core. Accepted hashes retain their full exact
spelling, including `sha256:` versus `sha256-tree/1:`.

A builtin implementation name obeys the compiler's already-frozen
`BackendId` scalar grammar, `[a-z0-9][a-z0-9._-]{0,63}`, and its behavior epoch
is nonzero. The private implementation constructor may create a candidate, but
`TransformPlan::build` owns these refusals so an invalid candidate never enters
identity. More than `u32::MAX` seeds is checked with conversion, never by
allocating a test-sized vector; dense order is `0..len`. Config values need no
second validator because T1's private fields and checked datetime constructors
make the neutral tree the boundary.

Source/document stages permit absent or present selectors. Lane/emitted refuse
any supplied selector, even a behaviorally unscoped one, because manifest
presence itself is illegal there. At source/document, a compiled selector whose
two dimension accessors are both absent canonicalizes to outer absence;
`applies_to` absent and `applies_to = {}` therefore have one behavioral
identity. A present empty dimension remains present and matches nothing.

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

### 4.1 Exact T2 frame schedule

All tags below are epoch-1 bytes and cannot follow Rust enum layout. Every
`field(x)` is exactly `u64_le(x.len()) || x`. A child SHA-256 is a 32-byte
**field** (`field(digest32)`), not raw bytes; no second digest primitive is
introduced. Display's `sha256:` prefix is an output projection and never enters
another digest.

Implementation digest, after its domain field:

```text
byte(0=builtin), field(name UTF-8), u32_le(behavior_epoch)
```

Byte `1` is reserved for R5 native implementation identity; R5 must freeze its
payload before use. Plan digest is computed only for a nonempty plan. After its
domain field:

```text
u64_le(entry_count)
for each entry in effective input order:
  field(canonical ExtensionKey spelling)
  byte(stage: 0=source, 1=document, 2=lane, 3=emitted)
  u32_le(dense assigned order)
  provider:
    byte(0=dependency)
      field(group raw), field(name raw), field(version exact)
      field(PackageKind::as_str()), field(ContentHash::as_str())
    byte(1=host)
      byte(host: 0=ungrouped, 1=coordinate, 2=virtual-workspace)
      ungrouped: field(raw authored project name)
      coordinate: field(group raw), field(name raw)
      virtual-workspace: no component field
      field(version exact)
      byte(kind present); if 1, field(PackageKind::as_str())
      byte(content_hash present); if 1, field(ContentHash::as_str())
  field(implementation_digest32)
  byte(config present); if 1, field(config_digest32)
  byte(selector present); if 1:
    byte(packages dimension present); if 1:
      u64_le(post-dedup count), field(pattern UTF-8) for each canonical member
    byte(paths dimension present); if 1:
      u64_le(post-dedup count), field(pattern UTF-8) for each canonical member
```

Canonical dimension members are copied, UTF-8 byte-sorted and deduplicated;
the count is taken **after** deduplication. `None` writes only presence byte 0;
`Some(empty)` writes presence byte 1 plus zero count. Provider roots, registry
all-view rows, wall clock, filesystem paths, rendered `Display` identities and
Rust layout never enter. T3 makes `CompiledSelector::Eq` use the same canonical
OR-set law while its raw accessors retain authored order/duplicates, so Rust
semantic equality cannot disagree with plan identity merely because members
were reordered.

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

### 6.1 T5 registry freeze

T5 lands one private `TransformBehavior: Send + Sync` trait with four typed,
level-preserving methods (`SourceIr`, `DocumentIr`, `LaneIr`, emitted bytes) and
`Option<&TransformConfig>` delivered to each. A behavior declares one name,
nonzero epoch and stage; calling another stage yields a typed wrong-stage
refusal. `TransformRegistry` stores name → `{epoch, stage, Arc<dyn behavior>}`
in deterministic order. Registration refuses collision; resolution of one T2
implementation/stage refuses bounded unknown name, epoch mismatch or stage
mismatch before returning a cloned behavior. `TransformPlan::build` remains
grammar-only and never consults the registry.

T5 deliberately reserves **no shipping no-op builtin names**. The four identity
behaviors are cfg-test vehicles in a test-only registry, named
`test-identity-source|document|lane|emitted`, epoch 1. Their exact sorted
`[(name, epoch, stage)]` golden and one frozen input→identical-output vector per
stage prove the registry and future T6 seams without silently making public
manifest vocabulary out of test scaffolding. The production registry contains
only behaviors that actually ship; R4.2 adds `xml-minify` with its own epoch and
golden when its binding lands. Lifecycle `log`, T2's historical `minify` test
spelling and package content hashes never enter this catalog.

T5 adds no pass and reads no manifest/registry row. Behavior objects remain in
their own cells; the existing syntax fence keeps `Arc`/`dyn` out of plan/config/
digest/refusal cells. T6 alone wraps resolved behaviors into schedule passes and
fixes pass-name/global-collision laws.

### 6.2 T6 execution split and interim safety

T6 lands as three independently gated commits rather than one cross-cutting
rewrite:

1. **T6a — fallible discovery substrate.** `worklist::discover` accepts a
   fallible `SourceIr -> Result<DocumentIr, E>` callback and returns
   `Result<Worklist, E>` through every use/source/embed/simple recursion. All
   existing callers wrap their current infallible parse and preserve exact
   bytes/errors; no transform type, registry or pass enters this atom. A parse/
   transform failure can then propagate without the current private `.expect`
   or being mislabeled as a use-resolution failure.
2. **T6b — identity positions.** Resolve every plan entry against one injected
   private TransformRegistry before executing anything, preserve plan order
   within each stage, and append wrappers to the one CompilerPipeline at source
   before parse, document after parse, lane after assemble and emitted after
   emit. Production uses the empty shipping registry; tests inject the one T5
   cfg-test catalog. Pass name is `transform:<stage>:<ExtensionKey>`, with the
   existing global name set as backstop. Config is cloned into the wrapper;
   behavior objects never enter the plan. Registry/schedule/refusal errors stay
   typed and precede the first parse.
3. **T6c — lane witness.** Before any changed LaneIr is accepted, retain the
   immutable pre-transform witness, run intrinsic lane validation and the
   transition/equivalence check manager-side. Identity output remains the first
   commissioning vector; T6 is not complete merely because a no-op crossed the
   position.

Four temporary states are explicit, never silently approximated. A nonempty
compatibility-fragment plan refuses (compatibility constructors themselves stay
empty forever). A selector-bearing source/document entry refuses until T7/T8
provide the typed DocumentSubject — never execute unconditionally, use an
unscoped subject or parse Display provenance. A lane behavior returning a
different `LaneIr` refuses until T6c owns the immutable witness, intrinsic
validation and transition/equivalence check; full `LaneIr` equality is the
temporary detector, never a substitute for that witness. An emitted behavior
returning different bytes refuses until T9 owns reconstruction of digest/
provenance; byte-equal output returns the original EmittedArtifact untouched.
These are typed capability gaps, not `todo!`, panic or skipped rows.

### 6.3 T6b construction, refusal and error freeze

Schedule construction is a two-step transaction. First, reject any nonempty
plan whose `ArtifactFrame` is `CompatibilityFragment`; the rule covers custom
test targets as well as the public `static-fragment` adapter, while the T4
retarget oracle still proves that no plan was silently dropped. Then walk every
entry in dense plan order. For each entry resolve exact builtin name → epoch →
stage through the one injected registry, then reject a still-present source/
document selector as the temporary subject capability gap. A selector whose
two dimensions were both absent has already canonicalized to outer `None` and
does not refuse; a present-empty dimension remains present and does. The first
fault in this order wins. Only after the whole walk succeeds may the resolved
rows be stably partitioned by stage and inserted; no name sort, registry order
or `BTreeMap` iteration may reorder rows within a stage.

Each wrapper owns one cloned `Arc<dyn TransformBehavior>`, exact cloned config,
dense order/stage and a bounded key preview for faults. It owns the exact
`transform:<stage>:<ExtensionKey>` `PassName` as schedule identity, but no
failure reconstructs key/stage/order by parsing that rendered name. The wrapper
cell may render this mandated name and hold the one `Arc<dyn …>` channel; it is
still fenced from manifest/collector/row/path/codec access, `Box` behavior
ownership, `SelectorSubject` and selector `matches`. Plan/config/digest cells
remain behavior- and registry-free under their existing stronger fence.

Production injects `TransformRegistry::builtins()` (empty until R4.2); cfg-test
code alone may inject T5's one shared identity catalog. Resolution, capability
and pipeline-insertion failures are typed and happen before the first source
read/parse. Runtime behavior/capability failures remain typed through T6a's
fallible discovery or the artifact segment; they must be downcast from the
pass manager before the generic string-rendering pass/backend arms.
`ArtifactCompileError` therefore gains one public transform-family variant
holding an opaque public `TransformCompileError`; its private source retains
the exact internal enum for crate tests. No TransformPlan/registry/behavior
type becomes public. Legacy public `CompileError` gains no unreachable variant:
the public compatibility path constructs an empty plan and keeps its exact old
mapping, while crate-private prefix/lane helpers may use the artifact-level
error family. Do not add `#[non_exhaustive]` or a public fault taxonomy merely
to anticipate T7–T10; T10 freezes external inspection when a real consumer
exists.

T6b intentionally retires T4's claim that an attached nonempty plan is inert.
The empty-plan half stays byte/error/schedule exact. A nonempty plan under the
empty production catalog now refuses before parse; the same plan under the
injected identity catalog adds the exact positions and preserves bytes/
provenance because its behaviors actually ran. Tests must distinguish those
causes rather than keep the old inert-carriage comparison green by accident.

## 7. Empty plan law

`TransformPlan::empty()` owns no entries/allocation and `digest() == None`.
Appending no pass reproduces the exact historical schedule, errors and bytes;
there is no transform header and no per-unit plan frame. Node lanes still
recompute and transactionally no-op on equal bytes. Equality/order are derived
from semantic members; wall clock, filesystem path and registry all-view rows
do not enter.

### 7.1 T4 ArtifactPlan carriage law

T4 adds one private `TransformPlan` field to `ArtifactPlan`. Every existing
constructor (`new`, `compatibility`, `static_lane`, test custom target) pins
`TransformPlan::empty()` without changing its signature. A crate-internal
whole-value replacer attaches an already-built plan and a read-only accessor
lends it; there is no mutable entry/order API. T10 widens only the minimum
needed by the workspace adapter.

Carriage is deliberately inert in T4: empty and nonempty plans add no pass,
header, fingerprint frame, wire member, bytes, error or mtime change. Execution
begins only with T5/T6; fingerprint/header wiring lands in its named later atom.
Compatibility wrappers therefore remain empty-plan forever. Any test vehicle
that rebuilds an `ArtifactPlan` for another backend must forward the whole plan
rather than reconstructing only contributions — silently dropping a nonempty
plan is the one carriage regression T4 must make red.

## 8. Implementation atoms and gates

1. **Done `b65f9958`:** T1 compiler digest primitive + semantic config lowering/digest.
2. **Done `49e944f0` (`87ef2df6` borrowed hash law):** T2
   TransformPlan/seed/provider/implementation + digest/refusals.
3. **Done `48d7dc75`:** T3 kernel selector public ABI, host subject and
   `enabled_at` view.
4. **Done `a252fcc8`:** T4 ArtifactPlan carries plan; compatibility pins empty;
   empty/nonempty schedule/byte/error/retarget REDs; map `5aa44611`.
5. **Done `0eb46c82`:** T5 private behavior registry/name/epoch golden with
   test-only identity behaviors; map `0f73cdfe`.
6. **Done `01f1522e`:** T6a fallible discovery propagates the exact generic
   callback error through every recursion; existing callers use one exhaustive
   `Infallible` adapter. Map `ce3e62bf`.
   **Current:** T6b identity positions, then T6c lane witness;
   per-document/per-artifact invocation REDs.
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
