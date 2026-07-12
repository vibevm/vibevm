# Comparative research template {#root}

**Scope of this document.** A copy-ready skeleton for a comparative
research document, then a clause-by-clause commentary on each
section, then one short worked fragment. The laws these clauses
enforce live in
[`COMPARATIVE-RESEARCH-PROTOCOL.md`](COMPARATIVE-RESEARCH-PROTOCOL.md);
this file is the shape you fill.

## The skeleton {#skeleton}

Copy this whole block into a new research document and fill each
placeholder. Keep the section order — it is the reading shape a
cold reader navigates by.

```markdown
# <SUBJECT> comparative research and <PROJECT> roadmap deltas

**Status.** Research document — self-contained, evergreen reference.
It ratifies nothing; each numbered delta in the deltas section maps
to a future spec home.

**Purpose.** <SUBJECT> (<primary URL>) occupies ground adjacent to
<PROJECT>: <one sentence on the overlap>. Understanding what they do
well — and what they don't — is load-bearing intelligence for our
own roadmap. This document inventories <SUBJECT> as of <capture
date>, finds where we trail and where we lead, and turns the
actionable subset into numbered deltas.

**Source corpus.** <where the material came from — docs, pitch,
changelog>. Verbatim quotes appear in fenced blocks with dates
throughout. Refresh via the re-fetch list at the end.

| Source | URL | Accessed | Subject version |
|--------|-----|----------|-----------------|
| <name> | <url> | <YYYY-MM-DD> | <version> |

**Reading shape.** §1 the subject in its own words · §2 capability
inventory · §3 where we trail · §4 where we lead · §5 numbered
deltas · §6 open questions · §7 re-fetch list.

---

## 1. The subject in its own words {#subject-words}

> "<verbatim pitch, quoted exactly>"
> — <source>, accessed <YYYY-MM-DD>

<One paragraph: their framing, restated only after the quote stands.>

## 2. Capability inventory {#inventory}

For each capability: the verbatim claim, then what it maps to in
our project.

### 2.1 <capability name>

> "<verbatim quote>"
> — <source>, accessed <YYYY-MM-DD>

**Maps to.** <our nearest equivalent, or "no equivalent">.

## 3. Where we trail {#trail}

Gaps where the subject does something we do not. Each names the
capability, what it would take to match, and the rough size.

### 3.1 <gap name>
**Their capability.** <what they have, quoted above in §2.>
**Our gap.** <what we lack.>
**Size.** <rough effort.>

## 4. Where we lead {#lead}

Decisions in our architecture the subject has not made, made
differently in a way we believe is wrong, or not exposed.

### 4.1 <lead name>
<What we do that they do not, and why it matters — same rigor as §3.>

## 5. Roadmap deltas {#deltas}

Numbered proposals. Each carries a priority and a target home. This
document ratifies none of them.

### D1 — <short delta title>
Maps to §3.1. **Priority:** HIGH | MEDIUM | LOW.
**Target home:** <future spec section / milestone>.
<One paragraph on the proposal.>

## 6. Open questions {#open}

- <what the corpus did not answer; what to ask on refresh.>

## 7. Re-fetch list {#refetch}

To refresh this study, re-fetch in this order and re-audit the delta
table against what changed:

- <url> — <what it provides> — accessed <YYYY-MM-DD>.

**Capture date:** <YYYY-MM-DD>. **Subject version at capture:**
<version>.
```

## Clause-by-clause commentary {#commentary}

- **Status line.** State up front that the document ratifies
  nothing. This is the deltas-not-decrees law made visible on line
  one, so no reader mistakes a proposal for a decision.
- **Purpose.** One paragraph, and it must contain the two-way promise
  — "what they do well *and* what they don't." A purpose that
  promises only trailing gaps has pre-committed to a one-directional
  study.
- **Source corpus + table.** Every source with an access date and the
  subject's version. This table is half of the re-fetch list; filling
  it now is cheaper than reconstructing it later.
- **Reading shape.** A one-line section map so a cold reader navigates
  without reading linearly. Months later, this is how a refresher
  finds the delta table in ten seconds.
- **§1 in their own words.** The subject's pitch, quoted, before any
  analysis. If your document opens with your *summary* of their pitch,
  you have already broken the quote-first law.
- **§2 inventory.** Quote-then-map, one capability at a time. "Maps
  to: no equivalent" is a finding, not a blank — it is a §3 gap in
  the making.
- **§3 trail / §4 lead.** Both are mandatory. A document that ships
  with §4 empty has not looked hard enough — every project made
  *some* decision its competitor did not.
- **§5 deltas.** Numbered, prioritized, homed. The number is what the
  downstream pipeline cites when a delta is accepted or rejected
  ([`from-research-to-roadmap.md`](from-research-to-roadmap.md)).
- **§6 open questions.** What the sources did not answer. Honest gaps
  in the study itself, and the first questions to ask on refresh.
- **§7 re-fetch list.** The refresh recipe: which URLs in what order,
  and the capture version — without it, refresh becomes rewrite.

## Worked fragment {#worked}

A short invented illustration — a study of a generic build tool
*Quarry*: two quotes, one delta. Real documents run longer.

```markdown
## 1. Quarry in its own words {#quarry-words}

> "Quarry caches every build step by content hash, so a clean build
> and an incremental build take the same time when nothing changed."
> — quarry.example/docs/caching, accessed 2026-07-01

Their pitch is determinism-as-speed: identical inputs, identical
outputs, served from cache.

## 2.1 Remote cache sharing

> "Team members share one remote cache; a step built on CI is never
> rebuilt on a laptop."
> — quarry.example/docs/remote, accessed 2026-07-01

**Maps to.** No equivalent — our cache is per-machine only.

## 3.1 Shared remote cache
**Their capability.** CI-built steps served to every developer.
**Our gap.** We hash steps locally but never share them.
**Size.** One storage backend + a cache-key protocol; ~medium.

## 4.1 Offline-first by default
Quarry's remote cache assumes connectivity; a laptop offline pays
full rebuild cost. Ours is local-first — no network on the hot path.
The trade is deliberate: we chose air-gapped reproducibility over
cross-machine reuse, and that is a moat for regulated users.

## 5. Roadmap deltas
### D1 — Optional shared remote cache
Maps to §3.1. **Priority:** MEDIUM.
**Target home:** a future caching spec section, behind an opt-in flag
so offline-first stays the default. Ratified here: nothing.
```

The fragment obeys all five laws in miniature: dated verbatim quotes
(Laws 1-2), §4 answering §3 the other way (Law 3), D1 numbered and
ratifying nothing (Law 4), every quote-URL refetchable (Law 5).

## Summary {#summary}

- Copy the skeleton whole; keep the section order — it is the reading
  shape.
- Fill the source table as you fetch; it is half the re-fetch list.
- Quote before mapping; map "no equivalent" as a real finding.
- Ship §3 *and* §4; an empty lead section means you stopped early.
- Number, prioritize, and home every delta; ratify none.
