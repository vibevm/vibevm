# UP-ROOT-01 — explicit personal-project initialization and status

Genre: implementation brief. Status: architect contract reviewed with builder;
owner authorized committed delivery on 2026-11-15. Production dispatch remains
gated on exact delivery verification and verified external-worker isolation.
This is not a worker packet.

## Inputs and ownership

Baseline: `b76de1197ecf8616bd392e6bd2f48f2382132c2e` in the two independently
verified clones. Governing contract: PROP-074 `#USER-PROJECT-ROOT-PATH`,
`#USER-PROJECT-INIT`, `#USER-PROJECT-INIT-SAFETY`, `#USER-PROJECT-STATUS` and
`#USER-PROJECT-ROOT-DECISION`, plus the admission, observation, publication/race
and report anchors referenced below. These clauses are not in the baseline commit.
The delivery is the commit containing this revision of the brief and PROP-074;
the architect sends its full hash after commit creation. Verify that exact hash
before integration; never use a moving branch tip as the input identity.
Source checkout: `/fast/git/vibevm-architect`, branch `spec/user-project-root`.
Integration owner and implementation lead: builder. Destination task branch/tree:
builder records the exact binding before dispatch under the owner's authorization.
No worker may start from an uncommitted snapshot or an unrelated dirty base.

Architect authors normative specs and this brief. Builder selects implementation
internals, writes bounded sanitized packets, owns workers, tests and integration.
Builder checkout is the only currently assigned implementation tree; any additional
worker tree requires builder's explicit isolation/binding. No shared peer target.

## Goal and non-goals

Provide `vibe user init` and `vibe user status` as explicit personal-project
operations, usable outside any repository, through existing settings and ordinary
project machinery. No installation, network, launcher, Git initialization or
implicit creation during reads. The contract owns semantics; this brief orders
implementation and verification, not an alternative normative copy.

Do not change install/update/uninstall/list/check -g routing, existing MCP root,
application/remap behavior, boot delivery, prefix composition, override bindings,
memory migration, or publication. Do not implement later-phase flags as placeholders.

## Allowed writes by purpose

- `crates/vibe-core/src/settings.rs` or an adjacent focused core module: pure
  personal root derivation from an injected settings path, using current authority.
- `crates/vibe-workspace/src/` focused personal-project initialization/status seam:
  ordinary Manifest parsing/admission and reusable safe filesystem ownership.
  Keep logic in a library, not solely CLI; inspect existing initialization owners
  before choosing the file. Do not create a second manifest/lock/compiler system.
- `crates/vibe-cli/src/cli/`, `cli.rs`, `commands/user*`, `commands/mod.rs`,
  `main.rs`, `command_progress.rs` and its command inventory/tests: minimal user
  namespace, init/status dispatch and normal progress/output integration.
- Relevant library `lib.rs` module exports for the new focused seam.
- Corresponding focused unit/CLI tests and implementation-facing help/examples.
- If required by the actual normal output contract: relevant schema/format registry
  and generated consumers, through existing generator only. Name exact formats
  before touching them; do not invent an unreviewed machine wire schema.

Not allowed: root instructions/rules, PROP/FEAT contracts, private corp inputs,
portable worktree, other campaigns, installed vibedeps, CI/secrets/toolchains,
publication or Git metadata by workers. If an overlap or necessary extra owner is
found, return a precise perimeter adjustment to architect before dispatch.

## Implementation checks before dispatch

1. Confirm exact input commit/dirty exclusions and chosen task branch.
2. Inspect ordinary init/manifest validation owners; report reusable seam and any
   semantic discrepancy before coding. In particular, validate role admission:
   project initialization must not accidentally accept a package-only manifest.
3. Report CLI grammar collision, output/JSON conventions and exact test target names.
4. Establish no-follow path admission, exclusive manifest creation and interruption
   ownership using existing utilities. Do not silently choose clobber/recovery.
5. Satisfy credential-isolation and autoload-suppression gate for the actual external
   execution lane. Synthetic namespace capability alone is insufficient.

## Builder review dispositions

