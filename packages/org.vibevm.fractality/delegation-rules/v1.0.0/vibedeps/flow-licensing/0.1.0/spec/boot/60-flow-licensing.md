# Flow: Licensing {#root}

This project has a **deliberate licensing posture**, decided once and
recorded, not inherited from whatever a scaffold dropped in. Two
things are always true and must not drift:

- The product ships under one stated licence (a `LICENSE.md` at the
  repository root; every sub-package states the same in its
  manifest).
- Every third-party dependency stays **permissive-only** — MIT /
  Apache-2.0 / BSD / Unlicense / equivalent. Strong copyleft
  (GPL / AGPL / LGPL) is forbidden by default; weak copyleft
  (MPL-2.0) is case-by-case.

## The dependency rule is load-bearing {#deps}

A dependency's licence mingles with the product's. A proprietary or
source-available product that links a copyleft library can be forced
to relicense — so the stricter the product's own licence, the *more*
important the permissive-only rule, not less. Reject a copyleft
dependency on licence grounds regardless of how good it is.

## When licence work happens {#when}

- Adding a dependency: check its licence before adopting it. A
  non-permissive licence is a hard no by default; surface it.
- Changing the product's licence, or any bulk relicensing: this is an
  **owner decision** and an irreversible-threshold operation — never
  do it autonomously.
- A change touching the licence file, the manifest `license` field,
  or the third-party carve-out updates all of them together, in one
  commit.

The `draft-eula` skill drafts or reviews the posture. Full detail:
[`LICENSING-PROTOCOL.md`](../flows/licensing/LICENSING-PROTOCOL.md).

## Never {#never}

- Never add a GPL / AGPL / LGPL dependency by default — surface it as
  an owner decision.
- Never relicense the product, or any part of it, without the owner's
  explicit instruction.
- Never let the manifest `license` field and the `LICENSE.md`
  disagree.
- Never reject a dependency for being "too heavy" — weight is not a
  licence problem; licence is.
- Never claim a licence is permissive without checking; when unsure,
  treat it as non-permissive and ask.
