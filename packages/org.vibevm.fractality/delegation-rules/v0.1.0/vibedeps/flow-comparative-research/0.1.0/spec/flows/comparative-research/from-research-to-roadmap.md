# From research to roadmap {#root}

**Scope of this document.** This file defines what happens to a
comparative research document *after* it is written: how a numbered
delta becomes a decision, what becomes of the deltas that are
declined, when a study goes stale and how it is refreshed, and the
single honesty rule that keeps the whole genre from decaying into
advocacy. The genre laws are in
[`COMPARATIVE-RESEARCH-PROTOCOL.md`](COMPARATIVE-RESEARCH-PROTOCOL.md);
this is the pipeline that runs downstream of them.

## The pipeline {#pipeline}

A research document ends in a table of numbered deltas. Each delta
travels one path:

```
delta (proposed in the study)
      │
      ▼
owner review  ──►  accepted  ──►  recorded decision + revisit trigger
      │                            (leaves the study; lands in the spec)
      ▼
   rejected  ──►  stays in the study, with its rejection reason
                  (the study is the archive of roads not taken)
```

The study **proposes**; the owner **decides**. Nothing on the
accepted branch happens inside the research document — that is the
deltas-not-decrees law
([protocol §law-deltas](COMPARATIVE-RESEARCH-PROTOCOL.md#law-deltas)).

## Owner review {#review}

The deltas do not ratify themselves and they do not auto-schedule.
A human reads the numbered table and, per delta, decides one of:

| Verdict | What it means | Where it goes |
|---|---|---|
| **Accept** | We will do this | Becomes a recorded decision (below) |
| **Reject** | We will not do this | Stays in the study with a reason (below) |
| **Defer** | Not now, revisit later | Stays as a delta, marked *deferred*, with a revisit trigger |

A delta with no verdict is not done being reviewed. The value of the
numbering is exactly this: the owner can say "accept D1, reject D4,
defer D7" and every party knows precisely which proposal is meant.

## Accepted deltas become decisions {#accepted}

An accepted delta does not just get "implemented" — it lands as a
**recorded decision with a revisit trigger**, at the spec anchor the
delta named as its target home. This is `flow:decision-records`
doing its job: the delta's argument (why match the competitor here)
becomes the record's *Why*, and the delta's number and the study's
title become the citation.

A recorded decision born from an accepted delta carries:

- **Decision** — the capability we will build, one line.
- **Why** — the gap the study found, cited back to the quoted
  evidence in the research doc (the verbatim quote is the data).
- **Considered and rejected** — including "do nothing", if the study
  weighed it.
- **When to revisit** — a trigger, because a competitive gap can
  close from either side: they might drop the feature, or the space
  might move.

The research document then notes, at the delta, that it was accepted
and points at the decision's anchor. The delta is now *history* in
the study and *live* in the spec.

## Rejected deltas stay in the study {#rejected}

A rejected delta is **not deleted**. It stays in the research
document, annotated with the reason it lost:

```markdown
### D4 — Background auto-update daemon
Maps to §3.4. **Priority:** LOW.
**Verdict: REJECTED (2026-07-05).** For a tool whose value is an
audit trail, silent background updates are the wrong default; every
install should be deliberated. Revisit only if users ask for it.
```

This is deliberate. The research document is **also the archive of
roads not taken**. A rejected delta left in place answers the
question before it is re-asked: the next person who says "why don't
we just add auto-update like they have" reads D4 and gets the reason
in one read instead of re-running the analysis. Delete the rejected
delta and you delete the immunity; the question comes back every
quarter.

The rejection reason is subject to the same standard as a decision's
*Why*: cite something. "Rejected because it's bad" invites the delta
to be re-proposed; "rejected because it contradicts the audit-trail
invariant" closes it.

## Refresh discipline {#refresh}

A comparative study is a **dated snapshot**, and the subject keeps
shipping. The staleness rule is mechanical:

> A study **older than its subject's last major release** is
> stale-flagged.

When a study is stale-flagged:

1. **Refresh via the re-fetch list**, not by rewriting. Walk the URLs
   in the study's re-fetch section, re-capture the quotes, record the
   new access dates and the new subject version.
2. **Append, don't overwrite.** Add a dated refresh block noting what
   changed materially since the last capture. Keep the old capture as
   a historical baseline — the point of an evergreen doc is that the
   old snapshot still has value.
3. **Re-audit the delta table.** Every delta gets re-checked against
   the new reality:
   - A trailing gap the subject *widened* → the delta's priority may
     rise.
   - A gap the subject *closed on their side* (dropped the feature) →
     the delta may become moot; mark it so.
   - A gap *we* closed (shipped the capability) → the delta is done;
     point it at the shipped decision.
   - A wholly new capability → a new delta.

Refreshing is cheaper than re-studying precisely because Law 1 and
Law 5 did their work: the quotes are already dated, the URLs are
already listed, and the previous version number is on record. A study
written without a re-fetch list cannot be refreshed — only redone.

## The honesty rule {#honesty}

One rule governs the whole genre, and it is worth stating alone:

> A study that only finds gaps where we trail is marketing for the
> competitor. A study that only finds where we lead is marketing for
> us. Neither is intelligence.

The two-way law ([protocol §law-two-way](COMPARATIVE-RESEARCH-PROTOCOL.md#law-two-way))
is not a formatting convention — it is the difference between a
document you can act on and a document that flatters a foregone
conclusion. Trail-only studies talk teams into copying features they
do not need. Lead-only studies talk teams out of taking a real
threat seriously. Both feel like analysis while producing the
opposite.

The test at review time: **read §3 and §4 side by side.** If one is
three pages and the other is three lines, the study is advocacy
wearing the genre's clothes. Send it back. A real study of a real
competitor finds meaningful gaps in both directions, because no two
teams make all the same decisions — where they diverged is exactly
what the study exists to surface.

## Summary {#summary}

- The study proposes numbered deltas; the owner reviews each and
  accepts, rejects, or defers it.
- An accepted delta becomes a recorded decision with a revisit
  trigger at its target anchor — `flow:decision-records` does the
  landing; the study points at the anchor.
- A rejected delta stays in the study with its cited reason; the
  document is the archive of roads not taken.
- A study older than its subject's last major release is
  stale-flagged, refreshed via the re-fetch list (append, don't
  overwrite), and its delta table re-audited.
- The honesty rule: trail-only is marketing for them, lead-only is
  marketing for us. Read §3 and §4 side by side — if they are not
  both substantial, send the study back.
