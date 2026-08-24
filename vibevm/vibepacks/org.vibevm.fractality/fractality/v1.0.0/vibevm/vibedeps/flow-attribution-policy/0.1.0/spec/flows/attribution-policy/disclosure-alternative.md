# The disclosure alternative {#root}

**Scope of this document.** This file defines the *open-disclosure*
attribution posture — the first-class alternative to this package's
concealment default: *what* it consists of, *when* it is the right
choice (and when it is the only available choice), *what* it costs,
and *how* to switch postures without corrupting history.

## The posture {#posture}

Every artifact with AI involvement carries an explicit, consistent
mark:

- **Commits:** a trailer per assisting system, e.g.
  `Co-Authored-By: <model name> <noreply@vendor.example>`, applied
  uniformly — every AI-assisted commit, the same trailer format,
  no exceptions.
- **Substantial generated files:** a header line naming the
  generator, where the file's ecosystem has such a convention.
- **Release notes / README:** one standing sentence describing the
  project's use of AI assistance, kept current.

Consistency is the whole value. A provenance record that marks some
AI-assisted commits and not others is worse than either posture done
properly — it reads as a claim ("unmarked = human-only") that is
false.

## When to choose it {#when}

**When it is mandatory — no choice exists:**

- Law or regulation applicable to the owner requires disclosing AI
  involvement in delivered software.
- The employer's or client's policy requires it.
- The upstream project or registry you contribute to requires it —
  a contribution follows the destination's rules, whatever your own
  repository does.

**When it is the better fit even though optional:**

- Research and audit contexts, where provenance is the point — which
  model wrote what is data you will want later.
- Teams measuring AI-assistance impact: the trailer *is* the
  dataset ("what fraction of shipped commits had AI involvement"
  becomes one `git log` query).
- Projects whose public stance is AI-forward, where the mark is
  marketing rather than liability.

## What it costs {#costs}

- **Permanence.** Git history is immutable-by-convention; every
  trailer is a permanent record. If regulation later attaches
  consequences to machine authorship, the record is already
  published and cannot be cleanly withdrawn.
- **Consistency burden.** The posture is only truthful if enforced
  as strictly as concealment — every tool, every contributor, every
  session, the same marks. The enforcement checklist applies with
  the polarity flipped (scan for *missing* trailers, not present
  ones).
- **Heterogeneity.** Multi-tool teams produce differently-shaped
  marks unless the format is pinned; pin it in the same single
  place the policy lives.

## Posture comparison {#comparison}

| | Concealment (default) | Disclosure |
|---|---|---|
| Repository surface | human-authored, uniformly | provenance-marked, uniformly |
| Regulatory posture | no hook for future regulation | compliant where disclosure is required |
| Provenance data | none in artifacts (sessions/logs only) | in-history, queryable |
| Main risk | none under present law; posture must be re-verified as law changes | permanent record; withdrawal impossible |
| Enforcement | scan for present marks | scan for missing marks |

## Switching postures {#switching}

A posture change is **forward-only**:

1. The owner edits the single policy place (the boot snippet) to the
   new posture, with a dated decision record and a revisit trigger
   (see `flow:decision-records`).
2. New work follows the new posture from that commit on.
3. **Existing history is never rewritten to match** — in either
   direction. Scrubbing old trailers or back-filling missing ones
   both mean rewriting published history; the frozen-history rule
   (`flow:atomic-commits`) wins. The dated decision record is what
   tells a future reader where the boundary lies.

## Summary {#summary}

- Disclosure is the mandatory posture where law, employer, or
  upstream requires it, and a legitimate choice where provenance is
  worth more than optionality.
- Its value is consistency; a partial record is worse than either
  clean posture.
- Its cost is permanence — a published record that cannot be
  withdrawn.
- Switching postures is one edit in one place, forward-only, with a
  dated decision record; history is never rewritten to match.
