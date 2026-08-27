# Compiler IR wire and compile trace — implementation architecture v0.1

Status: central implementation design, 2026-08-27. Semantic authority remains
PROP-054 `##IR-LEVELS`, `##WHOLE-IR-WIRE`, `##INTER-PASS-VERIFIER` and
`##OBS-TRACE`. The R6 compiler-IR JTD epoch is the data authority once landed.
This document fixes the implementation seams so R6.2 conversion and R3.4 trace
cannot independently invent two representations or two run models.

Execution status and dependency order live in
[`LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md`](LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md).

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

The open `artifact_target` vocabulary is lossless at this boundary. R6.2b
replaces the test-only borrowed custom spelling with an owned, validated
backend identity, and every valid custom-target corpus carrier must survive
wire→domain→wire exactly. This identity constructor is crate-private and does
not imply that an implementation is installed. R6.3 still owns registry
membership, backend selection and native invocation. Decode never leaks an
untrusted string, interns it globally or consults the runtime registry; an
`UnsupportedCustomTarget` refusal would be an undocumented sixteenth gate and
is forbidden.

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

The observer is diagnostic, not a new compile-failure channel. A snapshot
encode/write refusal becomes a typed `snapshot-failed` event and never changes
the pass result or compiler error identity. The worklist parser callback
therefore stays infallible; one shared recorder session (the schedule already
has an `Arc<Mutex<…>>` precedent in `CloseState`) serialises document events
without hiding a deferred compile error in a `RefCell`. Trace-disabled code
still passes no recorder at all.

## 4. Durable trace surface

### 4.1 Layout

```text
.vibe/trace/<run-id>/
  index.json
  0000-<enc-pass>-<kind>_<enc-scope>_<enc-artifact>-000.json
  0001-<enc-pass>-<kind>_<enc-scope>_<enc-artifact>-001.json
  0002-~<digest16>-000.json
  …
```

The full spelling is
`<seq:04>-<enc(pass)>-<kind>_<enc(scope-label)>_<enc(artifact)>-<ord:03>.json`.
Widths are minimum widths, not truncation. `enc` leaves only `[A-Za-z0-9.]`
raw and encodes every other UTF-8 byte as uppercase `%XX`, including `%`,
`-`, `_`, `~`, separators and `:`; the raw `-`/`_` separators are therefore
unambiguous. Each `(scope, pass)` spends dense ordinals `0..D-1` in encounter
order while the numeric prefix is one monotonically increasing run sequence.

A filename is at most 96 ASCII bytes. Path pressure or an overlong middle uses
`<seq:04>-~<digest16>-<ord:03>.json`, where `digest16` is the first 16 lowercase
hex characters of SHA-256 over the canonical encoded middle
`<enc(pass)>-<kind>_<enc(scope-label)>_<enc(artifact)>`; the index validator
recomputes it. The writer additionally clamps against its absolute run path
(`min(96, 250 - len(run_dir_abs))`, floor 32), while the index admits either
verified spelling because that absolute path is not wire data. `index.json`
retains the exact unencoded strings, so the short name is reversible through
the index. This exact contract is implemented by `compiler-trace-index/e1` at
`6f4a717d`; the writer must reuse that cell, not reimplement the codec.

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

Event sequences are dense globally; invocation ordinals are dense per
`(scope, pass)`. Root `status` is the compile/lifecycle outcome, not the health
of its observer: `snapshot-failed` and `snapshot-skipped-budget` are legal under
root `ok`, while pass/verifier failures are not. Conversely root `failed` may
carry only successful pass events when failure happened later (for example the
boot-artifact transaction rolled back a `StaticWrite`).

Durations use an explicitly bounded/scaled numeric representation with a
saturation marker; no narrowing wraps. The index contains no IR payload and no
secret/provider response body.

Each snapshot and each index update uses create-new/atomic replace with fsync
through an existing safe writer extracted from lifecycle/boot transaction
primitives. Trace writes are not part of the boot-artifact transaction: a
failed compile deliberately leaves a partial diagnostic run, while existing
boot artifacts remain untouched. Final index status is written last.

### 4.3 Retention and the per-run budget

