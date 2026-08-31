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
