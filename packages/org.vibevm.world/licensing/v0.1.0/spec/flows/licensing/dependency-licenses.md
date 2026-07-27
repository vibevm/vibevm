# Dependency licence discipline {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The operational side of the
permissive-only rule: which licences pass, which fail, how to check a
new dependency before adopting it, and how to keep the check honest
over time. @impl/done

##GUIDANCE-NOT-LEGAL-ADVICE Guidance, not legal advice. @impl/done

## The allow / deny table {#table}

| Class | Licences | Verdict |
|-------|----------|---------|
| ##ROW-CLASS-PERMISSIVE Permissive @spec/done | MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unlicense, Zlib, UPL-1.0 @spec/done | **allowed** @spec/done |
| ##ROW-CLASS-WEAK-COPYLEFT Weak copyleft @spec/done | MPL-2.0, EPL @spec/done | **case by case** — file-level copyleft usually does not taint consumers, but confirm the use @spec/done |
| ##ROW-CLASS-STRONG-COPYLEFT Strong copyleft @spec/done | GPL-2.0/3.0, AGPL, LGPL @spec/done | **forbidden by default** — an owner exception, if any, is explicit and recorded @spec/done |
| ##ROW-CLASS-UNCLEAR-OR-MISSING Unclear / missing @spec/done | no licence file, "all rights reserved", custom terms @spec/done | **treat as forbidden** until clarified — no licence means no rights @spec/done |

##why-lgpl-is-in-the-forbidden-row The `LGPL` entry is deliberately in the forbidden row: even though its
copyleft is weaker than GPL, its dynamic-linking obligations are easy
to violate accidentally in a statically-linked or bundled build, so
the default is no. @impl/done

## Checking a dependency before adoption {#check}

1. ##STEP-FIND-THE-LICENCE **Find the licence.** The package's manifest field, its
   `LICENSE` file, its repository. If these disagree, the most
   restrictive one wins until the discrepancy is resolved. @impl/done
2. ##STEP-CLASSIFY-IT-AGAINST-THE-TABLE **Classify it** against the table. Permissive → proceed.
   Case-by-case → confirm the specific obligation is met. Forbidden or
   unclear → stop and surface it as an owner decision. @impl/done
3. ##STEP-CHECK-THE-TRANSITIVE-SET **Check the transitive set, not just the direct dependency.** A
   permissive library that pulls a GPL transitive dependency is a GPL
   problem. Run the ecosystem's licence-listing over the whole
   resolved graph, not the top level. @impl/done
4. ##STEP-RECORD-THE-OUTCOME **Record the outcome** where dependency decisions live — an
   allowed non-obvious licence and any case-by-case call is a recorded
   decision with the reason. @impl/done

## Weight is not a licence concern {#weight}

##REJECT-FOR-LICENCE-ABANDONMENT-SECURITY-OR-API-NEVER-FOR-WEIGHT Reject a dependency for its licence, its abandonment, its security
history, or an API that would poison your interfaces — never for
being "heavy". @impl/done

##weight-is-not-a-reason-to-under-specify-a-load-bearing-component Binary size, crate/package count, and transitive
weight are not reasons to under-specify a load-bearing component;
that trades a one-time size cost for ongoing architectural debt. @spec/done

##KEEP-THE-TWO-CONVERSATIONS-SEPARATE Keep the two conversations separate: licence discipline here, weight
nowhere. @impl/done

## Keeping the check honest {#honest}

- ##AUTOMATE-THE-LISTING **Automate the listing.** A licence-lister run in CI over the full
  resolved graph turns "we think everything is permissive" into a
  fact that fails loudly when a forbidden licence enters. A rule with
  no checker is a wish. @impl/done
- ##RE-AUDIT-ON-A-SCHEDULE **Re-audit on a schedule.** A dependency can relicense between
  versions; a periodic audit line (see `flow:health-audit`) re-runs
  the listing and flags any new non-permissive entry. @impl/done
- ##POINT-THE-CARVE-OUT-AT-THE-GENERATED-LIST **Point the product's carve-out at the generated list**, never a
  hand-maintained copy — the hand copy drifts, the generated one
  cannot. @impl/done

## When a forbidden licence is the only option {#forbidden-only}

##sometimes-the-best-or-only-library-is-copyleft Sometimes the best-or-only library for a job is copyleft. @spec/done

##THAT-IS-AN-OWNER-DECISION-NOT-AN-AGENT-DECISION That is an
owner decision, not an agent decision. @impl/done

##surface-it-with-the-trade-off-stated-lead Surface it with the trade-off
stated: @impl/done

- ##TRADE-OFF-THE-CAPABILITY-GAINED the capability gained, @impl/done
- ##TRADE-OFF-THE-LICENCE-OBLIGATION-INCURRED the licence obligation incurred, @impl/done
- ##TRADE-OFF-THE-ALTERNATIVES and
  the alternatives (a permissive competitor, a reimplementation, doing
  without). @impl/done

##THE-OWNER-MAY-GRANT-A-RECORDED-SCOPED-EXCEPTION The owner may grant a recorded, scoped exception — or not. @impl/done

##NEVER-ADOPT-A-FORBIDDEN-LICENCE-SILENTLY Never adopt it silently. @impl/done

## Summary {#summary}

- ##SUM-THE-ALLOW-DENY-CLASSES Permissive allowed, weak copyleft case-by-case, strong copyleft and
  unclear licences forbidden by default. @impl/done
- ##SUM-CHECK-THE-WHOLE-TRANSITIVE-GRAPH Check the whole transitive graph before adopting; the most
  restrictive licence in the set governs. @impl/done
- ##SUM-WEIGHT-IS-NEVER-A-LICENCE-REASON Weight is never a licence reason — keep those conversations
  separate. @impl/done
- ##SUM-AUTOMATE-AND-RE-AUDIT Automate the listing in CI and re-audit on a schedule; point the
  carve-out at the generated list. @impl/done
- ##SUM-A-FORBIDDEN-LICENCE-NEEDS-A-RECORDED-OWNER-EXCEPTION A forbidden licence is only ever adopted by an explicit, recorded
  owner exception. @impl/done
