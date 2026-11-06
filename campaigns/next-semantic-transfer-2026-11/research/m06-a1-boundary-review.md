# A1 independent boundary review — candidate, root acceptance only

Read-only source review. No tests/builds/Cargo/Conform/Git/network or product edits. Reports quoted by the packet were read as worker evidence, not independently rerun proof. Two concrete source-backed findings; root was notified and is routing runtime probes to owners.

## P1 — whole-resolution preflight omits predictable declared-input refusal

Owner: `crates/vibe-workspace/src/install/source_admission.rs:110-156`, `validate_source_resolution`.

Observed: the shared preflight checks supported operation, full Manifest equality, package identity and manifest recheck, but does not check exact declared files, selector conflicts or unsupported directory-census declarations. Those checks occur only inside per-package `vibedeps/source_policy.rs::SourcePolicy::observe` (directory refusal around line 109, selector conflict around 124, missing input in capture around 157).

Causal scenario: supply resolution `[valid legacy dependency, declared Mixed dependency with a syntactically valid directory row]`, or use a missing exact-file input / equal incompatible file rows in the second dependency. The shared full-resolution preflight succeeds. `install.rs:255-271` then loops dependencies; the first slot is reconciled at 419 and its `pre_install.run_or_defer` is called at 432 before the second dependency reaches the rejecting materializer admission. At the upper install boundary, `vibe-install/src/apply.rs:309` invokes the same insufficient preflight before lifecycle construction, and `apply/metadata.rs:30` before `apply.rs:409` manifest publication. These are predictable source admission failures, not intervening filesystem races. They must be discovered before product effects under PROP-072 SOURCE-INSTALL-BOUNDARY. Allowed empty lease/run infrastructure does not cover these effects.

Existing tests: `install/source_admission/tests.rs::source_admission_rejects_entire_resolution_before_first_slot_effect` tests a declared boot source, which the shallow preflight does reject. `vibedeps/source_policy/tests.rs::malformed_missing_conflicting_and_unsupported_sources_refuse_before_slot_changes` tests the missing/conflict/directory cases only through a direct single materializer call. Neither source test bridges these cases through full-resolution admission. No runtime failure claimed by this review.

Correction direction: use the same bounded source selection/read admission for every selected declared source before lifecycle/metadata/slot effects. This does not require whole-tree atomicity or a new framework.

## P2 — equal bytes bypass captured permission preservation on rematerialization

Owners: `crates/vibe-workspace/src/vibedeps/slot_diff/plan.rs:53-58`, `slot_diff.rs:254-260` and `reconcile_slot` keep branch.

Observed: prepared Observed content retains permissions, and fresh placement correctly uses them. But DiffPlan passes only SHA256 to `inspect_destination`. For an existing recorded regular file, equal old/incoming/actual hashes return keep=true regardless of observed source permissions. Reconcile therefore never calls the captured-content placement path for a mode-only change.

Causal scenario on Unix: materialize a declared file with Copy and mode 0644, change only the original source mode to 0755, then invoke the direct Mixed materializer again against the recorded slot. SourcePolicy captures/rechecks 0755, yet the unchanged bytes select keep=true, leaving slot0644 and successful return. Likewise, destination-only mode drift remains uncorrected even with an unchanged observed source. This defeats the new captured bytes+permissions preservation boundary for an actual rematerialization; it is not a request to chmod a shared source inode or force all trusted install slots to be rechecked.

Existing tests: `declared_executable_mode_and_requested_hardlink_identity_survive` covers fresh Copy/Hardlink mode preservation; placement tests cover rejected shared candidate and captured-copy fallback. They do not exercise mode-only reconciliation. No runtime reproduction was run here.

Correction direction: include captured permissions in keep eligibility for observed payloads; replace the private destination when repair is needed, retaining unrecorded collision and shared-inode boundaries.

## Reviewed without additional findings

- Full typed known Manifest equality includes boot/requires when either side declares sources; known declarations cannot disappear via missing local manifest. Missing unclassified content-tree compatibility and explicit empty semantics remain distinct.
- SafeFS source observation uses held no-follow two-pass reads, bounded allocation, closing metadata/identity checks, and permits legitimate shared regular files.
- Package checkpoints re-admit selected logical roots and compare root identity/manifest; derived-root checkpoints do not canonicalize descendants as fresh authority.
- Captured Copy and Hardlink fallback consume captured bytes and permissions. Hardlink candidate checks identity/bytes/permissions and avoids chmod of shared source.
- Selected facts admission binds locked kind/group/name/version/content hash, original Mixed record and exact documents; rechecks precede upserts. Record hashes remain editable consistency witnesses.
- The suspected member rederive-config mismatch was discarded: incumbent `reinstall.rs::rederive_package` deliberately reads `workspace.root/vibe.toml`, matching selected admission. No new member-policy requirement is invented.

## Scope and drift

Read all ten original named owners, PROP-072, the selected URI-OBSERVED-SOURCE-READ fact, reports and relevant direct tests. Root explicitly authorized additional bounded call-order reads in install.rs, hook_output.rs, apply.rs, apply/metadata.rs, slot_diff/plan.rs, facts.rs and reinstall.rs. The JSON records hashes. Ten original owners were unchanged between initial and final captures; backend specs/tests could still evolve. These findings apply to captured source, not an acceptance verdict.

No whole-tree atomicity, hostile ABA/CAS, publisher authentication, power-loss or broader native-platform proof is inferred or demanded.

## Root resolution

Both concrete findings were reproduced and fixed before acceptance in264b07466. Early-hook RED3→GREEN3 (admission8 plus actual CLI missing-input refusal); mode RED1/control1→GREEN2 (materializer86). Member configuration concern remains discarded. See [acceptance](m06-a1-transfer-acceptance.md). Source hashes above bind the reviewed pre-fix candidate, not a claim final source is unchanged.
