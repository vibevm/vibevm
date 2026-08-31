# R5 native ABI architecture v0.1

Status: frozen for implementation by the central session, 2026-08-30.

This record closes the implementation choices left between PROP-054 §8/§14.4,
the accepted lifecycle spec-debt ruling §11.2, and ledger ruling 17. R5.1 is a
schema-first workstream with four serial implementation children and one gate:

1. R5.1-SHARED-STRICT — make one generated shared fragment honor a unanimous
   strict-reader role without weakening or duplicating it;
2. R5.1-WIRE — register and generate the three epoch-1 native roots;
3. R5.1-SDK — publish the safe `vibe-ext` author surface and four-symbol macro;
4. R5.1-GATE — prove the generated wire, C boundary, unwind and abort refusal;
5. accept R5.1 only after the integrated mutation-backed panel is green.

R5.2 alone owns loading a foreign library. R5.3 alone owns source/prebuilt
resolution and building. R5.4 owns pending bootstrap convergence. R5.5 owns
native compiler parity. R5.1 does not add `libloading`, resolve an artifact,
run Cargo in a package slot, activate an extension, or invent a compiler DTO.

## 1. Three roots, one wire source of truth

The format registry gains exactly these transient epoch-1 roots:

| id | schema | reader role | corpus |
|---|---|---|---|
| `native-context` | `schemas/native/e1/context.jtd.json` | `many` | `formats/corpora/native/e1` |
| `native-reply` | `schemas/native/e1/reply.jtd.json` | `none` | same |
| `native-manifest` | `schemas/native/e1/manifest.jtd.json` | `many` | same |

All are recoverable: invocation or manifest inspection recreates them. The host
authors context, foreign SDKs read it, so unknown context fields stay forward-
compatible. The host is the sole reply reader, so an unknown reply member is a
strict refusal. A manifest is a public foreign-authored declaration and remains
forward-compatible at the generated object-member layer; its vocabularies and
the host's relational checks remain strict.

`native-context` has the lifecycle context's epoch-1 root members exactly:
`envelope`, `point`, `execution`, `project`, `world`, `run`, `artifacts`, `io`,
and optional `slot_target`. `point` is an open string on the wire. It is parsed
into the host's closed `ExtensionPoint` only by R5.2, never by generated wire
code or the SDK.

`native-reply` has `envelope`, closed `status = ok|fail|skip`, `artifacts`, and
optional `message`. It has no `tasks` member. Its artifact row has `id`, `path`
and `kind`; the accumulated context artifact additionally has engine-owned
`phase`. The two rows must not collapse into one misleading type.

`native-manifest` is exactly:

```json
{"extensions":[{"id":"example","point":"phase:build","ir_schema":1}]}
```

Each extension requires `id` and open-string `point`; `ir_schema` is an optional
`uint32`. No ABI number is duplicated in the JSON: `vibe_ext_abi()` is its one
authority. An absent `ir_schema` is valid for phase-native work; compiler-native
selection will require and compare it when that path lands.

## 2. Shared lifecycle subshapes

Native schemas do not copy the lifecycle DTO definitions. Move the reusable
epoch-1 fragments into `formats/vocabularies.json` and let the existing codegen
shared-module phase emit them once under
`vibe_wire::generated::shared`:

- lifecycle execution, project, world, world-package, run, slot-target and IO;
- accumulated lifecycle artifact (including `phase`);
- reply artifact (without `phase`);
- reply status.

The existing lifecycle context/reply schemas and the two new native roots all
reference those fragments and declare their vocabulary closure. Generated
modules re-export the shared Rust types. Root `Context` and `Reply` values stay
format-specific because the roots intentionally differ; their nested records
are type-identical, not copied lookalikes.

### 2.1 Shared strictness compatibility rule

The first WIRE attempt stopped before editing because the existing generator
rejects a shared fragment as soon as any consumer has
`foreign_parsers = "none"`, even when every consumer is `none`. That guard was
correct for mixed reader roles but over-broad for unanimous roles. R5.1 adds
one bounded codegen prerequisite:

- if every registered consumer of a fragment is `none`, emit the shared
  fragment once with `#[serde(deny_unknown_fields)]` on each of its generated
  structs;
- if no consumer is `none`, emit it once with the existing permissive reader;
- if strict and permissive consumers mix, keep refusing and name both sets;
- enum/scalar fragments share the same role classification but receive only
  attributes their generated form supports;
- the role is computed from `formats/REGISTRY.toml` over the resolved schema
  closure; no schema-local override or new hand-maintained policy list exists.

This preserves one Rust type and the registry's computed reader policy. It does
not create strict/permissive twin modules. The R7.5 state/evidence duplicated
shapes are historical and remain untouched; this atom adds the missing
mechanism for future unanimous-strict sharing and then R5.1 is its first user.

The generator prerequisite is accepted only with focused unit tests proving an
all-`none` fragment is emitted strict, an all-permissive fragment stays byte-
compatible, and a mixed fragment still refuses. Independent mutations must make
the all-strict attribute disappear and the mixed-role refusal disappear, then
restore byte-exact. No schema, vocabulary or generated product wire changes in
that prerequisite commit.

**Ratified at R5.1-SHARED-STRICT acceptance (central, 2026-08-30).** Commit
`52edc577` computes one `FragmentReaderPolicy` from the existing registry loader
and resolved closure, threads it through the normal `rewrite_generated` pass
slot and stamps only unanimous-strict shared structs. The first worker PASS was
rejected because a central bypass of that match arm survived all helper tests;
the correction added a full-pipeline fixture that makes the exact bypass RED.
Two worker policy mutations and two independent central mutations failed and
restored byte-exact. All 17 shared-module tests and 234 xtask tests pass;
check-codegen is clean over 145 byte-identical generated files, strict clippy,
fmt and xtask conform (0 findings / 0 new) pass. No schema, vocabulary, registry
record, generated product or product crate changed.

**Ratified at R5.1-WIRE acceptance (central, 2026-08-30).** Commit `fd81a003`
registers `native-context`, `native-reply` and `native-manifest`, moves eleven
reusable lifecycle/reply types into the one generated shared module and emits
the three format-specific roots plus authored valid/invalid corpus. Context and
manifest remain permissive at root and nested foreign-reader boundaries; native
reply and its shared artifact row are strict. Point stays an open string,
manifest carries no ABI field, native reply carries no tasks, and accumulated
artifacts alone retain engine-owned phase. Five worker and three independent
central source mutations failed and restored byte-exact. Gates passed: focused
native wire 7, the complete `vibe-wire` test/doc suite (132 library tests),
check/check-codegen, strict clippy/fmt, conform 0-new and pre-publication
wire-diff over all 5 schema + 6 corpus + 2 format paths. Map `4c9378c9` is 6,833
units / 3,009 tagged items / 2,758 edges, with 0 suspects, gated orphans or
unresolved host edges and 25 standing warnings. R5.1-SDK is now the only next
consumer; no loader/build/activation behavior landed here.

**Ratified at R5.1-SDK acceptance (central, 2026-08-31).** Commit `bfaea140`
adds `vibe-ext` as a gated public workspace crate and the deliberate native-ABI
audit home. The author surface re-exports generated wire types and one macro;
authored code contains no unsafe, while the expansion confines raw pointers,
unsafe export attributes and exact boxed-slice reconstruction to one private
FFI module. ABI/manifest/invoke/free use the four exact unmangled C names,
proved through explicit `link_name` declarations rather than Rust re-exports.
Manifest storage is stable `OnceLock<CString>`; invoke initializes output slots,
borrows request bytes, validates envelope 1 and contains decode→handler→encode
inside `catch_unwind`; successful ownership is one `Box<[u8]>` reclaimed by the
exact pointer/length pair. Five worker plus three independent central mutations
failed and restored byte-exact, including a link-time LNK2019 when one
`no_mangle` was removed. Seven integration tests, check, strict clippy/fmt and
conform 0-new pass. The standalone real `panic=abort` profile fails solely with
the SDK's unwind-remediation compile error; removing that error makes the gate
unexpectedly green. No loader, artifact resolution, package-slot build or
activation path landed.

