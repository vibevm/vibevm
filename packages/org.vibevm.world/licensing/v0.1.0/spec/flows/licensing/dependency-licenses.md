# Dependency licence discipline {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The operational side of the
permissive-only rule: which licences pass, which fail, how to check a
new dependency before adopting it, and how to keep the check honest
over time. @status:impl/done

@fact:GUIDANCE-NOT-LEGAL-ADVICE Guidance, not legal advice. @status:impl/done

## The allow / deny table {#table}

| Class | Licences | Verdict |
|-------|----------|---------|
| @fact:ROW-CLASS-PERMISSIVE Permissive @status:spec/done | MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unlicense, Zlib, UPL-1.0 @status:spec/done | **allowed** @status:spec/done |
| @fact:ROW-CLASS-WEAK-COPYLEFT Weak copyleft @status:spec/done | MPL-2.0, EPL @status:spec/done | **case by case** — file-level copyleft usually does not taint consumers, but confirm the use @status:spec/done |
| @fact:ROW-CLASS-STRONG-COPYLEFT Strong copyleft @status:spec/done | GPL-2.0/3.0, AGPL, LGPL @status:spec/done | **forbidden by default** — an owner exception, if any, is explicit and recorded @status:spec/done |
| @fact:ROW-CLASS-UNCLEAR-OR-MISSING Unclear / missing @status:spec/done | no licence file, "all rights reserved", custom terms @status:spec/done | **treat as forbidden** until clarified — no licence means no rights @status:spec/done |

@fact:why-lgpl-is-in-the-forbidden-row The `LGPL` entry is deliberately in the forbidden row: even though its
copyleft is weaker than GPL, its dynamic-linking obligations are easy
to violate accidentally in a statically-linked or bundled build, so
the default is no. @status:impl/done

## Checking a dependency before adoption {#check}

1. @fact:STEP-FIND-THE-LICENCE **Find the licence.** The package's manifest field, its
   `LICENSE` file, its repository. If these disagree, the most
   restrictive one wins until the discrepancy is resolved. @status:impl/done
2. @fact:STEP-CLASSIFY-IT-AGAINST-THE-TABLE **Classify it** against the table. Permissive → proceed.
   Case-by-case → confirm the specific obligation is met. Forbidden or
   unclear → stop and surface it as an owner decision. @status:impl/done
3. @fact:STEP-CHECK-THE-TRANSITIVE-SET **Check the transitive set, not just the direct dependency.** A
   permissive library that pulls a GPL transitive dependency is a GPL
   problem. Run the ecosystem's licence-listing over the whole
   resolved graph, not the top level. @status:impl/done
4. @fact:STEP-RECORD-THE-OUTCOME **Record the outcome** where dependency decisions live — an
   allowed non-obvious licence and any case-by-case call is a recorded
   decision with the reason. @status:impl/done

## Weight is not a licence concern {#weight}

@fact:REJECT-FOR-LICENCE-ABANDONMENT-SECURITY-OR-API-NEVER-FOR-WEIGHT Reject a dependency for its licence, its abandonment, its security
history, or an API that would poison your interfaces — never for
being "heavy". @status:impl/done

@fact:weight-is-not-a-reason-to-under-specify-a-load-bearing-component Binary size, crate/package count, and transitive
weight are not reasons to under-specify a load-bearing component;
that trades a one-time size cost for ongoing architectural debt. @status:spec/done

@fact:KEEP-THE-TWO-CONVERSATIONS-SEPARATE Keep the two conversations separate: licence discipline here, weight
nowhere. @status:impl/done

## Keeping the check honest {#honest}

- @fact:AUTOMATE-THE-LISTING **Automate the listing.** A licence-lister run in CI over the full
  resolved graph turns "we think everything is permissive" into a
  fact that fails loudly when a forbidden licence enters. A rule with
  no checker is a wish. @status:impl/done
