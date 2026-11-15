# USER-PROJECT — personal specification projects and layered boot

_Status: EXECUTING architecture preparation, 2026-11-15. Baseline commit
`b76de1197ecf8616bd392e6bd2f48f2382132c2e`. No product implementation accepted.
Cold continuation: read governing contracts and actual state; this plan is not
permission to dispatch unresolved work or bypass worker isolation._

## 1. Mandate and ownership

Owner, 2026-11-15: "Начинай разработку следующего этапа (USER-PROJECT0".
Earlier same-session direction: "ты помнишь, что реальное кодирование делает агент
vibevm-builder?". Governing scope is
`spec://org.vibevm.core/vibevm/common/PROP-074-user-project`.
Architect owns contracts/briefs/architectural review; builder leads coding workers,
checks and implementation integration. Current collaboration policy is
`WORKING-RULES.project.md`. No external source/package publication is authorized.

## 2. Baseline and coverage arithmetic

At baseline: 0 accepted USER-PROJECT implementation atoms in this campaign.
Eight delivery obligations remain: personal lifecycle/routing; ordered global boot;
local composition/privacy; bindings/override authority; memory lifecycle;
AI-native reference isolation; all tree surfaces/controls; final integrated proof.
The first brief implements only safe root initialization/status, not all lifecycle.
No baseline test pass is inferred: implementation tests and full gate are unrun.

Verified locally: kind prefix validates declared kind (`package_ref.rs:420`), not
spec/application destination; application resolution requires `[application]`
(`vibe-app/src/resolve.rs:392`), without a kind==App test. Existing settings authority
is `vibe-core/src/settings.rs:43`; MCP root is
`vibe-cli/src/commands/install/global_mcp.rs:34`. Workspace staging excludes only
direct prompt artifacts (`publish/staging.rs:292`); direct publisher copy excludes
Git metadata only (`vibe-publish/src/git_publish/payload.rs:10`); layout Git-tree
input bypasses initial copy (`layout_publish.rs:340`).

Builder's bounded static report additionally identifies missing list/check -g,
application-only update --all, and classic router lacking personal discovery.
Treat these as reviewed report inputs, not runtime acceptance. Existing portable
work and collaboration-rule edits are excluded from implementation ownership.

## 3. Decisions and unresolved boundaries

Personal-root decisions are authoritative in PROP-074
`#USER-PROJECT-ROOT-PATH`, `#USER-PROJECT-INIT`, `#USER-PROJECT-INIT-SAFETY`,
`#USER-PROJECT-STATUS` and `#USER-PROJECT-ROOT-DECISION`, not duplicated here.
Their current status is spec/plan, not implemented.

Before dispatching dependent atoms, close these explicit refinement nodes:
- Routing: capability/operation/destination matrix; dual-form packages; no-argument
  and --all behavior; preserve existing MCP/app/remap meanings; one acquired
  manifest/version across preflight and execution.
- Ordered delivery: distinguish persisted P-only composition from the complete
  request-local stream. PROP-076's existing precedence-versus-placement allowance
  does not silently waive PROP-074's late-user order. Do not freeze continuation.
- Classic discovery: no SYSTEM opt-in required, no commands during boot, no personal
  path in shared output. Existing code offers no file-only descriptor for arbitrary
  relocated settings. Select an explicit bootstrap/discovery contract before code.
- Authority/binding: typed source targets, protected rules, opaque binding ids and
  explicit aliases; do not implement NLP conflict detection or Git-remote guessing.
- Memory: first authorized write through next-session read, explicit failure and
  durable opt-out. Boot remains read-only.

## 4. Phase 0 and execution gate

Read-only code probes are complete for initial routing/export/boot owners; no
Phase-0 commits or real harness calls. Builder reports synthetic bwrap filesystem,
process and network capability checks passed, but actual external launcher auth
mediation and instruction-load suppression remain NOT VERIFIED. That is a hard
external-worker gate, not evidence that the lane is safe. Use synthetic evidence,
never real secret reads or security weakening. Builder retains gate ownership.

## 5. Phases and dependencies

1. UP-ROOT-01: explicit personal root/init/status; brief `briefs/UP-ROOT-01.md`.
   Exit: bounded initialization and read-only status matrix, no global routing
   changes. Proposed future commit: `feat(user): isolate personal project initialization`.
2. UP-ROUTING: complete operation/destination contract then lifecycle implementation.
   Depends on root atom and routing decision. Exit: nonproject-cwd lifecycle and
   compatibility matrix. Proposed commit family: `feat(user): route personal operations explicitly`.
