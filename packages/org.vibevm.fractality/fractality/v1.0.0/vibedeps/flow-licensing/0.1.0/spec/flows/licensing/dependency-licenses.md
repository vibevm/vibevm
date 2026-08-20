# Dependency licence discipline {#root}

**Scope of this document.** The operational side of the
permissive-only rule: which licences pass, which fail, how to check a
new dependency before adopting it, and how to keep the check honest
over time. Guidance, not legal advice.

## The allow / deny table {#table}

| Class | Licences | Verdict |
|-------|----------|---------|
| Permissive | MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unlicense, Zlib, UPL-1.0 | **allowed** |
| Weak copyleft | MPL-2.0, EPL | **case by case** — file-level copyleft usually does not taint consumers, but confirm the use |
| Strong copyleft | GPL-2.0/3.0, AGPL, LGPL | **forbidden by default** — an owner exception, if any, is explicit and recorded |
| Unclear / missing | no licence file, "all rights reserved", custom terms | **treat as forbidden** until clarified — no licence means no rights |

The `LGPL` entry is deliberately in the forbidden row: even though its
copyleft is weaker than GPL, its dynamic-linking obligations are easy
to violate accidentally in a statically-linked or bundled build, so
the default is no.

## Checking a dependency before adoption {#check}

1. **Find the licence.** The package's manifest field, its
   `LICENSE` file, its repository. If these disagree, the most
   restrictive one wins until the discrepancy is resolved.
2. **Classify it** against the table. Permissive → proceed.
   Case-by-case → confirm the specific obligation is met. Forbidden or
   unclear → stop and surface it as an owner decision.
3. **Check the transitive set, not just the direct dependency.** A
   permissive library that pulls a GPL transitive dependency is a GPL
   problem. Run the ecosystem's licence-listing over the whole
   resolved graph, not the top level.
4. **Record the outcome** where dependency decisions live — an
   allowed non-obvious licence and any case-by-case call is a recorded
   decision with the reason.

## Weight is not a licence concern {#weight}

Reject a dependency for its licence, its abandonment, its security
history, or an API that would poison your interfaces — never for
being "heavy". Binary size, crate/package count, and transitive
weight are not reasons to under-specify a load-bearing component;
that trades a one-time size cost for ongoing architectural debt.
Keep the two conversations separate: licence discipline here, weight
nowhere.

## Keeping the check honest {#honest}

- **Automate the listing.** A licence-lister run in CI over the full
  resolved graph turns "we think everything is permissive" into a
  fact that fails loudly when a forbidden licence enters. A rule with
  no checker is a wish.
- **Re-audit on a schedule.** A dependency can relicense between
  versions; a periodic audit line (see `flow:health-audit`) re-runs
  the listing and flags any new non-permissive entry.
- **Point the product's carve-out at the generated list**, never a
  hand-maintained copy — the hand copy drifts, the generated one
  cannot.

## When a forbidden licence is the only option {#forbidden-only}

Sometimes the best-or-only library for a job is copyleft. That is an
owner decision, not an agent decision. Surface it with the trade-off
stated: the capability gained, the licence obligation incurred, and
the alternatives (a permissive competitor, a reimplementation, doing
without). The owner may grant a recorded, scoped exception — or not.
Never adopt it silently.

## Summary {#summary}

- Permissive allowed, weak copyleft case-by-case, strong copyleft and
  unclear licences forbidden by default.
- Check the whole transitive graph before adopting; the most
  restrictive licence in the set governs.
- Weight is never a licence reason — keep those conversations
  separate.
- Automate the listing in CI and re-audit on a schedule; point the
  carve-out at the generated list.
- A forbidden licence is only ever adopted by an explicit, recorded
  owner exception.
