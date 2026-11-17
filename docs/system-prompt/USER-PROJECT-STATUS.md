# USER-PROJECT: implementation and validation status

Verified milestones on 2026-11-17. The earlier bounded explicit-workflow acceptance
is a base, **not full USER-PROJECT acceptance**. The renewed UP-COMPLETE-01 mandate
includes automatic routing, explicit relations and full selected-node AI-native
authority. Old explicit-selector completion exemptions no longer apply.

See the [quickstart](README.md#personal-specification-boot),
[offline onboarding](user-project-onboarding.py), and
[shipping authority proof](user-project-authority-proof.py).

## Implemented and demonstrated

| Requirement family | Demonstrated behavior and evidence boundary |
| --- | --- |
| Personal root, init/status and lifecycle | Separate ordinary `<settings>/user-project`; explicit install/update/uninstall, list/check. No boot-time initialization or repair; MCP project remains separate. Existing connected onboarding covers authored memory, package lifecycle and next launch. |
| Automatic global routing | Existing declared/locked personal contexts use ordinary personal source policy after selected-source preflight. Original root admission, payload hash and Git commit identity remain checked. Repeated root/transitive install, conflict no-writes and preview tests pass. Source-owned continuation is committed in `082c05de1`. |
| Cancellation | Inferred and explicit personal install support pre-existing stop and cancellation during owned loopback HTTP preflight. Application/MCP/remap reject an explicit stop after classification before acquisition. Single network-session phase handoff is tested; this does not enable cancellation for legacy routes. |
| Late boot and cache-stable prefix | Complete project continuation precedes personal, then override. Actual argv-recording launch proves delivery, next-launch authored edits and shared prefix preservation, not model obedience or provider cache behavior. |
| Personal memory | Authorized authored record enrollment/live edit survives package removal. Guidance distinguishes personal/project facts, excludes credentials and honors opt-out. No autonomous fact writer, harness memory migration or synchronization is invented. |
| Independent controls | Personal enable/disable is project-scoped and independent of override binding/enablement. Schema1 observation does not migrate bytes; explicit mutation writes schema2. Capture and view tests cover the controls. |
| Explicit layer relations | Ordinary `[layers]` schema1 declares exact anchored targets and supplement/replace relations. Replacement requires the target owner's explicit preference grant; safety, attribution and commit discipline remain protected. Package owner and exact versions are preserved independently per layer. Protected replacement denies the optional layer. No prose inference or merged lock. |
| Layered visualization and delivery | Plain, JSON epoch2 and TUI use the same relation model; plain/TUI share qualified owner/version rendering. Capture/compose/launch compute precedence from actually delivered layers. Unindexed admitted relation source text is delivered by explicit notice; prefix bytes remain intact. Earlier console/hosted-PTY controls evidence is retained with its exact scope. |
| AI-native authority producer | Ordinary empty/Ready/no-op install publishes private `.vibe/specmap-authority.json` from exact planner per-node roots and actual durable edges. Two-node workspace tests prove each selected context excludes sibling dependencies. Missing/stale context fails with ordinary install remedy; reads never repair it. |
| Actual AI-native consumers | Sixteen shipping Rust/TypeScript CLI+MCP outcomes: four declared custom-spec-root/alias positives; four real direct-annotation positives (non-vacuous edge); four private/unknown namespace direct-reference refusals; four undeclared configured-root refusals. Generic non-Vibe explicit externals remain supported. Existing TypeScript compiler used in isolated fixture, no installation/provider. |
| Lifecycle authority freshness | Public install→no-op→reinstall→scoped update→uninstall regression passes; removed dependencies disappear from authority. Uninstall consumes its admitted removal plan roots without registry lookup. Valid-context reinstall stays source-free. Legacy reinstall without context repairs boot and diagnoses install remedy, without claiming fresh authority. |
| Deterministic publication | Repeated multi-node no-op installation preserves artifact bytes and mtime. Generated HashMap fields serialize through explicitly sorted JSON objects. All workspace-node manifests are fingerprinted. |
| Privacy | Reserved boot/local excluded before publisher descent; local compositions carry in-directory ignore rules. Personal/override roots cannot become project index/coverage authorities. Deliberate Git history publication remains the owner's responsibility; no secret/history-policing framework. |

## Evidence and remaining completion work

Reproducible authority proof:

```sh
python3 -I docs/system-prompt/user-project-authority-proof.py \
  /absolute/path/to/vibe /absolute/path/to/consumer/bin /scratch/parent \
  /absolute/path/to/node_modules/typescript
```

The script creates isolated settings/home/registry and records binary hashes,
command outputs and each expected consumer result. It performs no provider call.
The personal onboarding script separately covers launch/memory/override composition.
A final connected acceptance scenario must retain both boundaries, not substitute
helper tests for all user behavior.

Current completion frontier, not accepted as finished merely by this document:

- Final install lifecycle-tail freshness is enforced after callbacks. An actual
  `phase:install` fixture changed the manifest after durable installation: exit1,
  explicit stale-context/install remedy, and a measured failed report retaining
  materialised packages and regenerated nodes. No false rollback or success.
- Finite existing remote/SSH/locked-source/in-place validation passed 67 tests.
  Other solver/root-grammar cases require actual owner witnesses, never
  union-of-lock or raw-root approximation. The Fresh no-resolution regression was
  repaired in the existing Fresh admission owner, with original no-solve assertions
  unchanged; the final full orchestrator run passes 171/171. Path-only and member
  a→b→a tests pass, including no sibling authority grant and conflict refusal.
- Final affected strict lint passes across six crates/all targets; full conform
  reports 141 existing findings and zero new, with baseline unchanged. Real codegen
  leaves 242 outputs unchanged; sync verifies 51 pairs/9 sets/6 vendor directories.
- Freshly built host binary completes the connected journey: 37 recorded steps,
  36 assertions, including four real shipping consumers on that same personal
  root. The separate 16 positive/negative authority outcomes also pass on that
  binary. Classic wording expectations are refreshed for schema1/2 without
  weakening no-command/no-mutation checks.
- Scoped implementation and compatibility repairs are committed and reviewed.
  The latest full canonical host panel passes all eight stages after the
  owner-requested residual gate repairs. Earlier failed subjects remain recorded
  below rather than being relabeled successful. Owner acceptance and external
  release remain separate from technical implementation completion.

### Canonical panel and compatibility repairs

| Canonical step | Actual outcome |
| --- | --- |
| Workspace format | PASS |
| Workspace tests | PASS, full workspace rerun,1832s |
| Workspace Clippy, all targets, warnings denied | PASS |
| `vibe check --path . --quiet` | PASS |
| 1 MiB startup through Cargo's actual target directory | PASS, including external `CARGO_TARGET_DIR`; platform-specific interpreter selection and quoted paths separately checked |
| Full conform | PASS,141 existing/0new, baseline unchanged |
| Committed codegen check | PASS |
| Wire diff | PASS |

The resulting task-related failures were repaired in `38b573822`: local application
execution retains its selected source tree and original acquisition evidence;
classification relinquishes only its own signal registrations; offline authority
refresh can read the verified store; Fresh/remap projection remains source-free
and precedes publication; kind refusals retain their typed error. Positive
specmap/trace/MCP fixtures use the existing canonical publisher. Read-only assertions
compare authority bytes/mtime and directory contents; force-count, evidence equality,
SIGINT exit, monorepo single-capture and trace-state assertions remain meaningful.
Every task-related failed target first received a focused PASS. The subsequent
owner-requested full rerun now passes all eight canonical stages, exit0. The
original panel's27 failed targets and hardcoded-path exit127 remain historical
failure evidence, not the outcome of this latest subject.

Residual gate repairs in `40f69fe5e` reconcile actual contracts: the alias test
checks the existing typed short-name refusal and no writes; the exclusions test
executes the coverage owner's empty closed-set rule and rejects a historical
waiver; the schema census separately pins `version_entry.template` (added by
`4e2860880`) while retaining the historical104-site assertion. The current totals
account for that exact additional scalar. No excluded targets, restored waiver
list or baseline rewrite was used to obtain the green run.

The stack helper now uses Cargo metadata's target directory. After the full Linux
panel, its Windows/Unix interpreter choice was checked separately with synthetic
branches, a target path containing spaces and a1024KiB stack assertion, plus the
actual external-target binary. This is not a Windows runtime smoke claim.

Core cross-checks pass for Windows GNU, macOS ARM64 and Linux musl. These are
compilation checks only, not foreign runtime smoke tests.

## Explicit limits, not silently disabled modes

- Linux hermetic CLI, argv transport and existing hosted PTY are measured.
  An actual isolated `vibe term` attempt refused because the ordinary resolver
  could not find vibeterm; Xvfb exists, but the application runtime is absent.
  Graphical Electron and other-OS runtime smoke are therefore not established.
  Installed cross targets are not foreign runtime evidence. No packages,
  toolchains or configuration are changed merely to make a validation row green.
- Capture and compose produce inspectable caller materials, not an actual provider
  request or model-compliance proof. A nonexistent session sender is not invented;
  installer lifecycle chat does not gain personal authority.
- Global remap plan-only explicitly refuses without offer adoption. Broader remote
  modes are tested through existing supported owners, not inferred from one local
  registry fixture.
- Relation sources require ordinary addressable grouped owners and exact anchors;
  `~rN` relation sources explicitly refuse. No new self-address grammar or revision
  engine is introduced.
- Autonomous memory extraction/migration/sync, NLP preference inference and the
  rejected retained/native/history-policing framework are owner-excluded scope,
  not a hidden backlog or prerequisite to revive.
- Known canonical baseline lint at `explain.rs:99` must remain honestly reported if
  still present; a scoped allowance is not strict-clean.

Local scope commits/integration follow existing authorization. This status grants
no external push, release, provider call, credential/configuration change or peer
checkout write, and does not constitute owner acceptance.
