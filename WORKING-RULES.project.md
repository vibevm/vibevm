# Working arrangement policy — vibevm

Genre: project session instructions. Read with `WORKING-RULES.md`; this is the
project-owned configuration, not part of the portable protocol. Owner directives
recorded 2026-11-15. They apply to this project's explicitly bound architect/builder
arrangement only and do not authorize another project merely by being copied.
The local binding identifies participating checkouts and sessions. Ordinary
contributors without that binding remain in ordinary mode.

## Roles and integration

The builder is the coding team's lead and implementation integration owner. The
architect owns architecture, specifications, briefs and architectural review,
including the design/legal responsibilities in the protocol, not coding-worker
dispatch. The human owner retains final acceptance and approval-required decisions.
The integration branch is `main`; task briefs record actual task branches and full
base hashes. Do not displace another active mainline maintainer.

The builder owns implementation execution and any delegated workers, including
packets, isolation, scheduling, independent checks and rework. Parallel workers
with disjoint assignments are permitted. Available launchers, provider/model
choices, cost assumptions and local delegation requirements belong only in the
untracked local binding; this shared policy requires no private execution service.
Every worker/supervisor has an explicit task packet and remains non-central.
Credential isolation in the portable protocol is mandatory before any external
launch, including research.

Roles and lead profiles are defined in the working-rules protocol,
`vibevm/vibedeps/org.vibevm.world.working-rules/1.0.12/rules/protocol.md`, "Roles and
profiles". The local binding records the role and profile of each session and
lane. When the builder is an executor, the architect holds implementation
leadership, integration and worker dispatch for as long as that binding stands;
the role sentences above describe the two-lead case.

### Worker context configuration

Both bounded context and full admitted project boot are supported under the
portable protocol's safety constraints. The local binding may select the mode by
task complexity for its approved execution lanes; absent a local selection rule,
use bounded context. Record the selected mode and its reason in each packet.
Full-boot packets must not carry `##subagent-quiet-clause`, which suppresses boot;
bounded packets retain that marker and verified automatic-boot suppression.

Full context never grants central custody, host stewardship access or broader
writes. Verify the admitted boot closure and exclude unrelated host instructions,
private inputs and credentials. If required context cannot be supplied safely,
report the blocker rather than silently omitting it or weakening isolation.

## Standing grants retained from the owner's instructions

- **Local-rule synchronization:** either architect or builder may initiate it;
  the recipient may synchronize in their own checkout without renewed approval
  for each routine transfer. Compare current copies, preserve unrelated edits,
  verify identical bindings and untracked/excluded status. This is the explicit
  local-configuration exception to committed product transfers.
- **Repository synchronization:** either may initiate routine fetch, review and
  integration of already authorized work, including preparation before push,
  without renewed approval for each synchronization. Bind exact source/base hashes,
  destination branch and scope. Routine synchronization branches are permitted
  when needed; no peer writes or silent task-binding changes are permitted.
- **Ideal synchronization:** the owner may invoke the shared operation in either
  session at any time, including by saying "идеальная синхронизация". Such a request
  authorizes necessary routine local commits of reviewed, completed in-scope work,
  transfer and integration. Both repositories must reach the agreed Git/functional
  result and account for dirty work under the portable protocol. It does not by
  itself authorize external push.
- **Emergency preservation:** an explicit owner request to commit and push unfinished
  work authorizes checkpoint commits of that work and the expressly requested source
  push, not acceptance or release. Preserve divergent candidates rather than force
  integration; retain privacy and other non-routine gates. If the permitted remote
  checkpoint destination is unresolved, report remote preservation as blocked, not
  accomplished. Emergency is not automatically inferred from ordinary sync.

These grants preserve, rather than expand, the existing mandates. They do not allow
agents to create new product scope, waive safety, change harness permissions or
credential configuration, repair Git metadata, rewrite history, destroy work or
make legally consequential commitments. Harness permission boundaries remain binding.
Loading these documents does not invoke any granted operation.

## Routine task branches

Owner authorization, 2026-11-15: ordinary temporary and task branches may be
created in the session's own repository without another confirmation, now and in
future work. Verify the current checkout, base and dirty state; preserve existing
work and record the actual task binding. This grant does not authorize switching
or modifying a peer's branch, overwriting refs, deleting work, history rewrites,
metadata repair or external publication. A harness bypass/yolo mode is not itself
project authority and does not remove safety requirements.

## Trial use and continuous improvement

