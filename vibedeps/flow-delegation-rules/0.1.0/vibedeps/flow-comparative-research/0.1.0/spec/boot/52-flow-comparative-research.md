# Flow: Comparative Research {#root}

This project has a genre for studying external systems — a
competitor, a predecessor, an adjacent tool. A **comparative
research document** is a self-contained, evergreen study: readable
months after publication without the original sources, structured
as a two-way gap analysis, closing with numbered roadmap deltas.
Genre law:
[`spec/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.md`](../flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.md).

## When to reach for it {#when}

When the user asks to study, evaluate, or compare against an
external system — "what does X actually do?", "should we copy
X's feature?", "audit that space before we build" — start from
the skeleton in
[`spec/flows/comparative-research/research-template.md`](../flows/comparative-research/research-template.md)
and hold the laws:

- **Quote first, critique second.** The subject speaks in its own
  words — fenced verbatim quotes with access dates — before any
  judgement is written.
- **Two-way gaps.** One section for where we trail, one for where
  we lead, argued with equal rigor.
- **Deltas, not decrees.** Actionable findings become numbered
  proposals, each with a priority and a target home in the spec
  tree. The study never ratifies its own proposals; acceptance
  happens downstream, per
  [`from-research-to-roadmap.md`](../flows/comparative-research/from-research-to-roadmap.md).
- **Re-fetch list.** Every source URL with access date, plus the
  subject's version at capture, so the study can be refreshed
  instead of rewritten.

## Never {#never}

- Never paraphrase where a dated verbatim quote can stand — the
  quote survives link rot; the paraphrase decays into rumor.
- Never write a one-directional gap analysis — trail-only is
  marketing for the subject, lead-only is marketing for us.
- Never ratify a delta inside the research doc — it proposes;
  acceptance is recorded downstream.
- Never let a study silently outlive its subject's next major
  release — stale-flag it and refresh via the re-fetch list.
