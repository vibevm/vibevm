# Tool: fractality {#root}

This project has the **fractality delegation fabric** installed: an
expensive boss agent (you) can hand tasks to cheap worker agents —
isolated Claude Code processes under other providers — supervised by
pods under a mission-control daemon. Everything below assumes the
`fractality` binary on PATH (or a stated path) and a configured
`~/.fractality/profiles.toml`.

## When a task smells delegable {#trigger}

Before doing bulk, mechanical, or read-and-summarize work yourself,
consult the delegation matrix (`flow:org.vibevm.fractality/
delegation-rules`, boot snippet 77): **delegate when verification is
cheaper than generation.** Typical delegable shapes: templated test
suites, bounded refactors and sweeps, fixture/boilerplate generation,
module drafts against exact APIs, doc extraction and first-draft
summaries. The never-delegate set (secrets, irreversible ops,
architecture/spec authoring, ambiguity-as-design, review of delegated
output, tiny edits) stays with you — always.

## One command per shape {#commands}

```sh
fractality run --packet task.toml         # sync: spawn, wait, one-screen summary
id=$(fractality spawn --packet task.toml) # async: returns the run id at once
fractality wait "$id"                     # shell-wait semantics, semantic exit codes
fractality ps / show <id> / tree [<id>]   # observe (add --json for machines)
fractality questions / answer <id> "..."  # triage parked workers (D18 ask_boss)
fractality kill <id> [--tree]             # stop a run (or its whole call tree)
```

A worker with `Bash` allowed and the injected `FRACTALITY_*` env can
itself call `fractality spawn` — the call tree builds itself and
`kill --tree` can always dismantle it.

## The packet is the work order {#packet}

One TOML file (D7): self-contained `task.goal`, `task.acceptance`
commands (the pod runs them and records verdicts), `workspace.mode`
(worktree for code — the branch is the deliverable), `budget` (wall,
turns, tokens — exceeding kills the run), `routing.profile`/`model`
(`big` | `small` slots; the playbooks pick). Template:
`spec/examples/hello-glm.toml`; authoring loop: the
`fractality-delegate` skill.

## Never {#never}

- Never delegate the never-delegate set (snippet 77 enumerates it).
- Never skip the review loop: a worker's output is advisory until the
  diff is read and acceptance/gates are green — verification is your
  half of the bargain.
- Never hand a worker secrets or ambient credentials — profiles carry
  token *paths*; the clean-slate env (I1) exists so you cannot leak
  yours.
- Never leave a surprise unrecorded — field data feeds the model's
  playbook.
