# R5 compiler-native continuation architecture v0.1

Status: frozen for implementation by the central session, 2026-08-31.

This record amends only the serial order and implementation boundary after
accepted R5.3. It does not reopen the native ABI, loader, artifact or lifecycle
wiring accepted in `R5-NATIVE-ABI-ARCHITECTURE-v0.1.md`.

## 1. Evidence-driven serial route

R5.4 cannot clear `transforms-pending` merely because Cargo produced a cdylib:
until the compiler invokes that transform, removing the marker would claim
behavior that never ran. Generic compiler-native invocation must therefore
precede honest convergence.

Invocation itself cannot use the current native `Context`/`Reply`: those roots
carry lifecycle run/artifact/slot fields and no compiler payload. Smuggling IR
through those members, copying the compiler DTO, or weakening the existing
strict compiler reader are all rejected. The serial route is:

1. **R5.5-WIRE-PROJECTION** — one canonical strict compiler-IR generated type
   family plus an explicit schema-declared permissive request-field projection;
2. **R5.5-WIRE** — separate `native-compile-request` and
   `native-compile-reply` roots;
3. **R5.5-WIRE-GATE** — sharing, strictness, compatibility and codegen proof;
4. **R5.5-INVOKE** — generic four-stage compiler-native execution only;
5. **R5.4** — pending bootstrap and one build-fence recompile;
6. **R5.5-PARITY** — builtin/native minify parity and owner scenario 10.2.

## 2. Wire reader asymmetry

The existing phase/slot native `Context` and `Reply`, `vibe-ext` safe handler
surface, manifest and four C symbols remain byte- and behavior-compatible.
`native-compile-request` carries envelope, point, execution, project, world, IO
and one canonical compiler-IR payload — no lifecycle run, accumulated artifacts
or slot target. It is a foreign-reader root: unknown object members inside the
projected IR payload are ignored recursively, while required members, types,
discriminators and closed vocabularies still refuse.

`native-compile-reply` is host-strict and status-discriminated: `ok` requires
exactly one compiler payload; `skip` and `fail` forbid one. Both roots refer to
the SAME compiler-IR family. The canonical shared structs stay strict; a new
schema metadata marker may request a permissive deserialization projection only
on a reference in a permissive registry consumer whose canonical fragment has
an unprojected strict owner. Ordinary mixed shared-reader policy remains a
refusal. The adapter is generated from the resolved JTD closure, not a second
DTO, handwritten field list or runtime schema parser. Existing
`generated::compiler_ir::e1::ir::*` paths remain re-exports with identical
`TypeId`.

## 3. Generic invocation boundary

`vibe-spec` retains ownership of stage encoding/decoding, selectors,
cardinality, reconstruction, provenance and every intrinsic/transition
verifier. It receives one borrowed compiler-native invoker beside builtin
`TransformBehavior`; it never depends on lifecycle, loader or Cargo. The outer
composition owns the exact candidate/mechanism/routes epoch and injects the
ARTIFACT-backed implementation.

Source, document, lane and emitted calls use the four existing schedule
positions with `ir_schema = 1`. `ok` returns the exact expected carrier;
`skip` returns the original carrier unchanged; `fail`, malformed payload,
stage/carrier mismatch, loader error or plugin panic stops compilation with
bounded transform attribution. Emitted output is reconstructed by the manager,
which recomputes digest and pass provenance; native code never authors either.
Invocation resolves records, publishes immutable images and calls the shared
loader only. It never builds, probes Cargo/rustc or guesses `target/`.

## 4. Pending convergence boundary

Install compilation uses the exact incoming/post-install world rather than
turning ambient-lock disagreement into `TransformPlan::empty()`. Only a missing
buildable source artifact becomes pending; invalid declarations, unsupported
platforms and unavailable/corrupt terminal prebuilts remain failures. Pending
entries retain effective order, owner, implementation/config identity and
selector semantics; they are framed and fingerprinted but never appear in the
executed-transform header.

At the complete build fence the order is native source builds → one pending-lane
recompile → authored artifact targets → phase contributions. The second compile
must actually invoke every previously pending transform and return an empty
pending set. Empty pending causes no extra compile. Build/recompile failure
preserves the prior whole-artifact transaction; a later command retries, with
Cargo free to report fresh. R5.5-PARITY alone then commissions the native XML
minifier against the builtin byte oracle.

## 5. Acceptance shape

WIRE-PROJECTION proves that request permissiveness is generated from schema
metadata without changing strict canonical structs or ordinary mixed-reader
refusal. WIRE registers both new roots, strict status/payload grammar, corpus
and relational stage/carrier validation. WIRE-GATE proves exact sharing,
legacy native root byte stability, codegen idempotence and wire diff.

INVOKE proves 5/5/1/1 source/document/lane/emitted cardinality, selector
zero-call, exact config/order, `ir_schema = 1`, stage-specific reconstruction,
mandatory verification, no lazy build and real SDK→loader identity execution.
R5.4 proves pending framing/fingerprint, install without Cargo, one build-fence
retry and transaction-preserving failure. PARITY proves native and builtin XML
minify byte identity and the complete commissioning scenario.

## 6. R5.5-WIRE-PROJECTION ratification

**Accepted 2026-08-31.** Product `9051aade`, trace engine/sync `dc9cff48` and
map `5adea048` land the projection prerequisite without native compile roots or
runtime invocation. Compiler IR is now one strict shared family; the legacy
compiler module contains 118 exact re-exports and no declarations. The thin
schema names the canonical `ir` vocabulary root and its 55-member closure.

The new `x-reader-projection = "permissive"` marker is admitted only on one
object-member ref in a registry-permissive consumer with an unprojected strict
owner. Projected-only consumption does not enter the ordinary shared-policy
join, so unmarked strict/permissive mixtures still refuse. A generated local
serde visitor rejects duplicate object members at every depth, prunes unknown
members according to the resolved JTD closure, then decodes into the canonical
strict type. Required members, types, discriminator tags and closed vocabularies
remain strict; empty objects clear unknown members. Recursive marker census and
generated-field accounting make every marker exactly-once.

Independent native review rejected the initial PASS for three defects:
`serde_json::Value` duplicate collapse, consumer-marker census blind spots and
empty-object invalid Rust. The correction added duplicate root/nested/arm
refusals, arbitrary misplaced-marker refusals and an empty-object runtime pin.
Nine worker plus two central projection mutations were RED and restored.

Moving schema definitions exposed a traceability loss: the old scanner saw
only inline `definitions`, dropping 55 items and one root edge. The first repair
packet incorrectly targeted frozen core-ai-native v0.8 and was discarded. The
current authored v1.0 engine now reads one explicitly configured, canonically
contained vocabulary file and projects only same-named thin shared roots;
ordinary and alias schemas never duplicate units. Vocabulary member spans and
edge provenance come from authored vocabulary bytes. Review then rejected
absolute/traversal/symlink authority; the correction added bounded typed
refusals and a real Windows symlink-escape pin before official vendor sync.
Five worker plus two central trace mutations were RED and restored.

