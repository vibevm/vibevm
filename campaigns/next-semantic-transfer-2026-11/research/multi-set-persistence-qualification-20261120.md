# Multi-member default persistence: exact-version qualification

Candidate baseline `9abc2215358f14797beda29cf0a6eef4d70a458b`. Bounded qualification of PROP-069#PERSISTENCE / RECONFIGURATION: automatic set reliance is distinct from a fully explicit answer which happens to equal the old defaults. No production/schema/epoch/review mechanism changes.

## Existing evidence reused, not duplicated

In `crates/vibe-cli/tests/cli_common_package_optional/default_review.rs`:

- `zero_or_more_multi_default_basis_is_lossless_and_tamper_refuses`: two-member and declared-empty history, canonical basis, original omission, warm repeat and strict malformed/tampered history refusal.
- `zero_or_more_unselected_version_leaves_no_basis_and_changed_defaults_refuse_update`: selected 1.0.0 singleton set basis, unselected 1.1.0 exclusion, changed-default update via reopened `^1`, before-effect semantic-state preservation and actual rejection of a scalar review token obtained from the existing scalar review route.

These do not by themselves establish forced exact-next-version behavior for an already retained two-member set, the fully explicit equal-answer control, or registry-independent replay. The new test adds only those missing cells.

## Narrow added selector

`default_review::multi_member_defaults_exact_version_drift_and_explicit_equal_answer`

Two independent worlds start with exact 1.0.0 and declared defaults `[enabled, disabled]`. One retains original omission and canonical `consumer_choice_sets`; the other authors exact `[disabled, enabled]` under require-selection and has no automatic basis. Both initially install the actual optional EXTRA dependency. Each then invokes a fresh CLI process with its registry renamed out of reach, requiring held-source unchanged replay and byte/mode/link-equivalent project semantic state.

A new 1.1.0 changes declared defaults to `[disabled]` and supplies a distinct boot marker. The authored root is forced to exact `=1.1.0`, not a range or same-version live mutation. Assertions distinguish pre-effect automatic-reliance refusal from successful explicit-answer update, compare original consumers/effective features/basis, actual lock-selected version, optional slot and boot marker, and retain the authored root request. The refusal snapshot covers the lock, generated/materialized tree, hook log and authored files; existing `.vibe` trace/lease metadata is deliberately outside package semantic state.

## Review mechanism and boundary

`crates/vibe-cli/src/commands/update/review.rs` explicitly refuses nonempty old/new `consumer_choice_sets` with `multi-member automatic default renewal is not connected; scalar review tokens cannot authorize it`. There is no actual set-review mechanism in that owner. Existing rejection of a real scalar token is reused; no new token is invented and no successful renewal is claimed. This is an open product capability, not a reason to weaken the refusal oracle or claim complete persistence delivery.

## Verification

Build-only succeeded. The new exact selector executed one test / two worlds: registry-unavailable unchanged replay succeeded in both; forced exact 1.1.0 automatic-set update refused with all prior package semantic state and basis retained; the fully explicit equal-answer update succeeded at 1.1.0 with unchanged original consumers/features, no automatic basis, actual EXTRA slot retained and the new boot marker present. Result 1/1 PASS, 123 filtered. Runtime uses the existing explicit-artifact bwrap runner, filesystem/PID/network namespaces, cleared environment, synthetic HOME and read-only executable/system mounts; no repository or host auth/config/socket mounts. This proves registry-independent fresh-process replay with the existing complete store, not reconstruction from an empty store.

The first launch failed in the runner's argument parser because `--exact` needed the `--` separator; no test ran. That setup error is retained separately from the successful r2 runtime. Both existing selectors above reran independently at 1/1 PASS each, 123 filtered, including the actual scalar-token refusal. Strict affected fixture lint (`cargo clippy -p vibe-cli --test cli_common_package_optional --no-deps -- -D warnings`), affected rustfmt check and diff whitespace check passed. No full 124-test suite or host gate was run for this test-only qualification. Exact source/artifact/evidence hashes are sealed in `multi-set-persistence-candidate-r1.json` under the private evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z/`; runtime logs are `multi-set-persistence-runtime-r2.log`, `multi-set-persistence-existing-r1.log` and `multi-set-persistence-basis-r1.log`. CLI SHA-256 `17e63f07352364b35d8d6486045a8923eacf88196f1c4695487b243d077f85ae`; fixture SHA-256 `edb27e0f64bbbe3d3f69f2e1089ceae34206846f316ed1ec269ecdc878715d76`. Independent architectural acceptance remains separate from these results. No USER-PROJECT, real secrets, provider execution, publication, parent-state promotion, REMAINDER or IDE work.
