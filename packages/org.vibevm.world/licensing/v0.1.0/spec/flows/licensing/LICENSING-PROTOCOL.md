# Licensing Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines a product's *licensing
posture*: how to choose the product's own licence, why a
proprietary-with-relicense-intent placeholder is a legitimate
starting point, the non-negotiable permissive-only rule for
dependencies, and the third-party carve-out that keeps the two
straight. @impl/done

##IT-IS-GUIDANCE-NOT-LEGAL-ADVICE It is guidance for authoring a licence posture, not legal
advice — a lawyer signs off on the real thing. @spec/done

## Decide the posture; do not inherit it {#decide}

##a-new-projects-licence-is-too-often-whatever-a-scaffold-dropped-in A new project's licence is too often whatever a scaffolding tool
dropped in. @spec/done

##that-is-a-decision-made-by-accident That is a decision made by accident. @spec/done

##THE-PRODUCTS-LICENCE-DESERVES-A-DELIBERATE-CHOICE The product's
licence governs who may use, modify, and redistribute the work — it
deserves a deliberate choice, recorded with its reasoning like any
other foundational decision. @spec/done

##three-postures-lead Three postures a product commonly starts from: @spec/done

| Posture | What it says | When it fits |
|---------|--------------|--------------|
| ##ROW-POSTURE-PERMISSIVE-OPEN **Permissive open** (MIT / Apache-2.0 / UPL-1.0) @spec/done | anyone may use and redistribute, with attribution / patent terms @spec/done | the product is meant to be adopted widely from day one @spec/done |
| ##ROW-POSTURE-PROPRIETARY-SOURCE-AVAILABLE **Proprietary / source-available** @spec/done | source is readable, rights are reserved @spec/done | the owner wants evaluation and review but not yet redistribution @spec/done |
| ##ROW-POSTURE-PROPRIETARY-WITH-RELICENSE-INTENT **Proprietary with relicense intent** (the placeholder) @spec/done | reserved now, with a stated plan to open later @spec/done | the owner intends to open the product but has not finalized the terms @spec/done |

## The placeholder EULA {#placeholder}

##THE-THIRD-POSTURE-IS-A-USEFUL-DEFAULT-WHEN-OPENING-IS-INTENDED The third posture is a useful default when the intent is to open the
product eventually but the final licence is not settled. Its shape: @spec/done

- ##SHAPE-A-CLEAR-OWNERSHIP-AND-RESERVATION-LINE A clear ownership and reservation line (copyright, all rights
  reserved). @impl/done
- ##SHAPE-A-SHORT-INTERIM-GRANT A short **interim grant** naming exactly what is allowed now (read
  for evaluation; build locally) and what is not (redistribute,
  sublicense, publish, use commercially, remove the notice). @impl/done
- ##SHAPE-AN-EXPLICIT-RELICENSE-INTENT-CLAUSE An explicit **relicense-intent clause**: the owner intends to
  relicense under a named permissive licence (e.g. UPL-1.0) at a
  future date, and *that decision is not final*. This clause is the
  honest heart of the posture — it tells a reader the reserved rights
  are a way-station, not the destination. @impl/done
- ##SHAPE-A-CONTACT-LINE-FOR-LICENSING-INQUIRIES A contact line for licensing inquiries. @impl/done

##why-the-relicense-intent-clause-matters The relicense-intent clause matters because it sets expectations
truthfully: contributors and evaluators know the direction of travel
without being promised a date. @spec/done

##A-SKELETON-OF-THIS-TEXT-SHIPS-WITH-THE-DRAFT-EULA-SKILL A skeleton of this text ships with the
`draft-eula` skill. @impl/done

## The permissive-only dependency rule {#deps}

##EVERY-THIRD-PARTY-DEPENDENCY-MUST-BE-PERMISSIVELY-LICENSED Independent of the product's own licence, **every third-party
dependency must be permissively licensed** — MIT, Apache-2.0, BSD,
Unlicense, or equivalent. @impl/done

##STRONG-COPYLEFT-IS-FORBIDDEN-BY-DEFAULT Strong copyleft (GPL, AGPL, LGPL) is
forbidden by default. @impl/done

##WEAK-COPYLEFT-IS-ALLOWED-CASE-BY-CASE Weak copyleft (MPL-2.0) is allowed case by
case, because its file-level copyleft does not taint consumers the
way GPL does. @impl/done

##the-rule-tightens-as-the-product-licence-gets-stricter-lead The rule tightens, not loosens, as the product's own licence gets
stricter: @impl/done

> ##why-a-stricter-product-licence-tightens-the-rule A dependency's code mingles with the product's. A proprietary or
> source-available product that links a copyleft library can be
> forced to relicense the whole. So the more restrictive the
> product's own licence, the *more* important the permissive-only
> rule. @spec/done

##legitimate-reasons-to-reject-a-dependency-lead Legitimate reasons to reject a dependency: @impl/done

