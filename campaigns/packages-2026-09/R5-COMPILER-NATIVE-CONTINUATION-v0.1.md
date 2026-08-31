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
