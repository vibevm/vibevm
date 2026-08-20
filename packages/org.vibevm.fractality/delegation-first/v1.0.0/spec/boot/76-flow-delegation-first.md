# Flow: Delegation-First {#root}

The boss agent's context and reasoning are the scarcest, most expensive
resource in the room; the cheap worker slots sit idle, already paid for.
So **delegate execution by default** — for every non-trivial task the
first question is *"can this be delegated?"*. Keep the boss for
architecture, planning, judgment, and **review**; hand a worker the
token-heavy execution as a coarse, well-specified one-shot.

Full directive:
[`spec/flows/delegation-first/DELEGATION-FIRST-PROTOCOL.md`](../flows/delegation-first/DELEGATION-FIRST-PROTOCOL.md).
The decidable calculus it sits above — *delegate when verification is
cheaper than generation* — is the delegation-rules flow.

## Two standing obligations {#obligations}

- **Always review.** Delegated output is advisory until you read the
  diff like a contributor's pull request and the acceptance/gate is
  green. Delegation without review is abandonment.
- **Surface the analysis out loud.** On every non-trivial task, before
  executing, say how it could be delegated or parallelized — even when
  the verdict is "keep it boss-side", and then say why. Announce the
  harness/model once per session, and read it as a cached fact after.

## Route to the cheap slot, not the native tool {#route}

"Delegate" means the cheap `big` worker slot (GLM-5.2) reached through
**fractality** — it does *not* mean the harness's own sub-agent tool. On a
harness whose native `Agent` / `Task` / `Workflow` spawns **same-model**
workers (on Claude Code, Claude), using it for token-heavy execution offloads
your context window but spends the very scarce, expensive process this
directive exists to conserve — it satisfies the *word* "delegate" and defeats
its *purpose*. That frictionless native tool is the **default failure mode**;
name it and route through fractality instead. A same-model subagent is
justified only when the task's low verifiability would make reviewing a
foreign-model diff cost more than regenerating it (the delegation-rules test,
applied to the choice of worker) — and you say so out loud. Documented worker
friction (a cold build, a blown turn budget, a `failed` exit on complete work)
is a reason to design the packet well (a `check`-only self-verify, a generous
wall clock, reading the worktree even on "failed"), never a reason to fall back
to the native tool.

## Swarm, recursion, and the strong form {#more}

Swarm / fan-out is still **one first-level delegation**: hand the whole
fan-out to fractality and let it distribute the pieces by its own internal
rules — do not pre-split or manage the sub-workers yourself. **Enable RLM**
(recursive delegation — a worker that itself delegates or escalates) when a
task needs it, via the worker's profile capability. And a project may harden
the whole directive into a **mandatory law**. Full text: the protocol's
`#swarm`, `#recursion`, `#strong-form`.

## Never delegate {#never}

Secrets and credential surfaces; destructive or irreversible operations;
architecture, spec, and plan authoring; ambiguity that IS the design;
the review of delegated output; sub-minute edits. The economics never
justify it, and the project's ask-first gates bind *before* a task is
delegated, not only when it is done directly.