Trace diagnostics are bounded by two independent measures. When a recorder is
opened, it considers only no-follow directories whose names are exact
32-lowercase-hex run ids and whose terminal index proves a complete trace. It
keeps the newest nine completed runs, ordered by `index.json.started` (directory
mtime is the fallback ordering evidence when the timestamp cannot be read),
before creating the new run. Thus a successful open leaves at most nine older
complete traces plus the live tenth. A malformed, link-like or non-owned entry
is residue to report/refuse, never something retention silently deletes.

One run may publish at most 128 MiB of snapshot payload before the recorder
stands down. Once the spent snapshot-byte counter reaches that ceiling, later
pass invocations are recorded as `snapshot-skipped-budget`: pass/verify timing
remains, while encode timing and a snapshot filename are absent exactly as the
index epoch requires. `index.json` remains writable so a budget-limited run is
still readable and reconcilable. The counter uses checked/saturating arithmetic;
neither a large carrier nor a wrapping sum may reopen the budget. Retention
count and byte budget are separate REDs: twelve completed seed runs plus one
new run leave ten, and a budget-exhausted run keeps dense events/timings while
creating no further snapshot files.

### 4.4 Writer ownership, crash truth and concurrent budget decisions

The durable writer landed at `4d95a129`. Every cooperating trace writer takes
one nonblocking `.vibe/compile-trace.lock` before it inspects, reopens, retains
or creates `.vibe/trace`; the guard lives in the shared run state until the
last `TraceRun`/`TraceScope` clone drops. Identity proofs are revalidated
immediately before retention removal. Portable safe Rust has no
compare-identity-and-unlink syscall, so an uncooperative process racing the
final OS call is outside this guarantee; the implementation is neither called
handle-bound nor race-free against that actor.

Index publication keeps `PublishStage`. A before-publication failure retains
the prior whole index. After `PossiblyPublished`, the destination is re-read
through the pinned capability and compared byte-for-byte with the attempted
serialized index: exact bytes are the durable truth and retain the anomaly as
a warning; any other bytes leave the in-memory root running/not-finalised. A
fresh directory whose first index cannot land is named as residue rather than
silently removed.

Snapshot `PossiblyPublished` conservatively charges the attempted payload and
reserves its final name, including the crash-shaped two-hardlink state where
the stage survived. Two `Send + Sync` scopes may both receive `Encode` before
either crossing event records. The mutex gives the first recorder the one
allowed soft-ceiling crossing; an already-encoded loser becomes an honest
`snapshot-failed` event and publishes no bytes. Later decisions stand down
before encode. A closed/finalised scope also stands down at that pre-encode
seam, so a retained sink records and encodes nothing.

## 5. Manifest and CLI

`vibe-core` carries strict consumer-side `[compile] trace = bool`, default
false, with parse/write symmetry (`7adfbb5a`). The table is role-equipotent:
legal on project, package, virtual-workspace and combined roots, because a
package-rooted checkout is still a consumer. It is read only from the selected
root/workspace manifest; a dependency package's own `[compile]` table never
activates tracing for its host and is copied into no lock/index/activation
surface. `Manifest::compile_trace_enabled()` is the role-blind read seam.
`vibe install --trace-compile` and every higher lifecycle verb's shared args
enable the same option; CLI true OR manifest true enables, neither can
accidentally disable the other. There is no user-config rung.

Thread a `CompileTraceOptions`/sink through traced siblings of existing APIs;
keep current public wrappers as no-trace compatibility calls. `vibe-workspace`
owns artifact/node labels and persistence. `vibe-spec` owns pass events and IR
encoding. `vibe-cli` owns the human/JSON timing presentation. Neither workspace
nor CLI reaches into domain IR.

The install/lifecycle machine result includes trace run path and timing summary.
JSON output extends a JTD-owned report; it does not append an ad-hoc object.

### 5.1 One command owner, always at the workspace root

The outermost command opens at most one recorder, after it has the selected
manifest and the lifecycle `run_id`/original `started`, but before the first
compile. The storage/lock root is always canonical `Workspace::root`, including
an invocation entered through a member: install regenerates shared package
units plus every node, so two members may not acquire independent trace locks
for that same work. The selected node manifest alone decides activation:

```text
effective trace = --trace-compile OR selected_manifest.[compile].trace
```

Dependency manifests never activate host tracing. Lower workspace/install APIs
borrow `Option<&TraceRun>`; they never open, finish, clone into an outcome, or
retain a recorder past the command. Lock order once R7.4 lands is lifecycle
command lease → trace lock.

Existing no-trace APIs remain compatibility wrappers over traced siblings with
`None`. Off mode does not construct a descriptor, clock, recorder, buffer or
path and preserves boot bytes, mtimes and freshness exactly.

### 5.2 Scope attempts, output fingerprints and deterministic order

Workspace owns one portable scope constructor: unit identity is qualified
`(group,name)` plus its stable label/version; node identity is its canonical
workspace-relative path; target/artifact comes from the selected static format.
Absolute node paths never become ids. Package units are sorted by canonical
`(group,name)` before any scope declaration; nodes retain the existing
root-first, relative-path-sorted order.

An artifact base identity may compile repeatedly inside one adopted run. The
attempt allocator reacquires the latest exact `pending` occurrence after an
interruption, but after a terminal occurrence it deterministically mints the
next scope id. Thus a post-install park/resume can regenerate the same node and
append events without resetting the run or colliding with a compiled scope.

Dirty compile declares one occurrence, runs the real compiler once through
that sink and marks it `compiled` with the manager-owned emitted-bytes SHA-256.
Compiler refusal marks the scope `failed` and returns the original error
unchanged. A fingerprint-fresh unit first safely reads/validates its existing
emitted bytes. Only after that observation succeeds does it declare the
occurrence and record `skipped` with the same output digest, before returning
with zero events. If the already-fresh output cannot be observed safely, the
freshness decision still stands, the trace records a bounded warning and no
scope is declared; it must not publish a `pending` scope that would make a
successful command's terminal `ok` index impossible. Scope declaration or
resolution failure is likewise an observer warning and never becomes the
artifact error.

### 5.3 Park, adoption, displacement and one outcome funnel

Hosted park is suspension: take a running summary, do not call `finish`, drop
the recorder/last scope to release its lock, and report the running trace in
the one parked command root. Same-command resume reuses the persisted run id
and original start, reopens that running index and appends attempts/sequences.
A completed resume finalises `ok`; a failed resume finalises `failed`.

Trace activation is sticky in the lifecycle state. Add an optional epoch-1
`compile_trace` run member; new writes carry the effective value, old
nondelegated state defaults false, and adoption uses `current_request OR
prior.compile_trace`. A resume therefore keeps tracing even when the original
one-shot flag is absent and the manifest changed meanwhile.

The sticky bit proves an effective **request**, not that a recorder actually
opened. A fresh identity may create its run; an adopted identity must use an
`open-existing` seam. If the original invocation reported tracing unavailable
and no run directory exists, its resume stays unavailable instead of silently
starting a partial trace halfway through the lifecycle run. This also keeps a
reopen from converting a previously unavailable observer into an apparently
complete history with missing early compiles.

When identity selection deliberately displaces a state-owned parked run
(`--force`, changed command/chain/mode, and later selected-node mismatch), it
returns that exact prior run identity and trace bit. Before opening the fresh
run, the command attempts to **open the state-proven old trace only if it
already exists** and finalise it `failed` as superseded. The open-existing
operation takes the same cooperative project lock and applies the same
identity/link/index validation as a normal reopen, but never creates a missing
directory and never runs fresh-run retention. It never infers ownership from a
32-hex directory. This converts real abandoned running traces into
retention-eligible terminals, never manufactures a phantom trace merely to
supersede it, and keeps repeated force-reparks bounded.

Every exit after open passes one explicit outcome funnel:

- success → `finish(ok)`;
- compiler, publication, install or later lifecycle failure →
  `finish(failed)` while preserving the original command error;
- park/repark → running summary, no finish;
- final-index refusal → command result unchanged, report `finalised=false`.

No `Drop` implementation invents a timestamp or performs I/O. The funnel owns
the injected finish timestamp and runs before the one command report is built.