Final gates: xtask codegen 156, complete vibe-wire 321, compiler IR 7/6/5/8 +
sharing 2, whole generated tree 82-file hash-idempotent, staged
`check-codegen` clean, wire-diff one schema + one vocabulary / zero corpus,
strict check/clippy/fmt and conform zero-new. The authored specmap engine passes
153 unit + 7 doctests; six current vendor copies are hash-identical and
`sync-engines --check` is 51/51. Specmap remains 6,833 units / 3,036 tagged /
2,789 edges, 0 suspects, gated orphans or unresolved host edges, 25 warnings.
R5.5-WIRE is the next consumer.

## 7. R5.5-WIRE ratification

**Accepted 2026-08-31.** Product `ed6e7c2a` and map `f0dfaf33` register the
separate epoch-1 `native-compile-request` and `native-compile-reply` roots with
four authored corpus documents. The request contains only envelope, point,
execution, project, world, IO and the projected canonical IR payload. The reply
is a strict `ok | skip | fail` discriminator: `ok` requires the payload and the
other statuses forbid it. Both roots re-export the one shared compiler-IR
family; all 118 legacy compiler paths remain declarations-free re-exports and
the 12 pre-existing native format/corpus/schema hashes remain exact.

The behavior validator admits the four landed stage/carrier pairs plus `pass`,
requires envelope and IR epoch 1, refuses unknown points with typed bounded
control-safe diagnostics, and proves request/reply exchange shape rather than
rewriting it. Forward members are ignored only inside the request payload;
duplicate known keys, missing or mistyped members, discriminators and closed
vocabularies remain strict. The projection lint allowance is local to the
generated helper. Codegen now formats only the staged projected module after
rewiring, so generated bytes are idempotent without formatting unrelated
authored or generated files.

Independent native review rejected the initial PASS for permitting control
characters in the unsupported-point diagnostic and for a broad lint allowance
over the generated tree. Both were corrected and pinned. Ten worker wire
mutations, one generator-format mutation and two central behavior mutations
were RED and restored.

Final gates: native compile wire 10, reader projection 8, complete xtask
codegen 158 and vibe-wire 331; staged `check-codegen` clean; `wire-diff` reports
exactly two schemas, four corpus files and one registry edit; strict workspace
check/clippy/fmt and conform zero-new. Specmap is 6,833 units / 3,038 tagged /
2,789 edges, 0 suspects, gated orphans or unresolved host edges, 25 warnings.
R5.5-WIRE-GATE is the next consumer.

## 8. R5.5-WIRE-GATE ratification

**Accepted 2026-08-31.** Test gate `36b7f1e2` closes the relational mutation
surface without a product, schema, registry, corpus, generated-wire or specmap
change. One six-carrier table now proves all 30 point/carrier combinations:
the four stage points each admit exactly one carrier, `compile:pass` admits all
six, and every rejection equals the complete typed `StageCarrier` value. A
second table proves all 36 request/`ok` reply combinations, including exact
`documents-artifact` and `closure-artifact` shapes; all six equal pairs succeed,
all 30 unequal pairs equal the complete `ExchangeShape`, and skip/fail preserve
every request shape.

Raw duplicate-known-member pins now cover request root, an ordinary projected
payload object, reply root and a strict reply payload object. Twelve raw
known-level/cardinality substitutions prove the carrier-specific generated
single-value enums refuse structurally. This also resolved a review
disagreement: `validate_ir` does not need to duplicate those generated enum
checks, but the previously unpinned stage/exchange matrix did permit real
surviving mutations and therefore justified the separate test-only gate.

Three worker mutations and one different central stage/carrier mutation were
RED and restored: broaden source admission, narrow pass admission, bypass
unequal exchange, and admit lane at emitted. Final gates: native compile wire
11, reader projection 8, complete vibe-wire 332 and xtask 242; `check-codegen`
clean; post-publication `wire-diff` clean with the historical accepted delta
still exactly two schemas, four corpus files and one registry edit; all 12
legacy native paths unchanged; strict workspace check/clippy/fmt and conform
zero-new. Specmap remains 6,833 units / 3,038 tagged / 2,789 edges, 0 suspects,
gated orphans or unresolved host edges, 25 warnings. R5.5-INVOKE is next.

## 9. R5.5-INVOKE implementation freeze

**Frozen 2026-08-31 after five native `gpt-5.6-sol`/`xhigh` reviews.** The
current install compiler and the production native epoch are not yet one
transaction: node/unit static compilation happens inside prerequisite install,
while `phase::run` creates the `RitualPlan`, mechanism routes and native
candidates only afterwards. INVOKE therefore lands and proves the generic
manager, SDK, loader and artifact-backed execution boundary directly. It does
not thread a recomputed or selected-root-only epoch through production install.
R5.4 alone creates the one incoming/post-install owner runtime, builds pending
sources and performs the one native-aware recompilation through plain, traced
and observed node/unit paths.

### 9.1 Serial children

R5.5-INVOKE is a workstream with five serial children:

1. **INVOKE-MANAGER** — native plan identity, borrowed manager seam, strict
   reply conversion and four-stage execution;
2. **INVOKE-SDK** — safe compiler author macro and separate fixture;
3. **INVOKE-LOADER** — compile-specific raw exchange over the existing ABI;
4. **INVOKE-ARTIFACT** — exact-row artifact/image/shared-loader adapter;
5. **INVOKE-GATE** — fake-manager matrix plus real SDK→artifact→loader proof.

No child may absorb R5.4's pending interpretation, source build, production
workspace/install threading or epoch reordering.

### 9.2 Native plan identity

`TransformImplementation` becomes a private kind with `Builtin { name, epoch
}` and `Native { handler }`. Existing builtin digests remain byte-exact. Under
the already-reserved implementation tag `1`, native implementation digest
frames, in order: the existing implementation domain; tag `1`; ABI epoch `1`;
compiler IR schema `1`; `crate_dir` presence plus its canonical portable
authored spelling; `prebuilt` presence, count, then platform-key/path pairs in
key order. Extension key, provider, stage, dense order, config digest and
selector remain in their existing outer plan frames. Resolved roots, selected
platform artifact, image path, Cargo state, wall clock and run context never
enter this digest. Artifact bytes and source witness belong to R5.4's runtime
and build fingerprint.

### 9.3 Borrowed manager seam

`vibe-spec` owns a public `CompilerNativeInvoker`, borrowed as `&dyn` and never
wrapped in `Arc`. Its call carries the exact qualified `ExtensionKey`, typed
compile point, manager-assigned dense order, manager-projected handler-visible
effective config, the opaque manager-owned native implementation digest and
canonical generated `Ir`; it returns owned raw reply bytes through a typed
bounded error that preserves a distinct buildable-source-unavailable class for
R5.4. `Pass` loses only its pass-object `'static` bound;
`PassSegment`, `CompilerPipeline` and `BuiltinSchedule` carry the borrow
lifetime. IR payload and pass-error types remain `'static`.