**Ratified at R5.1-GATE and R5.1 acceptance (central, 2026-08-31).** The gate
adds no duplicate harness or product path. On exact main it composes the 17-test
unanimous shared-reader mechanism, 7 native wire tests and 7 SDK/raw-link tests;
post-commit check-codegen is clean, the real abort-profile fixture refuses only
with the SDK's unwind remediation, and specmap remains clean at 6,833 units /
3,009 tagged items / 2,758 edges with 0 suspects, gated orphans or unresolved
host edges and 25 warnings. The three implementation children already carry 20
accepted RED/restoration proofs (4 shared-strict, 8 wire, 8 SDK), so the gate
does not invent a second mutation ceremony. R5.1 is accepted: the wire and safe
plugin-side ABI exist; dynamic loading, library caching and host-side free-once
remain exclusively R5.2.

Wire acceptance requires authored valid/invalid corpus documents for all three
roots; registry completeness; codegen/check-codegen; exact generated-module
sharing assertions; native reply unknown-member refusal; native context and
manifest forward-member acceptance; open point round-trip; and an assertion
that context and reply artifacts remain different types with the stated field
sets.

## 3. The public Rust SDK

`crates/vibe-ext` becomes a root-workspace member and workspace dependency. It
re-exports the generated native `Context`, `Reply`, `Manifest` and their public
nested types from `vibe-wire`; it hand-authors no serde wire DTO.

The author surface is one safe function and one invocation:

```rust
fn handle(context: vibe_ext::Context) -> vibe_ext::Reply { /* safe Rust */ }

vibe_ext::vibe_extension!(
    manifest = vibe_ext::Manifest { /* generated fields */ },
    handler = handle,
);
```

The macro expands exactly one instance of each symbol. A second invocation in
one library naturally fails on duplicate symbol/item names. Its public ABI is:

```text
vibe_ext_abi() -> u32
vibe_ext_manifest() -> *const c_char
vibe_ext_invoke(req_ptr: *const u8, req_len: usize,
                resp_ptr: *mut *mut u8, resp_len: *mut usize) -> i32
vibe_ext_free(ptr: *mut u8, len: usize)
```

ABI 1 is the only returned version. The manifest is serialized from the
generated type once into a NUL-terminated `CString`; its pointer is static for
the loaded-library lifetime and is never passed to `vibe_ext_free`.

Request bytes are borrowed from the host for the duration of invoke. On entry,
valid response out-pointers are set to null/zero before request parsing or the
handler. A successful reply is JSON-serialized into one plugin-owned byte
allocation, transferred through pointer/length, and later reclaimed exactly
once by `vibe_ext_free`. Every failure returns a nonzero value and leaves
null/zero. Only zero versus nonzero is contract; no numeric error taxonomy is
public API.

Envelope `1` is checked after generated JSON decoding and before the author
handler is called. Malformed JSON, a different envelope, a panic, or reply
serialization failure cannot publish response ownership.

## 4. Unwind and unsafe boundaries

`catch_unwind(AssertUnwindSafe(...))` lives inside the cdylib side emitted by
the macro and encloses decode, envelope validation, the safe handler and reply
serialization. A handler panic produces nonzero/null/zero and the process stays
alive.

`panic=abort` cannot satisfy that promise. Every macro expansion therefore
contains a `cfg(panic = "abort")` compile error with direct remediation to build
the extension with unwind panics. A standalone abort-profile fixture invokes
the macro and must fail to compile for that reason. A normal unwind fixture must
compile and run. The negative Cargo command is an explicit campaign gate; it is
not converted into a fake custom cfg.