- @fact:RE-AUDIT-ON-A-SCHEDULE **Re-audit on a schedule.** A dependency can relicense between
  versions; a periodic audit line (see `flow:health-audit`) re-runs
  the listing and flags any new non-permissive entry. *Specified, not built,
  and the sibling it points at does not carry the line: `flow:health-audit`'s
  `spec/flows/health-audit/audit-checklist.md` has one dependency category, D4
  · Dependency staleness, whose «Look for» and «Aid» rows name outdated
  versions and security advisories (`npm audit`, `cargo audit`,
  `cargo outdated`) and no licence at all — a search of that package's whole
  `spec/` for `licen` · `copyleft` · `GPL` · `permissive` · `SPDX` returns
  nothing. Nor is there a listing to re-run: no `deny.toml`, no `about.toml`,
  no SBOM or SPDX manifest anywhere in the perimeter, and a lockfile that
  records no licence field. The adopter must therefore author the line itself;
  `audit-checklist.md` is explicitly «a starting set, not a closed one», which
  is where it would go.* @status:spec/done
- @fact:POINT-THE-CARVE-OUT-AT-THE-GENERATED-LIST **Point the product's carve-out at the generated list**, never a
  hand-maintained copy — the hand copy drifts, the generated one
  cannot. @status:impl/done

## When a forbidden licence is the only option {#forbidden-only}

@fact:sometimes-the-best-or-only-library-is-copyleft Sometimes the best-or-only library for a job is copyleft. @status:spec/done

@fact:THAT-IS-AN-OWNER-DECISION-NOT-AN-AGENT-DECISION That is an
owner decision, not an agent decision. @status:impl/done

@fact:surface-it-with-the-trade-off-stated-lead Surface it with the trade-off
stated: @status:impl/done

- @fact:TRADE-OFF-THE-CAPABILITY-GAINED the capability gained, @status:impl/done
- @fact:TRADE-OFF-THE-LICENCE-OBLIGATION-INCURRED the licence obligation incurred, @status:impl/done
- @fact:TRADE-OFF-THE-ALTERNATIVES and
  the alternatives (a permissive competitor, a reimplementation, doing
  without). @status:impl/done

@fact:THE-OWNER-MAY-GRANT-A-RECORDED-SCOPED-EXCEPTION The owner may grant a recorded, scoped exception — or not. @status:impl/done

@fact:NEVER-ADOPT-A-FORBIDDEN-LICENCE-SILENTLY Never adopt it silently. @status:impl/done

## Summary {#summary}

- @fact:SUM-THE-ALLOW-DENY-CLASSES Permissive allowed, weak copyleft case-by-case, strong copyleft and
  unclear licences forbidden by default. @status:impl/done
- @fact:SUM-CHECK-THE-WHOLE-TRANSITIVE-GRAPH Check the whole transitive graph before adopting; the most
  restrictive licence in the set governs. @status:impl/done
- @fact:SUM-WEIGHT-IS-NEVER-A-LICENCE-REASON Weight is never a licence reason — keep those conversations
  separate. @status:impl/done
- @fact:SUM-AUTOMATE-AND-RE-AUDIT Automate the listing in CI and re-audit on a schedule; point the
  carve-out at the generated list. *One of the three is built, one is a
  recorded exception, one is built by nothing. Built: the carve-out points at
  the generated list rather than a hand copy. A recorded exception rather than
  a gap: the CI listing was wanted, filed, and deliberately declined — the
  intent «cargo deny in CI (automated license check)» is registered and marked
  `rescoped`, its resolution reading «couples to the CI decision; the license
  policy itself is enforced by review», which couples it to a standing
  owner-level no-CI posture. Built by nothing: the scheduled re-audit, per
  `##RE-AUDIT-ON-A-SCHEDULE` above — no periodic licence line exists in the
  `health-audit` flow, in any adopting project's audit record, or in any
  tooling.* @status:spec/done
- @fact:SUM-A-FORBIDDEN-LICENCE-NEEDS-A-RECORDED-OWNER-EXCEPTION A forbidden licence is only ever adopted by an explicit, recorded
  owner exception. @status:impl/done
