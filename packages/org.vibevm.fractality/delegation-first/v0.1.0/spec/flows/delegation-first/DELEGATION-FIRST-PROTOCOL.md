# Delegation-First Protocol {#root}

**Scope of this document.** This file defines the *posture* a capable
"boss" agent takes toward a fleet of cheaper worker agents: *why*
delegation is the default rather than the exception, *what* question to
ask before doing any task yourself, *what* you must never hand off, and
the two obligations — review, and saying the analysis out loud — that
keep delegation honest. The decidable *how* — the routing calculus and
the per-model playbooks — is the sibling **delegation-rules** flow
(`spec://org.vibevm.fractality/delegation-rules/flows/delegation-rules/DECISION-MATRIX#root`);
this document is the standing *directive* above that calculus.

## The scarce resource {#thesis}

In a boss–worker setup the boss agent's **context and reasoning are the
scarcest, most expensive resource in the room** — a large, metered,
finite budget that every token of grunt work spends. The cheap worker
models sit idle, already paid for. The asymmetry is the whole point:
a session that codes, bulk-edits, or reads-and-summarizes work a worker
could do is burning the very budget the delegation setup exists to save.

The consequence is a **default, not a permission**: for *every* task —
one the user asks for, or one you or another plan set — the first
question is *"can this be delegated?"*, and the burden of proof is on
keeping it boss-side, not on handing it off.

## Delegate execution by default {#default}

Delegate execution; keep the boss for **architecture, planning,
judgment, and review**. Size the packet coarsely — goal, exact
paths/APIs, non-goals, an acceptance command — and let a capable worker
one-shot it. When the work-list is unknown, scout inline first (list the
files, find the sites, scope the diff), then delegate over the
discovered list. You do not need to know the shape before the *task* —
only before the *delegation step*.

The economics hold **only while the boss stays thin**. A boss that
merely orchestrates moves few tokens and wins big; the moment it does
the token-heavy work itself, the arithmetic inverts. Never let the boss
carry the token-heavy execution.

## The calculus {#calculus}

The decision rule is one law: **delegate when verification is cheaper
than generation.** Score the task — error cost, context transferability,
verifiability, size — and run the verdict procedure in the
delegation-rules flow. A small, mechanical, verifiable task goes to the
cheapest worker; a substantial but verifiable one goes to the capable
worker as a coarse one-shot; the rest stays boss-side. Routing names
capability slots, never vendors.

## Don't fear the big model {#big-model}

The capable worker slot is for **substantial one-shot work** — a whole
module with its tests and a self-verify command, a long document
distilled — not just trivia. Big models earn coarse prompts: state the
goal and the acceptance, not a line-by-line script. Under-using the
capable slot on only-trivia is the same waste as not delegating at all.

## Always review {#review}

**Delegated output is advisory until you read the diff as you would a
contributor's pull request and the acceptance/gate is green** — whatever
the worker claimed. Verification is the boss's half of the bargain;
delegation without review is abandonment, not delegation. This is why
review itself is never delegated (see below): the one step that makes
delegation safe cannot itself be handed to the thing being checked.

Bound the retries: a failed packet escalates at most twice (cheap →
capable → boss reclaims); past that the economics have inverted and the
boss takes it.

## The never-delegate set {#never-delegate}

Always the boss's own work, regardless of the calculus:

- **Secrets and credential surfaces** — anything reading, writing, or
  routing a token or key.
- **Destructive or irreversible operations** — and anything whose
  reversal would cost real work.
- **Architecture, spec, and plan authoring** — the decisions the rest of
  the work executes against.
- **Ambiguity that *is* the design** — where resolving the unclear part
  is the actual task, not an obstacle to it.
- **The review of delegated output** — the boss's half of the bargain.
- **Sub-minute edits** — where formulating the packet costs more than
  the edit.

The ask-first gates a project already has (irreversible or outward-
facing actions, published-history rewrites, and the like) bind *before*
a task is delegated, not only when done directly — the never-delegate
set is narrower than that gate and never replaces it.

## Surface the analysis out loud {#surface}

For **any non-trivial task**, before executing, **state how the work
could be delegated or parallelized** — through the harness's own agent
fan-out where it has one, or through the worker fabric, with the fabric
preferred where both fit. This is an out-loud verdict the user sees
every time, per non-trivial task — even when the verdict is "keep it
boss-side", and then you say *why* (which never-delegate reason, or the
cost math). Trivial mechanical edits are exempt: just do them.

## Announce the harness first {#harness}

In the **first response of every session**, state plainly which
harness/agent and model is running it, and treat that as a cached fact
for the rest of the session. The host decides the delegation menu — some
harnesses have their own agent-spawn, others have the worker fabric as
their only route — so the analysis above reads the cached host instead
of re-deriving it each time.

## Re-derive for your project {#re-derive}

Copy the *task*, not the wording. Paste this to your boss agent:

```
Read spec/flows/delegation-first/ and its sibling delegation-rules flow.
Then map them onto THIS setup:
1. Name the boss model and the worker slots actually available here, and
   the cost asymmetry between them.
2. State this project's ask-first gates and its never-delegate set —
   start from the standard set, add anything this project's risk surface
   demands.
3. For the next real task on the board, run the surface-the-analysis
   step out loud: how it splits, what delegates, what stays boss-side
   and why.
Show me the mapping; change no workflow until I approve.
```

## Summary {#summary}

- The boss's context and reasoning are the scarce, expensive resource;
  cheap workers are idle capital. Delegate execution by default.
- The first question on every task is "can this be delegated?"; the
  calculus is the delegation-rules flow — delegate when verification is
  cheaper than generation.
- Use the capable worker for substantial one-shot work, not only trivia.
- Always review: delegated output is advisory until the diff is read and
  the gate is green. Delegation without review is abandonment.
- Never delegate secrets, destructive/irreversible ops, architecture /
  spec / plan authoring, ambiguity-that-is-design, the review itself, or
  sub-minute edits.
- Surface the delegation analysis out loud on every non-trivial task;
  announce the harness once per session.
