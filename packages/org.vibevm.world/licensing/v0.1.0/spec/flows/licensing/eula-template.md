# EULA placeholder template {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** A copy-ready skeleton for the
proprietary-with-relicense-intent posture, followed by clause-by-clause
commentary. @impl/done

##IT-IS-A-STARTING-DRAFT-NOT-LEGAL-ADVICE It is a starting draft, not legal advice — have a
lawyer review any licence before you rely on it. @spec/done

## The skeleton {#skeleton}

```
# <Product> — Proprietary License (EULA placeholder)

Copyright (c) <year> <owner>. All rights reserved.

<Product> (including <the components>, and all associated
documentation, specifications, and configuration) is proprietary
software. No license, express or implied, is granted by distribution
of this source tree.

Until a formal End-User License Agreement is executed, the following
applies:

- You may read the source for evaluation.
- You may build the software locally for personal evaluation.
- You may NOT redistribute, sublicense, or publish the software or
  derivative works.
- You may NOT use the software for commercial purposes.
- You may NOT remove or obscure this notice.

The owner intends to relicense <Product> under a permissive
open-source license (most likely <the target license>) at a future
date. That decision is not final. Until it is, treat this project as
source-available proprietary software.

For licensing inquiries, contact the owner.

---

### Third-party dependencies

<Product> links against third-party components distributed under
permissive licenses (MIT, Apache-2.0, BSD, or equivalent). Their
terms are unaffected by this notice and continue to govern their
respective code. See <the dependency manifest> for the authoritative
list.

Reference material under <study-only paths> is the property of its
respective authors, redistributed under each work's own terms, and is
NOT part of the distribution — it must be removed before any
redistribution.
```

## Clause commentary {#commentary}

- ##CLAUSE-RESERVATION-LINE **Reservation line.** "All rights reserved" plus "No license …
  granted" makes the default explicit: absent this grant, nobody has
  any rights. Everything below is an exception carved out of that
  default. @impl/done
- ##CLAUSE-THE-INTERIM-GRANT **The interim grant.** Read-for-evaluation and build-locally are the
  minimum that lets someone assess the software; the four `NOT`
  clauses reserve everything that matters commercially. Adjust the
  list to the actual posture — some projects allow internal use, some
  do not. @impl/done
- ##CLAUSE-THE-RELICENSE-INTENT-CLAUSE **The relicense-intent clause.** The honest core. It names the
  intended target licence and states plainly that the decision is not
  final. It promises a *direction*, never a date. Removing it turns
  the posture from "opening later" into "reserved indefinitely" —
  keep it only if that is the truth. @impl/done
- ##CLAUSE-THE-THIRD-PARTY-SECTION **The third-party section.** Separated by a rule so no reader
  confuses the product's reserved terms with the dependencies'
  permissive ones. It points at the *generated* dependency list, not
  a hand-copied one that will drift. @impl/done
- ##CLAUSE-STUDY-ONLY-MATERIAL **Study-only material.** Anything vendored for reference but not
  shipped is named with the removal instruction, so a redistribution
  does not accidentally carry third-party work under the product's
  notice. @impl/done

## Adapting it {#adapting}

| You want | Change |
|----------|--------|
| ##ROW-WANT-PERMISSIVE-FROM-THE-START Permissive from the start @impl/done | discard this template; use the target licence's official text and set the manifest field to its SPDX id @impl/done |
| ##ROW-WANT-RESERVED-WITH-NO-OPENING-PLANNED Reserved with no opening planned @impl/done | drop the relicense-intent clause — but only if that is genuinely the intent @impl/done |
| ##ROW-WANT-INTERNAL-USE-ALLOWED Allow internal/organizational use @impl/done | add an explicit internal-use grant to the interim list @impl/done |
| ##ROW-WANT-A-DIFFERENT-TARGET-LICENCE A different target licence @impl/done | name it in the relicense clause and keep the file's own text ready to swap in at relicense time @impl/done |

## When it becomes the real licence {#promotion}

##AT-RELICENSE-TIME-THE-SWAP-IS-ONE-RECORDED-OPERATION At relicense time the placeholder is replaced wholesale by the target
licence's official text (not edited into it), every manifest
`license` field moves to the target's SPDX identifier in the same
commit, and the change is recorded as a dated decision. @impl/done

##the-placeholder-is-not-itself-the-destination The
placeholder's job was to hold the posture honestly until that moment;
it is not itself the destination. @spec/done

## Summary {#summary}

- ##SUM-THE-SKELETON-CAPTURES-THE-POSTURE The skeleton captures the proprietary-with-relicense-intent posture:
  reservation, a minimal interim grant, the honest relicense clause,
  and the third-party carve-out. @impl/done
- ##SUM-FILL-THE-PLACEHOLDERS-AND-KEEP-THE-CLAUSE-ONLY-IF-TRUE Fill the placeholders; keep the relicense clause only if opening is
  the real intent. @impl/done
- ##SUM-POINT-AT-THE-GENERATED-LIST-AND-NAME-STUDY-ONLY-MATERIAL Point the third-party section at the generated list; name study-only
  material for removal. @impl/done
- ##SUM-AT-RELICENSE-TIME-SWAP-WHOLESALE-AND-MOVE-EVERY-FIELD At relicense time, swap in the target's official text wholesale and
  move every manifest field with it. @impl/done
- ##SUM-IT-IS-A-DRAFT-TO-HAND-A-LAWYER It is a draft to hand a lawyer, not legal advice. @impl/done
