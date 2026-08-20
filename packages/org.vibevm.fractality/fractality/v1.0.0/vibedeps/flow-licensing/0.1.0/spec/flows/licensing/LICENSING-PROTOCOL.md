# Licensing Protocol {#root}

**Scope of this document.** This file defines a product's *licensing
posture*: how to choose the product's own licence, why a
proprietary-with-relicense-intent placeholder is a legitimate
starting point, the non-negotiable permissive-only rule for
dependencies, and the third-party carve-out that keeps the two
straight. It is guidance for authoring a licence posture, not legal
advice — a lawyer signs off on the real thing.

## Decide the posture; do not inherit it {#decide}

A new project's licence is too often whatever a scaffolding tool
dropped in. That is a decision made by accident. The product's
licence governs who may use, modify, and redistribute the work — it
deserves a deliberate choice, recorded with its reasoning like any
other foundational decision.

Three postures a product commonly starts from:

| Posture | What it says | When it fits |
|---------|--------------|--------------|
| **Permissive open** (MIT / Apache-2.0 / UPL-1.0) | anyone may use and redistribute, with attribution / patent terms | the product is meant to be adopted widely from day one |
| **Proprietary / source-available** | source is readable, rights are reserved | the owner wants evaluation and review but not yet redistribution |
| **Proprietary with relicense intent** (the placeholder) | reserved now, with a stated plan to open later | the owner intends to open the product but has not finalized the terms |

## The placeholder EULA {#placeholder}

The third posture is a useful default when the intent is to open the
product eventually but the final licence is not settled. Its shape:

- A clear ownership and reservation line (copyright, all rights
  reserved).
- A short **interim grant** naming exactly what is allowed now (read
  for evaluation; build locally) and what is not (redistribute,
  sublicense, publish, use commercially, remove the notice).
- An explicit **relicense-intent clause**: the owner intends to
  relicense under a named permissive licence (e.g. UPL-1.0) at a
  future date, and *that decision is not final*. This clause is the
  honest heart of the posture — it tells a reader the reserved rights
  are a way-station, not the destination.
- A contact line for licensing inquiries.

The relicense-intent clause matters because it sets expectations
truthfully: contributors and evaluators know the direction of travel
without being promised a date. A skeleton of this text ships with the
`draft-eula` skill.

## The permissive-only dependency rule {#deps}

Independent of the product's own licence, **every third-party
dependency must be permissively licensed** — MIT, Apache-2.0, BSD,
Unlicense, or equivalent. Strong copyleft (GPL, AGPL, LGPL) is
forbidden by default. Weak copyleft (MPL-2.0) is allowed case by
case, because its file-level copyleft does not taint consumers the
way GPL does.

The rule tightens, not loosens, as the product's own licence gets
stricter:

> A dependency's code mingles with the product's. A proprietary or
> source-available product that links a copyleft library can be
> forced to relicense the whole. So the more restrictive the
> product's own licence, the *more* important the permissive-only
> rule.

Legitimate reasons to reject a dependency: a non-permissive licence,
an abandoned upstream, a demonstrated security history, or an API so
bad it would poison your own interfaces. **Weight is not one of
them** — binary size and transitive dep count are not licence or
quality problems; do not reject a strong library for being heavy.

## The third-party carve-out {#carve-out}

The product's `LICENSE.md` states, in a clearly separated section,
that third-party dependencies keep their own terms, unaffected by the
product's notice, and points at the authoritative generated list (the
dependency manifest / lockfile, not a hand-maintained copy that will
rot). Reference material that is present for study but not part of
the shipped product (vendored examples, research corpora) is named
here too, with the instruction that it must be removed before any
redistribution.

## Keep the statements in sync {#sync}

A product states its licence in more than one place — the
`LICENSE.md` file, the manifest `license` field, sometimes a README
badge. These must never disagree. A change to one is a change to all,
in a single commit. A manifest that says `UPL-1.0` over a
`LICENSE.md` that still says "all rights reserved" is a
contradiction a consumer's compliance tooling will flag — and rightly
distrust.

## Relicensing is an owner decision {#relicense}

Moving the product from one licence to another — especially from the
placeholder to the intended permissive licence — is an
irreversible-threshold operation. Published artifacts under the old
terms do not retroactively change; downstream users relied on what
was published. Never relicense autonomously. When the owner calls it:
swap the `LICENSE.md`, update every manifest `license` field, adjust
any publish flags, and record the change as a dated decision.

## Re-derive for your project {#re-derive}

Copy the prompt-task, not the prompt-implementation. Paste this to
your agent in a fresh session:

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

- Choose the product's licence deliberately; a placeholder EULA with
  an honest relicense-intent clause is a valid starting posture.
- Dependencies are permissive-only; copyleft is forbidden by default
  and the rule tightens as the product's own licence gets stricter.
- The third-party carve-out points at the real generated list and
  names study-only material for removal.
- Keep `LICENSE.md` and every manifest `license` field in sync, always
  in one commit.
- Relicensing is an owner decision and an irreversible threshold —
  never autonomous.
