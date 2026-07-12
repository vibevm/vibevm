# Source Mirrors Protocol {#root}

**Scope of this document.** This file defines *the problem* that
multi-homing a git source across several hosts creates, *the model*
that dissolves it (a single-writer mainline with every host as a
downstream read-replica), *what the model buys*, *what it costs*, and
*how to re-derive* the practice for your own project. The mechanics of
the fan-out live in [`fanout-mechanics.md`](fanout-mechanics.md); the
maintainer's day-to-day loop in [`daily-loop.md`](daily-loop.md).

## The problem {#problem}

Suppose the same project must live on two git hosts at once — call
them **host A** and **host B**. The reasons are real: one audience
reaches host A and another reaches host B; one host may disappear and
the history must survive; a jurisdiction, a mirror-of-record, or a
review community lives on each. Both must always carry the same
history, and both are canonical *for reading*.

The naive way to keep two writable repositories in step is to let each
accept writes and mirror to the other. That is **multi-master
replication**, and it has one failure mode that never goes away: two
independent writes to the same branch **diverge**, and then something
must merge them or one must be force-overwritten. Every added host
multiplies the race. The moment a divergence appears, a human is doing
conflict archaeology on published history — the most expensive kind.

## The model {#model}

The fix is structural, not operational. Adopt the **hub-and-spoke /
benevolent-dictator** shape (the Linux-kernel workflow): make mainline
**single-writer**, and demote every host to a downstream replica.

### Mainline is one local tree; no host is primary {#mainline}

Mainline is the maintainer's integrated local `main`. It has **no
primary host** — it is not "the host A copy" or "the host B copy"; it
is what the maintainer has blessed, replicated equally to every host.
Because exactly one writer advances mainline, and does so **serially**,
two divergent writes to `main` cannot race. The multi-master problem is
**absent by construction, not patched after the fact.** The cost of
"both repos canonical" is paid once, in the model, not continuously, in
conflict resolution.

### Every host is a downstream read-replica {#replicas}

Each host is canonical for *reading* in its audience, and a replica for
*writing* — nobody writes a target directly. A direct write to a
target, or a force-push, makes it **diverge** from mainline. The model
does not paper over that: the tooling detects it and **fails loud**
rather than reconciling silently. A diverged target is a signal to
investigate, never something to clobber.

### Contributions arrive anywhere; the maintainer integrates {#integration}

A change reaches mainline only by the maintainer integrating it.
Proposals arrive however is convenient — a web PR on host A, a web PR
on host B, a branch on a fork, an emailed patch — and are reviewed
where they land. Accepting one means bringing its commits into local
mainline, then fanning out.

| Surface | Role |
|---------|------|
| A host's web PR UI | **Inbox** and review surface |
| The maintainer's local `main` | **Merge authority** |
| Every host after fan-out | **Read-replica** of that authority |

The web PR UIs are *inboxes and review surfaces, not the merge
authority*. This is exactly the kernel's "patches by email, integrated
in the maintainer's tree, pushed to a hub that mirrors out" — the web
UIs are merely nicer inboxes than a mailing list.

## What the model buys {#buys}

- **Divergence is impossible by construction.** One serial writer means
  no two writes to `main` can race. There is no reconciliation step to
  get wrong because there is no concurrent write to reconcile.
- **Any host can vanish without data loss.** Every host holds the full
  history; mainline holds it too. A host going dark, getting blocked,
  or deleting the repo costs a line in the manifest, not a commit.
- **Audience and jurisdiction per host.** Each host serves its own
  region, community, or compliance surface, while all serve identical
  history. Adding a host is one manifest entry.
- **The invariant is runnable capital.** "Never `--force`" is not a
  prose promise — it is pinned by a test over the push command
  ([`fanout-mechanics.md` §never-force-test](fanout-mechanics.md#never-force-test)).

## What the model costs {#costs}

Be honest about the bottleneck: **one human serializes every merge.**
Mainline advances only as fast as the maintainer integrates. There is
no parallel write path — that is the whole point, and it is also the
whole cost.

| Property | Multi-master | Single-writer mainline |
|----------|--------------|------------------------|
| Concurrent writes | Allowed, and they race | Serialized through one tree |
| Divergence | Possible; must be reconciled | Impossible by construction |
| Merge throughput | Many writers | One writer (the bottleneck) |
| Failure surface | Reconcile published history | A loud abort before any harm |

For a **small-team or single-maintainer project**, the trade is
strongly positive: integration is not the throughput limit (review and
design are), and the maintainer was serializing the important merges
anyway. The model just makes that serialization the *only* write path,
so nothing can sneak around it and diverge. When a project outgrows one
integrator — several full-time committers merging in parallel all day —
this model is the wrong tool, and the honest answer is to add
one-directional server-side mirroring or move to a shared-forge
workflow. Record that as a revisit trigger, not a someday-maybe.

## Re-derive for your project {#re-derive}

Do not copy the host names or the script verbatim — copy the *task*,
and let the agent derive the setup your project actually needs:

```
Read spec/flows/source-mirrors/ in full, then adapt it to this project:
1. List every git host this source must live on, and for each: is it a
   push target (we push to it) or self-mirroring (it pulls itself)?
2. Name the single mainline — the one local tree that is the merge
   authority. Confirm no host is treated as primary today; if one is,
   say so.
3. Draft the target manifest: one entry per host (name, url, mode,
   refs). No credentials in it — those stay in the maintainer's keys.
4. Adapt the reference fan-out script to that manifest, keeping it
   fast-forward-only with NO --force path, and add the invariant check
   (a test or a CI grep) that proves --force can never be emitted.
5. Show me the manifest and the script as diffs. Apply nothing until I
   approve, and never push to a host as part of this exercise.
```

## Summary {#summary}

- Multi-homing across hosts invites multi-master divergence; this model
  dissolves it instead of managing it.
- One mainline, single-writer, no primary host. Every host is a
  downstream read-replica. Contributions arrive on any host as inboxes;
  the maintainer's tree is the merge authority.
- What it buys: divergence impossible by construction, any host can
  vanish without data loss, audience/jurisdiction per host.
- What it costs: one human serializes merges. Acceptable — and cheaper
  than the alternative — for small-team projects; record a revisit
  trigger for the day it is not.
- The never-`--force` invariant is runnable capital, not a promise.
