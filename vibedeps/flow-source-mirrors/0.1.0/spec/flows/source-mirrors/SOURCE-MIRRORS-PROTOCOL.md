# Source Mirrors Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *the problem* that
multi-homing a git source across several hosts creates, *the model*
that dissolves it (a single-writer mainline with every host as a
downstream read-replica), *what the model buys*, *what it costs*, and
*how to re-derive* the practice for your own project. @impl/done

##sibling-document-pointers The mechanics of
the fan-out live in [`fanout-mechanics.md`](fanout-mechanics.md); the
maintainer's day-to-day loop in [`daily-loop.md`](daily-loop.md). @impl/done

## The problem {#problem}

##SUPPOSE-THE-PROJECT-MUST-LIVE-ON-TWO-HOSTS Suppose the same project must live on two git hosts at once — call
them **host A** and **host B**. @spec/done

##the-reasons-are-real The reasons are real: @spec/done

- ##REASON-AUDIENCE-PER-HOST one audience
  reaches host A and another reaches host B; @spec/done
- ##REASON-A-HOST-MAY-DISAPPEAR one host may disappear and
  the history must survive; @spec/done
- ##REASON-JURISDICTION-MIRROR-OR-COMMUNITY-PER-HOST a jurisdiction, a mirror-of-record, or a
  review community lives on each. @spec/done

##BOTH-MUST-CARRY-THE-SAME-HISTORY Both must always carry the same
history, and both are canonical *for reading*. @spec/done

##THE-NAIVE-WAY-IS-TO-LET-EACH-ACCEPT-WRITES The naive way to keep two writable repositories in step is to let each
accept writes and mirror to the other. @spec/done

##THAT-IS-MULTI-MASTER-REPLICATION That is **multi-master
replication**, and it has one failure mode that never goes away: two
independent writes to the same branch **diverge**, and then something
must merge them or one must be force-overwritten. @spec/done

##EVERY-ADDED-HOST-MULTIPLIES-THE-RACE Every added host
multiplies the race. @spec/done

##A-DIVERGENCE-MEANS-CONFLICT-ARCHAEOLOGY-ON-PUBLISHED-HISTORY The moment a divergence appears, a human is doing
conflict archaeology on published history — the most expensive kind. @spec/done

## The model {#model}

##THE-FIX-IS-STRUCTURAL-NOT-OPERATIONAL The fix is structural, not operational. @impl/done

##ADOPT-THE-HUB-AND-SPOKE-SHAPE Adopt the **hub-and-spoke /
benevolent-dictator** shape (the Linux-kernel workflow): make mainline
**single-writer**, and demote every host to a downstream replica. @impl/done

### Mainline is one local tree; no host is primary {#mainline}

##MAINLINE-IS-THE-MAINTAINERS-INTEGRATED-LOCAL-MAIN Mainline is the maintainer's integrated local `main`. @impl/done

##MAINLINE-HAS-NO-PRIMARY-HOST It has **no
primary host** — it is not "the host A copy" or "the host B copy"; it
is what the maintainer has blessed, replicated equally to every host. @impl/done

##ONE-SERIAL-WRITER-MEANS-TWO-WRITES-CANNOT-RACE Because exactly one writer advances mainline, and does so **serially**,
two divergent writes to `main` cannot race. @impl/done

##THE-MULTI-MASTER-PROBLEM-IS-ABSENT-BY-CONSTRUCTION The multi-master problem is
**absent by construction, not patched after the fact.** @impl/done

##the-cost-of-both-repos-canonical-is-paid-once The cost of
"both repos canonical" is paid once, in the model, not continuously, in
conflict resolution. @spec/done

### Every host is a downstream read-replica {#replicas}

##EACH-HOST-IS-CANONICAL-FOR-READING-AND-A-REPLICA-FOR-WRITING Each host is canonical for *reading* in its audience, and a replica for
*writing* — nobody writes a target directly. @impl/done

##A-DIRECT-WRITE-OR-A-FORCE-PUSH-MAKES-A-TARGET-DIVERGE A direct write to a target, or a force-push, makes it **diverge** from
mainline. @impl/done

##THE-MODEL-DETECTS-DIVERGENCE-AND-FAILS-LOUD The model
does not paper over that: the tooling detects it and **fails loud**
rather than reconciling silently. @impl/done

##A-DIVERGED-TARGET-IS-A-SIGNAL-TO-INVESTIGATE A diverged target is a signal to
investigate, never something to clobber. @impl/done

### Contributions arrive anywhere; the maintainer integrates {#integration}

##A-CHANGE-REACHES-MAINLINE-ONLY-BY-THE-MAINTAINER-INTEGRATING-IT A change reaches mainline only by the maintainer integrating it. @impl/done

##PROPOSALS-ARRIVE-HOWEVER-IS-CONVENIENT Proposals arrive however is convenient — a web PR on host A, a web PR
on host B, a branch on a fork, an emailed patch — and are reviewed
where they land. @impl/done

##ACCEPTING-ONE-MEANS-BRINGING-ITS-COMMITS-INTO-MAINLINE Accepting one means bringing its commits into local
mainline, then fanning out. @impl/done