Authors write no `unsafe`. The public macro wrappers are safe `extern "C"`
functions and call one crate-private implementation module that quarantines raw
pointer reads/writes and allocation reconstruction. The crate denies unsafe by
default and permits it only in that named module. R5.2 will use a different host
crate for `libloading`; the SDK must not gain a host-side loader dependency.

## 5. Behavioural and mutation gates

The SDK gate calls the macro-emitted symbols directly in an integration test
binary and proves:

- ABI is exactly 1 and the manifest pointer is stable, NUL-terminated and parses
  through the generated manifest root;
- a valid context reaches the safe handler and a valid generated reply round-
  trips through plugin allocation plus exactly one free;
- malformed JSON and envelope != 1 never call the handler and return
  nonzero/null/zero;
- a panicking handler returns nonzero/null/zero and a later valid invocation
  still succeeds;
- manifest memory is never response-owned and `free(null, 0)` is a no-op;
- the abort-profile fixture compile-refuses while the unwind profile compiles.

At least these independent mutations must turn a selected test red and be
restored byte-exact: bypass envelope validation; move the handler outside the
unwind boundary; publish a non-null response on failure; remove the native reply
strict-reader role; copy rather than share one lifecycle nested type; and allow
the abort-profile fixture to compile.

The final R5.1 panel is: native wire corpus and sharing cells, `vibe-wire`,
`vibe-ext`, check-codegen, format-id completeness, strict fmt/check/clippy,
conform with zero new findings, specmap with zero gated orphan/unresolved host
edges, the positive unwind fixture, and the expected-failing abort fixture.

## 6. Commit and acceptance boundary

WIRE, SDK, specmap and ratification remain separate atomic commits. Generated
files always land in the same commit as their schema source. R5.1 is accepted
only after central reads the full diff, reproduces independent REDs, runs the
integrated tree and records the actual gate counts. No R5.2 loader symbol,
manifest-to-host selection, native manifest activation or build integration may
ride these commits.

## 7. R5.2 host loader — frozen boundary

R5.2 adds one root-workspace crate, `vibe-native-loader`. It is the host-side
unsafe quarantine and the only production crate in this atom that depends on
`libloading`. It is gated and a narrow native-loader audit home. R5.2 does not
resolve `crate_dir`/`prebuilt`, invoke Cargo, choose a platform artifact, attach
to lifecycle dispatch, activate a contribution or interpret compiler IR; R5.3
owns artifact resolution/build plus the first lifecycle integration.

The safe public entry is one process-lifetime `NativeLoader` and one explicit
request value:

```rust
loader.invoke(NativeInvocation {
    library: absolute_path,
    extension_id: "minify",
    point: ExtensionPoint::Compile(CompilePoint::Emitted),
    ir_schema: None,
    context: &generated_native_context,
}) -> Result<generated_native_reply, NativeLoadError>
```

The caller supplies the selected absolute artifact path. The loader
canonicalizes it before opening and keys a strong-handle cache by that canonical
path; two aliases load once, concurrent first use cannot create two handles, and
libraries remain loaded until the `NativeLoader` drops. Missing/non-file paths,
canonicalization/load failures and a poisoned cache are typed refusals naming a
bounded path display. No ambient path search, home, network or token exists.

### 7.1 Unsafe and symbol boundary

Crate root denies unsafe; one named private `ffi` module alone permits it. That
module owns `Library::new`, exact lookup/copy of the four function pointers,
bounded NUL scan of the static manifest, invoke pointer conversion and the
response ownership guard. No `libloading::Library`, `Symbol`, raw response or
unsafe function is public. Function-pointer values are copied only while their
owning `Library` remains in the same cached object.

Opening resolves all four exact symbols before the object becomes cache-visible.
Missing one refuses by its literal symbol name and the half-open object is
dropped. ABI is checked before manifest and invoke; anything other than 1 refuses
with `rebuild: vibe build` remediation.