The selector verdict precedes the invoker call. `ok` processing is exactly:
duplicate-preserving strict reply-root decode → reply/request exchange
validation against the manager-authored point/payload → canonical generated-
to-domain conversion → stage admission/reconstruction → intrinsic and
transition verification of the final manager-owned carrier. `skip` returns the
original carrier; `fail`, malformed bytes, epoch/shape/conversion failure or an
invoker error stops compilation with bounded entry attribution. Each native
wrapper applies the existing intrinsic and transition verifier locally to its
own reconstructed result and immutable input witness. Native presence does not
enable pipeline-wide verification for adjacent builtins or change their
behavior; R6 retains ownership of general mandatory verify-each.

Source/document preserve manager identity and accept only their mutable body;
lane uses the existing witness plus intrinsic/transition admission. An emitted
plugin must return a wire value whose temporary provenance/digest is internally
self-consistent so strict conversion can read it, but the manager discards
those members and reconstructs from the original artifact plus returned bytes.
Only that reconstruction authors the authoritative digest and pass provenance.

### 9.4 SDK, loader and root-family law

`vibe_compile_extension!` is a backward-compatible sibling of
`vibe_extension!`: typed `CompileRequest -> CompileReply`, panic containment and
the same four ABI-1 symbols/boxed-slice ownership. One cdylib is root-family
homogeneous: lifecycle images declare only phase/slot entries; compiler images
declare only compile entries. Loader manifest admission rejects a mixed image
before invoke. A package needing both families ships two images. No ambiguous
double macro or implicit parse fallback exists.

`NativeLoader::invoke_compile` is compile-specific, accepts already encoded
request bytes and returns an owned raw response `Vec<u8>`. It shares the
canonical path cache, four-symbol/ABI/manifest admission, reply cap and exact-
once RAII free path with lifecycle invocation. It is not a generic serde/FFI
escape hatch and does not decode the compile reply; `vibe-spec` remains the
strict reader. Existing lifecycle `invoke` and its typed reply are unchanged.

### 9.5 Artifact adapter and request authority

The artifact invoker captures one ordered all-compile-row epoch plus its native
candidate/mechanism/routes epoch, injected shared `Project`/`World`, run id,
platform/offline/timestamp and selected root. It indexes the all-row sequence by
the manager's dense order, then requires exact qualified key, native handler,
point and effective-config agreement. It recomputes the frozen native
implementation digest from that retained row and requires equality with the
opaque manager value before artifact resolution; a changed `crate_dir` or
`prebuilt` map can never execute under an older plan identity. It never
recomputes order from the native-only subset. The retained row supplies
declaration id, provider identity and artifact authority; `execution.id` keeps
the shared lifecycle meaning (the declaration id), while the qualified key
remains the manager lookup identity.

The adapter builds the full request with envelope/schema 1, manager point and
payload, row provider and effective config, injected project/world and a
contained scratch path allocated only after selector admission. The plan keeps
absent and authored-empty config as different identities, while the existing
mandatory `Execution.config` map deliberately projects both to `{}`; every
handler-visible value otherwise stays exact. A selector miss performs no
scratch allocation, artifact resolution, image publication or loader call.

Artifact resolution revalidates a prebuilt or stable source record, publishes
the immutable digest image and calls the process-shared loader. Missing, stale,
corrupt, load, panic and reply-fail conditions are hard errors here, never skip;
no Cargo/rustc process, lazy build, mutable-artifact load or `target/` guess is
reachable. Only the typed buildable-source-unavailable class may later become
pending, and only R5.4 makes that decision.

### 9.6 Gate and no-go rules

The manager gate proves exact 5/5/1/1 source/document/lane/emitted calls,
selector zero-call, builtin/native authored order, dense order/config delivery,
schema 1, ok/skip/fail, hostile reply bounds, all stage reconstructions and
mandatory verification. SDK/loader/artifact gates prove the homogeneous-family
manifest law, legacy lifecycle byte behavior, exact response free, no lazy
build, same process loader and a real compiler fixture through
row→ARTIFACT→immutable image→four-symbol ABI.

No `Arc` invoker, lifecycle Context/Reply reuse, loader-side compile decode,
`serde_json::Value` reply collapse, adapter selector evaluation, second DTO,
lazy Cargo/probe, second ritual/runtime epoch or production workspace/install
threading is permitted in R5.5-INVOKE.

## 10. R5.5-INVOKE-MANAGER ratification

**Accepted 2026-08-31.** Product `846979f9` and map `150f0866` land the
manager-only child without SDK, loader, lifecycle artifact or production
install changes. The opaque transform implementation family now has byte-
compatible builtin and frozen native arms. Native digesting uses reserved tag
1, ABI/schema epoch 1 and exact fallible UTF-8 portable `crate_dir`/`prebuilt`
identity; invalid OS strings refuse typed and cannot collide through lossy
replacement. One public opaque digest function is shared by plan lowering and
the later ARTIFACT adapter.

The compiler accepts one stack-borrowed `CompilerNativeInvoker`. Calls carry
qualified key, typed point, dense order, effective config, opaque implementation
digest and the canonical generated IR. Pass/pipeline/schedule lifetimes borrow
that invoker without `Arc`; old plain/traced/observed entries refuse native
plans explicitly, while native-aware siblings share the same manager schedule.
Replies cross the raw duplicate-key walker, strict generated reply reader,
epoch/status/carrier checks and canonical conversion before native-local
intrinsic/transition verification. Source/document identities, lane admission
and emitted reconstruction remain manager-owned; native presence never enables
global verification for adjacent builtins.

The first native review rejected the initial PASS for lossy path digesting,
missing hostile/order/config/status/stage coverage and incomplete no-go fences.
The correction made path projection fallible, split test support/matrix/hostile
cells, and added per-cell plus exact import/DAG fences. A second review found
only one stale typed-Result doc sentence; after correction it returned PASS.

Final gates: focused native manager 22, complete vibe-spec 875 unit + 5/2/7
integration + 4 doctests, downstream workspace/orchestrator check,
`check-codegen`, strict workspace check/clippy/fmt and conform 48 standing/0
new. Ten worker and one different central carrier mutation were RED and
restored. Specmap is 6,833 units / 3,041 tagged / 2,792 edges, 0 suspects,
gated orphans or unresolved host edges, 25 warnings. R5.5-INVOKE-SDK is next.

## 11. R5.5-INVOKE-SDK ratification

**Accepted 2026-08-31.** Product `777dbb5e` adds a backward-compatible safe
compiler author surface without loader product changes. One hidden emitter now
owns the four ABI-1 symbols, stable manifest cache, output-slot initialization,
panic containment and exact boxed-slice publication/free for both public
macros. Existing `vibe_extension!` syntax and lifecycle behavior remain exact;
`vibe_compile_extension!` admits a generated `CompileRequest`, retains its
shape, moves the request into the handler without cloning IR, validates the
typed `CompileReply` against that shape and serializes only an admissible reply.

The wire behavior gained one shape-based reply helper, and the existing
exchange validator delegates to it without changing the 36-pair law. A separate
compiler-only `rlib`/`cdylib` fixture is registered through the loader's normal
dev-dependency graph; every fixture entry is compile-family/schema 1 and the
lifecycle fixture is byte-unchanged. Root-family enforcement remains correctly
owned by INVOKE-LOADER.

