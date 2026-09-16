# VibeVM next implementation campaign

The shared contract is [NEXT-IMPLEMENTATION-CAMPAIGN.xml](../../vibevm/vibespecs/terraforms/NEXT-IMPLEMENTATION-CAMPAIGN.xml). This is an implementation plan, not implemented product behavior. The owner's current instruction is planning only: Phase 0 and implementation are unstarted. NEXT-EXECUTION-AUTHORITY must remain blocked until a later explicit instruction starts execution; a generated GOAL is not authorization.

Pending research: [fractal planning, lowering and staged delivery](../../vibevm/vibespecs/design/fractal-planning-and-staged-delivery.xml) preserves the owner's concept and this discussion. It is a non-normative design idea, not an activated exception to existing rules or a change to this campaign's task graph.

The baseline is refined plan revision 4 at commit b1291b06d5704ee32f7486c86e1fae617fcbbddf. Its 17 milestones and 86 work packages are preserved as 201 smaller implementation tasks. The preview-transition addition contributes four supplemental groups and 11 tasks, for 212 tasks and 425 plan nodes. The original sixty XML units and 229 clauses remain intact; NEXT-A26 adds twelve clauses, for 61 units and 241 clauses to promote. Matching counts alone does not establish coverage: the checker compares stable ID sets and effective dependency edges. The historical planning baseline is not automatically the actual DP1 capture.

## Start in an exact local context

Read the project's common boot, then resolve the multi-user-planning context bound to this repository/worktree/branch. A new contributor initializes once from plan.seed.toml; binding.toml records the actual canonical roots/Git common directory/revision, and settings.toml selects goal_node = "NEXT". Personal state belongs under that user's ~/.vibe/steward/contexts/<id>/, never in this repository.

The seed is a portable initialization template. Do not overwrite an existing adaptive plan with it. Merge deliberate plan changes by stable IDs and preserve accepted/candidate evidence. The installed protocol and renderer live in vibevm/vibedeps/org.vibevm.world.multi-user-planning/1.0.0/.

Run these separately from the selected checkout, replacing <context> with its resolved local directory:

    python -B campaigns/next/campaign.py check --context <context>
    python -B campaigns/next/campaign.py frontier --context <context>
    python -B campaigns/next/campaign.py parallel-frontier --context <context>
    python -B campaigns/next/campaign.py task NEXT-P0.1 --context <context>

The no-context check validates only the shared seed. Frontier/task also validate complete local coverage, so a shortened UI or damaged local plan cannot silently hide work. Task output names the detailed JSON contract and active XML sources; after consumption it follows validated permanent targets.

For a fresh empty context, copy the seed to plan.toml only after creating the correct binding/settings. Refresh local GOAL.md and GOAL-CLAUDE.txt with the installed scripts/render_goal.py --context <context>. The renderer's --check diagnoses freshness; it does not execute product gates. Current selected terminal campaigns cannot be rendered: at real completion, mark the plan truthfully and report that limitation instead of keeping a fake active campaign.

## Execute small tasks, not whole milestone numbers

Read the complete task contract under tasks/. Each task has concrete inputs, a closed outer write perimeter, ordered steps, positive/negative cases, real check commands, expected results, a safe stop and a proposed commit subject. Existing module/registration/dependency files are included where a new service or module must become reachable.

A TO CREATE check is a required test family to implement, not a command that exists today. SELECT BEFORE DISPATCH recipes must be resolved to actual package/build targets and affected cases before execution. Inspect source/metadata first; list only the selected target when needed, since listing also compiles it. A test-name filter without target selection can still build unrelated executables. Require the intended nonzero selection; skipped platforms, producer PASS and matching counters are insufficient evidence.

One work package may need another workstream's early group before its own parent finishes. Follow the DAG: M-16-A sits between M-13-A/B; M-16-B precedes M-14-B/C; M-11-D is pulled forward; M-15-K bootstraps Linux before payload/backend work. Do not try to finish M-13 or M-16 in numerical isolation.

M-13-D.2 requires the accepted exact per-file ownership/version census before dispatch. M-17-A.2 requires finite handler-family batches from the accepted M-16 census; convert its local node to a group and register those additional task files in manifest.json refinements before execution. Each refinement has id equal to its parent task and tasks_file pointing to a JSON group with sequential .1… children. The checker retains the original denominator and validates the added contracts.

