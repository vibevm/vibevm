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

- builds the project map fresh in memory once when relations are requested;
- exposes current host edges without raw source/code bodies;
- reuses the existing carried `package.specmap.json` path for installed
  packages rather than rebuilding private package source that may not ship;
- reports `current | carried | stale | unavailable | invalid` with provenance;
- never writes `specmap.json`.

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

`inputs = null` means unavailable. An explicitly authored empty list is a
complete empty declared scope and remains distinguishable in the measurement.

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

1. select evidence-bearing prior rows: build/test (and any other prior row)
   that explicitly declares inputs, plus every accumulated artifact carrying a
   witness;
2. reconstruct the current declaration by the same provider-qualified key;
3. recompute input and artifact witnesses under the current selected world;
4. compare measured and observed values, preserving canonical row order;
5. derive the overall five-value status and `evidence_id`;
6. attach the generated member to phase outcome/report;
7. on `stale | missing | unstable`, stop verify before contribution dispatch;
   on `matched | unavailable`, continue to configured verify contributions.

`unavailable` is visible but is not a universal policy failure: a project with
no evidence-bearing contribution retains today's empty verify posture. A
project that requires evidence declares an ordinary verify contribution which
judges that generated context; VibeVM does not invent the project's policy.

If a later verify contribution fails, lifecycle `ok` is false while the
identity member may remain matched. Do not rewrite one axis into the other.

### 4.3 The two-invocation law

On `vibe verify` first run:

- build/test may measure the tree;
- create may then change a measured input or park for a host;
- resumed verify compares against the post-create tree and reports stale;
- it does **not** jump back.

An external second `vibe verify` causes incremental build/test to recompute,
create to fresh-skip when appropriate, and verify to match. This exact sequence
is the acceptance scenario, not prose.

## 5. P2 — requirements query library

### 5.1 Query domain

`RequirementsQuery` is a library type with:

- optional `address_prefix` (must be a `spec://` prefix, never a bare id);
- `limit` default 100, inclusive range 1..=256;
- `relations` boolean default false.

The selected project root is a separate trusted constructor input, never part
of MCP arguments.

### 5.2 Source semantics

Host and lock-selected package sources are enumerated deterministically.
Addressed facts with no authoring status still appear (`unmarked`). Consumer
adoption is:

- `not-applicable` for host-authored facts;
- `absent` when no package registry row exists;
- `indeterminate` when a row exists with no status;
- `recorded` with the exact closed status when present.

Registry-only orphans are reported as source observations, not silently
discarded and not promoted to authored facts.

Each fact row's address coordinate must equal `source.package`; host rows carry
`adoption=not-applicable`, package rows never do. With relations requested,
every row's package has one relation-source result even when it has no edges;
host sources can use only a fresh-project-map provenance, package sources only
a carried-package-map provenance. These are validator laws, not writer lore.

The observation id hashes canonical source identities/bytes, registry bytes,
query members and relation-provider result (when requested), excluding
`observed_at`. It is a digest of exactly this metadata answer, not the project
tree.

### 5.3 Optional relations

`relations = false` means `not-requested` and does not load/build any map.
When true, the provider returns typed source states and bounded edges. Missing
config/carried map is `unavailable`; a carried map whose own content witnesses
do not match is `stale`; malformed present data is `invalid`. Base fact rows
still return.

No relation provider may change authoring/adoption or lifecycle evidence.

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

Fixture:

- one addressed requirement fact with authoring status and no adoption row;
- one build/test contribution declaring an input that create will change;
- one hosted agent contribution writing that input;
- no `[llm]`, token, provider or network transport.

Sequence:

1. Query requirements and inspect independent metadata; the test chooses the
   work. No lifecycle invocation occurred.
2. Call `lifecycle_run({phase:"verify"})`. Earlier phases measure, create parks.
3. Read `lifecycle_tasks`, write the exact declared output, call the **same**
   verify phase to resume. Verify reaches evidence reconciliation and returns
   stale because create changed a measured input. No later phase/handler runs.
4. The test, and only the test, decides to call verify again. Build/test
   recompute, create fresh-skips, verification returns matched.
5. Repeat with relations unavailable: requirement enrichment changes, every
   lifecycle/evidence byte and provider-call counter stays identical.

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
| Q4 | mutate only adoption registry | only adoption + observation id move |
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
2. new `vibe-requirements` base query and optional-provider trait;
3. `vibe-trace` current/carried relation adapter;
4. `vibe-lifecycle` one-pass measurement + artifact witnesses;
5. verify reconciliation and report funnel.

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

All accepted facts now live here, in normative specs or in the forthcoming
tests; the untracked reports are disposable after P0 acceptance.