Owner directive, 2026-11-15: exercise this arrangement on real project work. The
general rules are now the package `org.vibevm.world/working-rules`, installed as a
dependency of this project; this policy and the local binding stay project-owned
files at the project root. The architect may improve these collaboration rules
when observed friction, ambiguity or failure demonstrates a need, and coordinate
the changes with the builder without seeking approval for each routine refinement.
Keep general mechanisms in the protocol, project grants in this policy and machine
facts in the local binding. Preserve existing mandates, role ownership and safety
floors; changes to owner-only decisions still require explicit approval.

For each refinement, identify the observed problem, explain the change and check
that it preserves existing authority and constraints. Changes to the protocol and
the templates are made in the package source,
`vibevm/vibepacks/org.vibevm.world/working-rules/v1.0.12/`, and reach each checkout
by reinstalling dependencies; the installed copies under `vibevm/vibedeps/` are
never edited by hand. Send the exact file changes and content hashes to the
builder for review and synchronization; each session writes only in its own
checkout and preserves unrelated edits. The owner authorizes this bounded
rules-maintenance procedure for the package source, project policy and local
binding, including matching uncommitted rule revisions. This is an explicit configuration-only exception to committed
product delivery, not permission to transfer dirty product code or to publish.
Verify equality after synchronization and retain the normal untracked/excluded
status of the local binding. Do not claim synchronization before confirmation.

Keep concise evidence of material rule changes and subsequent real use in existing
work reports or commit history; do not create a second process registry. Elapsed
time alone is not stability: look for a sustained period of actual collaboration
without substantive rule changes, recurring workarounds, unresolved process defects
or safety failures. No arbitrary duration or number of sessions is prescribed.
When that evidence supports freezing the protocol, present it to the owner for a
joint decision to establish a stable baseline of the package. Publishing the
package still requires owner authorization; this instruction is not permission to
publish it.

## External publication

Owner standing source-push authorization, 2026-11-19: publish the builder's
implemented work at substantial reviewed task boundaries to BOTH configured
GitHub and GitVerse source mirrors. Bind the exact integrated commit range and
approved rollout ref, inspect publication contents and relevant gate results, and
use PROP-016 and the existing mirror procedure fast-forward-only. Do not publish
every micro-edit, unrelated dirty work or unreviewed intermediate candidates.
Architect coordinates the boundary and review; each session operates only in its
assigned checkout. This grant authorizes routine source rollout without asking
again at each such boundary, not history rewrites, signing/credential changes,
secret publication, package publication, deployments or a public Vibe release.
If the current task branch is not an admitted rollout ref, resolve integration
through the existing mainline protocol; never silently push a different branch,
rewrite mainline or broaden the mirror target set.

External push, mirror rollout, package publication, release, deployment and shared
service changes require explicit owner authorization for that step. Older generic
routine-push grants do not create a second publisher in this pair.

Authorized source rollout follows `vibevm/vibespecs/common/PROP-016-source-mirrors.xml`
and the project's existing mirror procedure (`cargo xtask mirror`, target manifest
`mirrors.toml`), not independent direct pushes to individual replicas. This policy
adds no remote target or checkpoint-ref support. Before an emergency remote
checkpoint, verify the destination is supported and authorized; do not silently
publish WIP on main or bypass the protocol. Package publication, release and deploy
are separate operations, not consequences of source synchronization.

## Managed execution resources

`vibevm/vibedeps/org.vibevm.world.working-rules/1.0.12/rules/runs.md` is the
authoritative provider-neutral operational contract for run ownership,
retention, dialogue closure and cleanup. Read it before designing,
reviewing or applying managed-run cleanup. The local binding carries machine-specific
execution/recovery pointers; private runbooks hold commands, receipts and active run
state. Neither a runbook nor worker success overrides this contract. Its current
implementation status is not a claim that cleanup has been verified or performed.

## Verification and durable work

Owner clarification, 2026-11-19: during NST global repair, a key release1.0.12
product lens is use of VibeVM as the package-management backend of a separate
Steam-like graphical client. Prioritize complete client-driven user journeys
through existing specified capabilities and shared owners; do not assume terminal
interaction is the only adapter. The external application is a different,
currently inaccessible project. This is a specification-review filter, not a
new GUI project, guessed external contract or permission to expand/reopen accepted
task scope. Canonical campaign detail:
`campaigns/next-semantic-transfer-2026-11/NST-AI-NATIVE-GLOBAL-REPAIR.md`.

Owner directive, 2026-11-18: maintain
`campaigns/next-semantic-transfer-2026-11/NST-PHYSICAL-ACCEPTANCE.md` as NST advances.
Update affected user journeys, physical/manual proof boundaries, actual results
and owner exceptions with the corresponding delivery; do not turn the guide into
a frozen checklist, duplicate accepted unchanged tests or expand hypothetical
permutations. Architect owns the guide; builder supplies behavior changes and
execution evidence. Keep its remaining claims consistent with the authoritative
campaign plan and derived task tree.

