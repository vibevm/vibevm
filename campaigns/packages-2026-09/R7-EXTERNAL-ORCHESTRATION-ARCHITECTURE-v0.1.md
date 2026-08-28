# R7.5 — external orchestration evidence and requirements architecture

_Status: accepted central design, 2026-08-28. Implementation authority remains
PROP-054 §4.6/§14.7; this document freezes ownership, dependency direction,
landing order and acceptance for R7.5 P0–P3._

## 0. Outcome and hard boundary

R7.5 does not build a coding agent. It gives any external process three neutral
things:

1. existing bounded lifecycle control (`vibe <phase> --json` and MCP
   `lifecycle_run`/`lifecycle_tasks`);
2. one generated verification-evidence member on verify-bearing lifecycle
   reports, exact about the declared bytes/artifacts it measured;
3. one bounded read-only requirements metadata query over CLI and MCP.

The external process decides whether to accept, edit, replan, ask, invoke
another command or stop. VibeVM adds no Plan/Act policy, no next-task heuristic,
no automatic `verify → create` edge, no hidden attempt loop and no PDSA-named
machine vocabulary.

LLM support remains orthogonal. Requirements/evidence paths construct no
provider, read no credentials and work on a machine with no API access. An
optional LLM enhancement elsewhere follows PROP-054
`##LLM-ENHANCEMENT-MODES`; it is neither implied nor activated here.

## 1. Decisions that must survive implementation

### 1.1 Exact means scoped exact, never “hash the repository”

A verification claim identifies:

- the run id and selected workspace node;
- the provider-qualified execution and its declaration fingerprint;
- a canonical witness for its explicitly declared input patterns;
- independent witnesses for consumed/produced artifacts.

The input witness is a stable manifest of the regular, link-free project files
selected by those patterns: canonical relative path + exact bytes, sorted and
domain-separated. It records patterns, file count, byte count, algorithm and
digest. An absent `inputs` declaration is `unavailable`; it is never a digest
of the empty set disguised as a measurement.

This deliberately does not hash `target/`, `node_modules/`, `.git/`, `.vibe/`,
the package cache, unrelated `vibedeps/` trees or arbitrary machine data.
Explicit artifact verification may hash a large declared artifact; that is the
artifact the claim is about, not a search for identity in unrelated gigabytes.

### 1.2 Execution fingerprint and evidence witness are siblings, not aliases

The existing lifecycle fingerprint includes requested command, whole chain,
agent mode, world, current accumulated artifacts, manifest/lock and provider
material. It answers “may this execution fresh-skip in this invocation?” It is
not a pure tree witness and cannot be relabelled as one.

Refactor the one declared-input walk so it produces both:

- the existing execution fingerprint material, byte-compatible when evidence
  fields are not observed by old callers;
- a typed `InputMeasurement` from the same file observations, without a second
  tree walk.

The evidence identity carries the execution/declaration fingerprint **and**
the input measurement. Their difference is load-bearing.

### 1.3 Five evidence words, one pass word

The closed comparison vocabulary is:

`matched | stale | missing | unavailable | unstable`.

Only `matched` is a pass. `unmet`, `met`, `fulfilled`, `verified` and a boolean
equivalent do not exist. Command success and handler failure remain their own
axes: identity can match and a verify handler can still fail.

### 1.4 Four requirement observations never collapse

Requirements metadata keeps separately typed:

- authoring status from `vibe-specdoc`;
- consumer adoption from the `vibe-facts` registry;
- optional relation-provider state/provenance and edges;
- lifecycle verification evidence as a separate report root.

The requirements report does not attach a synthetic verification verdict to a
fact row. A `verifies` edge and a matching lifecycle evidence document are two
observations an external orchestrator may join; VibeVM does not perform the
policy join.

## 2. Dependency and ownership graph

```text
vibe-wire                     JTD-generated evidence + requirements contracts
   ↑
vibe-facts                    pure authoring/adoption scan + per-address join
   ↑
vibe-workspace                already depends on vibe-facts; source/layout APIs
   ↑
vibe-requirements             NEW read-only capability/library owner
   ↑                 ↑
vibe-cli            vibe-mcp             thin parsers/renderers
   ↑                 ↑
vibe-trace ─ implements optional RelationProvider adapter

vibe-lifecycle ─────────────→ vibe-wire   evidence measurement/reconciliation
       ↑
vibe-orchestrator                         existing lifecycle command owner
```

