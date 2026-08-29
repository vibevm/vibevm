# `flow:multi-user-planning` — campaign stewardship without a shared personal WAL {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-PURPOSE This flow lets several contributors, branches and central
coding-agent sessions work around one repository without treating one person's
active plan as project truth.  Personal execution state lives under
`~/.vibe/steward/`; the repository keeps only deliberately shared contracts,
normative facts, evidence and collision-free records. @status:impl/done

@fact:CENTRAL-HANDOFF-PURPOSE A central session may hand the same local context
to another model or harness on the same machine.  Custody transfers; prior
acceptance never does.  The incoming central agent verifies the tree, reads the
whole route, receipts the transfer, and independently accepts every candidate
it intends to land. @status:impl/done

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
- @fact:CONTENT-EXECUTION The central coordinator's long-campaign execution,
  review, gate-economy, recovery and wisdom-promotion discipline in
  `campaign-execution.xml`. @status:impl/done
- @fact:CONTENT-HANDOFF The custody state machine and planned, rollover and
  recovery handoffs in `custody-and-handoff.xml`. @status:impl/done
- @fact:CONTENT-COLLAB The roles, contribution records, acceptance tiers and
  project-fact promotion law in `collaboration-and-acceptance.xml`.
  @status:impl/done
- @fact:CONTENT-MIGRATION The composition and migration rules for projects
  leaving `wal`/`wal-specspaces`, including the redbook exclusion recipe, in
  `migration-and-composition.xml`. @status:impl/done
- @fact:CONTENT-SKILLS Two optional agent skills: status-only orientation and
  same-machine central handoff.  They are not `vibe.exe` commands.
  @status:impl/done

## Authority boundary {#authority}

@fact:LOCAL-STATE-IS-NOT-PROJECT-TRUTH `~/.vibe/steward/` is authoritative only
for this developer's central-session continuity.  It cannot grant repository
permissions, redefine product behaviour or overrule the human, specifications,
tests or code. @status:spec/done

@fact:REPO-FACTS-REMAIN-SHARED Stable project knowledge still belongs in the
repository: product specifications, tests, code, accepted decision rationale,
campaign mandates deliberately shared by an integrator, and immutable evidence
records with unique names. @status:spec/done

## Composition {#composition}

@fact:WAL-FAMILY-INCOMPATIBLE `flow:org.vibevm.world/wal` and
`flow:org.vibevm.world/wal-specspaces` are alternative single-writer continuity
owners.  Do not load either with this flow.  Their useful freshness, cold-resume
and target-scoping lessons are incorporated here without their shared mutable
`WAL.xml`/`CONTINUE.md` storage model. @status:spec/done

@fact:CAMPAIGN-PLANS-COMPATIBLE `flow:org.vibevm.world/campaign-plans` remains
compatible when its repository plan is treated as an integrator-owned shared
campaign contract, never as every contributor's personal cursor.  Contributors'
adaptive execution plans remain local. @status:spec/done

@fact:REDBOOK-ONE-IS-IMMUTABLE `redbook@1.0.0` is an immutable tested edition.
A consumer that dogfoods this flow excludes `wal` and `wal-specspaces` on its
redbook edge and requires this package directly.  A future redbook edition may
make that replacement its tested default; edition 1 is not rewritten.
@status:spec/done

## No product CLI pollution {#surface}

@fact:NO-VIBE-SUBCOMMAND This package adds no `vibe` subcommand and no runtime
dependency to products built with VibeVM.  A future optional companion may be
installed as the separate tool `vibe-steward` under `~/.vibe/bin`; its job is to
automate this protocol, not to change it. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See [LICENSE.md](LICENSE.md). @status:impl/done
