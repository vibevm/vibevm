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