Rules:

- `vibe-facts` never depends on `vibe-workspace` (existing direction is the
  reverse).
- `vibe-lifecycle` never depends on `vibe-trace`, `specmap-core`,
  `vibe-requirements` or `vibe-registry`.
- `vibe-requirements` does not depend on `vibe-trace`. It defines a small
  `RelationProvider` seam; `vibe-trace` supplies a local implementation.
- CLI and MCP already depend on `vibe-trace`; they optionally inject that
  adapter into the one `vibe-requirements::query` function.
- A Cargo feature is not a substitute for optionality: optional relation data
  is a runtime provider value, not a hidden engine dependency.

### 2.1 `vibe-facts` additions

Extract the third caller rather than copy CLI-private logic:

- a public full-address prefix builder using
  `vibe_spec::canonical_doc_path`;
- a read-only source scanner that accepts `(source root, package coordinate,
  source kind)` and returns addressed `AuthoredFact` rows through the existing
  `vibe-specdoc` pivot;
- a pure per-address join over authored rows and `Registry`, distinguishing
  `not-applicable | absent | indeterminate | recorded` adoption presence.

No sync/reconcile/write API is called by R7.5.

The scanner implements that frozen root-based signature directly. It uses the
public `vibe-core` layout spellings to walk the host/package source root,
excludes the generated boot INDEX/static lane, loads either Markdown or XML
through `vibe_specdoc::load_spec_text`, and mints the address through
`vibe_spec::canonical_doc_path`. It does not read a slot record or derived
manifest: materialisation changes only a spec document's extension, while the
canonical document path strips that extension, so source and materialised
output have one address. This keeps `vibe-facts` independent of
`vibe-workspace` and also covers in-place package slots that have no record.

### 2.2 `vibe-requirements` additions

The new crate owns:

- selected host + lock-selected materialised-package source enumeration using
  public `vibe-workspace`/`vibe-core` layout and lock types;
- bounded filters before expensive package parsing;
- one stable source/observation digest over exactly the spec/registry bytes
  read;
- optional relation-provider invocation once per query, never per fact;
- construction of the generated `RequirementsReport` and shared text summary.

Absent adoption home is an empty layer. Absent package participation or
relation data is a typed source state. A malformed present authored source is
reported as `invalid` for that source; it does not mutate or become a lifecycle
failure.

### 2.3 `vibe-trace` adapter

The adapter:

- implements `vibe-requirements::RelationProvider` and returns only
  `Available | Stale | Unavailable | Invalid` outcomes plus
  address-associated edges; the query library alone maps those outcomes to
  `current|carried` and provenance from base source kind;
- builds the selected host map fresh in memory once when relations are
  requested, only when `specmap.toml.namespace` equals the selected host
  coordinate; absent config is unavailable and a namespace mismatch is not a
  successful zero-edge map;
- exposes current host edges without raw source/code bodies;
- consumes A2's already lock-selected materialised root and expected content
  hash — never a second slot discovery — and reuses the carried
  `package.specmap.json` rather than rebuilding private package source that
  may not ship;
- accepts a carried map only when `.vibe-slot.toml.source_hash` equals the
  lock-selected content hash, the record owns a `package.specmap.json` row and
  one capability-relative no-follow/single-link read yields bytes whose
  SHA-256 equals that row and which are parsed without a second read. Missing
  record/map is unavailable, source-hash or map-hash mismatch is stale,
  matching malformed JSON is invalid. This certifies the exact published map
  byte, not a fictional consumer rebuild of transformed/unshipped source;
- emits every edge `file` workspace-root-relative: a selected-member host edge
  is prefixed by `observation.selected`, and a package edge by its
  materialised slot's workspace-relative path;
- never writes `specmap.json`.