- ##REJECT-REASON-A-NON-PERMISSIVE-LICENCE a non-permissive licence, @impl/done
- ##REJECT-REASON-AN-ABANDONED-UPSTREAM an abandoned upstream, @impl/done
- ##REJECT-REASON-A-DEMONSTRATED-SECURITY-HISTORY a demonstrated security history, @impl/done
- ##REJECT-REASON-AN-API-THAT-WOULD-POISON-YOUR-INTERFACES or an API so
  bad it would poison your own interfaces. @impl/done

##WEIGHT-IS-NOT-ONE-OF-THEM **Weight is not one of
them** — binary size and transitive dep count are not licence or
quality problems; do not reject a strong library for being heavy. @impl/done

## The third-party carve-out {#carve-out}

##THE-CARVE-OUT-STATES-THIRD-PARTY-TERMS-AND-POINTS-AT-THE-GENERATED-LIST The product's `LICENSE.md` states, in a clearly separated section,
that third-party dependencies keep their own terms, unaffected by the
product's notice, and points at the authoritative generated list (the
dependency manifest / lockfile, not a hand-maintained copy that will
rot). @impl/done

##STUDY-ONLY-REFERENCE-MATERIAL-IS-NAMED-WITH-ITS-REMOVAL-INSTRUCTION Reference material that is present for study but not part of
the shipped product (vendored examples, research corpora) is named
here too, with the instruction that it must be removed before any
redistribution. @impl/done

## Keep the statements in sync {#sync}

##A-PRODUCT-STATES-ITS-LICENCE-IN-MORE-THAN-ONE-PLACE A product states its licence in more than one place — the
`LICENSE.md` file, the manifest `license` field, sometimes a README
badge. @impl/done

##THESE-MUST-NEVER-DISAGREE These must never disagree. @impl/done

##A-CHANGE-TO-ONE-IS-A-CHANGE-TO-ALL-IN-A-SINGLE-COMMIT A change to one is a change to all,
in a single commit. @impl/done

##a-disagreement-is-a-contradiction-compliance-tooling-will-flag A manifest that says `UPL-1.0` over a
`LICENSE.md` that still says "all rights reserved" is a
contradiction a consumer's compliance tooling will flag — and rightly
distrust. @spec/done

## Relicensing is an owner decision {#relicense}

##RELICENSING-IS-AN-IRREVERSIBLE-THRESHOLD-OPERATION Moving the product from one licence to another — especially from the
placeholder to the intended permissive licence — is an
irreversible-threshold operation. @impl/done

##published-artifacts-do-not-retroactively-change Published artifacts under the old
terms do not retroactively change; downstream users relied on what
was published. @spec/done

##NEVER-RELICENSE-AUTONOMOUSLY Never relicense autonomously. @impl/done

##when-the-owner-calls-it-lead When the owner calls it: @impl/done

- ##RELICENSE-STEP-SWAP-THE-LICENCE-FILE swap the `LICENSE.md`, @impl/done
- ##RELICENSE-STEP-UPDATE-EVERY-MANIFEST-FIELD update every manifest `license` field, @impl/done
- ##RELICENSE-STEP-ADJUST-ANY-PUBLISH-FLAGS adjust
  any publish flags, @impl/done
- ##RELICENSE-STEP-RECORD-A-DATED-DECISION and record the change as a dated decision. @impl/done

## Re-derive for your project {#re-derive}

##COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION Copy the prompt-task, not the prompt-implementation. @impl/done

##re-derive-prompt-lead Paste this to
your agent in a fresh session: @impl/done

```
Read spec/flows/licensing/ end to end. Then establish THIS project's
licensing posture: (1) which posture does the owner want for the
product — permissive, proprietary, or the placeholder with relicense
intent? Draft the LICENSE.md accordingly. (2) State the
permissive-only dependency rule and list any current dependencies
that violate it. (3) Draft the third-party carve-out pointing at our
real dependency manifest. Show me all of it as a draft; treat the
product licence choice and any relicensing as MY decision, never
yours.
```

## Summary {#summary}

- ##SUM-CHOOSE-THE-LICENCE-DELIBERATELY Choose the product's licence deliberately; a placeholder EULA with
  an honest relicense-intent clause is a valid starting posture. @impl/done
- ##SUM-DEPENDENCIES-ARE-PERMISSIVE-ONLY Dependencies are permissive-only; copyleft is forbidden by default
  and the rule tightens as the product's own licence gets stricter. @impl/done
- ##SUM-THE-CARVE-OUT-POINTS-AT-THE-GENERATED-LIST The third-party carve-out points at the real generated list and
  names study-only material for removal. @impl/done
- ##SUM-KEEP-EVERY-STATEMENT-IN-SYNC Keep `LICENSE.md` and every manifest `license` field in sync, always
  in one commit. @impl/done
- ##SUM-RELICENSING-IS-AN-OWNER-DECISION Relicensing is an owner decision and an irreversible threshold —
  never autonomous. @impl/done