### 7.2 Manifest admission

The manifest pointer must be non-null, NUL-terminated within a fixed documented
byte cap, UTF-8 and valid generated `Manifest` JSON. Its bytes are copied
immediately; the pointer is never freed. Manifest extension ids must be unique.
The selected id must occur exactly once; its open-string point is parsed through
`vibe_core::lifecycle::ExtensionPoint` and must equal the typed expected point.
Its optional `ir_schema` must equal the caller's expectation exactly. All these
checks happen before invoke; diagnostics quote only bounded scalar/path previews,
never request/reply bodies.

### 7.3 Invoke and exactly-once free

The generated `Context` is serialized once and borrowed for the call. Host
output slots start null/zero. A nonzero plugin status is a typed invocation
failure. Success requires non-null/nonzero response ownership and a length below
the fixed reply cap and `isize::MAX`.

Every non-null response pointer is immediately wrapped in an internal RAII
guard holding the exact pointer, length, free function and cached library handle.
The guard calls `vibe_ext_free(ptr,len)` once on every exit — valid reply,
non-UTF-8, malformed/unknown-field JSON, wrong reply envelope, plugin failure
that illegally published ownership, or later validation failure. A null pointer
with nonzero length refuses without fabricating ownership; success with null or
zero refuses. Reply JSON parses through the strict generated `Reply`; reply
envelope must be 1. `ReplyStatus` remains a returned wire value for lifecycle to
interpret, not a loader error.

### 7.4 Tests and acceptance

Tests use two layers. Pure/fake export tests exhaust ABI/manifest/refusal and
response-guard branches with counted load/invoke/free calls. A dev-only native
fixture built as an ordinary Cargo dev-dependency (`rlib` + `cdylib`) proves the
real `libloading` path and exact SDK-produced ABI; tests locate exactly one
platform-named fixture artifact under their own target directory and refuse
ambiguity. Tests never spawn nested Cargo.

Acceptance pins: canonical path aliases and concurrent first use load once;
missing symbol and wrong ABI refuse before manifest/invoke; invalid/duplicate/
mismatched manifest refuses before invoke; valid invocation returns generated
Reply; every published response is freed once even when reply parsing fails;
library handles outlive guards; diagnostics are bounded. Mutations must remove
cache identity, skip ABI/manifest gates, bypass free on malformed reply and swap
a symbol name, each producing a selected RED before byte-exact restoration.

R5.2 lands as LOADER then a focused real-fixture/cache/free gate. Only after both
are accepted may R5.3 resolve source/prebuilt artifacts and connect lifecycle.

**Ratified at R5.2-LOADER acceptance (central, 2026-08-31).** Commit
`9f7b8854` / map `36efa500` lands the separate gated `vibe-native-loader`
crate and no R5.3 concern. The caller-supplied absolute path is canonicalized
into one strong process cache; four exact symbols and ABI 1 are admitted before
the copied bounded manifest, and selected id/typed point/exact optional schema
are admitted before invoke. Every published response enters an owner-carrying
RAII guard before status or strict reply validation. The fake layer passes 11
tests, the real SDK-produced Windows DLL passes 1 integration test, the public
surface passes 3 doctests, and the fixture marker passes its compiled doctest.
Strict clippy/check/fmt and scoped conform pass with zero findings. Five worker
plus three independent central RED/restoration probes prove cache identity,
ABI/manifest ordering, exact symbol lookup and free-on-malformed-reply. Specmap
is clean at 6,833 units / 3,014 tagged items / 2,763 edges with 0 suspects,
gated orphans or unresolved host edges and 25 standing warnings. The focused
R5.2-GATE remains next; Linux/macOS real-library execution remains platform-CI
evidence, and artifact resolution/build/lifecycle wiring remain R5.3.