`vibe-trace` may depend on `vibe-workspace` for the shared slot-record/SHA
helpers and on `vibe-safefs` for the identity-bound one-file read. Both edges
are acyclic and preferable to copying either filesystem grammar. The existing
streaming `sha256_file` remains streaming; the byte helper serves callers that
already own exact bytes and parity tests keep one SHA spelling.

## 3. P1 — JTD and state evolution

P1 lands schemas/registry/corpora/generated types before engine consumers.

### 3.1 Shared JTD fragments

Add to `formats/vocabularies.json` (names exact):

- `evidence_status` — closed five-value enum;
- `digest_witness` — `{ algorithm, digest, files?, bytes? }`, where `files`
  is `uint32` and `bytes` is a canonical unsigned-decimal string (JTD has no
  `uint64`; a declared input set above 4 GiB must remain representable);
- `input_measurement` — execution/phase/declaration/patterns/run + witness;
- `artifact_witness` — id/kind/path/run + digest witness;
- `verification_evidence` — the exact PROP-054 §14.7 root/member.

On both comparison rows, `measured_run_id` is present exactly when the
`measured` witness is present. The two members are one attributable
measurement: neither an unowned witness nor an id pointing at a measurement
the row says does not exist is a valid intermediate state. The status matrix
separately decides that only `unavailable` may omit the pair.

This follows the existing `compile_trace_report` pattern: one vocabulary
fragment becomes one shared generated Rust type and is imported by every
owning schema. Do not create a duplicate standalone evidence schema containing
the same definitions.

Every optional scalar declares `x-default`; collection declares `x-empty`;
enum site declares `x-vocabulary`. Relational laws JTD cannot express live in
`vibe-wire/src/behaviour/verification_evidence/` and are pinned against the
schema vocabulary by tests.

### 3.2 Additive lifecycle state

`schemas/lifecycle_state.jtd.json` imports the fragments and adds:

- `execution_record.input_measurement?`;
- `state_artifact.witness?` and `state_artifact.measured_run_id?`.

Do not widen `ExecutionRecordStatus`; it remains its current closed lifecycle
outcome vocabulary. Legacy state reads with absent witnesses. A fresh skip is
still an observation: the runner already computes current fingerprint inputs
and probes artifacts, so it may checkpoint the new witness without pretending
an old measurement existed.

Correct the stale run-id metadata: current post-R7.3 begins carry a real run id
for every invocation, not only delegated runs; legacy absence remains readable
when no ownership claim depends on it.

Compatibility is forward only for this recoverable state: current code reads
legacy absent members; a pre-R7.5 strict state reader may reject new members.
That downgrade is honest and recoverable by deleting/rebuilding lifecycle
state, not a reason to weaken the current strict reader.

### 3.3 Additive lifecycle report

`schemas/lifecycle_report.jtd.json` imports `verification_evidence` and adds one
optional `verification` member (`x-default = null`). It is present exactly
when the verify phase reached engine-owned evidence reconciliation, including
stale/missing/unstable outcomes. Reports from earlier phases and old corpora
remain byte-shape compatible.

Add at least these authored corpora:

- `report_verified.json` — matched inputs/artifacts;
- `report_verification_stale.json` — one measured/observed mismatch;
- `report_verification_unavailable.json` — no evidence-bearing rows.

Semantic corpus tests assert identity/status/order, not merely round-trip.

### 3.4 Requirements root

Add and register `schemas/requirements_report.jtd.json` with the exact
PROP-054 §14.7 vocabulary, generated as
`vibe_wire::generated::requirements_report::RequirementsReport`.

The root carries two source layers before rows:

- `sources[]`: base authored-source results keyed by package coordinate, with
  `kind` as a value and never a second identity component; state
  `available|unavailable|invalid|orphaned`, with the state-dependent digest,
  reason and adoption-entry count. Only `available` sources may own fact rows.
  One coordinate cannot occur once as `host` and once as `package`, because
  the enrichment layer below keys by coordinate alone and must recover exactly
  one kind to validate fresh-versus-carried provenance.
- `relation_sources[]`: optional enrichment state/provenance; every entry
  binds to exactly one base source result, and it never stands in for a
  malformed/missing authored source.

