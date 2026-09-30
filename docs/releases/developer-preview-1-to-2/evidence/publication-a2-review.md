# M-07-A.2 pure publication authority review

Accepted boundary at `4e7b63d9a`: one pure evaluator consumes the validated
1.1.0 publication subject and effect plan. It first requires the existing
apply-ready plan and its exact digest, then returns structured `Approved`,
`NeedsApproval` or `Refused` with a reason code. A receipt binds that digest,
subject, destination, principal, remote generation, smoke policy, draft/publish
and published-replacement posture, and validity interval. B evaluates the
same plan with policy epoch, generation, scope, expiry and revocation; it is
disabled unless an explicit test-mode switch is supplied. An absent A receipt
needs approval. Invalid, expired, revoked or out-of-scope records refuse.

`cargo test --locked --offline -p vibe-publish --lib
publication_authority::tests` passed 7/7. `cargo fmt -p vibe-publish --
--check` and `git diff --check` passed. Cases include exact A and explicit B
admission, disabled B, changed plan and authority scope, expiry/revocation,
policy epoch/generation, and skipped smoke refusal. No network or remote write
is reachable from this module.

This pure decision assumes the caller obtained a record from an owner-controlled
source; the enum asserting that provenance does not prove it. Persistence,
origin verification, one-use redemption and a pre-effect publication journal
remain later M-07/M-16 obligations. The B evaluator is deliberately limited
to an exact plan in this first implementation. No live B policy was enabled,
and no public Git push or release occurred.
