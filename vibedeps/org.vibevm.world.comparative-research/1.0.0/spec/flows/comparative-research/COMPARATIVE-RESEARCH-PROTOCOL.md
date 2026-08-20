# Comparative Research Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines what a *comparative
research document* is, why the genre exists, the five laws every
such document obeys, when to write one, and how the study connects
to the roadmap without ever ratifying itself. @status:impl/done

@fact:sibling-document-pointers Copy-ready skeleton:
[`research-template.md`](research-template.md); the downstream
pipeline: [`from-research-to-roadmap.md`](from-research-to-roadmap.md). @status:impl/done

## What the genre is {#what}

@fact:A-RESEARCH-DOCUMENT-IS-A-SELF-CONTAINED-STUDY-OF-ONE-SYSTEM A comparative research document is a **self-contained, evergreen
study of one external system** — a competitor, a predecessor, an
adjacent tool that occupies ground near yours. @status:impl/done

@fact:IT-INVENTORIES-QUOTES-MEASURES-AND-TRANSLATES It inventories what
that system does, quotes it in its own words, measures it in two
directions against your own project, and translates the actionable
findings into numbered roadmap proposals. @status:impl/done

@fact:evergreen-is-the-load-bearing-adjective "Evergreen" is the load-bearing adjective. @status:impl/done

@fact:THE-DOCUMENT-IS-WRITTEN-TO-OUTLAST-ITS-SOURCES The document is written
to be **re-readable months after publication without referring to
the original sources, and to outlast any one external project's URL
stability**. @status:impl/done

- @fact:ROT-MARKETING-PAGES-GET-REDESIGNED Marketing pages get redesigned, @status:spec/done
- @fact:ROT-DOCS-SITES-MOVE docs sites move, @status:spec/done
- @fact:ROT-PRODUCTS-GET-ACQUIRED-AND-DELETED whole
  products get acquired and deleted. @status:spec/done

@fact:a-study-that-links-out-expires A study that merely links out is
a study that expires. @status:spec/done

@fact:a-quoted-study-stands-after-the-links-rot A study that quotes verbatim with dates is a
study that stands on its own after the links rot. @status:spec/done

## Why the genre exists {#why}

@fact:three-costs-justify-the-effort-lead Three costs justify the effort of a full study over a quick glance: @status:spec/done

- @fact:COST-UNEXAMINED-COMPETITORS-COST-ROADMAP-MISTAKES **Unexamined competitors cost roadmap mistakes.** Building in a
  space someone else already occupies, without knowing what they
  built, means re-discovering their dead ends at your own expense —
  or missing the one feature that was the whole point. @status:spec/done
- @fact:COST-EXAMINED-COMPETITORS-YIELD-DELTAS **Examined competitors yield deltas.** Understanding what a rival
  does well — *and what it does not* — is load-bearing intelligence
  for your own roadmap. Every gap you find is a candidate decision:
  close it, decline it on record, or note it as already led. @status:spec/done
- @fact:COST-THE-STUDY-MUST-OUTLIVE-LINK-ROT **The study must outlive link rot and staff turnover.** The person
  who did the research leaves; the URL 404s; the competitor pivots.
  What remains is the document, and the document is only worth
  keeping if it is complete on its own terms. @status:spec/done

## The five laws {#laws}

@fact:EVERY-DOCUMENT-OBEYS-ALL-FIVE-LAWS Every comparative research document obeys all five. @status:impl/done

@fact:the-laws-separate-a-study-from-a-rant They are what
separate a study from a bookmark dump or a competitive-envy rant. @status:spec/done

### Law 1 — Self-containedness {#law-self-contained}

@fact:LAW-ONE-THE-DOCUMENT-MUST-STAND-WHEN-ITS-SOURCES-VANISH The document must stand when its sources vanish. @status:impl/done

@fact:LAW-ONE-QUOTE-VERBATIM-WITH-AN-ACCESS-DATE Quote the subject
**verbatim in fenced blocks, each carrying an access date**, rather
than linking and trusting the link. @status:impl/done

@fact:LAW-ONE-A-FUTURE-READER-RECONSTRUCTS-WITHOUT-FETCHING A future reader with no network
access, opening the file a year later, must be able to reconstruct
what the subject claimed without fetching anything. @status:impl/done