Final gates: compiler SDK author/raw ABI 1+8, unchanged lifecycle author/raw
ABI 1+6, complete vibe-ext 16 integration assertions, complete vibe-wire 132
unit plus all integrations and 2 doctests, fixture registration 1 plus 2
doctests, loader all-targets check, `check-codegen`, strict workspace
check/clippy/fmt and conform 48 standing/0 new. Eleven worker and one central
exchange-refactor mutation were RED and restored. Specmap remains 6,833 units /
3,041 tagged / 2,792 edges, 0 suspects, gated orphans or unresolved host edges,
25 warnings. R5.5-INVOKE-LOADER is next.

## 12. R5.5-INVOKE-LOADER ratification

**Accepted 2026-08-31.** Product `126dfc0b` and trace refresh `feb98591`
extend the one process-safe loader without changing SDK, fixtures, lifecycle or
artifact composition. `NativeCompileInvocation` carries an absolute resolved
library, exact extension id, typed `CompilePoint` and borrowed encoded request;
the loader fixes schema 1, passes request bytes through untouched and returns an
owned raw reply without decoding compiler JSON.

Lifecycle and compiler methods now share canonical path/cache/ABI/manifest and
one admitted-response RAII path. Lifecycle still strictly decodes generated
`Reply` while the guard lives; compiler copies raw bytes while the same guard
lives, then frees exactly once. Whole-manifest admission preserves duplicate-id
precedence, parses every point before a verdict, requires homogeneous typed
lifecycle or compiler family, then checks exact selected id/point/schema.

Independent review rejected the initial PASS because an early family mismatch
could hide a later invalid point and the response matrix compared two paths
without independently proving free counts. The correction collects all typed
points first and asserts 0/1 frees per lifecycle and compiler case; review then
passed.

Final gates: focused compile loader 9, complete loader 20 unit + 3 integration
+ 4 doctests, SDK compatibility 1+1+6+8, real compiler ok/skip/fail/panic/after
on one loader, `check-codegen`, strict workspace check/clippy/fmt and conform 48
standing/0 new. Fourteen worker and one central duplicate-precedence mutation
were RED and restored. Specmap remains 6,833 units / 3,041 tagged / 2,792
edges, 0 suspects, gated orphans or unresolved host edges, 25 warnings.
R5.5-INVOKE-ARTIFACT is next.

## 13. R5.5-INVOKE-ARTIFACT ratification

**Accepted 2026-08-31.** Product `6b5ab9e9` and map `a5a69a0c` add one
lifecycle-owned borrowed `ArtifactCompilerNativeInvoker` without production
workspace/install threading. It indexes the complete all-compile-row epoch by
manager dense order, then requires exact qualified key, enabled native handler,
typed point, shared effective config, opaque handler digest and pointer-identical
membership in the captured native candidate epoch. Injected `Project.root` must
canonically equal the selected project root before scratch.

After admission, contained scratch is keyed by the qualified row, the generated
schema-1 request receives declaration id, exact provider, manager-owned config,
Project/World and move-only canonical IR, then only a verified prebuilt or
stable source record reaches immutable image publication and the process-shared
raw loader. Compiler reply bytes return untouched. Missing source records alone
retain `BuildableSourceUnavailable`; stale/malformed/corrupt records, prebuilt,
image, loader, manifest and panic failures stay hard invocation errors.

Independent review rejected the initial PASS because an all-row from one epoch
could be resolved against another native candidate slice, and injected project
identity could disagree with the root owning scratch/artifact state. Pointer
membership and canonical root equality closed both before scratch; review then
passed.

Final gates: focused artifact adapter 14, complete lifecycle 607 passed / 3
privilege ignores + 39 doctests, manager 875 + 5/2/7 + 4 docs, SDK 1+1+6+8,
loader 20+3+4, downstream workspace/orchestrator, `check-codegen`, strict
workspace check/clippy/fmt and conform 48 standing/0 new. Eighteen worker and
one central provider-authority mutation were RED and restored. Specmap is 6,833
units / 3,042 tagged / 2,793 edges, 0 suspects, gated orphans or unresolved
host edges, 25 warnings. R5.5-INVOKE-GATE is next.

## 14. R5.5-INVOKE-GATE and parent acceptance

**Accepted 2026-08-31.** Test gate `9b1e4525` closes the one junction left
between the complementary accepted panels without changing product. The
compiler fixture adds one schema-1 `compile:source` id whose safe handler moves
and lawfully marks Source IR. One composed test lowers that exact retained row,
attaches it to a real artifact plan and executes public
`compile_artifact_native` through the exact-row ARTIFACT adapter, immutable
image, real four-symbol SDK/loader, raw reply, strict manager decode/conversion
and local transition verification. The marker reaches final emitted bytes.

Two native gate reviews initially split: the evidence review accepted the
complementary panels, while the adversarial review showed their positive paths
met only at `compile:pass`, which the manager never authors. The literal source
junction closes that survivor. Six worker mutations plus one central empty-plan
mutation were RED and restored: source→pass at fixture/adapter, marker removal,
request-as-reply, ignored Ok payload, digest admission and missing TransformPlan.

Final panels: composed junction 1, manager 22, artifact adapter 15, loader
20+3+4 docs, SDK 1+1+6+8, complete vibe-spec 875 + 5/2/7 + 4 docs, complete
lifecycle 608 passed / 3 privilege ignores + 39 docs, `check-codegen`, strict
workspace check/clippy/fmt and conform 48 standing/0 new. Specmap remains 6,833
units / 3,042 tagged / 2,793 edges, 0 suspects, gated orphans or unresolved
host edges, 25 warnings.

R5.5-INVOKE-MANAGER, SDK, LOADER, ARTIFACT and GATE are all accepted; parent
R5.5-INVOKE is done. R5.4 pending bootstrap convergence is next and alone owns
production owner-runtime epoch composition, source builds and the one native-
aware recompilation.

## 15. R5.4 pending convergence implementation freeze

**Frozen 2026-08-31 after three native `gpt-5.6-sol`/`xhigh` design reviews
and one arbitration.** R5.4 is a seven-child serial workstream:

1. **EPOCH-WORLD** — one ordered-resolution installed-world snapshot with
   retained package manifests/routes, no ambient lock reread;
2. **EPOCH-LOWER** — one node/unit owner runtime lowering over that snapshot;
3. **PENDING** — manager Collect/Resolve policy, ordered pending identity and
   truthful header/fingerprint;
4. **WORKSPACE** — native-aware plain/traced/observed node+unit compilation and
   one replay preparation/publication core;
5. **INSTALL** — exact Empty/Fresh/Ready epoch construction and transfer;
6. **FENCE** — native build → one strict replay → authored targets → phase rows;
7. **GATE** — multi-node, no-Cargo, failure, trace, freshness and mutation panel.

Children are serial. Parallel versions would create temporary duplicate world,
runtime, pending or replay authorities at the boundaries this work exists to
make singular.

### 15.1 One world and owner-runtime epoch

