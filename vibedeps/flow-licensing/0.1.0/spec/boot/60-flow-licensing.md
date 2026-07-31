# Flow: Licensing {#root}

<status stage="impl" state="done"/>

##THE-PROJECT-HAS-A-DELIBERATE-LICENSING-POSTURE This project has a **deliberate licensing posture**, decided once and
recorded, not inherited from whatever a scaffold dropped in. @impl/done

##two-things-are-always-true-lead Two
things are always true and must not drift: @impl/done

- ##ONE-STATED-PRODUCT-LICENCE The product ships under one stated licence (a `LICENSE.md` at the
  repository root; every sub-package states the same in its
  manifest). @impl/done
- ##EVERY-DEPENDENCY-STAYS-PERMISSIVE-ONLY Every third-party dependency stays **permissive-only** — MIT /
  Apache-2.0 / BSD / Unlicense / equivalent. Strong copyleft
  (GPL / AGPL / LGPL) is forbidden by default; weak copyleft
  (MPL-2.0) is case-by-case. @impl/done

## The dependency rule is load-bearing {#deps}

##a-dependencys-licence-mingles-with-the-products A dependency's licence mingles with the product's. @spec/done

##A-STRICTER-PRODUCT-LICENCE-MAKES-THE-RULE-MORE-IMPORTANT A proprietary or
source-available product that links a copyleft library can be forced
to relicense — so the stricter the product's own licence, the *more*
important the permissive-only rule, not less. @spec/done

##REJECT-A-COPYLEFT-DEPENDENCY-ON-LICENCE-GROUNDS Reject a copyleft
dependency on licence grounds regardless of how good it is. @impl/done

## When licence work happens {#when}

- ##WHEN-ADDING-A-DEPENDENCY Adding a dependency: check its licence before adopting it. A
  non-permissive licence is a hard no by default; surface it. @impl/done
- ##WHEN-CHANGING-THE-PRODUCTS-LICENCE Changing the product's licence, or any bulk relicensing: this is an
  **owner decision** and an irreversible-threshold operation — never
  do it autonomously. @impl/done
- ##WHEN-A-CHANGE-TOUCHES-ANY-LICENCE-STATEMENT A change touching the licence file, the manifest `license` field,
  or the third-party carve-out updates all of them together, in one
  commit. @impl/done

##THE-DRAFT-EULA-SKILL-DRAFTS-OR-REVIEWS-THE-POSTURE The `draft-eula` skill drafts or reviews the posture. @impl/done

##sibling-document-pointers Full detail:
@spec://org.vibevm.world/licensing/flows/licensing/LICENSING-PROTOCOL#root. @impl/done

## Never {#never}

- ##NEVER-ADD-A-COPYLEFT-DEPENDENCY-BY-DEFAULT Never add a GPL / AGPL / LGPL dependency by default — surface it as
  an owner decision. @impl/done
- ##NEVER-RELICENSE-WITHOUT-THE-OWNERS-EXPLICIT-INSTRUCTION Never relicense the product, or any part of it, without the owner's
  explicit instruction. @impl/done
- ##NEVER-LET-THE-MANIFEST-AND-THE-LICENCE-FILE-DISAGREE Never let the manifest `license` field and the `LICENSE.md`
  disagree. @impl/done
- ##NEVER-REJECT-A-DEPENDENCY-FOR-BEING-TOO-HEAVY Never reject a dependency for being "too heavy" — weight is not a
  licence problem; licence is. @impl/done
- ##NEVER-CLAIM-A-LICENCE-IS-PERMISSIVE-WITHOUT-CHECKING Never claim a licence is permissive without checking; when unsure,
  treat it as non-permissive and ask. @impl/done
