# Comparative Research Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines what a *comparative
research document* is, why the genre exists, the five laws every
such document obeys, when to write one, and how the study connects
to the roadmap without ever ratifying itself. @impl/done

##sibling-document-pointers Copy-ready skeleton:
[`research-template.md`](research-template.md); the downstream
pipeline: [`from-research-to-roadmap.md`](from-research-to-roadmap.md). @impl/done

## What the genre is {#what}

##A-RESEARCH-DOCUMENT-IS-A-SELF-CONTAINED-STUDY-OF-ONE-SYSTEM A comparative research document is a **self-contained, evergreen
study of one external system** — a competitor, a predecessor, an
adjacent tool that occupies ground near yours. @impl/done

##IT-INVENTORIES-QUOTES-MEASURES-AND-TRANSLATES It inventories what
that system does, quotes it in its own words, measures it in two
directions against your own project, and translates the actionable
findings into numbered roadmap proposals. @impl/done

##evergreen-is-the-load-bearing-adjective "Evergreen" is the load-bearing adjective. @impl/done

##THE-DOCUMENT-IS-WRITTEN-TO-OUTLAST-ITS-SOURCES The document is written
to be **re-readable months after publication without referring to
the original sources, and to outlast any one external project's URL
stability**. @impl/done

- ##ROT-MARKETING-PAGES-GET-REDESIGNED Marketing pages get redesigned, @spec/done
- ##ROT-DOCS-SITES-MOVE docs sites move, @spec/done
- ##ROT-PRODUCTS-GET-ACQUIRED-AND-DELETED whole
  products get acquired and deleted. @spec/done

##a-study-that-links-out-expires A study that merely links out is
a study that expires. @spec/done

##a-quoted-study-stands-after-the-links-rot A study that quotes verbatim with dates is a
study that stands on its own after the links rot. @spec/done

## Why the genre exists {#why}

##three-costs-justify-the-effort-lead Three costs justify the effort of a full study over a quick glance: @spec/done

- ##COST-UNEXAMINED-COMPETITORS-COST-ROADMAP-MISTAKES **Unexamined competitors cost roadmap mistakes.** Building in a
  space someone else already occupies, without knowing what they
  built, means re-discovering their dead ends at your own expense —
  or missing the one feature that was the whole point. @spec/done
- ##COST-EXAMINED-COMPETITORS-YIELD-DELTAS **Examined competitors yield deltas.** Understanding what a rival
  does well — *and what it does not* — is load-bearing intelligence
  for your own roadmap. Every gap you find is a candidate decision:
  close it, decline it on record, or note it as already led. @spec/done
- ##COST-THE-STUDY-MUST-OUTLIVE-LINK-ROT **The study must outlive link rot and staff turnover.** The person
  who did the research leaves; the URL 404s; the competitor pivots.
  What remains is the document, and the document is only worth
  keeping if it is complete on its own terms. @spec/done

## The five laws {#laws}

##EVERY-DOCUMENT-OBEYS-ALL-FIVE-LAWS Every comparative research document obeys all five. @impl/done

##the-laws-separate-a-study-from-a-rant They are what
separate a study from a bookmark dump or a competitive-envy rant. @spec/done

### Law 1 — Self-containedness {#law-self-contained}

##LAW-ONE-THE-DOCUMENT-MUST-STAND-WHEN-ITS-SOURCES-VANISH The document must stand when its sources vanish. @impl/done

##LAW-ONE-QUOTE-VERBATIM-WITH-AN-ACCESS-DATE Quote the subject
**verbatim in fenced blocks, each carrying an access date**, rather
than linking and trusting the link. @impl/done

##LAW-ONE-A-FUTURE-READER-RECONSTRUCTS-WITHOUT-FETCHING A future reader with no network
access, opening the file a year later, must be able to reconstruct
what the subject claimed without fetching anything. @impl/done

##LAW-ONE-LINKS-ARE-POINTERS-QUOTES-ARE-EVIDENCE Links are
pointers for refresh; quotes are the evidence of record. @impl/done

### Law 2 — Quote first, critique second {#law-quote-first}

##LAW-TWO-THE-SUBJECT-SPEAKS-BEFORE-IT-IS-JUDGED The subject speaks **in its own words before it is judged**. @impl/done

##LAW-TWO-PRESENT-THE-QUOTE-THEN-ANALYZE Present
the verbatim quote — the pitch, the feature description, the design
claim — and only *then* analyze, praise, or fault it. @impl/done

##critiquing-a-paraphrase-is-critiquing-a-straw-man Critiquing a
paraphrase is critiquing a straw man: the paraphrase is already your
reading, and a reader cannot check your judgement against a summary
you wrote. @spec/done

##LAW-TWO-QUOTE-THEN-JUDGE-EVERY-TIME Quote, then judge, in that order, every time. @impl/done

### Law 3 — Two-way gaps {#law-two-way}

##law-three-runs-in-both-directions-lead The analysis runs in **both directions**: @impl/done

- ##LAW-THREE-A-SECTION-FOR-WHERE-YOU-TRAIL a section for where you
  trail the subject, @impl/done
- ##LAW-THREE-A-SECTION-FOR-WHERE-YOU-LEAD and a section for where you lead it. @impl/done

##one-directional-gap-analysis-is-advocacy-lead One-directional
gap analysis is not analysis — it is advocacy: @spec/done

