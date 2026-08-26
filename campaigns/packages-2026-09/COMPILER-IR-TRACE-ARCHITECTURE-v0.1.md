# Compiler IR wire and compile trace — implementation architecture v0.1

Status: central implementation design, 2026-08-27. Semantic authority remains
PROP-054 `##IR-LEVELS`, `##WHOLE-IR-WIRE`, `##INTER-PASS-VERIFIER` and
`##OBS-TRACE`. The R6 compiler-IR JTD epoch is the data authority once landed.
This document fixes the implementation seams so R6.2 conversion and R3.4 trace
cannot independently invent two representations or two run models.

## 1. Outcome and non-goals

One compiler value has exactly one machine projection:

```text
R3 domain AnyIr
  ↕ strict, lossless conversion
generated compiler_ir/e1 JTD type
  ├─ native pass request/reply payload
  └─ R3.4 after-pass JSON snapshot
```

There is no serde derive on domain IR, handwritten trace DTO, `serde_json::Value`
carrier, handle/callback ABI or trace-specific copy of a DocTree. Trace metadata
(event number, pass, timing, artifact scope and snapshot filename) is a separate
JTD record; it never duplicates an IR field.

R3.4 does not add persistent compiler caching, plugin positioning, native
loading or token analysis. Its observer seam must be reusable by those later
features without making them prerequisites.

## 2. Domain ↔ wire conversion

### 2.1 Home and API

Conversion lives in `vibe-spec`, beside the private domain fields it must see:

```text
crates/vibe-spec/src/compiler/wire.rs
crates/vibe-spec/src/compiler/wire/{address,tree,closure,lane,emitted}.rs
```

`vibe-spec` depends downward on `vibe-wire`; `vibe-wire` never depends on
`vibe-spec`. The public/native-facing surface stays byte-oriented:

```rust
encode_compact(&AnyIr) -> Result<Vec<u8>, IrWireError>
encode_pretty(&AnyIr)  -> Result<Vec<u8>, IrWireError>
decode(&[u8])          -> Result<AnyIr, IrWireError>
```

Both encoders first build the same generated `Ir` value. Pretty versus compact
is a serializer choice, not a second projection. Trace uses pretty; native ABI
uses compact. Tests may expose the typed conversion crate-privately.

### 2.2 Reader order

Wire bytes cross gates in this order, so malformed indices never panic and a
semantic error has one stable owner:

1. generated strict JSON reader;
2. root `ir_schema`, shape, level and cardinality redundancy;
3. scalar gates: nonblank ids, canonical address fields, lowercase hex,
   canonical padded base64, portable open target;
4. checked integer narrowing and arena/span bounds;
5. construction through crate-internal domain constructors, never field
   transmute or `unsafe`;
6. immutable `IrVerifier` over the constructed carrier;
7. for a plugin pass, the manager's pre/post transition witness under that
   pass name.

The conversion implements every `x-conversion-gates` item in the schema. A
test compares that named set to the implemented gate registry; an undocumented
gate and an unimplemented label are both red. Set-backed wire lists are sorted
and unique before construction, so wire→domain→wire cannot normalise silently.

Domain→wire is also fallible: every `usize` narrows explicitly, addresses are
reconstructed field-by-field, digests/base64 are recomputed where the manager
owns them, and a domain value that cannot fit epoch 1 refuses rather than
truncating.

### 2.3 DocTree encapsulation

`DocTree` gains a crate-private checked parts constructor/view. Conversion does
not expose mutable fields publicly and does not reparse source text (a plugin is
allowed to return a changed tree). The constructor enforces exact root shape,
forest/back-references/reachability/order, spans/heading lines, fact/heading
shape and the derived anchor/duplicate views before returning.

### 2.4 Native channel-role fork remains explicit

Before R6.3 native invocation lands, the format registry must represent the two
directions honestly without copying `ir.jtd.json`:

- request: independent plugin readers exist;
- reply: the host reader remains strict and must not drop unknown fields.

The accepted implementation may extend registry/codegen with directional
reader roles or apply a generated-type-preserving strict boundary wrapper. It
may not duplicate the schema/DTO, call the host permissive by accident, or
claim `foreign_parsers = none` after foreign requests ship. This is a required
R6.3 decision/gate, not something R6.2a silently settles.

## 3. One trace run, many artifact compilations

### 3.1 Scope

The recorder is created once in CLI install/lifecycle execution immediately
after the existing `RunMetadata.run_id` is allocated and the selected manifest
is read. The same mutable recorder crosses every path:

- empty/fresh boot regeneration;
- ready install through `vibe-install`;
- every dirty package-unit artifact;
- every workspace-node artifact.

Creating a recorder inside `compile_artifact` is wrong: one `vibe install`
would produce unrelated run directories and reset event numbers. A lifecycle
verb and its prerequisite install share the already allocated run id.

### 3.2 Cardinality

For artifact `a`, with `D(a)` uniquely discovered addressed documents, the
current built-in schedule invokes:

```text
parse                  D(a) times
gather                 once, but it is a scheduler barrier, not a pass
close..assemble         once each
emit:<backend>          once
```

