---
name: fractality-delegate
description: Delegate a task to a fractality worker (or a swarm) — guided packet authoring, spawn, observation, collection, and the boss review loop. Use when a task scores delegable on the delegation-rules matrix, when the user asks to "delegate / offload / swarm" work, or when several disjoint mechanical tasks could run in parallel workers.
---

# fractality-delegate

Drive one delegation end to end. The law: **delegate when verification
is cheaper than generation** — if the task fails the matrix
(`flow:org.vibevm.fractality/delegation-rules`), stop here and do it
yourself.

## 1. Route

Score the task: error cost / context transferability / verifiability /
size (matrix §axes, §verdict) — the calculus is a verb:

```sh
fractality route --error-cost reversible --context boot-loadable \
    --verify mechanical --size L      # → delegate · slot=big · scenario=2
```

Survivors route: small × mechanical →
`model = "small"`; bigger → `model = "big"` as a coarse one-shot. Pick
scenario 1 (compile exact files, APIs, patterns, commands into the
goal) or scenario 2 (order the worker to read named corpus files
first). Choosing neither is banned.

## 2. Author the packet

Start from `spec/examples/hello-glm.toml`. The goal must be
self-contained (the worker sees nothing else): exact paths, exact
patterns, non-goals, and — always — acceptance commands that PROVE the
deliverable:

```toml
schema = 1
[task]
title = "short-name"
goal = """<work order: context, steps, exact APIs, self-verify>"""
acceptance = ["cargo test -p <crate>"]
[workspace]
mode = "worktree"          # code → branch deliverable; dir → file artifacts
repo = "."
base = "main"
[budget]
wall_secs = 1800           # playbook defaults per model
max_turns = 40
[routing]
profile = "glm"
model = "big"              # or "small" per the matrix
```

## 3. Fire and observe (never blind)

- One task, synchronous: `fractality run --packet task.toml` — parks
  print a loud notice; exit codes are semantic (0/1/2/3/4).
- Several disjoint tasks: `id=$(fractality spawn --packet …)` each,
  then `fractality wait $A $B $C`; watch with `ps` / `tree` / `show`.
- If a worker parks: `fractality questions`, then
  `fractality answer <id> "<decision>"`.

## 4. Review (your half of the bargain)

1. Read the run summary: state, usage, cost, result provenance
   (`worker` beats `extracted`), acceptance verdicts.
2. Read the diff (worktree mode: the `fractality/<id>` branch) as a
   contributor PR — does it do the work order, nothing else?
3. Run the project's own gates; green gates + read diff = merge;
   anything else → one bounded retry (small → big) or reclaim.
4. Record surprises in the producing model's playbook. The session's
   delegation counters accumulate on the bus automatically
   (`fractality scoreboard` shows them); keep the "kept: why" half in
   your session notes — the scoreboard can only measure what you DID.