@fact:LAW-ONE-LINKS-ARE-POINTERS-QUOTES-ARE-EVIDENCE Links are
pointers for refresh; quotes are the evidence of record. @status:impl/done

### Law 2 — Quote first, critique second {#law-quote-first}

@fact:LAW-TWO-THE-SUBJECT-SPEAKS-BEFORE-IT-IS-JUDGED The subject speaks **in its own words before it is judged**. @status:impl/done

@fact:LAW-TWO-PRESENT-THE-QUOTE-THEN-ANALYZE Present
the verbatim quote — the pitch, the feature description, the design
claim — and only *then* analyze, praise, or fault it. @status:impl/done

@fact:critiquing-a-paraphrase-is-critiquing-a-straw-man Critiquing a
paraphrase is critiquing a straw man: the paraphrase is already your
reading, and a reader cannot check your judgement against a summary
you wrote. @status:spec/done

@fact:LAW-TWO-QUOTE-THEN-JUDGE-EVERY-TIME Quote, then judge, in that order, every time. @status:impl/done

### Law 3 — Two-way gaps {#law-two-way}

@fact:law-three-runs-in-both-directions-lead The analysis runs in **both directions**: @status:impl/done

- @fact:LAW-THREE-A-SECTION-FOR-WHERE-YOU-TRAIL a section for where you
  trail the subject, @status:impl/done
- @fact:LAW-THREE-A-SECTION-FOR-WHERE-YOU-LEAD and a section for where you lead it. @status:impl/done

@fact:one-directional-gap-analysis-is-advocacy-lead One-directional
gap analysis is not analysis — it is advocacy: @status:spec/done

| One-directional study | What it actually is |
|---|---|
| @fact:ROW-ONE-DIRECTIONAL-TRAIL-ONLY Only finds where you trail @status:spec/done | Marketing for the competitor @status:spec/done |
| @fact:ROW-ONE-DIRECTIONAL-LEAD-ONLY Only finds where you lead @status:spec/done | Marketing for yourself @status:spec/done |
| @fact:ROW-FINDS-BOTH-ARGUED-EQUALLY Finds both, argued equally @status:spec/done | Intelligence you can act on @status:spec/done |

@fact:WHERE-YOU-LEAD-DESERVES-THE-SAME-RIGOR Where you lead deserves the same rigor as where you trail: name the
decision you made that they did not, and say why it matters. @status:impl/done

@fact:envy-blinds-you-to-the-moat-you-already-have Envy in
one direction blinds you to the moat you already have. @status:spec/done

### Law 4 — Deltas, not decrees {#law-deltas}

@fact:LAW-FOUR-FINDINGS-BECOME-NUMBERED-PRIORITIZED-HOMED-DELTAS Actionable findings become **numbered roadmap deltas**, each with a
**priority** and a **target home** — "maps to a future spec section",
not a change ratified here. @status:impl/done

@fact:LAW-FOUR-THE-DOCUMENT-PROPOSES-IT-DOES-NOT-DECIDE The research document *proposes*; it does
not *decide*. @status:impl/done

@fact:a-self-ratifying-study-skips-the-review A study that quietly rewrites the roadmap inside itself
has skipped the review where a human weighs the proposal against
everything else competing for the same effort. @status:spec/done

@fact:LAW-FOUR-KEEP-THE-TWO-ACTS-SEPARATE Keep the two acts
separate: the study argues, the owner decides
([`from-research-to-roadmap.md`](from-research-to-roadmap.md)). @status:impl/done

### Law 5 — The re-fetch list {#law-refetch}

@fact:LAW-FIVE-THE-DOCUMENT-CLOSES-WITH-THE-RE-FETCH-LIST The document closes with **every source URL, its access date, and
the subject's version at capture** — the exact list needed to refresh
the study later. @status:impl/done

@fact:the-study-has-a-shelf-life Because the subject keeps shipping, the study has a
shelf life; the re-fetch list is what lets a future session *update*
the study instead of starting over. @status:impl/done

@fact:record-enough-that-the-refresh-is-mechanical-lead Record enough that the refresh is
mechanical: @status:impl/done