The core task metadata is the canonical recipe. Shared XML stages product laws; permanent PROP/FEAT anchors are created before code/tests cite them. Final retirement is a separate evidence gate, not a reason to defer permanent source contracts.

## Parallel execution on the existing graph

`parallel-frontier` uses the installed multi-user-planning `steward-parallel` analyzer. It reports dependency-ready leaves, recorded active work, candidates awaiting review, read/write overlaps, unknown perimeters and proposed compatible batches. It does not launch jobs, reserve resources, accept results or change dependencies. Its `dispatch_authorized: false` is deliberate: a proposed batch still needs actual input, semantic, live-job and resource checks.

Before dispatch, bind exact worker files inside each task's existing outer perimeter, the accepted contract/input capture, selected tests, shared resources and one integration owner. Shared `lib.rs`, manifests, schema registrations and generator outputs must have an assigned writer. Read/write conflicts matter even when output files differ. An isolated worktree retains candidate changes but does not prove their compatibility; retain compatible build artifacts and keep heavy command concurrency separate from worker count.

Optional `--bindings <local-json>` refines the same task IDs with narrower read/write paths and resource demands. The input uses schema 1, plan id/revision/raw SHA and per-task entries; see the installed skill for its exact fields. It is an ephemeral analysis input, not a second plan or a repository worker queue. Missing resources and external documentation roots remain unknown. Current file/contract identity, actual running jobs and physical resource capacity are still checked by the coordinator.

Use parallelism between existing ready tasks or bounded independent worker packets inside one ready task. All parent acceptance obligations remain. Preparation and review can overlap independent production on captured inputs, but cannot bypass a prerequisite. Keep candidate production within review capacity; do not add a reviewer/tester trio or duplicate full suites for every small task.

This addition preserves the 425-node graph and all 212 task contracts byte-for-byte. It does not remove the `.1` to `.2` sequencing rule, change Phase 0 ordering or move `NEXT-PREVIEW-DOCS.2` ahead of its 17 workstream dependencies. The incoming documentation will be bound at its existing input gate; then only affected packets, topic maps and evidence need adaptation.

## Change accounting, migration and documentation

[preview-control.json](preview-control.json) binds the cross-task obligations in [NEXT-A26.xml](../../vibevm/vibespecs/terraforms/next/contracts/NEXT-A26.xml). These are planning artifacts. Their pending-capture/input fields describe what authoring has not measured; actual release state belongs in the future permanent transition index.

| Step | Result | When it can run |
|---|---|---|
| NEXT-PREVIEW-BASE.1 | Exact DP1 product/package/source capture | After Phase 0, before any surviving implementation |
| NEXT-PREVIEW-BASE.2/.3 | Registered record format, tools and contribution checks | Establish the accounting bootstrap; all original M-tasks wait for BASE.3 |
| NEXT-PREVIEW-BIND | Adopt the same records in native change roles and link migrations/capabilities | After the named M-02/M-13/M-16 prerequisites |
| NEXT-PREVIEW-DOCS.1 | Bind the independently supplied documentation | As soon as the input gate and BASE are ready |
| NEXT-PREVIEW-DOCS.2/.3 | Apply and verify documentation changes | After all 17 workstreams |
| NEXT-PREVIEW-RELEASE | Reconcile the full delta, review guidance and rehearse migration | After implementation, binding and documentation proof |
| NEXT-CLOSE.3 | Seal final DP2 and retain permanent evidence | After all campaign scaffolding is removed |

Every accepted contribution must name permanent change records or separate reviewed reasons for no user impact and no documentation impact. Record before/after behavior, affected users/interfaces/platforms, exact evidence, migration actions and documentation topics. Include unplanned fixes, generated surfaces, reverts and supersession. At workstream and release gates compare the actual commit range and interface/capability census with the records; one row per task is insufficient.

The future permanent homes are configured-spec-root `changes/<immutable-change-id>/release-impact.json` and `docs/releases/developer-preview-1-to-2/`. The latter retains `transition.json`, the reviewed `MIGRATION.md`, `documentation-map.json` and evidence. Existing format break notes and M-13 migration descriptors remain authoritative. The early records do not require M-02's parser; that parser later adopts their existing identities and non-normative payloads.