Corpora cover:

- host + package facts with distinct authoring/adoption states;
- relations not requested;
- current host relations + carried package relations;
- partial/invalid/orphaned authored source and unavailable relation provider;
- a small reviewable truncation corpus plus explicit 256/257 validator bounds;
- no prose/body canary.

## 4. Evidence measurement and verify algorithm

### 4.1 Measurement at execution/fresh-skip

Refactor `vibe-lifecycle/src/state/fingerprint.rs` around an internal prepared
measurement:

1. validate each declared input pattern;
2. walk only its declared project scope with existing shippable exclusions;
3. refuse symlink/junction/reparse/hardlink/physical-alias ambiguity;
4. open/read with stable identity+metadata checks; a moving file is
   `unstable`;
5. sort canonical relative paths and frame patterns/path/bytes once;
6. return execution fingerprint + optional `InputMeasurement`.

Stability is an observed property, not a claim that Vibe took an atomic
filesystem snapshot. A measured file is accepted only through no-follow,
single-link handle identity, equal pre/post identity and length, and two
consecutive bounded reads yielding identical bytes; any observed disagreement
is `unstable`. Even that cannot prove that an adversarial writer never wrote
the same bytes between observations, so diagnostics say what was detected,
never “the file could not have moved”. Hardlinked input bytes keep feeding the
legacy execution fingerprint for byte compatibility, but their evidence
measurement is refused: enabling evidence may not silently change freshness.

`inputs = null` means unavailable. An explicitly authored empty list is a
complete empty declared scope and remains distinguishable in the measurement.

The one walk produces two deliberate projections. The legacy execution
fingerprint replays its existing pattern-major stream byte-for-byte: each
authored pattern in declaration order, then every matching path/byte in path
order, including the historical repeat when patterns overlap. The evidence
manifest uses the deduplicated union so one physical path contributes one file
and one byte count. Its SHA-256 domain is
`sha256:vibe-input-manifest-v1\0epoch=1\0`; with the §4.2 length frame it writes
`pattern_count`, every declaration-order `pattern`, `file_count`, then every
union file in sorted forward-slash path order as `path`, `size`, `bytes`, and
finally `total_bytes`. Counts are canonical decimal UTF-8. `Some([])` therefore
has a real digest with zero patterns/files/bytes; `None` has no measurement.
Changing requested chain/config may move only the execution fingerprint;
changing a selected input byte moves both.

For each accepted artifact:

- regular file → `sha256:file-v1` over exact bytes;
- directory → `sha256:tree-v1` over canonical relative paths + file bytes;
- anything non-regular, linked, aliased, escaping or moving refuses the
  witness and therefore cannot become matched evidence.

State retains the existing absolute machine path needed to reopen the
artifact. The external evidence comparison normalises it against the canonical
selected root and carries a safe project-relative forward-slashed path. That
portable path plus `run.selected` is exact and does not leak the developer's
home.

The witness is checkpointed in the same state transaction as the execution
record. No `.vibe/evidence.*` second state file is created.

### 4.2 Engine-owned verify reconciliation

When the phase runner reaches `verify`, before user verify contributions:

1. select CURRENT-plan executions that explicitly declare inputs and look up
   their durable rows, plus EVERY currently accumulated artifact — including
   an unwitnessed legacy artifact, which must become an explicit `unavailable`
   comparison rather than vanish;
2. reconstruct the current declaration by the same provider-qualified key;
3. recompute input and artifact witnesses under the current selected world;
4. compare measured and observed values, preserving canonical row order;
5. derive the overall five-value status and `evidence_id`;
6. attach the generated member to phase outcome/report;
7. on `stale | missing | unstable`, stop verify before contribution dispatch;
   on `matched | unavailable`, continue to configured verify contributions.

A durable success row whose declaration is no longer in the current plan is
not selected: lifecycle state is a freshness cache, not an append-only audit
log, and a removed contribution must not poison every future verify. A current
declaration with no attributable measurement is selected and reported
`unavailable`; a current declaration whose fingerprint/identity differs is
`stale`. The generated member rides both success and the measured failure
carrier, so a stale stop and a later verify-handler failure retain the exact
comparison that existed before dispatch — failure projection may not rebuild
or drop it.