3. UP-BOOT: origin-qualified ordered materials and classic/launch/API delivery.
   Depends on lifecycle and discovery/order decisions. Exit: actual sequence tests
   including interleaved gated rows, no-template classic path, fresh observations.
4. UP-PRIVACY: local composition transaction/freshness and exporter admission.
   Before any personal artifact ships: reserved subtree exclusion before traversal,
   pinned-input rejection without altering identity, staged/final payload canaries.
5. UP-OVERRIDE: binding aliases, independent resolution, typed authority and toggles.
   Depends on boot/privacy. Exit: worktree/reclone ambiguity and protected-rule tests.
6. UP-MEMORY and UP-CHECKERS: separate independently accepted atoms. Memory depends
   on boot; checker isolation depends on origin/authority. Canonical package edits
   only, not installed dependency copies. Exit: record-to-next-boot and shared/local
   reference/coverage matrices respectively.
7. UP-TREE: layer-qualified plain/JSON then console/vibeterm. Depends on override
   model. Exit: hide/disabled/unavailable/effective separation; next-launch effect.
8. UP-FINAL: all required consumers, comprehensive gate and owner acceptance.
   Depends on all preceding obligations. No publication implied.

Only the first brief has a bounded write perimeter. Later phase commit sets and
commands are refined before dispatch, not guessed as runnable today.

## 6. Predictions

P1: root/status can ship without modifying global install/update/uninstall dispatch.
P2: existing ordinary project/manifest owners suffice; no second parser/compiler.
P3: layered complete delivery requires a richer ordered runtime material carrier;
unchanged project-prefix bytes alone do not establish correct ordering.
P4: workspace staging alone is insufficient to prove personal-data export exclusion.
P5: no-template classic discovery requires an explicit new runtime/file contract.
Final report records confirmed/falsified/unknown for each, with evidence.

## 7. Risks and fallback

- Unsafe worker auth: no external launch; retain local design and synthetic probing.
- Unresolved discovery: hold dependent boot atom, continue independent bounded work.
- Dirty integration base: preserve it; no automatic stash/reset or unrelated commits.
- Input drift: rebind/review exact inputs; never silently follow moving branches.
- Insufficient test coverage: widen affected cases, record the gap; do not claim pass.

## 8. Non-goals and deferrals

No working-rules package, automatic personal sync, automatic harness-memory import,
private corporate content, portable repair, unrelated harness repair, service-root
migration or publication. These are outside this mandate or separate owner work.
All explicit PROP-074 obligations remain in scope; none is silently deferred.

## 9. Cold start and verification

```sh
git --no-optional-locks rev-parse HEAD
git --no-optional-locks status --short --branch
```

Read the actual task brief and applicable collaboration rules. Bind builder test
commands to the real diff and target names before implementation. Root-atom cases
are listed in its brief. Preserve the project's designated comprehensive final
campaign gate; builder supplies an executable whole-campaign acceptance script
before final integration, covering all eight delivery obligations. It is not yet
written or run; no placeholder command is described as an executed gate.

## 10. Review points

No permission to relax protected authority, memory default, classic support,
credential isolation or publication boundaries. Escalate any necessary change to
those requirements. Unresolved mechanical choices remain explicit refinement work,
not owner acceptance inferred from this plan.

## 11. Execution ledger

2026-11-15: campaign opened; exact architect local context created without custody
claim or importing builder plan. Initial root contract drafted; first brief sent
for builder feasibility/review. Product implementation and tests not yet accepted.

2026-11-15, builder feasibility review: full CLI init overproduces artifacts; use
ordinary manifest constructors/parser and retained safe-filesystem primitives
instead. Four clarified contract anchors now cover absolute personal-root admission,
manifest-only initialized criterion (project+workspace admitted), directory-before-
manifest publication and explicit concurrent/incomplete outcomes. Required CLI
progress/inventory/module-export wiring added to the brief. Machine-report format
was subsequently defined in PROP-074: shared typed observation, three-state owned
effects (none/changed/unknown), existing generic stderr failure attachment and
existing cancellation envelope retained. Owner authorized branch/commit/local delivery on 2026-11-15. The first delivery
is the commit containing this plan revision, the root brief and PROP-074 amendment;
its full hash is reported after creation. Verified external execution lane remains
open; implementation is not dispatched until that gate and exact-input review pass.

## 12. Report

Open. No campaign-completion or release claim.
