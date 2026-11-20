# Explicit named profile with consumer-owned boolean — implementation candidate

Accepted qualification `1c648fd49`; implementation contract integrated `5fa89fb85`. FMT-084 records the bounded composition; no new persisted field or epoch, no update guard relaxation.

The existing evaluator now admits a named parameterless feature profile while the same real consumer explicitly supplies one required public bool without default. The profile selects features through existing normalization; original profile ID, original empty authored selections and original parameter map remain retained. No profile parameter preset, default answer, automatic profile consent or synthesized automatic_basis is introduced. Explicit false and true stay typed values; missing required input reaches its actual missing-parameter diagnosis. Private/defaulted/preset parameter composition and forwarding remain refused.

## Actual gates

Private root `/data/git/vibevm-keyring-storage-20261120T0100Z`. `profile-boolean-first-full-r2.log` records all53 public tests PASS on executable SHA-256 `fac5fe80add7cedfb57bd94d497ddfae6ecec36a056107d14bd6c95baea6ee5a`, fixture SHA-256 `04fe335df8afeb933caecc70f860247a8aa8b6f00e5bf2612cc4a122034ffe69`.

The superseded qualification fixture now includes combined explicit false and true positive worlds plus missing/parameterless-profile/no-profile controls. Public install actually materializes the selected dependency and generated boot marker; retained original profile/parameters are compared independently of selected feature data, public parameters inspection agrees, and cold lock/boot/hooks are byte-stable. Missing required, unknown profile, wrong type, private/default/preset and conflicting actual workspace consumers refuse before lock or any slot, including mandatory base, generated static boot or hooks. Compatible workspace consumers preserve both original named-profile requests.

The retained prior accepted d73e07db011c01b9843c36cdfb93d94e60c516fd4d9c0f7c4b91b25c581b627a reader and install writer both refuse newly emitted combined records with unchanged lock/slot manifest/boot/hooks. Changed relied-on profile preset update refuses before effects under the unchanged historical guard. Core and CLI/bin/fixture strict lint passed; final formatting/diff and exact source manifest bind independent acceptance. Historical qualification remains evidence of its earlier unsupported boundary.

No parameter source from a profile, implicit profile selection, broader default/profile/forwarding support, agent/provider/network or USER-PROJECT adoption is claimed. UP adoption remains UNRUN; NST parent and frozen REMAINDER/AR/IDE states are unchanged.