### 5.4 One JTD trace member across four command reports

`formats/vocabularies.json` becomes the single schema home for the shared
compiler-trace duration, timing row and command report fragments. The trace
index and `install`/`lifecycle`/`update`/`reinstall` schemas reference those
shared fragments; generated modules re-export the same `TimingRow` type. Each
command root gains one optional `trace` member. Disabled omits it byte-for-byte.

The shared report distinguishes `unavailable|running|ok|failed` and carries:

- exact run id and optional absolute forward-slashed run path;
- `finalised`, `budget_exhausted`;
- lossless canonical-decimal event/snapshot/snapshot-byte counts (JTD has no
  `uint64`; no checked-conversion failure may turn the observer into a veto);
- ordered generated aggregate timing rows;
- bounded warnings, including requested-but-unavailable open/scope failures.

Requested but unavailable tracing compiles untraced and reports the bounded
reason. Human mode renders one table from the same typed member; quiet mode
keeps one line with a compact suffix. JSON never emits a standalone trace
object: exactly one registered command report root owns the member, even when
deferred plan documents precede it.

`--trace-compile` is accepted by install, update, reinstall and every default
lifecycle verb (including a chained clean continuation). Clean-only compiles
nothing and opens no trace. `init`, publish staging and uninstall are real
compile sites but have no lifecycle `RunMetadata`/one-of-four report contract;
they are explicitly outside R3.4 rather than given one-off identities. Their
future trace surface must be command-owned and JTD-first in the same way.

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
16. Two barrier-synchronised scopes publish exactly one soft-ceiling crossing;
    the encoded racer publishes nothing and is reported truthfully.
17. A closed/finalised retained sink returns the compiler's no-encode decision.
18. A post-publication index mutation cannot masquerade as the attempted
    terminal bytes.
19. Cooperating writers serialize per project; swapped lock-file identity is
    re-contended before a guard is returned.
20. Dirty unit plus root node share one run/global sequence; unit order is
    stable under permuted resolution/map construction.
21. A fresh unit is event-silent and carries the same emitted-output digest as
    its prior dirty compile. An unreadable already-fresh output produces a
    warning and no scope, never an orphaned `pending` occurrence.
22. Recompiling one artifact after a hosted park allocates the next scope
    occurrence; an interrupted pending occurrence is reacquired exactly.
23. Park leaves a running trace; resume without the original one-shot flag
    reopens it, appends sequence and eventually finalises the same run. If the
    original requested trace never opened, resume reports it unavailable and
    does not create a misleading mid-run history.
24. Repeated traced force-reparks leave one active running trace and terminal,
    retention-eligible state-proven predecessors; superseding a state-proven
    but absent trace directory never creates a phantom terminal run.
25. Install/lifecycle/update/reinstall use one shared generated trace member;
    disabled old JSON still round-trips with it absent and no standalone trace
    document is emitted.
26. Empty/fresh/ready install, scoped/whole update, plain/empty-force/normal-
    force reinstall and post-compile failure all pass through the same outcome
    funnel without changing command success/error identity.

## 7. Implementation order

1. Land R6.2a schema/corpus.
2. Implement strict bidirectional conversion + gate registry (**landed at
   `17afb5b6`**).
3. Add in-memory pass observer/timing with no-trace compatibility wrappers,
   including the pre-encode `snapshot-skipped-budget` decision seam
   (**landed at `fa0662a9`**).
4. Add trace-index JTD/generated types (**metadata contract `6f4a717d`**) and
   atomic run writer/newest-nine retention/budget (**landed at `4d95a129`**).
5. Add attempt-aware scope allocation and borrowed traced siblings through
   workspace/vibe-install while keeping every no-trace wrapper exact.
6. Add sticky lifecycle activation, the command-owned outcome funnel, shared
   JTD report member, flags/presentation and cross-command e2e.
7. Add final failure/park/resume/displacement and byte/mtime identity tests.
8. Only then expose the same conversion to native compiler passes in R6.3.

This order makes R3.4 a real consumer of the public epoch and makes the native
ABI reuse a wire already exercised by human debugging, rather than freezing a
contract no tool has ever read.