`evidence_id` uses one writer recipe, never JSON pretty-printing: SHA-256 is
seeded with `vibe-verification-evidence-id\0epoch=1\0`, then every member except
`evidence_id` and `observed_at` is framed in schema order with the existing
`field = be64(label_len) || label || be64(value_len) || value` primitive.
Unsigned values and array lengths use canonical decimal UTF-8; enums use their
wire spelling; every optional member first frames an explicit `0|1` presence;
arrays frame their count and then their already-canonical rows. This includes
the evidence epoch/status, complete run header, inputs and artifacts, every
witness/count and every reason. Cross-language implementations can therefore
reproduce the id without depending on Rust struct layout or JSON key order.

`unavailable` is visible but is not a universal policy failure: a project with
no evidence-bearing contribution retains today's empty verify posture. A
project that requires evidence declares an ordinary verify contribution which
judges that generated context; VibeVM does not invent the project's policy.

If a later verify contribution fails, lifecycle `ok` is false while the
identity member may remain matched. Do not rewrite one axis into the other.

### 4.3 Two invocations and hosted resume are distinct laws

The stale→recompute acceptance path is one uninterrupted first invocation:

- build/test measures the tree;
- a non-hosted create contribution then changes a measured input;
- verify compares the durable pre-create measurement with the post-create
  tree, reports stale and does **not** jump back;
- an external second `vibe verify` causes incremental build/test to recompute,
  create to fresh-skip when appropriate, and verify to match.

A hosted park is different. The first invocation stops at create before verify.
Calling the same phase to resume re-enters the inclusive chain from its prior
phases; if the host's output changed a build/test input, that deterministic
predecessor reruns and checkpoints a new measurement before the delegated
create row is accepted. Verify may therefore match on the resume itself. This
is the existing linear-resume law, not a hidden retry, and it must not be
weakened merely to manufacture a stale example. P3 proves hosted park/resume
and uninterrupted stale→external-second-invocation as two adjacent scenarios.

## 5. P2 — requirements query library

### 5.1 Query domain

`RequirementsQuery` is a library type with:

- optional `address_prefix` (must be a `spec://` prefix, never a bare id);
- `limit` default 100, inclusive range 1..=256;
- `relations` boolean default false.

The selected project root is a separate trusted constructor input, never part
of MCP arguments. A `QueryContext` also carries an injected `observed_at` and
optional validated lifecycle run id. CLI and MCP already depend on
`vibe-lifecycle` and obtain that id through the same read-only
`LifecycleStateStore::peek`; `vibe-requirements` does not pull the lifecycle
engine into a metadata library merely to read one join key.

### 5.2 Source semantics

Host and lock-selected package sources are enumerated deterministically.
Addressed facts with no authoring status still appear (`unmarked`). Consumer
adoption is:

- `not-applicable` for host-authored facts;
- `absent` when no package registry row exists;
- `indeterminate` when a row exists with no status;
- `recorded` with the exact closed status when present.

Registry-only orphans are reported as source observations, not silently
discarded and not promoted to authored facts. When a lock-selected source has
no slot but the registry still carries entries for its coordinate, `orphaned`
wins over `unavailable` and its reason names both facts: the positive
adoption-entry count is the more informative observation. A malformed
registry or malformed lock aborts the query with a typed read error — neither
has a representable source-result state and silently returning a host-only or
empty overlay would claim a scope the reader never established.

Each fact row's address coordinate must equal `source.package`; host rows carry
`adoption=not-applicable`, package rows never do. With relations requested,
every row's package has one relation-source result even when it has no edges;
host sources can use only a fresh-project-map provenance, package sources only
a carried-package-map provenance. These are validator laws, not writer lore.

One-read byte binding lives below the query without exposing bodies. The
`vibe-specdoc` pivot accepts caller-read raw text for its one extension/project
decision; `vibe-facts` returns sorted document witnesses
`{relative_path, raw_sha256, bytes}` beside facts (or beside an invalid parse),
and `Registry::load_with_witnesses` parses the same bytes whose witnesses it
returns. Existing text/registry APIs are wrappers over those seams; A2 never
re-walks or re-reads a source merely to hash it.