`regenerate_boot_from_traced` already receives the exact ordered resolution;
it must stop rereading ambient `vibe.lock`. Missing, malformed or disagreeing
worlds, owner closure failures and unknown owners are typed failures, never
`TransformPlan::empty()`. An explicit `ExtensionWorldEpoch` is built once from
the supplied ordered resolution and retained parsed manifests/materialised slot
roots. Fresh uses the durable lock snapshot; Empty uses an explicit empty
package sequence; Ready uses the in-memory post-install overlay after manifest
and deferred in-place updates, before boot generation and lock write.

The epoch projects exactly one `OwnerRuntime` per workspace-relative node and
one per package `UnitId`. Each runtime owns the one collected owner view and
retains its complete compile-row order, TransformPlan, mechanism registry and
routes, native candidate indices, request Project/World and stable runtime
identity. Candidate indices refer to the same owned row storage; no
self-referential borrowed vectors or native-only reorder exists. Root/member
node controls may differ; one package-unit runtime is shared globally wherever
that unit is referenced.

Run id, selected state root, platform, offline posture and timestamp live once
in epoch-common facts. A borrowed owner-runtime view constructs the accepted
ARTIFACT invoker. The selected node's retained runtime is the source of normal
phase planning after install; `phase::run` does not recollect a second ritual,
registry, route or compile epoch.

### 15.2 Pending is runtime state, not plan identity

`TransformPlan` and its frozen digest remain unchanged. A native-aware compiler
policy has three modes: ordinary hard **Fail**, install-time **Collect**, and
build-replay **Resolve(expected)**. Only typed
`BuildableSourceUnavailable`, observed after selector admission, may Collect:
the original carrier continues and one pending reference is recorded. Every
other artifact/prebuilt/platform/declaration/loader/protocol/panic error remains
hard. Repeated document calls coalesce by original dense order; conflicting
captures refuse.

One pending reference is `(plan digest, original dense order, qualified key)`;
the plan digest already binds provider, stage, native implementation/config,
selector and effective order. Workspace frames it beneath a portable owner and
artifact target/format, then additionally binds current platform, source
witness, handler/config witness, `build:cargo` route and selected build-provider
semantic identity under domain `vibe-transform-pending-v1\0epoch=1\0`.
Absolute roots, mutable target/image paths, run id, wall clock, Cargo fresh bit
and traversal order never enter.

After build, convergence evidence binds the original pending fingerprint,
immutable built-artifact digest, toolchain witness and actual `(owner, order)`
invocation receipts from replay. Resolve requires every expected order to be
invoked, no unexpected pending, and an empty resulting pending set. No third
compile is possible.

### 15.3 Truthful published pending artifacts

PROP-054 `BOOTSTRAP-ORDER` is literal: install with a missing buildable compiler
native publishes an honest usable artifact compiled without that transform and
marks it `transforms-pending`. Pending rows are excluded from the executed
`vibe:transforms` header; selector misses and ordinary handler skip keep their
existing active-header semantics. A separate generated comment payload is:

`vibe:transforms-pending sha256:<pending-fingerprint> <order>=<encoded-key> ...`

in dense order using the one generated-comment key codec. It may also project
the plan digest in its fingerprint input. The header is evidence only and is
never parsed to recover runtime state; direct install may drop its in-memory
continuation. A later build's prerequisite install rediscovers pending from the
same exact world and republishes the same bytes/fingerprint. No second durable
pending journal or declaration DTO is introduced.

Pending header/fingerprint bytes participate in artifact/output freshness, but
never in executed-transform provenance. A pending output is a successful
ordinary per-owner artifact transaction, not an unpublishable provisional
result. Install runs no Cargo.

### 15.4 Workspace replay and publication law

Plain, traced and observed native-aware compiler entries share one policy/result
core. Both package-unit and node lanes use the same owner-runtime view; a fresh
skip includes the runtime/pending fingerprint so a missing or changed native
cannot hide behind a plan-only fingerprint. No static entries means no compile,
trace occurrence, pending item or replay lane. Selector miss means no invoker,
scratch, pending or build request.

Initial install compilation publishes pending artifacts normally. During the
same `vibe build`, prerequisite install returns an owned replay set and the same
runtime epoch to the build fence. After source build, one workspace replay
prepares every affected lane's bytes before touching its published artifact,
forces those lanes through their full retained plans under Resolve, and requires
complete invocation receipts plus zero pending. Empty pending causes zero replay
calls. Only recorded affected lanes recompile once; freshness cannot skip them.

Build or replay failure leaves the already-published pending artifact set
untouched. Publication uses the existing crash-recoverable per-owner
INDEX/selected-STATIC/stale-STATIC transaction: caught pre-commit failures
restore that owner's pending artifact; post-commit/cleanup failures may retain
committed-new state plus that transaction's durable recovery intent and converge
later. No global cross-owner atomic rollback is claimed without a new explicit
coordinator. All replay bytes are nevertheless prepared before the first
publication, and a caught publication error stops later owners and runs the
available per-owner recovery.

### 15.5 Install and build-fence sequence

Ready splits materialisation from boot generation: apply manifest/deferred
in-place updates, materialise/prune and pre-install lifecycle, finalize the
in-memory lock/resolution, compose the post-install epoch, run pending-aware
boot generation, then write the lock and run post-install lifecycle. The apply
report returns the exact workspace plus epoch; Empty and Fresh return equivalent
epochs from their exact inputs. Direct `vibe install` may discard the runtime
after publishing pending output. Prerequisite install for phase execution moves
the exact epoch into the ritual and dispatch.

At the complete build fence the order is exactly:

`build native sources (same epoch) → if pending, one strict workspace replay →
authored/lowered artifact targets → ordinary phase:build rows`.

Native build failure stops before replay/targets/rows. Successfully written
source records may remain and a later Cargo call may report fresh. Replay
failure/residual pending stops before targets/rows and preserves pending output.
Authored-target failure occurs after converged boot publication and suppresses
phase rows under the existing law. A chain without the build fence never builds
or replays.

### 15.6 Gate

Acceptance proves Ready sees newly installed natives before disk lock changes;
Empty host and Fresh dependency natives use one epoch; root/member/package
owners keep distinct controls/routes with shared unit runtimes; install Cargo
count is zero; valid prebuilt runs immediately; only missing buildable source
becomes pending; corrupt terminal prebuilt and every invalid state fail hard;
pending order/fingerprint/header are exact; native build/replay/target/row order
is exact; empty pending has no replay; one replay invokes every expected row and
returns empty; build/replay failure preserves pending bytes; plain/traced/
observed node+unit paths agree; and builtin-only/empty fingerprints remain byte-
compatible.

## 16. R5.4-EPOCH-WORLD ratification

**Accepted 2026-08-31.** Product `c559916b` and map `492ed834` establish one
public `ExtensionWorldEpoch` from the exact ordered `ResolvedDep` sequence.
The epoch retains each supplied parsed package manifest, its materialised root,
content witness, effective resolved edges, extension/mechanism declarations and
routes. Root and member nodes take distinct host seats over that one package
snapshot; package units use the package-owner projection. Exact empty is a
first-class epoch. Duplicate packages/edges, missing hashes/slots, identity or
materialization disagreement, unknown owners and closure defects refuse typed;
none can become `TransformPlan::empty()`.

