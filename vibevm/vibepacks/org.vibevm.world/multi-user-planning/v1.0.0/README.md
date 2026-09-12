# `flow:multi-user-planning` — campaign stewardship without a shared personal WAL {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-PURPOSE This flow lets several contributors, branches and central
coding-agent sessions work around one repository without treating one person's
active plan as project truth.  Personal execution state lives under
`~/.vibe/steward/`; the repository keeps only deliberately shared contracts,
normative facts, evidence and collision-free records. @status:impl/done

@fact:CASUAL-CONTINUATION-DEFAULT A timeout, crash, compaction, new chat or
model/account change, quota exhaustion or provider overload is ordinary
continuation, not a handoff. Any owner-facing central
agent may resolve the exact binding, read settings and the complete plan,
verify the repository tree and continue under the owner's current instruction.
It need not create or receive a session, handoff, receipt or takeover document.
Custody metadata is advisory in this path. @status:impl/done

@fact:CENTRAL-HANDOFF-PURPOSE When the owner explicitly requests a formal
central transfer, a session may hand the same local context to another model or
harness on the same machine. The incoming central agent verifies the tree,
reads the whole route, receipts the transfer, and independently accepts every
candidate it intends to land. This opt-in protocol is not required for ordinary
continuation. @status:impl/done

@fact:HANDOFF-BY-CONTEXT In an explicitly requested formal handoff, a human
never pastes the handoff body into a new session. The outgoing procedure creates
`HANDOFF.md` inside the local context; the incoming command `ACCEPT HANDOFF FROM
&lt;context-id&gt;` / `ПРИМИ ХЭНДОФФ ИЗ &lt;context-id&gt;` resolves and reads it. After
receipt, the agent prints the refreshed detailed goal and the exact bounded
`/goal …` command for the human. @status:impl/done

@fact:HANDOFF-CREATION-PRINTS-LOCATOR After sealing `HANDOFF CENTRAL TO
&lt;target&gt;`, the outgoing agent always prints the context id, handoff id and
exact receive command on screen. The user never has to discover a UUID in
`~/.vibe` manually. @status:impl/done

@fact:NOT-A-CODING-AGENT The flow does not implement a coding agent or prescribe
one vendor's workflow.  It supplies lifecycle-neutral planning, custody and
evidence mechanics that any capable agent or human coordinator can use.
@status:spec/done

## What ships {#contents}

- @fact:CONTENT-BOOT A short central-session boot contract at
  `vibevm/vibespecs/boot/12-flow-multi-user-planning.xml`. @status:impl/done
- @fact:CONTENT-PROTOCOL The authority, roles and multi-contributor laws in
  `MULTI-USER-PLANNING-PROTOCOL.xml`. @status:impl/done
- @fact:CONTENT-LOCAL-STORE The user-local layout, context selection, persistent
  `ultra`/`standard` planning profile and independent `auto`/`collab`
  interaction mode in `user-local-state.xml`. @status:impl/done
- @fact:CONTENT-PLAN The lossless hierarchical plan and receding-horizon
  planning discipline in `plan-hierarchy.xml`. @status:impl/done
- @fact:CONTENT-GOAL The deterministic, user-local goal projection in
  `goal-projection.xml`: one selected campaign node, its governing mandates,
  accepted boundary, candidates, complete remaining route and closure proof;
  no model memory or LLM call. The same snapshot emits a bounded
  `GOAL-CLAUDE.txt` containing the exact `/goal …` command a human pastes into
  Claude Code. @status:impl/done
- @fact:CONTENT-EXECUTION The central coordinator's long-campaign execution,
  review, gate-economy, recovery and wisdom-promotion discipline in
  `campaign-execution.xml`, including essential-first verification and the
  suspicion-triggered bounded mutation policy. @status:impl/done
- @fact:CONTENT-VERIFICATION Target-first affected checks, bounded scope
  escalation, compatible evidence reuse and fail-fast panel resumption in
  `verification-selection.xml`. @status:impl/done
- @fact:CONTENT-RECOVERY Bounded provider retries, ambiguous-effect recovery,
  live-worker reconciliation and durable decision summaries in
  `interruption-recovery.xml`. It uses existing plan/evidence surfaces and
  introduces no required state files. @status:impl/done
- @fact:CONTENT-HANDOFF Advisory custody diagnostics, concurrent-writer safety
  and the explicit formal offer/receipt protocol in
  `custody-and-handoff.xml`. @status:impl/done
- @fact:CONTENT-COLLAB The roles, contribution records, acceptance tiers and
  project-fact promotion law in `collaboration-and-acceptance.xml`.
  @status:impl/done
- @fact:CONTENT-MIGRATION The composition and migration rules for projects
  leaving `wal`/`wal-specspaces`, including the redbook exclusion recipe, in
  `migration-and-composition.xml`. @status:impl/done
