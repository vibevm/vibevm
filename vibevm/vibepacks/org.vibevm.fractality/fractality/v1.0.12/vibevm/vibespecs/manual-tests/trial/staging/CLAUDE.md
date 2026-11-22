# mini_logfmt — project notes

A small internal logfmt utility crate. Definition of done for any
change: `cargo test` passes.

# Tool: fractality

This project has the **fractality delegation fabric** installed: an
expensive boss agent (you) can hand tasks to cheap worker agents —
isolated Claude Code processes under other providers — supervised by
pods under a mission-control daemon. The `fractality` binary is on
PATH and the daemon is running.

## When a task smells delegable

Before doing bulk, mechanical, or read-and-summarize work yourself,
consult the delegation matrix: **delegate when verification is
cheaper than generation.** The matrix is executable — score the task
and read the verdict:

```sh
fractality route --error-cost reversible --context compilable \
    --verify mechanical --size M        # exit 0 delegate · 1 keep
```

Typical delegable shapes: templated test suites, bounded refactors
and sweeps, fixture/boilerplate generation, module drafts against
exact APIs, doc extraction and first-draft summaries. The
never-delegate set (secrets, irreversible ops, architecture/spec
authoring, ambiguity-as-design, review of delegated output, tiny
edits) stays with you — always.

## Your scoreboard

`fractality scoreboard` shows measured facts for your session and the
fabric: delegations, outcomes, parked questions with ages (`--line`
is the one-line strip).

## One command per shape

```sh
fractality run --packet task.toml         # sync: spawn, wait, one-screen summary
id=$(fractality spawn --packet task.toml) # async: returns the run id at once
fractality wait "$id"                     # shell-wait semantics, semantic exit codes
fractality ps / show <id> / tree [<id>]   # observe (add --json for machines)
fractality questions / answer <id> "..."  # triage parked workers
fractality kill <id> [--tree]             # stop a run (or its whole call tree)
```

## The packet is the work order

One TOML file: self-contained `task.goal` (the worker sees nothing
else — compile exact files, APIs, patterns, and self-verify commands
in), `task.acceptance` commands (run in the workspace, verdicts
recorded), `workspace.mode` (`dir` for artifacts, `worktree` for
code — the branch is the deliverable), `budget`, `routing`:

```toml
schema = 1
[task]
title = "short-name"
goal = """<the work order>"""
acceptance = ["cargo test"]
[workspace]
mode = "worktree"
repo = "."
base = "main"
[budget]
wall_secs = 900
max_turns = 40
[routing]
profile = "glm"
model = "big"              # or "small" for bounded mechanical work
```

## Never

- Never delegate the never-delegate set.
- Never skip the review loop: a worker's output is advisory until the
  diff is read and acceptance/gates are green — verification is your
  half of the bargain.
- Never hand a worker secrets or ambient credentials.