The three digest recipes reuse the §4.2 length frame, canonical decimals,
schema/wire order and explicit optional presence bits:

- `SourceResult.digest`: domain
  `vibe-requirements-source-digest\0epoch=1\0`; kind, package, document count,
  then each sorted document's path, byte count and SHA-256 of its exact raw
  bytes. The aggregate never carries raw prose across the crate boundary.
- `observation.source_digest`: domain
  `vibe-requirements-scope-digest\0epoch=1\0`; selected node, every sorted
  available/invalid source's kind/package/digest, then every sorted registry
  file witness `{path, bytes, raw_sha256}`. It excludes query, clock, provider
  result and lifecycle run id. Registry bytes are source bytes for this member,
  so changing only the adoption registry changes `source_digest`.
- `observation.observation_id`: domain
  `vibe-requirements-observation-id\0epoch=1\0`; every canonical report member
  except `observation_id` and `observed_at`, including requirements epoch,
  selected, `source_digest`, lifecycle-run-id presence/value, effective query,
  source results, relation-source results, rows/edges and `truncated`. It is
  never a JSON/Rust-layout hash. Thus a changed run join key changes the exact
  observation id while the clock alone does not.

### 5.3 Optional relations

`relations = false` means `not-requested` and does not load/build any map.
The one writer emits an explicit `not-requested/none` relation-source row for
every enumerated base source; the schema permits an empty list for foreign
writers, but our reference result has one deterministic form.
When true, the provider returns typed source states and bounded edges. Missing
config/carried map is `unavailable`; a carried map whose own content witnesses
do not match is `stale`; malformed present data is `invalid`. Base fact rows
still return.

No relation provider may change authoring/adoption or lifecycle evidence.
The provider is called once with the selected/workspace roots, every enumerated
base source (including its optional materialised root and, for a lock-selected
package, expected content hash) and the sorted limited addresses. It returns
per-package `available|stale|unavailable|invalid` plus edges; it never chooses
`current|carried` or provenance. The library derives those wire values from
the base source kind, making host-carried and package-current combinations
unrepresentable. A whole-provider failure maps every requested source to typed
unavailable enrichment; base rows still return. Provider output for an
unrequested address is a provider-invalid source result, never silently
attached elsewhere.

## 6. P3 — surfaces

### 6.1 CLI

Add one top-level read-only command:

```text
vibe requirements
  [--address-prefix <spec://prefix>]
  [--limit <1..256>]
  [--relations]
  [--path <project>]
  [global --json/--quiet]
```

Args live below `cli.rs` because that file has almost no line budget. Do not
extend the mutating `vibe facts` family. Human output is a bounded table/summary
from the same generated report; JSON is the report exactly.

No new `vibe evidence` command: `vibe verify --json` is already the evidence
control/report surface, and a second reader would immediately create a
reference-implementation split about revalidation timing.

### 6.2 MCP

Add exactly one tool:

```json
requirements_query({
  "address_prefix": "spec://…", // optional
  "limit": 100,                 // optional
  "relations": false            // optional
})
```

Runtime uses a `#[serde(deny_unknown_fields)]` private decoder before any file
access; descriptor derives the same caps/defaults. Unknown member, wrong type,
zero/overflow and invalid prefix are text-only argument errors with zero
filesystem/state mutation. There is no `path`, provider, model, write/sync or
lifecycle option.

No new `lifecycle_evidence` tool: `lifecycle_run({phase:"verify"})` returns the
generated member directly. `lifecycle_tasks` remains the mailbox reader.

### 6.3 Composition owner

Both surfaces call the same `vibe_requirements::query` and receive the same
generated root. They differ only in argument framing and text projection.
Production stdio tests prove MCP emits one frame even while relation scanning
runs.

## 7. Fake external PDSA reference test

The test process is the orchestrator; VibeVM never sees PDSA vocabulary.

Two fixtures share one addressed requirement fact with authoring status, no
adoption row and no `[llm]`, token, provider or network transport.

