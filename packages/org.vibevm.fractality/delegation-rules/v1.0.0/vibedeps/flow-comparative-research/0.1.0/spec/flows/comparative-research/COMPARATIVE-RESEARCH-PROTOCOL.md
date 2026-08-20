# Comparative Research Protocol {#root}

**Scope of this document.** This file defines what a *comparative
research document* is, why the genre exists, the five laws every
such document obeys, when to write one, and how the study connects
to the roadmap without ever ratifying itself. Copy-ready skeleton:
[`research-template.md`](research-template.md); the downstream
pipeline: [`from-research-to-roadmap.md`](from-research-to-roadmap.md).

## What the genre is {#what}

A comparative research document is a **self-contained, evergreen
study of one external system** — a competitor, a predecessor, an
adjacent tool that occupies ground near yours. It inventories what
that system does, quotes it in its own words, measures it in two
directions against your own project, and translates the actionable
findings into numbered roadmap proposals.

"Evergreen" is the load-bearing adjective. The document is written
to be **re-readable months after publication without referring to
the original sources, and to outlast any one external project's URL
stability**. Marketing pages get redesigned, docs sites move, whole
products get acquired and deleted. A study that merely links out is
a study that expires. A study that quotes verbatim with dates is a
study that stands on its own after the links rot.

## Why the genre exists {#why}

Three costs justify the effort of a full study over a quick glance:

- **Unexamined competitors cost roadmap mistakes.** Building in a
  space someone else already occupies, without knowing what they
  built, means re-discovering their dead ends at your own expense —
  or missing the one feature that was the whole point.
- **Examined competitors yield deltas.** Understanding what a rival
  does well — *and what it does not* — is load-bearing intelligence
  for your own roadmap. Every gap you find is a candidate decision:
  close it, decline it on record, or note it as already led.
- **The study must outlive link rot and staff turnover.** The person
  who did the research leaves; the URL 404s; the competitor pivots.
  What remains is the document, and the document is only worth
  keeping if it is complete on its own terms.

## The five laws {#laws}

Every comparative research document obeys all five. They are what
separate a study from a bookmark dump or a competitive-envy rant.

### Law 1 — Self-containedness {#law-self-contained}

The document must stand when its sources vanish. Quote the subject
**verbatim in fenced blocks, each carrying an access date**, rather
than linking and trusting the link. A future reader with no network
access, opening the file a year later, must be able to reconstruct
what the subject claimed without fetching anything. Links are
pointers for refresh; quotes are the evidence of record.

### Law 2 — Quote first, critique second {#law-quote-first}

The subject speaks **in its own words before it is judged**. Present
the verbatim quote — the pitch, the feature description, the design
claim — and only *then* analyze, praise, or fault it. Critiquing a
paraphrase is critiquing a straw man: the paraphrase is already your
reading, and a reader cannot check your judgement against a summary
you wrote. Quote, then judge, in that order, every time.

### Law 3 — Two-way gaps {#law-two-way}

The analysis runs in **both directions**: a section for where you
trail the subject, and a section for where you lead it. One-directional
gap analysis is not analysis — it is advocacy:

| One-directional study | What it actually is |
|---|---|
| Only finds where you trail | Marketing for the competitor |
| Only finds where you lead | Marketing for yourself |
| Finds both, argued equally | Intelligence you can act on |

Where you lead deserves the same rigor as where you trail: name the
decision you made that they did not, and say why it matters. Envy in
one direction blinds you to the moat you already have.

### Law 4 — Deltas, not decrees {#law-deltas}

Actionable findings become **numbered roadmap deltas**, each with a
**priority** and a **target home** — "maps to a future spec section",
not a change ratified here. The research document *proposes*; it does
not *decide*. A study that quietly rewrites the roadmap inside itself
has skipped the review where a human weighs the proposal against
everything else competing for the same effort. Keep the two acts
separate: the study argues, the owner decides
([`from-research-to-roadmap.md`](from-research-to-roadmap.md)).

### Law 5 — The re-fetch list {#law-refetch}

The document closes with **every source URL, its access date, and
the subject's version at capture** — the exact list needed to refresh
the study later. Because the subject keeps shipping, the study has a
shelf life; the re-fetch list is what lets a future session *update*
the study instead of starting over. Record enough that the refresh is
mechanical: which URLs, in what order, and what version number this
capture reflects.

## When to write one {#when}

Not every glance at a competitor warrants a full study. Write one
when:

| Trigger | Why it warrants the full genre |
|---|---|
| Before building in a space others occupy | Cheaper to learn their dead ends than to re-walk them |
| A competitor ships something alarming | An emotional reaction needs a structured study to become a decision |
| "Why don't we just do what X does?" recurs | A recurring question deserves a durable, citable answer, not a repeated verbal one |

For a one-off factual lookup — "does X support Windows?" — a study is
overkill; answer it and move on. The genre is for a *system* worth
understanding whole, whose shape will inform decisions more than once.

## What it is not {#not}

- **Not a decision.** It proposes deltas; ratification is downstream.
- **Not a rant.** Competitive frustration is the trigger; the
  document is the disciplined product, two-way and quoted.
- **Not a live dashboard.** It is a dated snapshot with a refresh
  procedure — a baseline, not a feed. When it ages, refresh it via
  the re-fetch list; keep the old capture as historical record.
- **Not a link farm.** Links rot; the quotes are the evidence.

## Re-derive for your project {#re-derive}

Do not copy this protocol's framing verbatim — copy the *task*, and
let the agent produce the study your project actually needs:

```
Read spec/flows/comparative-research/ in full, then run one study:
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

- A comparative research document is a self-contained, evergreen
  study of one external system, written to outlast its own sources.
- It exists because unexamined competitors cost roadmap mistakes and
  examined ones yield deltas — and the study must survive link rot.
- Five laws: self-containedness (dated verbatim quotes), quote-first
  then critique, two-way gaps (trail *and* lead), deltas-not-decrees
  (numbered, prioritized, homed, unratified), and the re-fetch list.
- Write one before building in an occupied space, when a competitor
  alarms, or when "why not do what X does" keeps recurring.
- The study proposes; the owner decides. See
  [`from-research-to-roadmap.md`](from-research-to-roadmap.md).
