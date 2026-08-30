# R5 native ABI architecture v0.1

Status: frozen for implementation by the central session, 2026-08-30.

This record closes the implementation choices left between PROP-054 §8/§14.4,
the accepted lifecycle spec-debt ruling §11.2, and ledger ruling 17. R5.1 is a
schema-first workstream with three serial implementation children and one gate:

1. R5.1-WIRE — register and generate the three epoch-1 native roots;
2. R5.1-SDK — publish the safe `vibe-ext` author surface and four-symbol macro;
3. R5.1-GATE — prove the generated wire, C boundary, unwind and abort refusal;
4. accept R5.1 only after the integrated mutation-backed panel is green.

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