**Ratified at R5.2-GATE and R5.2 acceptance (central, 2026-08-31).** The gate
adds no duplicate harness or product path. On exact main it composes the loader's
11 unit + 1 real DLL + 3 doctests, the SDK's 7 link/ABI tests and the native
wire's 7 schema/corpus/sharing tests. Post-commit check-codegen is clean; full
conform carries its 28 standing findings with 0 new; specmap remains clean at
6,833 units / 3,014 tagged items / 2,763 edges with 0 suspects, gated orphans
or unresolved host edges and 25 warnings. The accepted LOADER already carries
five worker plus three independent central RED/restoration proofs, so the gate
does not invent a second mutation ceremony. R5.2-LOADER, R5.2-GATE and parent
R5.2 are accepted. R5.3 now exclusively owns source/prebuilt artifact
resolution, in-slot build and the first lifecycle connection.

## 8. R5.3 source/prebuilt build and lifecycle connection — frozen boundary

R5.3 joins three already-landed owners without moving their boundaries. The
extension registry remains the one declaration/control/order authority;
`vibe-workspace` remains the provider-root, slot and generated-ignore authority;
the lifecycle mechanism layer remains the build-phase/Cargo/artifact-record
authority; and `vibe-native-loader` continues to accept only an already-resolved
absolute library path. R5.3 adds the adapter between them. It does not reparse a
manifest, enumerate `vibedeps`, guess an artifact filename, or add resolution to
the loader.

### 8.1 Candidate and platform law

The build set is every **enabled** native registry row in the registry's one
effective order. Disabled and inactive rows do not build. Selector-bearing
compile rows remain in the set because there is no document subject at build
time; dropping them then would make a later matching document unable to execute.
Rows are grouped by exact provider identity plus `crate_dir`, so one cdylib that
declares several extension ids builds once. Prebuilt rows do not enter a Cargo
group.

The closed platform value has exactly three keys from PROP-000:
`windows-x86_64`, `linux-x86_64`, and `macos-aarch64`. A testable constructor
maps exact OS/architecture pairs; the production constructor reads
`std::env::consts::{OS, ARCH}` once. Unsupported pairs refuse naming the three
supported keys. There is no alias, case folding, host-target inference or silent
nearest match. Each key owns one suffix: `.dll`, `.so`, `.dylib` respectively.

For one row the resolution order is exact:

1. a declared entry for the current platform wins, even when `crate_dir` is
   also present;
2. that declared path must exist as a regular suffix-matching file whose
   canonical path stays inside the provider root — a missing/corrupt
   current-platform prebuilt refuses and never falls back to source;
3. when the current key is absent, `crate_dir` is the source fallback;
4. without either, refuse naming the missing key and the bounded declared set.

All authored paths have already passed the declarant-path grammar. The runtime
still joins them component-by-component, canonicalizes the provider root and
candidate, requires containment and rejects a directory/link escape. Prebuilt
bytes are ordinary shippable package content: their provider `content_hash` and
an exact artifact SHA-256 are evidence; no build receipt chooses them.

### 8.2 One build plane, provider-root execution

Source native build uses the existing logical `build:cargo` mechanism key and
the same mechanism registry/host routes as authored artifact targets. A route
or explicit provider that displaces the builtin is therefore honoured; until
the non-builtin transport lands it refuses through the existing typed transport
boundary. Native code does not create `build:native` or a private provider
registry. `native` is the artifact/handler genre; Cargo is the build mechanism.

Inside the build fence, native source groups execute before authored/lowered
`[[artifacts.build]]` targets and before every `phase:build` contribution. This
lets the remainder of the build phase observe a complete extension machine and
leaves R5.4 one unambiguous point from which to recompile pending lanes. A
partial install epoch still arms no build fence.

The Cargo adapter is split below its manifest-facing R8 executable projection.
Both consumers reuse argv-only invocation, metadata preflight, lenient Cargo
JSON parsing, exact artifact selection, toolchain evidence, containment and
streamed digest. They do not share a false root model: R8 builds a project-owned
declared executable under the project build root; R5.3 builds a provider-owned
cdylib under that provider root.