The four root-contract clarifications are now in PROP-074
`#USER-PROJECT-ROOT-ADMISSION`, `#USER-PROJECT-INITIALIZED-CRITERION`,
`#USER-PROJECT-INIT-PUBLICATION` and `#USER-PROJECT-INIT-RACES`.
Use these anchors rather than guessing relative-path, role, directory or race
semantics. Existing project+workspace is admitted without member traversal; root
status is not a whole-project health check. Relative settings are refused only at
this operation's admission, not by changing the global settings resolver.

Do not invoke the existing full CLI init pipeline: it produces lock/ignore/boot
outputs outside this atom. Reuse `Manifest::new_project`, `parse_str` and
`require_project`; do not substitute `Manifest::read`'s neighboring SYSTEM checks.
Inspect retained `vibe-safefs` capabilities and no-replace publication; the existing
MCP prepare helper alone does not establish ancestor safety. Its unrelated repair
is not in scope. Builder suggested focused target `cli_user_project_root`, adjacent
`cli_init` role tests, core settings tests and workspace focused tests. These are
selection candidates until actual test definitions and commands are confirmed.

Machine report and grammar are defined by PROP-074 `#USER-PROJECT-ROOT-REPORT`,
`#USER-PROJECT-ROOT-REPORT-EFFECTS`, `#USER-PROJECT-ROOT-REPORT-TRANSPORT` and
`#USER-PROJECT-ROOT-GRAMMAR`. The approved schema destination is
`schemas/user_project_report.jtd.json`, format `cli-user-project-report`, protocol
`vibe-user-project/1`; generated consumers use existing codegen. Extend
`crates/vibe-cli/src/output/structured.rs` and focused tests for the optional typed
`user_project` error observation. The library returns domain outcomes; CLI maps
these to the wire type without making the library depend on CLI output machinery.
Allow narrowly scoped cancellation-publication plumbing and tests in
`composition/outcome.rs` only if required to preserve the existing terminal contract;
no new cancellation schema or duplicate terminal report is authorized.

Effects use a three-state enum, not the proposed ambiguous changed boolean.
Distinguish pre-existing residue from this invocation's effects, and preserve
restored versus never-started publication. Add positive, refusal, partial-write,
restored, unknown-outcome and cancellation-before/after-publication envelope tests.
Verify missing/relative settings root=null, nonexistent absolute root=string,
context stamping and exact stdout/stderr placement. No runtime pass is claimed.

## Acceptance matrix

- Absent root and existing empty directory initialize; second init leaves bytes and
  mtimes unchanged. Existing authored valid project data is preserved.
- Nonproject cwd is supported; explicit settings override determines root. Missing
  settings/home produces an actionable failure, never cwd fallback.
- Malformed/unsupported manifest, package-only/nonproject manifest, nonempty foreign
  directory, file root and symlink/reparse aliases refuse without altering inputs.
- Concurrent initialization never overwrites; interruption/cancellation does not
  delete pre-existing data or claim ownership of another initializer's files.
- Status on absent root succeeds as absent and creates nothing. Invalid/unavailable
  state is explicit with nonzero exit. Initialized status does not compile/install.
- No changes to `<settings>/mcp/user-project`, application/remap stores, invoking
  repository, harness configuration or Git state. No network/harness invocation.
- No fabricated lock/boot outputs or SYSTEM template; ordinary project parsing
  accepts the created manifest and authored-tree layout.
- Test isolated settings/HOME, synthetic fixtures and sentinel before/after trees;
  never use real user state. Include existing adjacent command/manifest consumers.

Run focused targets with expected nonzero selected test counts and ordinary
negative/invariant cases. Choose exact commands after inspecting test organization;
record results, failures and unrun tests. Documentation changes do not trigger a
workspace-wide test run by themselves. No real harness test is needed for this atom.

## Delivery and safe stop

Return exact implementation base/delivered commit hashes when commits are authorized,
branch/root, diff, tests/results, output samples and outstanding decisions. No worker
commit or push. Builder review precedes architect contract review; neither is owner
final acceptance. A root-only accepted atom does not claim global personal installs,
late boot, memory or the complete USER-PROJECT feature exists.

Safe stop: reviewed source/tests and an honest candidate report, or an explicit
input/security/contract blocker with existing work preserved. Do not widen into
routing or implement around a blocked external lane in violation of delegation.