Future source/document transforms run once per addressed document. Lane and
emitted transforms run once per complete artifact. Repeated pass names therefore
need a global event sequence and an invocation ordinal; overwriting
`NN-parse.json` is forbidden.

### 3.3 Observer boundary

`PassSegment` measures the pass body around `run_erased`. After output-shape
checking and semantic/transition verification succeed, it asks the trace
collector to encode the accepted carrier. A pass failure or verifier failure
records status/timing but produces no certified snapshot. Trace serialization
time is recorded separately from pass time.

With trace disabled, the old public wrappers call the old execution path with
`None`; they do not allocate a clock, buffer or filesystem handle. Existing
compile APIs and no-transform boot bytes remain unchanged.

The worklist parser callback must become a fallible `FnMut` (or an equally typed
single-owner session), because document invocations mutate the shared recorder
and wire conversion can refuse. `RefCell`-hiding of an error until later is not
the production design.

## 4. Durable trace surface

### 4.1 Layout

```text
.vibe/trace/<run-id>/
  index.json
  0000-<pass>--<artifact>--<invocation>.json
  0001-<pass>--<artifact>--<invocation>.json
  …
```

The numeric prefix is one monotonically increasing run sequence. Pass,
artifact scope and document/invocation suffixes use one reversible UTF-8
percent codec over a Windows-safe unreserved alphabet; raw `:` from
`emit:static-xml`, trailing dot/space, separators, device names and length
overflow never reach a filename. `index.json` retains the exact unencoded
strings, so shortening an overlong filename with a digest remains reversible
through the index.

Package-unit iteration is sorted before tracing. The current `HashSet` walk is
not trace order authority. Each event also carries the outer node/unit label,
because `ArtifactId` alone repeats `static-md`/`static-xml` across artifacts.

### 4.2 Trace index is JTD-first metadata, not IR

Add one format epoch for the index. At minimum it records:

- schema, run id, project root identity, `running|ok|failed`;
- ordered artifact scopes (`unit|node`, node-relative identity, artifact id,
  target);
- ordered events: sequence, artifact, invocation, pass, input/output shape,
  `ok|pass-failed|verification-failed|snapshot-failed`, pass/verify/encode
  durations, optional snapshot filename and bounded diagnostic;
- aggregate timing rows used by the CLI table.

Durations use an explicitly bounded/scaled numeric representation with a
saturation marker; no narrowing wraps. The index contains no IR payload and no
secret/provider response body.

Each snapshot and each index update uses create-new/atomic replace with fsync
through an existing safe writer extracted from lifecycle/boot transaction
primitives. Trace writes are not part of the boot-artifact transaction: a
failed compile deliberately leaves a partial diagnostic run, while existing
boot artifacts remain untouched. Final index status is written last.

## 5. Manifest and CLI

`vibe-core` gains strict consumer-side `[compile] trace = bool`, default false,
with parse/write symmetry. `vibe install --trace-compile` and every higher
lifecycle verb's shared args enable the same option; CLI true OR manifest true
enables, neither can accidentally disable the other.

Thread a `CompileTraceOptions`/sink through traced siblings of existing APIs;
keep current public wrappers as no-trace compatibility calls. `vibe-workspace`
owns artifact/node labels and persistence. `vibe-spec` owns pass events and IR
encoding. `vibe-cli` owns the human/JSON timing presentation. Neither workspace
nor CLI reaches into domain IR.

The install/lifecycle machine result includes trace run path and timing summary.
JSON output extends a JTD-owned report; it does not append an ad-hoc object.

## 6. Acceptance matrix

1. Every valid R6 corpus carrier converts wire→domain→wire identically.
2. Every real built-in carrier converts domain→wire→domain identically.
3. Semantic reds cover every named conversion gate and cannot panic/hang.
4. Trace after every successful invocation uses only compiler-IR JTD.
5. Failed pass/verifier gets a timing row and no certified snapshot.
6. Two documents through `parse` create two noncolliding ordered files.
7. One install shares sequence across a dirty unit and root node artifact.
8. Pass/artifact filename encoding is reversible and Windows-safe.
9. `[compile] trace` and `--trace-compile` produce the same surface.
10. Trace disabled writes nothing, allocates no recorder, and preserves boot
    artifacts byte-for-byte and mtime/freshness behavior.
11. Adjacent qualify snapshots visibly differ at the renamed fields.
12. Partial failed trace remains readable with `status=failed`.
13. Timings print in human mode and ride the generated JSON report in JSON mode.
14. Package-unit ordering is stable across filesystem enumeration order.
15. Full existing compiler/workspace characterization and boot byte oracle stay
    green.

## 7. Implementation order

1. Land R6.2a schema/corpus.
2. Implement strict bidirectional conversion + gate registry.
3. Add in-memory pass observer/timing with no-trace compatibility wrappers.
4. Add trace-index JTD/generated types and atomic run writer.
5. Thread one recorder through workspace/install/CLI and add flags/config.
6. Add end-to-end trace, failure and byte-identity tests.
7. Only then expose the same conversion to native compiler passes in R6.3.

This order makes R3.4 a real consumer of the public epoch and makes the native
ABI reuse a wire already exercised by human debugging, rather than freezing a
contract no tool has ever read.