Ready-style `regenerate_boot_from_traced` trusts only its supplied resolution
and ignores stale or malformed ambient lock bytes. Fresh regeneration, check
and analysis instead read one strict durable lock at their command boundary,
project only its named slots in lock order, use the lock's hashes and effective
dependency edges, ignore orphan slots and treat a missing lock as the explicit
empty world. A present malformed/nonregular lock or a missing/disagreeing named
slot fails through the extension-world error family. Lock-selected
materialization is cross-checked before the epoch can derive a provider root.

Two independent native reviews rejected earlier versions: first, unordered
slot enumeration and raw manifest requirements could replace lock order/hash
and effective graph; second, unchecked materialization could redirect a locked
row to an opposite-layout orphan and the initial composed RED confounded lock
order with host activation order. The accepted tests separate those
authorities: exhaustive registry rows preserve reverse lock order, authored
activations preserve their own reversed order, dependency-first XML follows the
lock's effective edge rather than the raw orphan edge, written and analyzed
bytes agree, and fresh verification is clean without slot records.

Final gates: `vibe-workspace --lib` 494/494, workspace check and all-target
clippy with warnings denied, fmt, conform 48 standing/0 new, and specmap 6,833
units / 3,048 tagged / 2,799 edges with zero suspects, gated orphans or
unresolved host edges and 25 warnings. R5.4-EPOCH-LOWER is next.

## 17. R5.4-EPOCH-LOWER implementation freeze

**Frozen 2026-08-31 after two native `gpt-5.6-sol`/`xhigh` boundary
reviews.** Neutral owner runtimes live in `vibe-workspace`: that crate already
depends on the registry, compiler plan and wire types, while adding
`vibe-lifecycle` would create a dependency cycle. `vibe-extension-registry`
adds an opaque stable row index and effective-order index views; it never
exports storage positions as compiler dense order and never clones candidate
rows.

One `LoweredOwnerRuntimes` owns exactly one runtime per workspace-relative
node and one per installed package coordinate. Each `OwnerRuntime` owns its
single `ExtensionRegistry`, compile-order indices, native-candidate indices,
`TransformPlan`, `MechanismRegistry`, exact owner routes and portable owner
identity. Compile and native borrowed slices are projected only after borrowing
the immutable runtime, and therefore point into the same registry allocation;
no self-reference, `Arc` invoker or cloned candidate epoch exists. Manager
dense order is the position in the complete compile sequence, never a registry
storage index or native-only renumbering.

For each owner, request facts are observed while the one `ExtensionWorld` is
still borrowable; mechanisms are collected by reference first; the same view is
then consumed once by extension collection with explicitly injected node
presets; row indices and the plan are lowered once. Nodes keep distinct
controls/routes. Package units use their retained package controls/routes and
are lowered once globally even when several nodes reference them. Selected
`Project`/`World` are physically common values built once from the selected
node view; owner runtime views borrow them rather than cloning competing
authorities.

The runtime has two stages. EPOCH-LOWER produces neutral
`LoweredOwnerRuntimes` with explicit selected node and preset inputs. INSTALL
later binds the real run id, state/selected root, native platform, offline
posture and timestamp into `OwnerRuntimeEpoch`, transports that exact value
through prerequisite install and makes `RitualPlan` consume it. Production
`phase.rs` is not switched in EPOCH-LOWER: today's workspace regeneration
returns only node names and discards its epoch, so switching phase now would
either recollect or prematurely absorb the workspace→install carriage. Hidden
defaults, a global cache and disk side channels are forbidden.

EPOCH-LOWER lands in three internal serial slices: registry indices; workspace
runtime construction plus boot consumption of retained plans; then focused
proof that common selected facts and preset injection survive without I/O.
It does not invoke a native artifact, classify pending, build source, run Cargo,
replay a lane, change the install result or alter the build fence. INSTALL owns
the production transfer and removal of the final phase recollection.

## 18. R5.4-EPOCH-LOWER ratification

**Accepted 2026-08-31.** Registry product `ab0ab90f`, workspace product
`d2b06561` and trace/map `55595f1f` implement the two-stage neutral runtime
authority frozen above. `RegistryRowIndex` is origin-token-bound rather than a
row address: moving a registry preserves indices, cloning mints a fresh token,
and a stale index remains inert after origin drop. Public foreign projection is
optional and non-panicking; internal same-registry projection is infallible.
Complete compile and all-family native index sequences share the registry's one
effective-order authority, retain selectors, exclude inactive/disabled rows and
never clone candidates or confuse registry position with manager dense order.

`LoweredOwnerRuntimes` owns one `OwnerRuntime` for every workspace node and
installed package coordinate. Each runtime owns one registry, plan, mechanism
registry, exact routes and opaque row subsets. Mechanisms are collected from
the owner view before that same value is consumed by extension collection;
plan lowering occurs at the single authority and package units are sorted and
lowered once globally. Temporary compile/native slices borrow pointer-identical
rows. Selected member `Project`/`World` facts are built once from its distinct
closure and borrowed by node/unit views; run facts bind by move without adding
a second selected-root authority.

Boot regeneration now has a prepared result carrying nodes plus the neutral
runtime set. The complete set is lowered before publication; unit fingerprints,
unit emission, node writes, verification and analysis consume its retained
plans. Compatibility wrappers explicitly select root/no-presets and discard the
runtime. Production install carriage, native invocation, pending policy,
source builds, replay and phase rewiring remain absent and owned by later R5.4
children.

Reviews rejected the first index witness for allocator ABA and silent
`filter_map` loss, then required direct plan spec edges and a real slot-native
all-family RED. Runtime review rejected weak owner-seat/routes/exact-once and
circular parity evidence, then required unwind-safe observation and structural
single-authority counts. The accepted panel proves exact Host/Dependency seats,
three exact route authorities, preset isolation, canonical first refusal,
publication-free lowering failure, selected-member closure, common-fact pointer
sharing and an in-memory Ready overlay that disagrees with stale lock before a
matching-lock Fresh runtime and byte comparison. Final independent verdict is
PASS.

Final gates: registry 61/61, workspace 495/495, workspace/registry check and
all-target clippy with warnings denied, fmt, conform 48 standing/0 new, and
specmap 6,833 units / 3,049 tagged / 2,804 edges with zero suspects, gated
orphans or unresolved host edges and 26 warnings. R5.4-PENDING is next.

## 19. R5.4-PENDING implementation freeze

**Frozen 2026-08-31 after two native `gpt-5.6-sol`/`xhigh` boundary
reviews.** `TransformPlan` remains unchanged. `vibe-spec` owns a shared
compiler-native policy/session with three modes: existing hard `Fail`,
install-time `Collect`, and replay-time `Resolve(expected)`. Policy is applied
inside the manager after selector admission, while the manager still owns the
original carrier. Only the exact typed `BuildableSourceUnavailable` result may
Collect; source/document/lane/emitted then continue with that original value.
Every other invoker, reply, carrier, handler, verification or configuration
fault remains on the existing hard path.

