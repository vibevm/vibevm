# Flow: Comparative Research {#root}

<status stage="impl" state="done"/>

##THE-PROJECT-HAS-A-GENRE-FOR-STUDYING-EXTERNAL-SYSTEMS This project has a genre for studying external systems — a
competitor, a predecessor, an adjacent tool. @impl/done

##A-RESEARCH-DOCUMENT-IS-SELF-CONTAINED-AND-EVERGREEN A **comparative
research document** is a self-contained, evergreen study: readable
months after publication without the original sources, structured
as a two-way gap analysis, closing with numbered roadmap deltas. @impl/done

##sibling-document-pointers
Genre law:
@spec://org.vibevm.world/comparative-research/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL#root. @impl/done

## When to reach for it {#when}

##reach-for-the-skeleton-and-hold-the-laws-lead When the user asks to study, evaluate, or compare against an
external system — "what does X actually do?", "should we copy
X's feature?", "audit that space before we build" — start from
the skeleton in
@spec://org.vibevm.world/comparative-research/flows/comparative-research/research-template#root
and hold the laws: @impl/done

- ##LAW-QUOTE-FIRST-CRITIQUE-SECOND **Quote first, critique second.** The subject speaks in its own
  words — fenced verbatim quotes with access dates — before any
  judgement is written. @impl/done
- ##LAW-TWO-WAY-GAPS **Two-way gaps.** One section for where we trail, one for where
  we lead, argued with equal rigor. @impl/done
- ##LAW-DELTAS-NOT-DECREES **Deltas, not decrees.** Actionable findings become numbered
  proposals, each with a priority and a target home in the spec
  tree. The study never ratifies its own proposals; acceptance
  happens downstream, per
  @spec://org.vibevm.world/comparative-research/flows/comparative-research/from-research-to-roadmap#root. @impl/done
- ##LAW-THE-RE-FETCH-LIST **Re-fetch list.** Every source URL with access date, plus the
  subject's version at capture, so the study can be refreshed
  instead of rewritten. @impl/done

## Never {#never}

- ##NEVER-PARAPHRASE-WHERE-A-QUOTE-CAN-STAND Never paraphrase where a dated verbatim quote can stand — the
  quote survives link rot; the paraphrase decays into rumor. @impl/done
- ##NEVER-WRITE-A-ONE-DIRECTIONAL-GAP-ANALYSIS Never write a one-directional gap analysis — trail-only is
  marketing for the subject, lead-only is marketing for us. @impl/done
- ##NEVER-RATIFY-A-DELTA-INSIDE-THE-RESEARCH-DOC Never ratify a delta inside the research doc — it proposes;
  acceptance is recorded downstream. @impl/done
- ##NEVER-LET-A-STUDY-SILENTLY-GO-STALE Never let a study silently outlive its subject's next major
  release — stale-flag it and refresh via the re-fetch list. @impl/done