| Surface | Role |
|---------|------|
| ##ROW-SURFACE-WEB-PR-UI A host's web PR UI @impl/done | **Inbox** and review surface @impl/done |
| ##ROW-SURFACE-LOCAL-MAIN The maintainer's local `main` @impl/done | **Merge authority** @impl/done |
| ##ROW-SURFACE-HOST-AFTER-FAN-OUT Every host after fan-out @impl/done | **Read-replica** of that authority @impl/done |

##THE-WEB-PR-UIS-ARE-INBOXES-NOT-THE-MERGE-AUTHORITY The web PR UIs are *inboxes and review surfaces, not the merge
authority*. @impl/done

##this-is-exactly-the-kernels-workflow This is exactly the kernel's "patches by email, integrated
in the maintainer's tree, pushed to a hub that mirrors out" — the web
UIs are merely nicer inboxes than a mailing list. @spec/done

## What the model buys {#buys}

- ##BUYS-DIVERGENCE-IS-IMPOSSIBLE-BY-CONSTRUCTION **Divergence is impossible by construction.** One serial writer means
  no two writes to `main` can race. There is no reconciliation step to
  get wrong because there is no concurrent write to reconcile. @impl/done
- ##BUYS-ANY-HOST-CAN-VANISH-WITHOUT-DATA-LOSS **Any host can vanish without data loss.** Every host holds the full
  history; mainline holds it too. A host going dark, getting blocked,
  or deleting the repo costs a line in the manifest, not a commit. @impl/done
- ##BUYS-AUDIENCE-AND-JURISDICTION-PER-HOST **Audience and jurisdiction per host.** Each host serves its own
  region, community, or compliance surface, while all serve identical
  history. Adding a host is one manifest entry. @impl/done
- ##BUYS-THE-INVARIANT-IS-RUNNABLE-CAPITAL **The invariant is runnable capital.** "Never `--force`" is not a
  prose promise — it is pinned by a test over the push command
  ([`fanout-mechanics.md` §never-force-test](fanout-mechanics.md#never-force-test)). @impl/done

## What the model costs {#costs}

##ONE-HUMAN-SERIALIZES-EVERY-MERGE Be honest about the bottleneck: **one human serializes every merge.** @impl/done

##MAINLINE-ADVANCES-ONLY-AS-FAST-AS-THE-MAINTAINER-INTEGRATES Mainline advances only as fast as the maintainer integrates. @impl/done

##there-is-no-parallel-write-path There is
no parallel write path — that is the whole point, and it is also the
whole cost. @spec/done

| Property | Multi-master | Single-writer mainline |
|----------|--------------|------------------------|
| ##ROW-COST-CONCURRENT-WRITES Concurrent writes @spec/done | Allowed, and they race @spec/done | Serialized through one tree @spec/done |
| ##ROW-COST-DIVERGENCE Divergence @spec/done | Possible; must be reconciled @spec/done | Impossible by construction @spec/done |
| ##ROW-COST-MERGE-THROUGHPUT Merge throughput @spec/done | Many writers @spec/done | One writer (the bottleneck) @spec/done |
| ##ROW-COST-FAILURE-SURFACE Failure surface @spec/done | Reconcile published history @spec/done | A loud abort before any harm @spec/done |

##for-a-small-team-the-trade-is-strongly-positive For a **small-team or single-maintainer project**, the trade is
strongly positive: integration is not the throughput limit (review and
design are), and the maintainer was serializing the important merges
anyway. @spec/done

##THE-MODEL-MAKES-SERIALIZATION-THE-ONLY-WRITE-PATH The model just makes that serialization the *only* write path,
so nothing can sneak around it and diverge. @impl/done

##when-a-project-outgrows-one-integrator-this-is-the-wrong-tool When a project outgrows one
integrator — several full-time committers merging in parallel all day —
this model is the wrong tool, and the honest answer is to add
one-directional server-side mirroring or move to a shared-forge
workflow. @spec/done

##RECORD-THAT-AS-A-REVISIT-TRIGGER Record that as a revisit trigger, not a someday-maybe. @impl/done

## Re-derive for your project {#re-derive}

##re-derive-lead Do not copy the host names or the script verbatim — copy the *task*,
and let the agent derive the setup your project actually needs: @impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-source-mirrors/<version>/spec/flows/source-mirrors/`, check `vibe.lock`) in full, then adapt it to this project:
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

- ##SUM-MULTI-HOMING-INVITES-DIVERGENCE-AND-THIS-MODEL-DISSOLVES-IT Multi-homing across hosts invites multi-master divergence; this model
  dissolves it instead of managing it. @impl/done
- ##SUM-ONE-MAINLINE-EVERY-HOST-A-REPLICA One mainline, single-writer, no primary host. Every host is a
  downstream read-replica. Contributions arrive on any host as inboxes;
  the maintainer's tree is the merge authority. @impl/done
- ##SUM-WHAT-IT-BUYS What it buys: divergence impossible by construction, any host can
  vanish without data loss, audience/jurisdiction per host. @impl/done
- ##SUM-WHAT-IT-COSTS What it costs: one human serializes merges. Acceptable — and cheaper
  than the alternative — for small-team projects; record a revisit
  trigger for the day it is not. @spec/done
- ##SUM-THE-NEVER-FORCE-INVARIANT-IS-RUNNABLE-CAPITAL The never-`--force` invariant is runnable capital, not a promise. @impl/done
