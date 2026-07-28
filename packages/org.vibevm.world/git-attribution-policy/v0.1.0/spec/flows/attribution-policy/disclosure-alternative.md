# The disclosure alternative {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines the *open-disclosure*
attribution posture — the first-class alternative to this package's
concealment default: *what* it consists of, *when* it is the right
choice (and when it is the only available choice), *what* it costs,
and *how* to switch postures without corrupting history. @impl/done

## The posture {#posture}

##every-marked-artifact-carries-a-consistent-mark-lead Every artifact with AI involvement carries an explicit, consistent
mark: @impl/done

- ##DISCLOSURE-MARK-COMMITS **Commits:** a trailer per assisting system, e.g.
  `Co-Authored-By: <model name> <noreply@vendor.example>`, applied
  uniformly — every AI-assisted commit, the same trailer format,
  no exceptions. @impl/done
- ##DISCLOSURE-MARK-GENERATED-FILES **Substantial generated files:** a header line naming the
  generator, where the file's ecosystem has such a convention. @impl/done
- ##DISCLOSURE-MARK-RELEASE-NOTES **Release notes / README:** one standing sentence describing the
  project's use of AI assistance, kept current. @impl/done

##CONSISTENCY-IS-THE-WHOLE-VALUE Consistency is the whole value. @impl/done

##a-partial-record-reads-as-a-false-claim A provenance record that marks some
AI-assisted commits and not others is worse than either posture done
properly — it reads as a claim ("unmarked = human-only") that is
false. @spec/done

## When to choose it {#when}

##when-it-is-mandatory-lead **When it is mandatory — no choice exists:** @spec/done

- ##MANDATORY-LAW-OR-REGULATION Law or regulation applicable to the owner requires disclosing AI
  involvement in delivered software. @spec/done
- ##MANDATORY-EMPLOYER-OR-CLIENT-POLICY The employer's or client's policy requires it. @spec/done
- ##MANDATORY-UPSTREAM-OR-REGISTRY The upstream project or registry you contribute to requires it —
  a contribution follows the destination's rules, whatever your own
  repository does. @spec/done

##when-it-is-the-better-fit-lead **When it is the better fit even though optional:** @spec/done

- ##BETTER-FIT-RESEARCH-AND-AUDIT Research and audit contexts, where provenance is the point — which
  model wrote what is data you will want later. @spec/done
- ##BETTER-FIT-MEASURING-ASSISTANCE-IMPACT Teams measuring AI-assistance impact: the trailer *is* the
  dataset ("what fraction of shipped commits had AI involvement"
  becomes one `git log` query). @spec/done
- ##BETTER-FIT-AI-FORWARD-PROJECTS Projects whose public stance is AI-forward, where the mark is
  marketing rather than liability. @spec/done

## What it costs {#costs}

- ##COST-PERMANENCE **Permanence.** Git history is immutable-by-convention; every
  trailer is a permanent record. If regulation later attaches
  consequences to machine authorship, the record is already
  published and cannot be cleanly withdrawn. @spec/done
- ##COST-CONSISTENCY-BURDEN **Consistency burden.** The posture is only truthful if enforced
  as strictly as concealment — every tool, every contributor, every
  session, the same marks. The enforcement checklist applies with
  the polarity flipped (scan for *missing* trailers, not present
  ones). @impl/done
- ##COST-HETEROGENEITY **Heterogeneity.** Multi-tool teams produce differently-shaped
  marks unless the format is pinned; pin it in the same single
  place the policy lives. @spec/done

## Posture comparison {#comparison}

| | Concealment (default) | Disclosure |
|---|---|---|
| ##ROW-COMPARE-REPOSITORY-SURFACE Repository surface @impl/done | human-authored, uniformly @impl/done | provenance-marked, uniformly @impl/done |
| ##ROW-COMPARE-REGULATORY-POSTURE Regulatory posture @spec/done | no hook for future regulation @spec/done | compliant where disclosure is required @spec/done |
| ##ROW-COMPARE-PROVENANCE-DATA Provenance data @impl/done | none in artifacts (sessions/logs only) @impl/done | in-history, queryable @impl/done |
| ##ROW-COMPARE-MAIN-RISK Main risk @spec/done | none under present law; posture must be re-verified as law changes @spec/done | permanent record; withdrawal impossible @spec/done |
| ##ROW-COMPARE-ENFORCEMENT Enforcement @impl/done | scan for present marks @impl/done | scan for missing marks @impl/done |

## Switching postures {#switching}

##a-posture-change-is-forward-only-lead A posture change is **forward-only**: @impl/done

1. ##SWITCH-EDIT-THE-SINGLE-POLICY-PLACE The owner edits the single policy place (the boot snippet) to the
   new posture, with a dated decision record and a revisit trigger
   (see `flow:decision-records`). @impl/done
2. ##SWITCH-NEW-WORK-FOLLOWS-FROM-THAT-COMMIT New work follows the new posture from that commit on. @impl/done
3. ##SWITCH-HISTORY-IS-NEVER-REWRITTEN **Existing history is never rewritten to match** — in either
   direction. Scrubbing old trailers or back-filling missing ones
   both mean rewriting published history; the frozen-history rule
   (`flow:git-atomic-commits`) wins. The dated decision record is what
   tells a future reader where the boundary lies. @impl/done

## Summary {#summary}

- ##SUM-MANDATORY-WHERE-REQUIRED Disclosure is the mandatory posture where law, employer, or
  upstream requires it, and a legitimate choice where provenance is
  worth more than optionality. @spec/done
- ##SUM-VALUE-IS-CONSISTENCY Its value is consistency; a partial record is worse than either
  clean posture. @impl/done
- ##SUM-COST-IS-PERMANENCE Its cost is permanence — a published record that cannot be
  withdrawn. @spec/done
- ##SUM-SWITCHING-IS-FORWARD-ONLY Switching postures is one edit in one place, forward-only, with a
  dated decision record; history is never rewritten to match. @impl/done