For a native source group the command is a release library build from
`<provider-root>/<crate_dir>/Cargo.toml`, with `--lib`,
`--message-format=json-render-diagnostics` and the engine-fixed
`--target-dir <provider-root>/target`. The run's offline posture is preserved.
Package-supplied Cargo uses the existing positive toolchain environment genre
from the PROP-025 binary builder (including Windows linker discovery), not the
selected-project R8 provider's broader environment. This is a real trust/root
difference, not duplicate policy. Dependency builds first call the existing
race-safe `ensure_build_output_ignores`; authored host roots keep their own
repository ignore policy.

Metadata must identify exactly one selected package library target whose crate
types include `cdylib`. The build stream must then contain exactly one matching
`compiler-artifact`, and its `filenames` must contain exactly one regular file
with the current platform suffix under `<provider-root>/target`. `.pdb`, `.rlib`,
Windows import libraries, zero matches and multiple matches are never resolved
by order. The verified outcome retains Cargo's `fresh` bit. Every scheduled
build still calls Cargo: R1's preserved mtimes and the stable target directory
make the second invocation Cargo's genuine no-op; VibeVM invents no competing
source-freshness algorithm to skip it.

### 8.3 Durable hand-off without a second state plane

A verified source cdylib is recorded atomically through the landed artifact
record writer in the selected project's state. No `native-library` wire kind is
added: a cdylib is the existing `file` artifact kind. The record's relative
root is `slot` for an installed dependency and `project` for an authored host;
the recorded path is relative to that exact provider root, never an unchecked
absolute authority. The stable record id is the full 64-hex SHA-256 of provider
identity, exact `crate_dir` and platform under a native-build domain. The record
binds the selected build provider, provider version/hash when present, exact
handler/config fingerprint, toolchain, platform, artifact digest and injected
run time.

Dependency source identity is the lock-selected `content_hash`. An authored
host source has no lock hash, so R5.3 computes the existing shippable-tree
recipe over its provider root (which excludes `target/`/`node_modules`) and
binds that witness into the same record. A later process may reuse a source
artifact only when record identity, platform, source/config witness, relative
containment and current artifact bytes all revalidate. Missing/stale/corrupt
state is an unavailable source artifact with `vibe build` remediation, never a
guessed target path. The next build overwrites the stable record only after a
successful verified Cargo result.

### 8.4 First lifecycle connection

`HandlerRuntime` gains one injected native backend beside its binary, package
binding and agent backends. Production composition owns one process-lifetime
`NativeLoader`; tests inject a fake backend. Every lifecycle-executed native row
— both `phase:` and `slot:` — resolves prebuilt or a validated source record and
then invokes that loader with the declaration id, typed point,
`ir_schema = None` and the generated native context. It never builds lazily. A source native
needed before the build fence therefore requires a valid earlier build record;
prebuilt works directly.

Lifecycle and native context roots are converted field-for-field while their
shared nested generated types retain identity. Native reply status, artifacts
and message become the ordinary lifecycle reply with an empty tasks list.
Loader/resolver/build failures enter one typed native handler refusal and obey
the existing phase/slot failure law; `skip` remains bookkeeping, and the SDK /
loader panic boundary keeps the process alive. Compiler-native payloads,
`ir_schema = 1`, pending tolerance and lane invocation are deliberately absent.

### 8.5 Serial atoms and acceptance

R5.3 lands as three serial children plus a gate:

1. **ARTIFACT** — platform/prebuilt/source resolution; factored Cargo cdylib
   selection; provider-root build; shared artifact-record hand-off;
2. **WIRING** — retain enabled native candidates in the ritual, run them first
   at the build fence, and connect phase/slot dispatch through the process-owned
   loader;
3. **GATE** — compose source and prebuilt on Windows, non-host platform units,
   Cargo no-op/mtime evidence, artifact-record revalidation and lifecycle
   failure/skip/panic behavior; then accept parent R5.3.