**Hosted-control fixture:** earlier deterministic phases measure; an agent
create row parks. The test reads `lifecycle_tasks`, writes the exact declared
output and calls the same verify phase. Resume re-enters the inclusive chain,
reruns any predecessor invalidated by that output, accepts the hosted row and
returns matched evidence with zero provider calls. This proves park/resume and
that no engine-owned agent loop exists.

**Stale/recompute fixture:** a local deterministic create test contribution
changes a build/test input later in one uninterrupted invocation, but does not
declare that file as its own freshness input. Verify returns stale before its
sentinel contribution. The test, and only the test, calls verify again;
build/test recompute, create fresh-skips and verification returns matched.

The test queries requirements before lifecycle work to choose the attempt, and
repeats enrichment with relations unavailable: requirement metadata changes,
while every lifecycle/evidence byte and provider-call counter stays identical.

Assertions count external invocations and scan product vocabularies to prove no
automatic back-edge or PDSA enum/phase/verb exists.

## 8. Decisive negative/mutation matrix

| ID | Mutation/probe | Must go red / remain true |
|---|---|---|
| E1 | relabel execution fingerprint as input digest | command/chain-only change demonstrates the two identities differ |
| E2 | omit declared-input byte from witness | edit that file leaves stale undetected → RED |
| E3 | treat absent `inputs` as empty matched set | unavailable fixture becomes matched → RED |
| E4 | hash excluded `target/`/node_modules | cache-only change moves evidence → RED |
| E5 | omit artifact digest/reprobe | mutate produced file after build → verify incorrectly matches → RED |
| E6 | follow symlink/junction/reparse | alias/escape fixture accepted → RED |
| E7 | consult only file existence | same-length content mutation survives → RED |
| E8 | continue after stale/missing/unstable | sentinel verify contribution runs → RED |
| E9 | auto-call create from verify | invocation counter / vocabulary fence → RED |
| E10 | widen `ExecutionRecordStatus` | wire vocabulary equality gate → RED |
| Q1 | query while lifecycle lease held | succeeds and creates nothing |
| Q2 | absent `.vibe`/registry/specmap | generated partial result, no path created |
| Q3 | raw fact/body canary | absent from JSON/text/MCP frame |
| Q4 | mutate only adoption registry | adoption + source digest + observation id move; authoring/source-result/relations/lifecycle stay byte-identical |
| Q5 | `relations=false` with exploding provider | provider counter remains zero |
| Q6 | remove carried package map | only its relation source becomes unavailable |
| Q7 | unknown MCP member + valid prefix | refuses before filesystem access |
| Q8 | add `unmet|fulfilled|verified` field/value | schema/generated/source vocabulary fence → RED |
| Q9 | surface builds/join rows itself | source fence allows only shared library call |
| Q10 | any provider/LLM dependency in requirements/evidence cells | dependency/source fence → RED |
| Q11 | row address coordinate differs from source package | source-coherence RED |
| Q12 | host row carries package adoption (or inverse) | status/source matrix RED |
| Q13 | malformed/unavailable/orphaned source emits a fact row | source-result matrix RED |
| Q14 | relations requested but a row package has no relation-source state | relation coverage RED |

## 9. Landing order and gates

### P0 — specification and architecture

- amend PROP-054 evidence/requirements/LLM/PDSA/status laws;
- reconcile PROP-000, boot/00-core and VIBEVM-SPEC vocabulary;
- land this architecture;
- XML parse, markup 508/0, `vibe check`, specmap 0 suspects/orphans/unresolved.

### P1 — wire/state substrate

1. shared vocabulary fragments + requirements schema + registry/corpora;
2. codegen + semantic wire tests;
3. additive lifecycle-state/report schema consumers;
4. state compatibility/strictness and report corpus tests.

### P2 — libraries

1. `vibe-facts` address/scanner/join extraction;
2. one-read `vibe-specdoc`/`vibe-facts` document+registry witness seams;
3. new `vibe-requirements` base query and optional-provider trait;
4. `vibe-trace` current/carried relation adapter;
5. `vibe-lifecycle` one-pass measurement + artifact witnesses;
6. verify reconciliation and report funnel.