- @fact:REFETCH-WHICH-URLS which URLs, @status:impl/done
- @fact:REFETCH-IN-WHAT-ORDER in what order, @status:impl/done
- @fact:REFETCH-WHAT-VERSION-THIS-CAPTURE-REFLECTS and what version number this
  capture reflects. @status:impl/done

## When to write one {#when}

@fact:not-every-glance-warrants-a-full-study Not every glance at a competitor warrants a full study. Write one
when: @status:spec/done

| Trigger | Why it warrants the full genre |
|---|---|
| @fact:ROW-TRIGGER-BEFORE-BUILDING-IN-AN-OCCUPIED-SPACE Before building in a space others occupy @status:spec/done | Cheaper to learn their dead ends than to re-walk them @status:spec/done |
| @fact:ROW-TRIGGER-A-COMPETITOR-SHIPS-SOMETHING-ALARMING A competitor ships something alarming @status:spec/done | An emotional reaction needs a structured study to become a decision @status:spec/done |
| @fact:ROW-TRIGGER-A-RECURRING-WHY-NOT-QUESTION "Why don't we just do what X does?" recurs @status:spec/done | A recurring question deserves a durable, citable answer, not a repeated verbal one @status:spec/done |

@fact:a-one-off-lookup-does-not-warrant-a-study For a one-off factual lookup — "does X support Windows?" — a study is
overkill; answer it and move on. @status:impl/done

@fact:THE-GENRE-IS-FOR-A-SYSTEM-WORTH-UNDERSTANDING-WHOLE The genre is for a *system* worth
understanding whole, whose shape will inform decisions more than once. @status:impl/done

## What it is not {#not}

- @fact:NOT-A-DECISION **Not a decision.** It proposes deltas; ratification is downstream. @status:impl/done
- @fact:NOT-A-RANT **Not a rant.** Competitive frustration is the trigger; the
  document is the disciplined product, two-way and quoted. @status:impl/done
- @fact:NOT-A-LIVE-DASHBOARD **Not a live dashboard.** It is a dated snapshot with a refresh
  procedure — a baseline, not a feed. When it ages, refresh it via
  the re-fetch list; keep the old capture as historical record. @status:impl/done
- @fact:NOT-A-LINK-FARM **Not a link farm.** Links rot; the quotes are the evidence. @status:impl/done

## Re-derive for your project {#re-derive}

@fact:re-derive-lead Do not copy this protocol's framing verbatim — copy the *task*, and
let the agent produce the study your project actually needs: @status:impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-comparative-research/<version>/spec/flows/comparative-research/`, check `vibe.lock`) in full, then run one study:
1. Name the external system to study and why now (building nearby /
   they shipped something / a recurring "why not do what X does").
2. Fetch its primary sources — docs, pitch, changelog — and record
   each URL with today's date and the subject's current version.
3. Quote the subject verbatim in fenced blocks before any judgement;
   never critique a paraphrase.
4. Write BOTH gap directions: where we trail, where we lead, each
   argued with equal weight and concrete evidence.
5. Turn the actionable trailing gaps into numbered deltas, each with
   a priority and a target home in our spec tree. Ratify none.
6. Close with the re-fetch list. Show me the draft; apply nothing.
```

## Summary {#summary}

- @fact:SUM-A-SELF-CONTAINED-EVERGREEN-STUDY A comparative research document is a self-contained, evergreen
  study of one external system, written to outlast its own sources. @status:impl/done
- @fact:SUM-WHY-THE-GENRE-EXISTS It exists because unexamined competitors cost roadmap mistakes and
  examined ones yield deltas — and the study must survive link rot. @status:spec/done
- @fact:SUM-THE-FIVE-LAWS Five laws: self-containedness (dated verbatim quotes), quote-first
  then critique, two-way gaps (trail *and* lead), deltas-not-decrees
  (numbered, prioritized, homed, unratified), and the re-fetch list. @status:impl/done
- @fact:SUM-WHEN-TO-WRITE-ONE Write one before building in an occupied space, when a competitor
  alarms, or when "why not do what X does" keeps recurring. @status:impl/done
- @fact:SUM-THE-STUDY-PROPOSES-THE-OWNER-DECIDES The study proposes; the owner decides. See
  [`from-research-to-roadmap.md`](from-research-to-roadmap.md). @status:impl/done