Owner directive, 2026-11-18: whenever campaign plans change, update the existing
architecture task-tree projection through its configured authoritative source plan
and installed refresh procedure, and verify the resulting snapshot contains the
changed nodes, dependencies and acceptance requirements. Updating an architect's
private plan or a campaign brief alone is not dashboard publication. Coordinate
with the bound source-plan owner rather than editing a peer's personal plan,
rebinding the dashboard or hand-editing generated HTML/JSON/JavaScript. The local
installation's README and refresh configuration identify its source and renderer;
keep machine-specific paths out of this project policy. Report a pending or failed
refresh honestly instead of claiming the tree is current.

Preserve existing project verification contracts and designated comprehensive gates.
Use focused affected tests, negative/invariant cases and named consumers for each
implementation atom; report executed, failed and unrun checks accurately. Do not
weaken a gate or mark functionality complete because a worker reported success.
New and modified normative specifications are written in English.

### Architectural quality of delegated implementation

Owner directive, 2026-11-19: delegated code may come from a substantially less
capable implementation model. Passing tests and formally working behavior are
necessary evidence, not sufficient acceptance of its design. The builder retains
architectural responsibility; a worker is an executor, not the design authority.

Before integrating a worker result, the builder performs a proportionate baseline
architectural review against the implementation brief, intended design and Vibe's
existing standards. Check responsibility and ownership boundaries, use of existing
mechanisms, dependency direction, state/lifecycle and error handling, duplication,
workaround shims, and unnecessary abstraction or complexity. Do not accept a design
merely because it compiles, satisfies a narrow test or was confidently explained
by the worker. Independently inspect the actual diff and affected integration.

If the design is unsatisfactory, return concrete findings and the intended design
to the executor for bounded rework through the approved lane; rerun affected checks
and review the revised result. Do not silently normalize design debt as accepted
implementation or bypass the execution lane by labeling product edits as review.
A genuine architectural ambiguity returns to the architect; ordinary correction
to an already decided design does not require another planning approval cycle.

At feature/milestone transfer, the builder includes its architectural disposition,
material design deviations and rework, and unresolved risks in the existing review
packet. The architect independently reviews the integrated design with awareness
of the executor's limitations, not just its functional evidence or builder verdict.
Blocking design findings require remediation even when tests are green. This is
not a demand for speculative redesign or endless polishing: findings must name an
actual design/contract defect or consequential maintainability problem and a finite
remedy. Reuse valid evidence after bounded fixes; keep the complete product journey
and existing campaign stopping conditions in view.

### Cross-cutting architectural review at substantial milestones

Owner directive, 2026-11-15: before declaring a substantial milestone or nested
subcampaign closed, the builder submits its integrated result to the architect for
a cross-cutting review. This includes meaningful milestones within a larger
campaign, not just the final campaign delivery. Identify these boundaries in the
existing campaign route; this is not a review of every small edit or worker packet.

The review packet binds exact base and integrated candidate commit hashes and scope,
the disposition of constituent atoms, independent checks with results and unrun
gates, and coverage of the original owner mandate. It identifies interactions across
interfaces, compatibility, security/privacy, cancellation and formats, plus deferred
debt and open questions. Do not substitute a collection of isolated patch reviews
for inspection of the integrated result, or present missing evidence as a pass.

The architect records a verdict and findings against that exact candidate in the
existing campaign review/report surface. Blocking findings return to the builder
for remediation through the locally approved implementation lane; the builder
retains worker dispatch, tests and integration ownership. Re-review affected claims
and cross-cutting consequences after changes, reusing unchanged valid evidence
rather than restarting an unbounded review loop. Worker success and atom-level
checks do not replace milestone review. Architectural clearance is review evidence,
not the owner's final acceptance, permission to publish or permission to delete run
resources. Until the required review and blocking remediation are complete, report
the milestone as a candidate or review-pending, not closed. This requirement does
not change lightweight-wrapper context restrictions or mandate a full test panel
on every review; existing verification-selection and final-gate contracts apply.

Use existing project campaign/brief/report locations. Keep personal execution state
under the exact checkout's user-local stewardship context as root instructions
require; copying repository history does not import the peer's plan or custody.
Do not store project decisions only in personal harness memory.

This separation of protocol, policy and machine binding preserves the owner's
current permissions. The protocol and templates ship as the package
`org.vibevm.world/working-rules`; publishing it requires owner authorization.
