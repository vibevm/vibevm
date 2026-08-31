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