Parallelism is by write perimeter; schema/state/lifecycle edits serialize.

### P3 — surfaces/e2e

1. CLI `requirements`;
2. MCP `requirements_query` + skill template;
3. CLI↔MCP report parity and stdout-frame test;
4. fake external PDSA scenario and vocabulary/source/dependency fences.

Use exact affected tests/clippy/conform/specmap per atom. Run the next full
panel only after coherent R7.5, not per sub-atom.

## 10. Rejected alternatives

- Built-in coding agent or lifecycle back-edge — policy, owner-rejected.
- Whole-repository/tree hash — expensive, overclaims scope, hashes unrelated
  caches/dependencies and still cannot create an atomic filesystem snapshot.
- Execution fingerprint renamed “tree digest” — mixes different semantics.
- `unmet`/`verified` fact boolean — collapses four owners into planning policy.
- Specmap as fact-status authority — relations do not author/adopt/verify.
- `vibe-facts → vibe-workspace` or lifecycle→specmap dependency — cycles or
  optional enrichment becoming engine floor.
- Separate `.vibe/evidence.*` — second state transaction and crash window.
- Add evidence words to `ExecutionRecordStatus` — breaks a closed vocabulary.
- Put requirements under mutating `vibe facts` — read-only boundary becomes a
  flag convention.
- Add `vibe evidence`/`lifecycle_evidence` — duplicates existing verify report
  surfaces and creates timing/identity drift.
- PDSA-named product phases/fields — turns a reference scenario into policy.

## 11. Accepted worker findings and disposition

The read-only P0 audit's stale-anchor/vocabulary findings are applied to
PROP-054, PROP-000, boot/00-core and VIBEVM-SPEC. Its proposed per-agent-row
`off` default is rejected: a pure agent contribution has no algorithmic twin;
enhancement modes apply to core features, while an explicitly activated agent
workload fails or parks honestly.

The API census's dependency graph, reusable seam list, JTD optional-shape
constraints, file budgets and negative probes are accepted. Its separate
evidence commands are rejected as duplicate surfaces; its suggestion that the
execution fingerprint vector *is* the measured tree is narrowed to the
declaration fingerprint plus a separately typed input manifest, because
requested chain/current artifacts are not source-tree identity.

The P2 A4/A5 audit's call graph and failure-funnel finding are accepted with
central corrections: current-plan declarations are the evidence universe;
unwitnessed current artifacts remain visible as `unavailable`; removed success
rows are not audit history; hardlinks preserve legacy fingerprint bytes while
evidence refuses them; and the generated member must survive both stale-stop
and later-handler-failure carriers. Its report correctly exposed that hosted
resume reruns invalidated predecessors, so the former hosted-stale fixture is
split into the two scenarios above. Its claim of an atomic/stable filesystem
epoch is narrowed to the explicit detection-bound observation law in §4.1.

The P2 A2 audit's missing one-read seams and query/provider split are accepted
with central rulings: malformed registry/lock abort because their scope has no
wire state; orphaned wins when adoption entries survive a missing slot; run id
is a surface-injected read-only join key and is included in observation id;
the reference writer emits explicit not-requested rows; provenance belongs to
the library, not the provider. Raw bytes never cross into the report or public
query result — only per-file witnesses do — and Q4 now agrees with the P1
source-digest contract.

The P2 A3 audit's carried-map witness gap is resolved without pretending the
specmap schema carries a source-tree digest: A2 passes the lock content hash;
A3 checks it against the slot record and checks the map byte against the
recorded file row. A match means “this is the map byte published in the exact
lock-selected package snapshot”, not “the consumer rebuilt private source”.
The audit's namespace gate, no-second-discovery finding,
workspace-root-relative edge rule and cycle-safe
`vibe-trace → vibe-workspace` dependency are accepted. Its claim that no
producer exists is narrowed: `vibe specmap` already produces
`package.specmap.json`; today's installed slots merely carry none, so answers
are honestly unavailable until a package ships one.

All accepted facts now live here, in normative specs or in the forthcoming
tests; the untracked reports are disposable after P0 acceptance.
