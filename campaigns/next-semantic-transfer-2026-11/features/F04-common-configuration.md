# F04 — Common configuration from the working application wizard

Status: active. The common user journey is not accepted merely because a pure kernel is extracted. Canonical contract: PROP-069-common-configuration.xml.

## Why this feature is being generalized

The owner identified the existing app console/GUI choice system as a stronger starting point than the prototype's manual choice-host route. Main has describe → choose → plan → install → record → modify/update consumers. Next contributes important semantics: cardinalities, positive guards, original omitted/empty/additive/exact intent, multiple consumers and dormant choices. Neither a set of booleans nor a parallel host service completes the combined requirement.

The target is one kind-neutral local configuration capability, adapters to existing effects and state owners, and the same answers through console and serialized external input. Read-only doc and policy-consent remap boundaries remain distinct. All nine package kinds must have an honest supported-operation matrix before final acceptance.

## First slice and explicit limits

The first slice extracts local feature closure into vibe-core::configuration and returns application behavior through a compatibility adapter. It performs no I/O, registry operation, installation or persistence. Rich consumer choices, parameters, effect integration, retained intent and common machine surfaces are following work. The existing app wire documents are not changed by this extraction.

A closure constraint is not a consumer-owned choice. The former constrains the computed feature set; the latter requires and retains an answer and may not be selected accidentally by ordinary implication. This distinction belongs to the normalized rule, not the package kind. Closure-group activity derives from its members' parents; arbitrary positive choice guards are deliberately excluded from this first primitive.

## Problems found while characterizing main

1. Ordinary requires closure and group fallback interleaved. Reordering root/relay/chosen declarations could change success to conflict; renaming groups could activate a later fallback prematurely. Fix: close ordinary implications, collect eligible fallbacks from one snapshot, apply as a batch, repeat. Conflicting defaults refuse without searching for a different answer.
2. Profile/prompt selections were classified as direct additions. A valid gtk profile plus --feature qt conflicted; --no-feature gui conflicted with a profile child instead of removing it. PROP-065 already says direct flags apply on top. Normalize profile/prompt as the base, flags as additions/disables; direct contradictions remain errors.
3. Group replacement could silently remove a directly enabled descendant of the replaced branch and report success. The requested fix is a named conflict, while base-only descendants remain overridable. The standalone repair is integrated in main and its regression passes.

The first two standalone patches are separate from kernel extraction. This separates actual behavior repairs from the claim of a compatible refactor. A malformed first profile fixture mixed installation sugar and steps; correcting the fixture is not behavioral RED evidence.

## Verification so far

Original evaluator against intended regressions: 3 passed, 5 failed, exit 101. Repaired standalone evaluator in main: 8 passed, 0 failed. Public app wizard after the repairs: 1 passed, 0 failed (describe/plan/install/events/record/cancel/resume). Scoped app conform: 0 new findings; one pre-existing ambient-env observation is nonblocking under current policy. The existing public wizard passed before extraction. Shared-kernel and maintenance evidence will be appended after review.

## Remaining acceptance

Shared kernel review and integration; rich intent/guard/default/consumer semantics; typed parameters and secret handling; actual feature-effect and solver integration; authored/install-owner persistence; ordinary console and versioned external views; complete install/update/modify and all-kind scenarios. Prototype receipts and pure tests are evidence for bounded claims, not proof of these outcomes.
