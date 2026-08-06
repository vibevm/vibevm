# The disclosure alternative {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines the *open-disclosure*
attribution posture — the first-class alternative to this package's
concealment default: *what* it consists of, *when* it is the right
choice (and when it is the only available choice), *what* it costs,
and *how* to switch postures without corrupting history. @status:impl/done

## The posture {#posture}

@fact:every-marked-artifact-carries-a-consistent-mark-lead Every artifact with AI involvement carries an explicit, consistent
mark: @status:impl/done

- @fact:DISCLOSURE-MARK-COMMITS **Commits:** a trailer per assisting system, e.g.
  `Co-Authored-By: <model name> <noreply@vendor.example>`, applied
  uniformly — every AI-assisted commit, the same trailer format,
  no exceptions. @status:impl/done
- @fact:DISCLOSURE-MARK-GENERATED-FILES **Substantial generated files:** a header line naming the
  generator, where the file's ecosystem has such a convention. @status:impl/done
- @fact:DISCLOSURE-MARK-RELEASE-NOTES **Release notes / README:** one standing sentence describing the
  project's use of AI assistance, kept current. @status:impl/done

@fact:CONSISTENCY-IS-THE-WHOLE-VALUE Consistency is the whole value. @status:impl/done

@fact:a-partial-record-reads-as-a-false-claim A provenance record that marks some
AI-assisted commits and not others is worse than either posture done
properly — it reads as a claim ("unmarked = human-only") that is
false. @status:spec/done

## When to choose it {#when}

@fact:when-it-is-mandatory-lead **When it is mandatory — no choice exists:** @status:spec/done

- @fact:MANDATORY-LAW-OR-REGULATION Law or regulation applicable to the owner requires disclosing AI
  involvement in delivered software. @status:spec/done
- @fact:MANDATORY-EMPLOYER-OR-CLIENT-POLICY The employer's or client's policy requires it. @status:spec/done
- @fact:MANDATORY-UPSTREAM-OR-REGISTRY The upstream project or registry you contribute to requires it —
  a contribution follows the destination's rules, whatever your own
  repository does. @status:spec/done

@fact:when-it-is-the-better-fit-lead **When it is the better fit even though optional:** @status:spec/done

- @fact:BETTER-FIT-RESEARCH-AND-AUDIT Research and audit contexts, where provenance is the point — which
  model wrote what is data you will want later. @status:spec/done
- @fact:BETTER-FIT-MEASURING-ASSISTANCE-IMPACT Teams measuring AI-assistance impact: the trailer *is* the
  dataset ("what fraction of shipped commits had AI involvement"
  becomes one `git log` query). @status:spec/done
- @fact:BETTER-FIT-AI-FORWARD-PROJECTS Projects whose public stance is AI-forward, where the mark is
  marketing rather than liability. @status:spec/done

## What it costs {#costs}

- @fact:COST-PERMANENCE **Permanence.** Git history is immutable-by-convention; every
  trailer is a permanent record. If regulation later attaches
  consequences to machine authorship, the record is already
  published and cannot be cleanly withdrawn. @status:spec/done
- @fact:COST-CONSISTENCY-BURDEN **Consistency burden.** The posture is only truthful if enforced
  as strictly as concealment — every tool, every contributor, every
  session, the same marks. The enforcement checklist applies with
  the polarity flipped (scan for *missing* trailers, not present
  ones). @status:impl/done
- @fact:COST-HETEROGENEITY **Heterogeneity.** Multi-tool teams produce differently-shaped
  marks unless the format is pinned; pin it in the same single
  place the policy lives. @status:spec/done

## Posture comparison {#comparison}

| | Concealment (default) | Disclosure |
|---|---|---|
| @fact:ROW-COMPARE-REPOSITORY-SURFACE Repository surface @status:impl/done | human-authored, uniformly @status:impl/done | provenance-marked, uniformly @status:impl/done |
| @fact:ROW-COMPARE-REGULATORY-POSTURE Regulatory posture @status:spec/done | no hook for future regulation @status:spec/done | compliant where disclosure is required @status:spec/done |
| @fact:ROW-COMPARE-PROVENANCE-DATA Provenance data @status:impl/done | none in artifacts (sessions/logs only) @status:impl/done | in-history, queryable @status:impl/done |
| @fact:ROW-COMPARE-MAIN-RISK Main risk @status:spec/done | none under present law; posture must be re-verified as law changes @status:spec/done | permanent record; withdrawal impossible @status:spec/done |
| @fact:ROW-COMPARE-ENFORCEMENT Enforcement @status:impl/done | scan for present marks @status:impl/done | scan for missing marks @status:impl/done |

## Switching postures {#switching}

@fact:a-posture-change-is-forward-only-lead A posture change is **forward-only**: @status:impl/done

1. @fact:SWITCH-EDIT-THE-SINGLE-POLICY-PLACE The owner edits the single policy place (the boot snippet) to the
   new posture, with a dated decision record and a revisit trigger
   (see `flow:decision-records`). @status:impl/done
2. @fact:SWITCH-NEW-WORK-FOLLOWS-FROM-THAT-COMMIT New work follows the new posture from that commit on. @status:impl/done
3. @fact:SWITCH-HISTORY-IS-NEVER-REWRITTEN **Existing history is never rewritten to match** — in either
   direction. Scrubbing old trailers or back-filling missing ones
   both mean rewriting published history; the frozen-history rule
   (`flow:git-atomic-commits`) wins. The dated decision record is what
   tells a future reader where the boundary lies. @status:impl/done

## Summary {#summary}

- @fact:SUM-MANDATORY-WHERE-REQUIRED Disclosure is the mandatory posture where law, employer, or
  upstream requires it, and a legitimate choice where provenance is
  worth more than optionality. @status:spec/done
- @fact:SUM-VALUE-IS-CONSISTENCY Its value is consistency; a partial record is worse than either
  clean posture. @status:impl/done
- @fact:SUM-COST-IS-PERMANENCE Its cost is permanence — a published record that cannot be
  withdrawn. @status:spec/done
- @fact:SUM-SWITCHING-IS-FORWARD-ONLY Switching postures is one edit in one place, forward-only, with a
  dated decision record; history is never rewritten to match. @status:impl/done
