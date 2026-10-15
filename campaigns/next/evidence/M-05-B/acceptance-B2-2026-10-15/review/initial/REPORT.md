# M-05-B.2 review: changes required

One P2 finding; no P1. All455 source subjects matched the final evidence snapshots and were unchanged before/after review.

## B2-F1 [P2] Enforce the shared domain/instance identity invariant in result admission

[consumer_configuration.rs:160](/fast/git/v/vibevm-next/crates/vibe-wire/src/behaviour/consumer_configuration.rs:160); compare [resolver grouping](/fast/git/v/vibevm-next/crates/vibe-resolver/src/features/configuration.rs:33).

Construct two valid result edges with identical domain and instance strings, different consumers, and configuration values config-a/config-b (or different package identity). Give each a Resolved outcome with representative 0/1 and empty active_features/selected/needs/conflicting_edges/defaults_used. admit_request accepts the edges because duplicate detection compares full ConfigurationInstance. admit_result also compares full instance equality, so the two different configurations are treated as distinct instances and coverage succeeds. decode_result therefore accepts this result. The public resolver groups by domain+instance and correctly rejects the identical edge input as configuration/package disagreement.

The new public result codec admits a result that violates the exact instance identity law and cannot be produced by its own resolver. This is semantic admission of known identity data, not a request to re-run definition solving inside the codec. It undermines the generated request/result owner/domain/provenance contract and the permanent M05B2-CONSUMERS rule that a reused instance cannot change package/configuration.

Correction: Share or apply the same domain+instance consistency check at wire admission so incompatible configuration/package identities refuse before any result is admitted. Add one focused codec regression with conflicting same-key edges and keep the existing different-domain positive. No new engine, full suite or extra platform gate required.

Deterministic source-derived counterexample; reviewer ran no Rust test or reproduction.

## Full contract review

- Core reuses accepted owner-qualified DefinitionId, optional LockId, TargetInstanceId, target role and DependencyRole; domains/instance/configuration explicit, native versions opaque.
- Public resolver key uses domain+instance independent of selected members and rejects configuration/package changes, so conflicting selection is not hidden by deriving an instance from the selection.
- Four cardinalities, positive package-local guard DAG, dormant validation and retained intent, omission versus explicit-empty, explicit UseDeclaredDefaults policy, exact/additive conflict with both edges, separate environment domains and implicit all-features/closure refusal are implemented.
- Generated request admission reaches public resolver, which composes accepted B1 FeatureProgram/evaluator. NeedsSelection and conflict return no expansion; implementation has no I/O calls.
- Old B1 typed/legacy/default and unavailable-export paths remain unchanged; B3 grammar/migration/install delivery not claimed. Permanent PROP003 adds four clauses preserving prior text.

## Evidence and limits

21 fresh successful nonignored cases: core4 + resolver9 + wire3 + affected B1 exact5. All final command source snapshots match current455.

Independently compared all53 authorized old generated module files to HEAD: exactly one changed comment line adding schemas/consumer_configuration/e1/, non-comment bytes unchanged. V7 reports old schemas unchanged, no unqualified generated drift and prior PROP003 preserved. Required mod/FormatId registration changes are separate. Cargo.lock changes only existing resolver dependency edge vibe-wire as approved.

Actual codegen0/check-codegen1/wire-diff0. Preintegration index-based check remains FAILED, not accepted green. After correction Root owns one integrated index-aware check. Original missing x-empty generator failures and original V7 protected-header failure retained separately; no blanket protection relaxation found.

Read-only named source/evidence and Git baseline inspection plus hashes; no tests/builds/product edits/Git mutations/models/process/store/acceptance actions.
