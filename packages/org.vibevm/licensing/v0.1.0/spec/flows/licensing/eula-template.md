# EULA placeholder template {#root}

**Scope of this document.** A copy-ready skeleton for the
proprietary-with-relicense-intent posture, followed by clause-by-
clause commentary. It is a starting draft, not legal advice — have a
lawyer review any licence before you rely on it.

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

- **Reservation line.** "All rights reserved" plus "No license …
  granted" makes the default explicit: absent this grant, nobody has
  any rights. Everything below is an exception carved out of that
  default.
- **The interim grant.** Read-for-evaluation and build-locally are the
  minimum that lets someone assess the software; the four `NOT`
  clauses reserve everything that matters commercially. Adjust the
  list to the actual posture — some projects allow internal use, some
  do not.
- **The relicense-intent clause.** The honest core. It names the
  intended target licence and states plainly that the decision is not
  final. It promises a *direction*, never a date. Removing it turns
  the posture from "opening later" into "reserved indefinitely" —
  keep it only if that is the truth.
- **The third-party section.** Separated by a rule so no reader
  confuses the product's reserved terms with the dependencies'
  permissive ones. It points at the *generated* dependency list, not
  a hand-copied one that will drift.
- **Study-only material.** Anything vendored for reference but not
  shipped is named with the removal instruction, so a redistribution
  does not accidentally carry third-party work under the product's
  notice.

## Adapting it {#adapting}

| You want | Change |
|----------|--------|
| Permissive from the start | discard this template; use the target licence's official text and set the manifest field to its SPDX id |
| Reserved with no opening planned | drop the relicense-intent clause — but only if that is genuinely the intent |
| Allow internal/organizational use | add an explicit internal-use grant to the interim list |
| A different target licence | name it in the relicense clause and keep the file's own text ready to swap in at relicense time |

## When it becomes the real licence {#promotion}

At relicense time the placeholder is replaced wholesale by the target
licence's official text (not edited into it), every manifest
`license` field moves to the target's SPDX identifier in the same
commit, and the change is recorded as a dated decision. The
placeholder's job was to hold the posture honestly until that moment;
it is not itself the destination.

## Summary {#summary}

- The skeleton captures the proprietary-with-relicense-intent posture:
  reservation, a minimal interim grant, the honest relicense clause,
  and the third-party carve-out.
- Fill the placeholders; keep the relicense clause only if opening is
  the real intent.
- Point the third-party section at the generated list; name study-only
  material for removal.
- At relicense time, swap in the target's official text wholesale and
  move every manifest field with it.
- It is a draft to hand a lawyer, not legal advice.
