M-05-A.2 SAME logical task, internal retry2: unreviewed characterization candidate.
No acceptance/count change. Current baseline: 68 accepted contracts plus separate M03A2 plan safe-stop (owner-supplied). No delegation, staging, commits, dependency downloads, production solver/activation edits, next task, M10 process/store/content edits, PROP074 or derivative fixture edits.

Selected before execution: retained vibe-resolver --lib m_05_a_2_cases family, eight #[test] names inspected directly. Executed:
CARGO_TARGET_DIR=/tmp/vibevm-managed-m05a2/cargo-target cargo test --offline --locked -p vibe-resolver --lib m_05_a_2_cases
Exit 0. 8 executed, 8 passed, 0 failed, 0 ignored, 0 measured, 91 filtered out. Log: resolver-test.log. No remaining locked missing crate observed. Owner preflight was compilation-only; these are actual tests.
Cases:
qualified_forwarding_first_slash_defect
weak_forwarding_does_not_activate_dependency
exclusive_at_most_one_and_all_features_validation_defect
requires_any_allows_both_alternatives
absent_first_or_alternative_backend_gap
wrong_version_name_present_disjunction_defect
direct_routes_omit_features_and_conditional_targets
negated_self_trigger_changes_truth_without_iteration_claim

Core evidence reused, not rerun:
CARGO_TARGET_DIR=/tmp/vibevm-managed-m05a2/cargo-target cargo test --offline --locked -p vibe-core --lib m_05_a_2_cases
Historical exit 0; 4 executed/passed, 0 failed/ignored/measured, 615 filtered out. Log /tmp/vibevm-managed-m05a2/core-test-rerun.log. Exact retained owner/test hashes match prior retry report. Cases: qualified_activation_strings_roundtrip_unchanged; qualified_dependency_key_preserves_identity_and_version; inline_dependency_feature_options_are_unsupported; negated_target_and_requires_any_preserve_constraints. No Rust bytes edited in retry2.

Checks executed in retry2:
rustfmt --check --edition 2024 --config skip_children=true crates/vibe-resolver/src/lib.rs crates/vibe-resolver/src/m_05_a_2_cases.rs crates/vibe-core/src/manifest/package.rs crates/vibe-core/src/manifest/package/m_05_a_2_cases.rs
Exit 0; scoped-fmt.log.
git diff --check -- <six candidate paths listed below>
Exit 0; diff-check.log. New test files also passed scoped rustfmt; final patch whitespace check recorded separately.
Python XML parse and JSON structural comparison: exit 0. Compared task JSON with HEAD after substituting only M-05-A.2 read_paths: equal. No other task/criteria/check field changed.

Historical cargo fmt --all -- --check: exit 1; not rerun broadly. Known out-of-perimeter diagnostics, not repaired:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs:571:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:10:
Diff in /fast/git/v/vibevm-next/crates/vibe-core/src/manifest/change/m_02_b_1_cases.rs:188:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:83:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:92:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:234:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs:100:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs:2:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission.rs:48:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:34:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:184:
Diff in /fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs:289:

Findings and evidence boundaries:
- Runtime: qualified dep:group/name stays intact; group/name/feature and group/name?/feature split at first slash, qualified weak misclassified strong (confirmed defect). Unqualified weak records a request without activation, strong activates.
- Runtime: exclusive permits zero/one members, all-features violates alternatives; validator treats all-features activation as definition validity and rejects a valid table (confirmed defect). Core rejects inline dependency features/optional/default-features as unsupported wire syntax.
- Runtime: all backends allow both requires_any alternatives. Naive/SAT fail absent-first alternative and accept a chosen name at an incompatible version (defects/capability gap); resolvo finds the available alternative and rejects wrong-version unsatisfiability. Direct routes omit feature/target activation; explicit incompatible roots fail for every backend.
- Runtime: context(not flow:b) changes true to false when b is inserted; presence supplies no version implication. This proves neither loop convergence nor a production retention defect. No magic iteration count used.
- Source-audited actual dispatch: PackageSourceOptions.solver -> builder.validate_solver -> Local/Multi/Embedded -> source.cell_selection -> cells.selection_flags (None=resolvo) -> cells.dep_solver. All nine provider/backend combinations are present. Unknown surface solver rejected; invalid internal combination unreachable panic. No automatic backend switch. SAT explicitly uses naive as its branch checker; resolvo uses solve_hard plus recommendation trials.
- Builder explicit registry selects Local; discovered local family selects Embedded with optional declared Multi walk; otherwise available declared/git/offline-store route selects Multi or errors. RegistryPackageSource delegates all three solver/metadata methods. PinnedLocalFactory uses explicit Local/default resolvo and exact-pin qualifier, refuses online, prior lock, or embedded input.
- solve_masked_cell forwards same selected name/provider to vibe_install::solve_with_visibility_mask; metadata_manifest_cell forwards corresponding provider to metadata_manifest. Their helper implementation files are not in the literal read roster; no deeper dispatch claim.
- plan.rs invokes source.solve, retries unheld roots through the same source on held-pin conflict, resolves visibility, fetches with request.features, then calls expand_conditional_deps with those features. This is actual caller inventory, not proof of feature omission from production installs. plan.rs imports fetch_or_defer and expand_conditional_deps from mod fetch; crates/vibe-install/src/plan/fetch.rs is not listed. End-to-end weak consumption/conditional retention is unverified, explicitly named rather than silently inferred. No package-source integration suite or installation was run.

Dispositions in captured acceptance order:
1 satisfied: permanent capability matrix records direct runtime behavior, actual source-audited caller/backend selection, unsupported combinations, and explicit evidence boundaries.
2 satisfied: executed conditional repro asserts before/after truth and version outcomes, never iteration-count proof.
These dispositions characterize behavior and do not assert that known production defects were fixed or that global formatting passes. Root review/acceptance remains outstanding.

Patch contains only these six paths (no index mutations):
campaigns/next/tasks/M-05-A.json
crates/vibe-core/src/manifest/package.rs
crates/vibe-resolver/src/lib.rs
vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml
crates/vibe-resolver/src/m_05_a_2_cases.rs
crates/vibe-core/src/manifest/package/m_05_a_2_cases.rs

Retained Rust SHA-256:
{
  "crates/vibe-core/src/manifest/package.rs": "4e15b00f49551a0c59a60cfcc308d5174d6988b394252a95798a908bae97497f",
  "crates/vibe-resolver/src/lib.rs": "8bed408f057d983d0ebe6c20a621bec6fa823238c278bf5af55417dc2cae5705",
  "crates/vibe-resolver/src/m_05_a_2_cases.rs": "4ddaabf4bca56c14c2bd3bebb1c9716bdaf5afbfdfbec09f7306c1f7bac5cf0d",
  "crates/vibe-core/src/manifest/package/m_05_a_2_cases.rs": "ec1ac6c1eea112d3afc8a818b8e26d19fd8ee984bdf3c2922722db9a1c6d03fc"
}

Final added-patch-line whitespace validation: exit 0; patch-whitespace.log. git apply --stat (read-only): exit 0, six files, 387 insertions, one deletion.
