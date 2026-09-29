---
name: steward-parallel
description: Inspect an existing campaign for parallel-ready tasks, path and resource conflicts, and review backlog. Use when choosing a parallel work window or diagnosing contention; produces advisory JSON without dispatching jobs or changing the plan.
---

<status stage="impl" state="done"/>

# Steward parallel {#root}

@fact:PARALLEL-SKILL-SCOPE Analyze the current plan without changing its format,
ids, dependencies or acceptance. The helper is read-only and grants no execution
authority. A coordinator resolves the exact current context and owner scope by
the ordinary stewardship rules; reuse a verified warm context. A worker uses
only its packet's named inputs and never discovers or claims personal central
state. @status:impl/done

@fact:PARALLEL-SKILL-LAW Read
[parallel-execution.xml](../../flows/multi-user-planning/parallel-execution.xml)
when selecting or reviewing a parallel window. The coordinator still owns
dispatch judgment and acceptance. Parallelize ready independent tasks or bounded
disjoint subwork inside one ready task; do not remove prerequisites. Review and
preparation can overlap production on separately identified stable subjects.
@status:impl/done

@fact:PARALLEL-SKILL-RUN Run the packaged Python 3.11+ helper with the actual
plan and existing task descriptions; repeat `--task-file` as needed:
@status:impl/done

```text
python -B scripts/parallel_frontier.py --plan <plan.toml> --repo <repository-root> --task-file <task-group.json>
```

@fact:PARALLEL-SKILL-BINDINGS Optional `--bindings <bindings.json>` supplies
schema 1, `plan_id`, `plan_revision`, `plan_sha256` of the raw plan TOML and
`tasks` entries with `id`, `read_paths`, `write_paths`, and `resources` entries
with `id` and `mode` (`exclusive` or `shared`). Narrow the task's existing outer
paths; never use bindings to expand authorization. This local disposable input
is neither a second plan nor a repository index, job queue, live reservation or
proof of source freshness. The helper accepts repository-contained paths;
external documentation requires separately bound coordination. Read `--help`
for the current command interface. @status:impl/done

@fact:PARALLEL-SKILL-INTERPRET Inspect `ready`, `active`, `review_queue`,
`blocked`, `conflicts` and `proposed_batches` in the JSON. Missing paths or
resource bindings remain unknown. `policy_state` is `requires-binding` and
`dispatch_authorized` is always `false`, even when explicit perimeters are
compatible. Before an authorized dispatch, verify actual source snapshots,
live writers/jobs, semantic independence, current resource capacity and one
integration owner. The helper does not acquire locks or measure resources.
Resolve gaps under existing authority without asking again merely because the
tool is advisory. @status:impl/done

@fact:PARALLEL-SKILL-NEXT Return the useful candidate window, conflicts and
missing facts, including any need to judge existing candidates before adding
producers. A planning/status request ends with that analysis. During authorized
execution the coordinator can issue exact packets after resolving those facts;
this helper itself starts nothing. Select economical checks under
[verification-selection.xml](../../flows/multi-user-planning/verification-selection.xml).
After interruption, reconcile surviving jobs and retry episodes under
[interruption-recovery.xml](../../flows/multi-user-planning/interruption-recovery.xml)
before replacing a worker. @status:impl/done