Required mutation REDs include: map Windows to another key; reverse prebuilt
precedence; remove canonical containment; choose the first Cargo filename;
move target output to the selected project; omit the source/config witness;
trust an absolute record path; build a disabled row; place authored targets
before native builds; bypass resolver admission; recreate the loader per call;
and dispatch a compile-native row in R5.3. Every mutation is restored byte-exact.

R5.4 alone owns `transforms-pending`, install-time absent-transform tolerance and
one-invocation build→lane-recompile convergence. R5.5 alone owns compiler-native
invocation and builtin/native minify byte parity. No R5.3 acceptance claim may
use either as hidden evidence.

**Ratified at R5.3-ARTIFACT acceptance (central, 2026-08-31).** Product commit
`1baac652`, traceability `0ef041c7` and map `fd31533e` land the artifact half
without lifecycle wiring. The exact three-platform resolver gives a declared
current prebuilt terminal precedence; source fallback resolves the world's
`build:cargo` route, runs release Cargo under the provider root and chooses one
cdylib only from metadata/compiler-artifact filenames. The shared package-Cargo
environment preserves the existing binary builder, and the shared artifact
record carries `file` plus project/slot-relative path, labelled source/config/
toolchain/platform witnesses and current bytes. The initial worker PASS was
rejected for an unauthorised target pin, legacy/unlabelled host witness, a false
SDK-fixture claim and 28 new conform findings; a native `gpt-5.6-sol`/`xhigh`
correction removed all four defects. Gates pass native 9, Cargo wire 13, shared
record 8, workspace environment 2, check/strict clippy/fmt, lifecycle conform
2 standing/0 new and workspace conform 0/0. Six worker plus two independent
central mutations were RED and restored byte-exact. Specmap is clean at 6,833
units / 3,023 tagged items / 2,775 edges with 0 suspects, gated orphans or
unresolved host edges and 25 warnings. R5.3-WIRING remains next; no loader,
handler, fence or compiler connection landed in ARTIFACT.

**Ratified at R5.3-WIRING acceptance (central, 2026-08-31).** Product commit
`332f8e28` and map `48fcb39e` retain every enabled native row in registry order
without premature selector filtering, carry that exact epoch with its mechanism
registry and host routes, and run source-native builds before authored artifact
targets at the complete build fence. Phase and slot compositions inject the
same ARTIFACT-backed native backend and one process-lifetime loader; compile
rows remain build candidates and never enter lifecycle dispatch. Context roots
cross field-exactly, reply status/artifacts/message retain their wire values,
and lifecycle tasks/streams remain empty.

Central review rejected the first worker PASS on two integration defects: a
source-record lookup spawned `cargo -Vv`/`rustc -V`, and loading mutable Cargo or
prebuilt paths into the process cache could retain stale code or lock a Windows
DLL across rebuild. The native `gpt-5.6-sol`/`xhigh` correction leaves the
build-time toolchain digest in the artifact record but makes resolution process-
free, then publishes either admitted origin as a recoverable, non-authoritative,
no-overwrite image at `.vibe/native-load/e1/<sha256>/<basename>`. Link-free
contained parents, copied-byte verification, atomic hard-link publication and
final non-symlink digest validation make the canonical loader key immutable.
A real same-loader A→B source rebuild returns B while A remains loaded; the slot
path proves the same image rule for prebuilt and refuses a missing source record
without creating a slot target tree. Central independently made build order,
loader lifetime, digest identity and production image bypass RED, then restored
all four. Gates pass native 14, handlers 10, mechanism wiring 11, install native
1, strict check/clippy/fmt, conform at lifecycle 2 standing/0 new and
orchestrator/install 0/0. Specmap is 6,833 units / 3,028 tagged items / 2,781
edges with 0 suspects, gated orphans or unresolved host edges and 25 warnings.
R5.3-GATE remains the only child before parent acceptance.