The public pending reference is only `(raw plan digest, manager dense order,
qualified key)`. A private capture also binds point, semantic configuration and
native implementation digest for conflict checking. Repeated document calls
coalesce by order; mixed success/unavailability or any capture disagreement
refuses rather than publishing a partially transformed lane. Resolve consumes
the expected set, records successful actual invocations (including handler
skip), permits ordinary nonexpected successful rows, refuses residual or
unexpected unavailability, and requires every expected order to have a receipt.
No third compile path exists.

Existing native APIs remain Fail wrappers with byte/error/trace/observer
compatibility. Additive managed plain/traced/observed entries return either a
Ready artifact with receipts or an opaque non-publishable `PendingArtifact`.
Pending analyzer deltas do not masquerade as executed transform deltas. The
plan digest and executed-transform semantics never absorb pending state. A pure
active-header projection can exclude validated pending orders, but no schedule-
time integration is allowed: `compile:emitted` may discover pending only after
the current backend already inserted the active header.

Workspace owns the pure `vibe-transform-pending-v1` fingerprint and separate
`vibe:transforms-pending` payload. Typed inputs bind portable node/unit owner,
artifact target/format, raw plan digest and ordered refs, platform, source
witness, handler/config witness, `build:cargo` key and selected provider
semantic digest. Absolute roots, target/image paths, run id, clock, Cargo fresh
bit and traversal order are unrepresentable. Empty pending returns no evidence;
keys use only the shared generated-comment codec; no parser or journal exists.

This child does not choose node/unit lanes, associate real artifact-invoker
build facts, rewrite final artifact bytes, complete trace, publish, replay,
resolve source records or run Cargo. WORKSPACE joins the pure manager result to
runtime/artifact facts, consumes `PendingArtifact` through a finalizer that
rebuilds executed and pending headers/digests, and performs publication.
Internal serial route: PENDING-STATE → PENDING-DRIVER → PENDING-FRAME.

## 20. R5.4-PENDING ratification

**Accepted 2026-09-01.** STATE `36bdfab8`, DRIVER `e851c82b`, FRAME
`828e10df` and map `5257658b` implement the pure pending policy and evidence
boundary without changing `TransformPlan`.

The compiler owns non-authorable, non-Clone ordered pending sets and consuming
Fail/Collect/Resolve policy. Private captures bind raw plan digest, dense order,
key, point, semantic config and native implementation. Repeated identical calls
coalesce; capture conflicts and mixed success/unavailability refuse. Resolve
consumes the expected set, counts actual strict successes and skips, permits
ordinary nonexpected success, refuses residual/unexpected unavailability and
requires every expected receipt. Fail is a stateless no-session compatibility
path.

Managed plain/traced/observed entries share one stack session. Only exact typed
post-selector buildable-source absence continues the original source/document/
lane/emitted carrier. Strict success is receipted only after reply admission and
local verification. Pending lane/emitted calls emit no analyzer transform delta
while trace attempts remain. Ready results expose publishable bytes and
receipts; Pending results hide provisional bytes and expose only read-only refs
or a consuming set-only extraction. The pure active-header projection excludes
validated pending orders but remains deliberately unwired to artifact bytes.

Workspace evidence uses one frozen domain-separated length frame over portable
node/unit owner, closed BootStatic target, SpecFormat, raw plan digest, dense
refs, the exact three native platforms, source/config/provider digests and
`build:cargo`. Facts are one-to-one and full-semantic duplicate/conflict checked.
Node paths reject absolute, backslash, colon and traversal forms. Empty pending
produces no evidence. The header payload records exact digest and codec-owned
`order=key` tokens; no parser, journal, environment or publication API exists.

Reviews rejected STATE for private Debug leakage and stateful Fail, DRIVER for
having no public consuming route to Resolve's owned set, and FRAME for an open
platform vocabulary plus partial duplicate comparison. All were corrected
before final PASS. Final gates: `vibe-spec` 899/899 plus integration/doctests,
`vibe-workspace` 499/499, check, all-target clippy with warnings denied, fmt,
conform 48 standing/0 new, and specmap 6,833 units / 3,052 tagged / 2,807 edges
with zero suspects, gated orphans or unresolved host edges and 26 warnings.

Final artifact header rewrite/digest, trace completion, node/unit threading,
publication and replay remain R5.4-WORKSPACE. R5.4-PENDING is accepted;
WORKSPACE is next.

## 21. R5.4-WORKSPACE implementation freeze

**Frozen 2026-09-01 after two native `gpt-5.6-sol`/`xhigh` design
reviews.** WORKSPACE owns the complete path from opaque compiler-pending output
to truthful per-owner publication plus a non-Clone replay continuation. It is a
six-slice serial workstream: FINALIZE → FACTS → COMPILE → FRESHNESS →
REPLAY-PREPARE → REPLAY-PUBLISH.

FINALIZE lives in `vibe-spec`. It consumes `CompilerPendingArtifact`, validates
the exact retained plan/set, derives the filtered active header and generates
the pending header from a raw workspace fingerprint. It replaces only the
expected fixed opening framing in Markdown/XML, preserves the body and every
provenance member, appends no executed transform, recomputes byte digest/output
fingerprint, and returns publishable artifact plus the still-owned pending set.
Tampered/moved/missing original framing refuses.

FACTS uses a workspace-defined structured binding port implemented by
`ArtifactCompilerNativeInvoker` in `vibe-lifecycle`—the dependency direction
already runs lifecycle → workspace. It joins manager order/key to the same
retained row and returns closed platform, raw source/config/provider semantic
witnesses and exact `build:cargo`, never parsing error text. Missing/duplicate/
conflicting recorder state converts the call to a hard typed failure. No Cargo
call is made.

COMPILE is one plain/traced/observed node+unit core over retained owner
runtimes. No static input returns before compiler/scope/facts/replay allocation.
Ready joins successful structured facts. Pending joins one-to-one facts, builds
workspace evidence, consumes FINALIZE, and only then completes trace with the
final output fingerprint. Pending analyzer deltas remain absent. The same core
produces identical final bytes/evidence in all observation modes.

FRESHNESS preserves historical builtin-only fingerprints. A unit containing
native compile rows cannot use the plan-only early skip: it compiles once so
selector/runtime/artifact truth is observed, then byte-equal transactional
publication preserves mtime. Pending adds its fingerprint to runtime/boot
freshness and is always rediscovered on the next prerequisite install; Resolve
removes that frame.

The owned replay set contains only affected node/unit lane descriptors, exact
pending sets and retained semantic compile/publication inputs—never a trace
borrow, invoker, provisional bytes, header-parsed state or journal.
REPLAY-PREPARE consumes every expected set under Resolve, forces each affected
lane exactly once, requires complete receipts/Ready, and prepares every owner’s
INDEX/STATIC bytes before any write. REPLAY-PUBLISH then walks deterministic
owners through the existing per-owner crash-recoverable transaction. It stops
later owners on failure, preserves prior committed owners, restores caught
precommit current-owner failure, permits committed-new plus recovery intent on
postcommit cleanup failure, and claims no global rollback.