| One-directional study | What it actually is |
|---|---|
| ##ROW-ONE-DIRECTIONAL-TRAIL-ONLY Only finds where you trail @spec/done | Marketing for the competitor @spec/done |
| ##ROW-ONE-DIRECTIONAL-LEAD-ONLY Only finds where you lead @spec/done | Marketing for yourself @spec/done |
| ##ROW-FINDS-BOTH-ARGUED-EQUALLY Finds both, argued equally @spec/done | Intelligence you can act on @spec/done |

##WHERE-YOU-LEAD-DESERVES-THE-SAME-RIGOR Where you lead deserves the same rigor as where you trail: name the
decision you made that they did not, and say why it matters. @impl/done

##envy-blinds-you-to-the-moat-you-already-have Envy in
one direction blinds you to the moat you already have. @spec/done

### Law 4 — Deltas, not decrees {#law-deltas}

##LAW-FOUR-FINDINGS-BECOME-NUMBERED-PRIORITIZED-HOMED-DELTAS Actionable findings become **numbered roadmap deltas**, each with a
**priority** and a **target home** — "maps to a future spec section",
not a change ratified here. @impl/done

##LAW-FOUR-THE-DOCUMENT-PROPOSES-IT-DOES-NOT-DECIDE The research document *proposes*; it does
not *decide*. @impl/done

##a-self-ratifying-study-skips-the-review A study that quietly rewrites the roadmap inside itself
has skipped the review where a human weighs the proposal against
everything else competing for the same effort. @spec/done

##LAW-FOUR-KEEP-THE-TWO-ACTS-SEPARATE Keep the two acts
separate: the study argues, the owner decides
([`from-research-to-roadmap.md`](from-research-to-roadmap.md)). @impl/done

### Law 5 — The re-fetch list {#law-refetch}

##LAW-FIVE-THE-DOCUMENT-CLOSES-WITH-THE-RE-FETCH-LIST The document closes with **every source URL, its access date, and
the subject's version at capture** — the exact list needed to refresh
the study later. @impl/done

##the-study-has-a-shelf-life Because the subject keeps shipping, the study has a
shelf life; the re-fetch list is what lets a future session *update*
the study instead of starting over. @impl/done

##record-enough-that-the-refresh-is-mechanical-lead Record enough that the refresh is
mechanical: @impl/done

- ##REFETCH-WHICH-URLS which URLs, @impl/done
- ##REFETCH-IN-WHAT-ORDER in what order, @impl/done
- ##REFETCH-WHAT-VERSION-THIS-CAPTURE-REFLECTS and what version number this
  capture reflects. @impl/done

## When to write one {#when}

##not-every-glance-warrants-a-full-study Not every glance at a competitor warrants a full study. Write one
when: @spec/done

| Trigger | Why it warrants the full genre |
|---|---|
| ##ROW-TRIGGER-BEFORE-BUILDING-IN-AN-OCCUPIED-SPACE Before building in a space others occupy @spec/done | Cheaper to learn their dead ends than to re-walk them @spec/done |
| ##ROW-TRIGGER-A-COMPETITOR-SHIPS-SOMETHING-ALARMING A competitor ships something alarming @spec/done | An emotional reaction needs a structured study to become a decision @spec/done |
| ##ROW-TRIGGER-A-RECURRING-WHY-NOT-QUESTION "Why don't we just do what X does?" recurs @spec/done | A recurring question deserves a durable, citable answer, not a repeated verbal one @spec/done |

##a-one-off-lookup-does-not-warrant-a-study For a one-off factual lookup — "does X support Windows?" — a study is
overkill; answer it and move on. @impl/done

##THE-GENRE-IS-FOR-A-SYSTEM-WORTH-UNDERSTANDING-WHOLE The genre is for a *system* worth
understanding whole, whose shape will inform decisions more than once. @impl/done

## What it is not {#not}

- ##NOT-A-DECISION **Not a decision.** It proposes deltas; ratification is downstream. @impl/done
- ##NOT-A-RANT **Not a rant.** Competitive frustration is the trigger; the
  document is the disciplined product, two-way and quoted. @impl/done
- ##NOT-A-LIVE-DASHBOARD **Not a live dashboard.** It is a dated snapshot with a refresh
  procedure — a baseline, not a feed. When it ages, refresh it via
  the re-fetch list; keep the old capture as historical record. @impl/done
- ##NOT-A-LINK-FARM **Not a link farm.** Links rot; the quotes are the evidence. @impl/done

## Re-derive for your project {#re-derive}

##re-derive-lead Do not copy this protocol's framing verbatim — copy the *task*, and
let the agent produce the study your project actually needs: @impl/done

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

- ##SUM-A-SELF-CONTAINED-EVERGREEN-STUDY A comparative research document is a self-contained, evergreen
  study of one external system, written to outlast its own sources. @impl/done
- ##SUM-WHY-THE-GENRE-EXISTS It exists because unexamined competitors cost roadmap mistakes and
  examined ones yield deltas — and the study must survive link rot. @spec/done
- ##SUM-THE-FIVE-LAWS Five laws: self-containedness (dated verbatim quotes), quote-first
  then critique, two-way gaps (trail *and* lead), deltas-not-decrees
  (numbered, prioritized, homed, unratified), and the re-fetch list. @impl/done
- ##SUM-WHEN-TO-WRITE-ONE Write one before building in an occupied space, when a competitor
  alarms, or when "why not do what X does" keeps recurring. @impl/done
- ##SUM-THE-STUDY-PROPOSES-THE-OWNER-DECIDES The study proposes; the owner decides. See
  [`from-research-to-roadmap.md`](from-research-to-roadmap.md). @impl/done