The owner has not supplied the independent documentation yet. `NEXT-PREVIEW-DOCS-INPUT` binds its exact source/revision and allowed scope when delivered; that input may be accepted during planning without starting the campaign. Pending input blocks only its dependents. Update documentation by comparing the supplied baseline, current authored documentation and accepted product changes; include new topics and preserve concurrent edits. Future checker stages distinguish intake, edited documentation and verified claims so an early success cannot imply later proof.

No migration tool, release capture or updated documentation is claimed by this planning change. The supplemental tasks explicitly mark prospective commands TO CREATE. Final DP2 includes updated product/documentation and scaffold removal; a separate evidence/index attestation names that verified commit without self-referential hashes. The guide, recipes, records and proof remain usable after `campaigns/next/` is deleted.

## Authority and resource boundaries

D-022 remains open. Generic architecture proceeds; Arch backend implementation waits for the owner's A/B ruling. B requires additional compatibility decomposition.

Native smoke targets, Docker and isolated external bridge checkouts have explicit resource gates. All new external bridge writes use bound isolated checkouts, not the live sibling repositories. CI installation waits for its concrete prepared authorization. M-12-C is demand-optional; M-12-D.2 and its public-approval horizon are deferred actual execution under D-015. Publication implementation, isolated tests and exact ready-to-approve evidence remain required. Pending publication is never reported as published; an owner may reactivate its stable task before campaign close, or its retained prepared action moves into a permanent operator workflow.

The Linux egress proof uses a dedicated internal-network controller/BuildKit/guard fixture. It covers all product/native/build paths exercised inside that boundary. It does not turn Linux evidence into native Windows coverage or alter host firewall/DNS settings.

## Validation and evidence

Authoring checks:

    python -B campaigns/next/campaign.py check
    python -B -m unittest discover -s campaigns/next -p test_campaign.py
    git diff --check

[verification-policy.json](verification-policy.json) binds the owner's affected-test rule. Each task packet requires a verification selection covering changed behavior, negative cases, invariants and named contract consumers, with actual target/filter/flags and relevant input identity. The task's permitted write perimeter alone does not trigger codegen, wire, exhaustive-facts or whole-project checks. Unknown test names are unresolved bindings, never permission to run the whole crate.

The ordinary full product panel is reserved for NEXT-CLOSE.3, using tools/self-check.sh through Git Bash on Windows. Phase 0 and the 17 workstream reviews use sufficient affected checks and matching prior evidence. Additional broad verification needs an explicit full-check request or demonstrated cross-cutting impact with no sufficient narrower proof. A commit, milestone, push or worker completion is not that reason. Reuse compatible build artifacts; a warm build is not a passed test.

After a panel failure, run failed/invalidated steps and the never-run tail; retain successful steps only for the same complete relevant subjects. Account for the complete required denominator and report combined evidence honestly. The script has no resume flag and invoking it still runs its full panel. No product panel was run during this planning correction.

campaign.py is read-only. It checks task/schema/coverage shape, exact frozen inputs, effective scheduling cycles, accepted-parent consistency and promotion witnesses. It neither accepts work nor runs commands, writes plans, issues permissions or deletes files. Semantic review and real gate execution remain central responsibilities.

## Consume the campaign

promotion.toml starts with every clause pending. A promotion records real permanent anchor(s), content hashes, independently retained evidence, an acceptance commit and semantic review. Witnesses must exist both at that commit and in the current accepted tree. Mere anchor existence, a matching count or an unrelated ancestral commit is insufficient.

After the required workstream gates and semantic promotion:

    python -B campaigns/next/campaign.py retirement-check --context <context> --unit NEXT-A02

This returns only structural eligibility. Inspect tracked and nonignored new permanent inputs, explicitly remove the exact unit, mark it retired and run affected permanent checks. The next packet reads the permanent replacements. Never create production code/spec/build dependencies on temporary NEXT anchors or task paths.

At final close, promote all unique laws, rationale, predictions, findings, evidence and named horizons. Remove the entire campaigns/next/ zone and the corresponding terraforms/next/ contract directory plus master XML. Then run permanent source/reference/codegen/conform/full-product and decisive end-to-end checks with those paths absent. The final proof cannot require this checker, seed, baseline copies or README.

The surviving result is ordinary VibeVM specifications, tests, code and user/operator documentation. Git may retain the historical execution record; it must not be the only home of a product law.