INSTALL later chooses Empty/Fresh/Ready, supplies Collect binding and carries
runtime+replay values. FENCE later orders native build → one workspace replay →
authored targets → phase rows. Neither sequence lands in WORKSPACE.

## 22. R5.4-WORKSPACE-FINALIZE ratification

**Accepted 2026-09-01.** Commit `b82ce151` lands the compiler-owned boundary
that converts an opaque non-publishable pending artifact into truthful
publishable bytes plus the still-owned pending set.

The finalizer consumes the pending artifact, validates the retained plan and
raw pending fingerprint, and supports only static Markdown and static XML. It
derives the exact complete original opening, accepts it only at byte offset
zero, rejects any surviving reserved transform framing in the preserved
suffix, filters pending orders out of the active header, and emits the pending
header through the shared codec-owned spelling. The document body and every
provenance member stay byte-exact; no executed transform is appended. The
canonical byte digest and output fingerprint are recomputed from the finalized
bytes before a publishable artifact is returned with the non-Clone set.

The RED corpus covers first, middle, last and all-pending Markdown/XML plans;
body, provenance and digest preservation; plan, order, key and fingerprint
tampering; moved, corrupted, duplicate and non-static framing; and ownership
and Debug boundaries. An independent review found one duplicate-framing
survivor in the preserved suffix; the bounded survivor fence and RED landed
before final PASS.

Final gates: focused FINALIZE 5/5, `vibe-spec` 904/904 plus integration and
doctests, `vibe-workspace` 499/499 plus integrations and 28 doctests, workspace
check, all-target clippy with warnings denied, fmt, conform 48 standing/0 new,
and `git diff --check`. FACTS is next; compile threading, freshness,
publication and replay remain unimplemented by this slice.

## 23. R5.4-WORKSPACE-FACTS ratification

**Accepted 2026-09-01.** Commit `8de4151e` adds the workspace-owned,
object-safe compiler-native fact binding and implements it directly on
`ArtifactCompilerNativeInvoker`. The compiler and structured fact drain
therefore borrow the same object; lifecycle remains above workspace in the
dependency direction and no diagnostic text is parsed.

Each binding owns one terminal mutex recorder. Exact repeated observations at
one manager order coalesce, semantic drift at the same order terminally
conflicts, and extraction is one-shot. A drain borrows the pending set, emits
facts in its exact order, and refuses missing, extra, conflicting, poisoned or
already-taken state. Only the exact typed missing-source-record result records
a fact, after retained row, key, point, config, implementation and candidate
identity have passed. Prebuilt success or failure, a valid source record, a
later loader failure, selector absence and every other hard artifact failure
record nothing.

Source and handler/config identities now retain raw 32-byte digests from the
same frame instance whose lowercase hex enters the existing artifact record.
The missing-record branch carries those same raw values and the already
selected `build:cargo` mechanism row; it never repeats the host tree hash or
configuration projection. Frozen pre-refactor source and empty-config literal
goldens prove the persistent wire did not change.

The provider digest frames the selected typed row rather than its display
coordinate alone: stable provider variant/identity/version/kind/content hash,
pin, logical key, complete reachable handler shape, protocol, portable config
schema, freshness and admitted enabled state. Roots, ordinals, clocks,
toolchain and resolved artifact paths stay outside it. The durable artifact
record continues to store the exact `org.vibevm/vibe#cargo` pin.

The first independent review was NOT PASS: the raw/hex test was tautological
and an alleged duplicate-recorder state was reachable only through a test
hook. Both claims were corrected before final PASS. Independent old-wire
literals now break on framing drift; the artificial production state/API was
removed; real scoped concurrency proves shared-invoker coalescence and
terminal conflict. The freeze's earlier “duplicate recorder state” phrase is
therefore narrowed honestly: exact repeats coalesce, no independent recorder
seat exists, and duplicate fact vectors remain a hard refusal in the existing
workspace evidence join.

Final gates: focused FACTS 9/9, workspace port 2/2, `vibe-lifecycle` 617 passed
and 3 ignored plus 39 doctests, `vibe-workspace` 501/501 plus integrations and
29 doctests, workspace check, all-target clippy with warnings denied, fmt,
conform 48 standing/0 new and `git diff --check`. COMPILE is next; no Cargo
build call, compile threading, freshness, publication or replay landed here.

## 24. R5.4-WORKSPACE-COMPILE implementation freeze

**Frozen 2026-09-01 after two independent native
`gpt-5.6-sol`/`xhigh` architecture reviews and a bounded observer follow-up.**
COMPILE owns one node/unit static-artifact core and one post-compiler outcome
funnel. Plain, traced and observed are a closed mode enum, never competing
optional parameters, and every mode reaches the same Ready/Pending join.

The core returns before plan cloning, binding creation, scope acquisition,
compiler invocation, fact drain, evidence or continuation allocation when the
owner has no static entry. After fallible input/plan preparation it inspects
the retained runtime's compile/native intersection. A builtin-only plan uses
the exact historical compiler path and never asks for a binding. A native plan
uses a lazy workspace-defined GAT provider; lifecycle can return its concrete
owner-borrowing `ArtifactCompilerNativeInvoker` plus Collect/Resolve policy
without reversing the dependency DAG. Native work without a provider refuses;
it never falls back to unmanaged compilation.

The managed result retains manager receipts for Ready, or finalized pending
evidence plus the exact non-Clone pending set for Pending. Ready terminally
drains the same binding and requires an empty fact recorder. Pending borrows
its set, drains one-to-one facts from that binding, builds evidence from the
exact `OwnerRuntimeId`, `BootStatic` and `SpecFormat`, and consumes the opaque
artifact through FINALIZE. Provisional bytes and their fingerprint never leave
the funnel.

Scope acquisition remains immediately before the one compiler call. Every
compiler, fact, evidence or finalization result passes through one terminal
branch: failure marks the acquired scope failed; success completes it only
with the final publishable output fingerprint. No post-acquisition `?` may
leave an occurrence pending. Transaction publication remains after this
funnel, so a finalization refusal preserves existing node/unit artifacts.

Observed managed compilation buffers the compiler's one emission event in a
vibe-spec one-shot proxy. Stage deltas still use the existing panic-contained
delivery immediately; pending lane/emitted calls remain absent and FINALIZE
adds no delta. After Ready or FINALIZE produces the publishable artifact, the
proxy reframes the retained witness-derived contribution rows to the final
total byte length and frame complement, then delivers exactly once through the
existing panic boundary. A failed join drops provisional emission evidence;
no tape or generated comment is parsed.

New bound-epoch regeneration and analyzer siblings consume exact retained
node/package runtimes and the lazy provider. Unit compilation cannot borrow a
selected-node runtime; root and member ids remain distinct. Existing wrappers
and no-native byte/error/trace behavior remain compatibility paths. COMPILE
may carry the finalized pending continuation, but does not change freshness,
publish replay, construct durable replay descriptors, thread production
INSTALL, sequence FENCE or call Cargo.
