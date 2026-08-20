# From research to roadmap {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines what happens to a
comparative research document *after* it is written: how a numbered
delta becomes a decision, what becomes of the deltas that are
declined, when a study goes stale and how it is refreshed, and the
single honesty rule that keeps the whole genre from decaying into
advocacy. @status:impl/done

@fact:sibling-document-pointers The genre laws are in
[`COMPARATIVE-RESEARCH-PROTOCOL.md`](COMPARATIVE-RESEARCH-PROTOCOL.md);
this is the pipeline that runs downstream of them. @status:impl/done

## The pipeline {#pipeline}

@fact:A-RESEARCH-DOCUMENT-ENDS-IN-A-DELTA-TABLE A research document ends in a table of numbered deltas. @status:impl/done

@fact:each-delta-travels-one-path-lead Each delta
travels one path: @status:impl/done

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

@fact:THE-STUDY-PROPOSES-THE-OWNER-DECIDES The study **proposes**; the owner **decides**. @status:impl/done

@fact:NOTHING-ON-THE-ACCEPTED-BRANCH-HAPPENS-IN-THE-STUDY Nothing on the
accepted branch happens inside the research document — that is the
deltas-not-decrees law
([protocol §law-deltas](COMPARATIVE-RESEARCH-PROTOCOL.md#law-deltas)). @status:impl/done

## Owner review {#review}

@fact:DELTAS-DO-NOT-RATIFY-OR-AUTO-SCHEDULE-THEMSELVES The deltas do not ratify themselves and they do not auto-schedule. @status:impl/done

@fact:a-human-decides-one-of-lead A human reads the numbered table and, per delta, decides one of: @status:impl/done

| Verdict | What it means | Where it goes |
|---|---|---|
| @fact:ROW-VERDICT-ACCEPT **Accept** @status:impl/done | We will do this @status:impl/done | Becomes a recorded decision (below) @status:impl/done |
| @fact:ROW-VERDICT-REJECT **Reject** @status:impl/done | We will not do this @status:impl/done | Stays in the study with a reason (below) @status:impl/done |
| @fact:ROW-VERDICT-DEFER **Defer** @status:impl/done | Not now, revisit later @status:impl/done | Stays as a delta, marked *deferred*, with a revisit trigger @status:impl/done |

@fact:A-DELTA-WITH-NO-VERDICT-IS-NOT-DONE-BEING-REVIEWED A delta with no verdict is not done being reviewed. @status:impl/done

@fact:the-value-of-the-numbering The value of the
numbering is exactly this: the owner can say "accept D1, reject D4,
defer D7" and every party knows precisely which proposal is meant. @status:impl/done

## Accepted deltas become decisions {#accepted}

@fact:AN-ACCEPTED-DELTA-LANDS-AS-A-RECORDED-DECISION An accepted delta does not just get "implemented" — it lands as a
**recorded decision with a revisit trigger**, at the spec anchor the
delta named as its target home. @status:impl/done

@fact:this-is-decision-records-doing-its-job This is `flow:decision-records`
doing its job: the delta's argument (why match the competitor here)
becomes the record's *Why*, and the delta's number and the study's
title become the citation. @status:impl/done

@fact:a-recorded-decision-carries-lead A recorded decision born from an accepted delta carries: @status:impl/done

- @fact:RECORD-FIELD-DECISION **Decision** — the capability we will build, one line. @status:impl/done
- @fact:RECORD-FIELD-WHY **Why** — the gap the study found, cited back to the quoted
  evidence in the research doc (the verbatim quote is the data). @status:impl/done
- @fact:RECORD-FIELD-CONSIDERED-AND-REJECTED **Considered and rejected** — including "do nothing", if the study
  weighed it. @status:impl/done
- @fact:RECORD-FIELD-WHEN-TO-REVISIT **When to revisit** — a trigger, because a competitive gap can
  close from either side: they might drop the feature, or the space
  might move. @status:impl/done

@fact:THE-STUDY-NOTES-THE-ACCEPTANCE-AND-POINTS-AT-THE-ANCHOR The research document then notes, at the delta, that it was accepted
and points at the decision's anchor. @status:impl/done

@fact:THE-DELTA-IS-HISTORY-IN-THE-STUDY-AND-LIVE-IN-THE-SPEC The delta is now *history* in
the study and *live* in the spec. @status:impl/done

## Rejected deltas stay in the study {#rejected}

@fact:A-REJECTED-DELTA-IS-NOT-DELETED A rejected delta is **not deleted**. @status:impl/done

@fact:it-stays-annotated-with-the-reason-lead It stays in the research
document, annotated with the reason it lost: @status:impl/done

```markdown
### D4 — Background auto-update daemon
Maps to §3.4. **Priority:** LOW.
**Verdict: REJECTED (2026-07-05).** For a tool whose value is an
audit trail, silent background updates are the wrong default; every
install should be deliberated. Revisit only if users ask for it.
```

@fact:this-is-deliberate This is deliberate. @status:impl/done

@fact:THE-DOCUMENT-IS-ALSO-THE-ARCHIVE-OF-ROADS-NOT-TAKEN The research document is **also the archive of
roads not taken**. @status:impl/done

@fact:a-rejected-delta-answers-the-question-before-it-is-re-asked A rejected delta left in place answers the
question before it is re-asked: the next person who says "why don't
we just add auto-update like they have" reads D4 and gets the reason
in one read instead of re-running the analysis. @status:impl/done

@fact:deleting-the-delta-deletes-the-immunity Delete the rejected
delta and you delete the immunity; the question comes back every
quarter. @status:spec/done

@fact:THE-REJECTION-REASON-MUST-CITE-SOMETHING The rejection reason is subject to the same standard as a decision's
*Why*: cite something. @status:impl/done

@fact:a-cited-rejection-closes-the-question "Rejected because it's bad" invites the delta
to be re-proposed; "rejected because it contradicts the audit-trail
invariant" closes it. @status:spec/done

## Refresh discipline {#refresh}

@fact:A-STUDY-IS-A-DATED-SNAPSHOT A comparative study is a **dated snapshot**, and the subject keeps
shipping. @status:impl/done

@fact:the-staleness-rule-is-mechanical-lead The staleness rule is mechanical: @status:impl/done

> @fact:A-STUDY-OLDER-THAN-THE-LAST-MAJOR-RELEASE-IS-STALE-FLAGGED A study **older than its subject's last major release** is
> stale-flagged. @status:impl/done

@fact:when-a-study-is-stale-flagged-lead When a study is stale-flagged: @status:impl/done

1. @fact:STALE-REFRESH-VIA-THE-RE-FETCH-LIST **Refresh via the re-fetch list**, not by rewriting. Walk the URLs
   in the study's re-fetch section, re-capture the quotes, record the
   new access dates and the new subject version. @status:impl/done
2. @fact:STALE-APPEND-DONT-OVERWRITE **Append, don't overwrite.** Add a dated refresh block noting what
   changed materially since the last capture. Keep the old capture as
   a historical baseline — the point of an evergreen doc is that the
   old snapshot still has value. @status:impl/done
3. @fact:STALE-RE-AUDIT-THE-DELTA-TABLE **Re-audit the delta table.** Every delta gets re-checked against
   the new reality: @status:impl/done
   - @fact:RE-AUDIT-A-GAP-THE-SUBJECT-WIDENED A trailing gap the subject *widened* → the delta's priority may
     rise. @status:impl/done
   - @fact:RE-AUDIT-A-GAP-THE-SUBJECT-CLOSED A gap the subject *closed on their side* (dropped the feature) →
     the delta may become moot; mark it so. @status:impl/done
   - @fact:RE-AUDIT-A-GAP-WE-CLOSED A gap *we* closed (shipped the capability) → the delta is done;
     point it at the shipped decision. @status:impl/done
   - @fact:RE-AUDIT-A-WHOLLY-NEW-CAPABILITY A wholly new capability → a new delta. @status:impl/done

@fact:refreshing-is-cheaper-because-laws-one-and-five-did-their-work Refreshing is cheaper than re-studying precisely because Law 1 and
Law 5 did their work: @status:impl/done

- @fact:REFRESH-CHEAP-QUOTES-ARE-ALREADY-DATED the quotes are already dated, @status:impl/done
- @fact:REFRESH-CHEAP-URLS-ARE-ALREADY-LISTED the URLs are
  already listed, @status:impl/done
- @fact:REFRESH-CHEAP-THE-VERSION-NUMBER-IS-ON-RECORD and the previous version number is on record. @status:impl/done

@fact:A-STUDY-WITHOUT-A-RE-FETCH-LIST-CANNOT-BE-REFRESHED A study
written without a re-fetch list cannot be refreshed — only redone. @status:impl/done

## The honesty rule {#honesty}

@fact:one-rule-governs-the-whole-genre-lead One rule governs the whole genre, and it is worth stating alone: @status:impl/done

> @fact:THE-HONESTY-RULE A study that only finds gaps where we trail is marketing for the
> competitor. A study that only finds where we lead is marketing for
> us. Neither is intelligence. @status:spec/done

@fact:THE-TWO-WAY-LAW-IS-NOT-A-FORMATTING-CONVENTION The two-way law ([protocol §law-two-way](COMPARATIVE-RESEARCH-PROTOCOL.md#law-two-way))
is not a formatting convention — it is the difference between a
document you can act on and a document that flatters a foregone
conclusion. @status:impl/done

@fact:trail-only-studies-talk-teams-into-copying Trail-only studies talk teams into copying features they
do not need. @status:spec/done

@fact:lead-only-studies-talk-teams-out-of-a-real-threat Lead-only studies talk teams out of taking a real
threat seriously. @status:spec/done

@fact:both-feel-like-analysis-while-producing-the-opposite Both feel like analysis while producing the
opposite. @status:spec/done

@fact:THE-TEST-AT-REVIEW-TIME The test at review time: **read §3 and §4 side by side.** @status:impl/done

@fact:a-lopsided-study-is-advocacy If one is
three pages and the other is three lines, the study is advocacy
wearing the genre's clothes. @status:spec/done

@fact:SEND-IT-BACK Send it back. @status:impl/done

@fact:no-two-teams-make-all-the-same-decisions A real study of a real
competitor finds meaningful gaps in both directions, because no two
teams make all the same decisions — where they diverged is exactly
what the study exists to surface. @status:spec/done

## Summary {#summary}

- @fact:SUM-THE-STUDY-PROPOSES-THE-OWNER-REVIEWS The study proposes numbered deltas; the owner reviews each and
  accepts, rejects, or defers it. @status:impl/done
- @fact:SUM-AN-ACCEPTED-DELTA-BECOMES-A-DECISION An accepted delta becomes a recorded decision with a revisit
  trigger at its target anchor — `flow:decision-records` does the
  landing; the study points at the anchor. @status:impl/done
- @fact:SUM-A-REJECTED-DELTA-STAYS-WITH-ITS-REASON A rejected delta stays in the study with its cited reason; the
  document is the archive of roads not taken. @status:impl/done
- @fact:SUM-A-STALE-STUDY-IS-REFRESHED-AND-RE-AUDITED A study older than its subject's last major release is
  stale-flagged, refreshed via the re-fetch list (append, don't
  overwrite), and its delta table re-audited. @status:impl/done
- @fact:SUM-THE-HONESTY-RULE The honesty rule: trail-only is marketing for them, lead-only is
  marketing for us. Read §3 and §4 side by side — if they are not
  both substantial, send the study back. @status:impl/done