- @fact:CONTENT-SKILLS Three optional agent skills: status-only orientation,
  explicit same-machine formal handoff, and deterministic goal refresh. They are not
  `vibe.exe` commands.
  @status:impl/done

## Authority boundary {#authority}

@fact:LOCAL-STATE-IS-NOT-PROJECT-TRUTH `~/.vibe/steward/` is authoritative only
for this developer's central-session continuity.  It cannot grant repository
permissions, redefine product behaviour or overrule the human, specifications,
tests or code. @status:spec/done

@fact:CUSTODY-IS-ADVISORY `custody.toml` records observations useful for
coordination; `vacant`, `held`, stale timestamps and mismatched holder/session
ids do not grant or deny authority. A valid `offering` created by an explicit
owner-requested formal handoff is the sole protocol write fence. An observed
live conflicting writer stops affected writes. Changed input invalidates its
snapshot: recapture expected own writes and reconcile understood changes;
escalate unresolved semantic or writer conflicts. Stale metadata alone does not
establish a conflict. @status:spec/done

@fact:REPO-FACTS-REMAIN-SHARED Stable project knowledge still belongs in the
repository: product specifications, tests, code, accepted decision rationale,
campaign mandates deliberately shared by an integrator, and immutable evidence
records with unique names. @status:spec/done

## Composition {#composition}

@fact:WAL-FAMILY-INCOMPATIBLE `flow:org.vibevm.world/wal` and
`flow:org.vibevm.world/wal-specspaces` are alternative single-writer continuity
owners. Do not let them and this flow govern the same personal context at once.
Package presence alone is not a global conflict: another explicitly scoped
project/context can use a different continuity owner. Their freshness, cold-resume
and target-scoping lessons are incorporated here without their shared mutable
`WAL.xml`/`CONTINUE.md` storage model. @status:spec/done

@fact:CAMPAIGN-PLANS-COMPATIBLE `flow:org.vibevm.world/campaign-plans` remains
compatible when its repository plan is treated as an integrator-owned shared
campaign contract, never as every contributor's personal cursor.  Contributors'
adaptive execution plans remain local. @status:spec/done

@fact:REDBOOK-ONE-IS-IMMUTABLE Retired immutable-roster rule. The current rule
is REDBOOK-COMPOSITION-MUTABLE below. @status:spec/void

@fact:REDBOOK-COMPOSITION-MUTABLE The owner may change the roster of any
Redbook version. Before an agent overwrites an existing version's roster, it
offers publishing a new version as an option; an already explicit owner choice
does not need repeating. The current prototype remains 1.0.0. Selection between
continuity alternatives belongs to Redbook's composition group, not a global
package incompatibility. This package update does not change the Redbook roster
or implement its planned variant selector. @status:spec/done

## Efficient continuation {#execution}

@fact:EXECUTION-WITHOUT-MIGRATION Keep the complete schema-1 plan, stable task
ids and accepted evidence. During authorized execution, choose a useful outcome
from existing tasks, finish viable candidates, then dispatch only ready packets
with enough context to implement and verify them. A warm coordinator reuses its
complete-plan understanding after checking revision and input changes; a cold
session reads the whole plan. Verification can be reused only for the same
subject and actual covered obligations. Mutation is a bounded diagnostic for
concrete suspicion, never a routine completion tax. @status:spec/done

@fact:AFFECTED-VERIFICATION-DEFAULT Select the build/test target first, then
exact affected cases or a small family and named contract consumers. Include
negative cases and invariants; zero selected tests is not a pass. Atom/commit
completion, public surfaces and milestone labels do not trigger a full panel.
Widen only for a named coverage gap, an explicit full-panel request or the
designated comprehensive final campaign gate. Reuse compatible build artifacts
and valid per-step evidence; resume failed or invalidated steps and the unrun
tail. Keep the complete gate denominator and identify aggregate proof honestly.
Existing overbroad recipes may be narrowed with a recorded equivalent
claim-to-proof mapping, without changing the plan format. @status:spec/done

@fact:RECOVERY-WITHOUT-REPLAY A model interruption may lose reasoning while
commands or workers continue. Inspect their status, diffs and receipts first.
Retain concise decisions, evidence and the next check at useful boundaries;
never claim to restore unrecorded reasoning. Transient retries obey server hints
and a bounded episode that survives session/account changes. Exhausted quota
needs a real availability change; this flow neither switches credentials nor
promises background wakeups. Unknown external effects must be reconciled before
repeating an operation. Planning-only instructions and execution holds survive
all these transitions. @status:spec/done

## No product CLI pollution {#surface}

@fact:NO-VIBE-SUBCOMMAND This package adds no `vibe` subcommand and no runtime
dependency to products built with VibeVM.  A future optional companion may be
installed as the separate tool `vibe-steward` under `~/.vibe/bin`; its job is to
automate this protocol, not to change it. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See [LICENSE.md](LICENSE.md). @status:impl/done
