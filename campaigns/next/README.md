# VibeVM next implementation campaign

The shared contract is [NEXT-IMPLEMENTATION-CAMPAIGN.xml](../../vibevm/vibespecs/terraforms/NEXT-IMPLEMENTATION-CAMPAIGN.xml). This is an implementation plan, not implemented product behavior. The owner's current instruction is planning only: Phase 0 and implementation are unstarted. NEXT-EXECUTION-AUTHORITY must remain blocked until a later explicit instruction starts execution; a generated GOAL is not authorization.

The baseline is refined plan revision 4 at commit b1291b06d5704ee32f7486c86e1fae617fcbbddf. Its 17 milestones and 86 work packages are preserved as 201 smaller implementation tasks. The complete seed has 407 nodes, including the execution hold, probes, gates, promotion/removal tasks and explicit future horizons. Sixty temporary XML units contain 229 source clauses to promote. Matching counts alone does not establish coverage: the checker compares stable ID sets and effective dependency edges.

## Start in an exact local context

Read the project's common boot, then resolve the multi-user-planning context bound to this repository/worktree/branch. A new contributor initializes once from plan.seed.toml; binding.toml records the actual canonical roots/Git common directory/revision, and settings.toml selects goal_node = "NEXT". Personal state belongs under that user's ~/.vibe/steward/contexts/<id>/, never in this repository.

The seed is a portable initialization template. Do not overwrite an existing adaptive plan with it. Merge deliberate plan changes by stable IDs and preserve accepted/candidate evidence. The installed protocol and renderer live in vibevm/vibedeps/org.vibevm.world.multi-user-planning/1.0.0/.

Run these separately from the selected checkout, replacing <context> with its resolved local directory:

    python -B campaigns/next/campaign.py check --context <context>
    python -B campaigns/next/campaign.py frontier --context <context>
    python -B campaigns/next/campaign.py task NEXT-P0.1 --context <context>

The no-context check validates only the shared seed. Frontier/task also validate complete local coverage, so a shortened UI or damaged local plan cannot silently hide work. Task output names the detailed JSON contract and active XML sources; after consumption it follows validated permanent targets.

For a fresh empty context, copy the seed to plan.toml only after creating the correct binding/settings. Refresh local GOAL.md and GOAL-CLAUDE.txt with the installed scripts/render_goal.py --context <context>. The renderer's --check diagnoses freshness; it does not execute product gates. Current selected terminal campaigns cannot be rendered: at real completion, mark the plan truthfully and report that limitation instead of keeping a fake active campaign.

## Execute small tasks, not whole milestone numbers

Read the complete task contract under tasks/. Each task has concrete inputs, a closed outer write perimeter, ordered steps, positive/negative cases, real check commands, expected results, a safe stop and a proposed commit subject. Existing module/registration/dependency files are included where a new service or module must become reachable.

A TO CREATE check is a required test family to implement, not a command that exists today. Run the actual crate checks, inspect the test list and require nonzero exact selection before using a new filter. No zero-test invocation, skipped platform, producer PASS, shell transcript or matching counter is sufficient evidence.

One work package may need another workstream's early group before its own parent finishes. Follow the DAG: M-16-A sits between M-13-A/B; M-16-B precedes M-14-B/C; M-11-D is pulled forward; M-15-K bootstraps Linux before payload/backend work. Do not try to finish M-13 or M-16 in numerical isolation.

M-13-D.2 requires the accepted exact per-file ownership/version census before dispatch. M-17-A.2 requires finite handler-family batches from the accepted M-16 census; convert its local node to a group and register those additional task files in manifest.json refinements before execution. Each refinement has id equal to its parent task and tasks_file pointing to a JSON group with sequential .1… children. The checker retains the original denominator and validates the added contracts.

The core task metadata is the canonical recipe. Shared XML stages product laws; permanent PROP/FEAT anchors are created before code/tests cite them. Final retirement is a separate evidence gate, not a reason to defer permanent source contracts.

## Authority and resource boundaries

D-022 remains open. Generic architecture proceeds; Arch backend implementation waits for the owner's A/B ruling. B requires additional compatibility decomposition.

Native smoke targets, Docker and isolated external bridge checkouts have explicit resource gates. All new external bridge writes use bound isolated checkouts, not the live sibling repositories. CI installation waits for its concrete prepared authorization. M-12-C is demand-optional; M-12-D.2 and its public-approval horizon are deferred actual execution under D-015. Publication implementation, isolated tests and exact ready-to-approve evidence remain required. Pending publication is never reported as published; an owner may reactivate its stable task before campaign close, or its retained prepared action moves into a permanent operator workflow.

The Linux egress proof uses a dedicated internal-network controller/BuildKit/guard fixture. It covers all product/native/build paths exercised inside that boundary. It does not turn Linux evidence into native Windows coverage or alter host firewall/DNS settings.

## Validation and evidence

Authoring checks:

    python -B campaigns/next/campaign.py check
    python -B -m unittest discover -s campaigns/next -p test_campaign.py
    git diff --check

The full product panel is tools/self-check.sh through Git Bash on Windows. It is intentionally not part of authoring validation and is unmeasured until Phase 0. Phase/task execution must record actual successful, failed and unavailable steps; never discard the child exit code.

campaign.py is read-only. It checks task/schema/coverage shape, exact frozen inputs, effective scheduling cycles, accepted-parent consistency and promotion witnesses. It neither accepts work nor runs commands, writes plans, issues permissions or deletes files. Semantic review and real gate execution remain central responsibilities.

## Consume the campaign

promotion.toml starts with every clause pending. A promotion records real permanent anchor(s), content hashes, independently retained evidence, an acceptance commit and semantic review. Witnesses must exist both at that commit and in the current accepted tree. Mere anchor existence, a matching count or an unrelated ancestral commit is insufficient.

After the required workstream gates and semantic promotion:

    python -B campaigns/next/campaign.py retirement-check --context <context> --unit NEXT-A02

This returns only structural eligibility. Inspect tracked and nonignored new permanent inputs, explicitly remove the exact unit, mark it retired and run affected permanent checks. The next packet reads the permanent replacements. Never create production code/spec/build dependencies on temporary NEXT anchors or task paths.

At final close, promote all unique laws, rationale, predictions, findings, evidence and named horizons. Remove the entire campaigns/next/ zone and the corresponding terraforms/next/ contract directory plus master XML. Then run permanent source/reference/codegen/conform/full-product and decisive end-to-end checks with those paths absent. The final proof cannot require this checker, seed, baseline copies or README.

The surviving result is ordinary VibeVM specifications, tests, code and user/operator documentation. Git may retain the historical execution record; it must not be the only home of a product law.
