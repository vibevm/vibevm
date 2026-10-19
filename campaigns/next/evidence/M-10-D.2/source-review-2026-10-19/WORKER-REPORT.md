# M-10-D.2 — derivative activation, revocation and recovery

Implementation and focused evidence are ready for Root review. This report is not Work acceptance. The unchanged original task and both criteria remain in force.

## Changes and actual entrypoints

Preserved the prior dirty/untracked implementation and its generic 0600 publication-mode correction. Added the missing ordinary trusted-host library consumer: `vibe_lifecycle::retained_mechanism::FixedDeployHostAdmission::issue_from_trusted_host` -> `with_derivative(held entry, base, local)` -> `submit_fixed_bound_vibe_bin` -> existing C1/C2 bridge `submit_with_request_mode` -> `DerivativeAdmission::execute_with_hook` -> protected bound executor -> `BoundVibeBinProvider` and `Transaction::apply_bound`. The new positive fixture drives this public constructor/prepared-input/submission route, not only an internal helper.

The bridge retains exact derivative result identity, artifact digest, binding/principal, bound plan and real receipt evidence. Independently read actual stored payload and launcher/pointer post-digests join the receipt. A second exact result with newly issued non-cloneable host admission updates generation 0 to 1 in the SAME bound deployment home. No legacy/unbound transaction fallback, wire-only grant conversion or fixture-stop relabelling is used. Base/local source checks still use MaterializedDerivative::read independently at generation boundaries. Actual code publication is the observed effect; the derivative payload is never launched.

Bound updates require exact immutable prior receipt history, the committed lock sidecar/plan, current prior destination digests, current host/principal/root/generation authority, and an unchanged owned destination set. Receipt/intent/sidecar values are rechecked under the incumbent locks. Pending intent recovery binds the same plan, own generation and prior/desired states; third digests refuse. Immutable history permits recovery after history publication, receipt publication or sidecar promotion without inventing a new generation, even when the recovery clock changes. Old same-key uncertain submissions remain attached to their original run rather than being replayed as absent.

The SAME bound owner's retained cleanup route is `FixedDeployCleanupPlan::prepare_from_trusted_host` -> `approve_from_trusted_host` -> `cleanup_fixed_bound_vibe_bin`. Preparation reads retained C1 capsule, original verified receipt and closed builtin descriptor semantics; it does not resolve a current descriptor/provider or reopen an artifact source. Approval issues a separate non-cloneable cleanup carrier from fresh trusted host facts, not from ReadOnlyDeployAdmission or an old forward permit. Execution retains physical/deployment/resource locks, foreign ownership census, receipt/sidecar snapshots and independent observation. Missing descriptor/source fixtures remain inspectable and removable, preserving a FOREIGN neighbor.

Cleanup extends the existing inverse owner with a tagged `bound-cleanup/1:<exact receipt hash>` causal handle, not invented prior bytes. That marker precedes deletion and admits only original post-digests or marker-qualified absence on restart. Independent absence verification precedes the empty RolledBack receipt, bound sidecar retirement and marker retirement. The all-or-none bound sidecar law is preserved by removing the retired sidecar, never publishing an unbound empty one. The incumbent immutable bridge ResultRecord owner publishes `cleanup-result` for the final cleanup receipt. A missing final result remains Uncertain; fresh authority re-verifies absence and finishes publication without repeating removal. Ordinary retained inspection then returns the actual cleaned receipt. No new lifecycle, grant truth store or strict wire shape was introduced.

## Scope, exposure and independent laws

This is an explicitly configured embedding-library surface. The real non-test callers are fixed_deploy::submit_fixed_bound_vibe_bin, bridge::submit_with_request_mode, and the retained cleanup submission/bridge/transaction chain named above; lib.rs already publicly exports retained_mechanism. The positive fixture exercises the actual public library entrypoints. No new CLI/MCP activation command, automatic discovery, production vendor commissioning or general native execution surface is claimed. Existing OpenCode/skill consumers retain their ordinary ownership APIs; they are not claimed to gain the bound builtin grant.

Supported bound updates retain the exact artifact/descriptor ownership join and destination set. Arbitrary artifact-id/destination retargeting, cross-binding migration, and restoration of an earlier payload generation are not claimed. Cleanup removes active owned launcher/pointer resources and retains immutable CAS payloads and verified receipt history. Revocation/missing descriptors cannot grant forward work. New cleanup authority remains tied to the exact retained target binding and currently admitted roots/principal.

The existing external partial carrier has no inverse member. Its existing unavailable-detail representation explicitly carries pending inverse status rather than silently reporting no outstanding work; no orchestrator/CLI/MCP or generated carrier was edited. Additive PROP074 text replaces only the prior attempt's first-generation implementation limit. All 16 HEAD section IDs, attributes and text are preserved exactly; one additive section is present. Current-source, no-inherited-grant, binding/principal/physical-root, ownership, passive five-family and downstream laws remain independent. XML parsed; no strict carrier/fact registration changed and no codegen closure is claimed.

Root-supplied dispatch authority states that D1 and mandatory source prerequisites were qualified and assigns the exclusive Cargo target and private TMP. Those are attributed coordinator facts, not predecessor acceptance recreated by this worker. No Git mutations, delegation, installs, full suites, external deployment, credential/history scan or sandbox claim occurred.

## Actual verification and retained prior evidence

Current continuation executed 33 DISTINCT Rust cases successfully: 11 in the derivative D2 family, 7 incumbent bound-effect cases, 10 incumbent bridge cases and 5 incumbent pinned-provider cases. Four D2 cases are new in this continuation. Repeated exact cases and subscenario loop iterations are NOT additional distinct tests. After the final foreign-ownership census addition, the affected ordinary-host four-case family passed again. The exact ordinary join case was rerun with --nocapture for two scalar joins, not as an extra distinct pass.

All Cargo tests used absolute /home/olegchir/.cargo/bin/cargo, --offline --locked -j2, selected --lib families, CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target and TMPDIR=/tmp/vibevm-m10d2-continuation-l92lb22s. No target install/version change or full-suite execution. Test effects were confined to the injected temporary fixture roots. Existing adapter fixture version probes do not constitute derivative payload execution.

The earlier 12 distinct reported passes are preserved as attributed prior evidence: 7 derivative + 3 generic + 1 projection + 1 workspace. The changed lifecycle path's 7 derivative cases were rerun for concrete changed invariants; unchanged generic/projection/workspace families were not blanket-rerun. Current source inspection confirms the preserved workspace case loops all five families (commonmark, xml-data, vibe-spec, opaque-file, nested-tree), exact bytes and independent base/local drift; the preserved projection case uses ordinary receipt reconciliation to update/remove a tree while preserving FOREIGN and inert script bytes. The existing generic cases cover separate exact grant/update/revocation/cleanup/recovery semantics. Prior execution is not relabelled as this continuation's own run. Root's original host-run verification slots remain owned by Root.

Current deciding case obligations:
- ordinary_host_derivative_submission_updates_the_same_bound_owner: public prepared host route, two exact result/artifact/plan/grant/effect/receipt joins, generation update, independent actual payload/resource reads, history, descriptor/source removal, fresh cleanup approval, foreign-principal refusal, actual cleanup and final public inspection.
- ordinary_host_interruption_recovers_same_generation_and_refuses_third_digest: destination/history/receipt/sidecar windows, retained intent, fresh current-source recovery under a changed clock, foreign third-digest refusal with no overwrite.
- ordinary_host_retained_cleanup_interruption_and_drift_keep_exact_ownership: cleanup effect/receipt/lock/result windows, actual retained partial inspection, fresh cleanup restart, foreign third-digest refusal and neighbor preservation.
- ordinary_host_changed_receipt_refuses_forward_and_prepared_cleanup_without_effect: mutate a genuinely produced receipt through the incumbent writer, refuse forward and already-prepared cleanup, leave destination and drifted receipt untouched, create no new intent/inverse.

The remaining seven derivative D2 cases cover source drift before/after effect, changed result/principal/descriptor/host, upstream artifact identity, missing provider, exact first publication/receipt join, and ordinary OpenCode inert-skill publication/removal. The incumbent regression families were selected because this continuation changed their executor/bridge/provider invariants, not because the earlier run stopped.

Literal original cargo fmt --all -- --check: EXIT 1, 12 hunks across 7 unchanged baseline files. This failure is not waived or converted into scoped-format success. git diff --quiet HEAD for those exact 7 files returned 0. Literal git diff --check: EXIT 0 after the final source change. XML parse and exact preservation of existing section text/attributes passed. No tests failed at the final source state; globalfmt remains the explicit required-check failure.

## Retained failures and limits

Actual intermediate failures remain in the retained logs and excerpts below:
1. cleanup-compile.log: Cargo exit 101, 0 tests executed, nonexistent ClientExecutables::default. Corrected with explicit unavailable clients; closed builtin cleanup does not invoke clients.
2. cleanup-check.log: Cargo exit 101, 1 actual test failed on the real bound sidecar all-or-none invariant. Corrected by retiring the bound sidecar only after verified removal and empty receipt, preserving the invariant.
3. derivative-final-3.log: Cargo exit 101, 0 tests executed, missing MechanismError conversion in cleanup's independent finalization-tail verify. Corrected through incumbent DeployError conversion.
The earlier attempt's other failures remain attributed in its already-published partial report, not erased or recounted as current successes.

Both implementation criteria are submitted as satisfied for Root review at the bounded library/fixture level: exact result/grant/artifact/effect/receipt joins and five-family plus downstream active/inert evidence. This is not Work acceptance or a claim that every global check passed. The 7-file global-format baseline and independent host verification/acceptance remain explicit owner work. New ordinary-host recovery scenarios completed. Intentional refusal-only scenarios retain uncertain local journals until owned temporary-fixture teardown. No external deployment occurred.

## Scalar ordinary-host joins (actual stdout)

```text
M10D2_ORDINARY_JOIN {"result":"sha256:2f8b273c0f22860a9b71df8048d4e8b0cea00c137324621b4540bea923e92bfe","artifact_id":"helper.exe","artifact_digest":"301ce033b5e9cdfaaffd4c369bad137508b68d6007dafa770a15ac21d95960c5","binding_id":"sha256:ae33297c8beaa8ef97f4a9f5f04fe5372bf4c1b262a1a040d7df94006f3e3ccd","plan_sha256":"sha256:5670885f70de4a7ffcdedddf9e79b464571a521c8c51c88a4c14518aa39c4252","receipt_sha256":"sha256:2b4f7537f8b8d5d1d0bed460f1ad4aee8606f251faf59c51012ed16498d48c2f","run_id":"ec78071f6dbd4fdd3dd008203aba9dfe","generation":0,"observed_resources":[{"post_digest":"817a7cd7316c1b968cece67aff0e5c609260b7fc22fd19cce7908304262589ac","resource":"bin/local-helper"},{"post_digest":"325d7fc483ce286141016cde8a878cf3a3f43a808e9883e26bcc297cfa6a92a3","resource":"bin/local-helper.current"}],"payload_executed":false}
M10D2_ORDINARY_JOIN {"result":"sha256:0e5aa2cbe6d37a746bf2970e20df11fed8c224090fe88ac149ec0c84ef20e3af","artifact_id":"helper.exe","artifact_digest":"911a551078fddd7f779e0f91bb214388fcb72a06aee79d64496c642e1e4e94bd","binding_id":"sha256:ae33297c8beaa8ef97f4a9f5f04fe5372bf4c1b262a1a040d7df94006f3e3ccd","plan_sha256":"sha256:4498cf120240621d3be9686ef8b437809a026a1ea30898d7a3e4da4739e207fe","receipt_sha256":"sha256:06a18e41223741a31834fdd194e37d00b060894eff05bd9c639ababedf3c6e93","run_id":"81d5e25239826096e6201336015c8c96","generation":1,"observed_resources":[{"post_digest":"817a7cd7316c1b968cece67aff0e5c609260b7fc22fd19cce7908304262589ac","resource":"bin/local-helper"},{"post_digest":"37f4d3f9ba1c8c69342878629c6f271e1020ef271f96695ed217410ebc25862f","resource":"bin/local-helper.current"}],"payload_executed":false}
```

## Final selected output

derivative-final-4.log

```text
running 11 tests
test mechanism::deploy::derivative::m_10_d_2_cases::upstream_artifact_identity_cannot_enter_active_consumer ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::missing_provider_after_admission_refuses_forward_without_erasing_result ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::base_and_local_changes_between_plan_and_effect_refuse ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::source_drift_after_publication_retains_bound_intent_and_uncertain_effect ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::changed_result_principal_descriptor_and_host_refuse_before_effect ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::exact_result_artifact_grant_effect_and_independent_receipt_join ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_changed_receipt_refuses_forward_and_prepared_cleanup_without_effect ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_derivative_submission_updates_the_same_bound_owner ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::opencode_existing_consumer_publishes_inert_derivative_skill_and_removes_only_owned_members ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_retained_cleanup_interruption_and_drift_keep_exact_ownership ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_interruption_recovers_same_generation_and_refuses_third_digest ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.60s

```

ordinary-final.log

```text
running 4 tests
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_changed_receipt_refuses_forward_and_prepared_cleanup_without_effect ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_derivative_submission_updates_the_same_bound_owner ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_interruption_recovers_same_generation_and_refuses_third_digest ... ok
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_retained_cleanup_interruption_and_drift_keep_exact_ownership ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 966 filtered out; finished in 0.59s

```

bound-regression.log

```text
running 7 tests
test mechanism::deploy::bound_effect_tests::changed_body_or_stale_host_generation_refuses_before_effect ... ok
test mechanism::deploy::bound_effect_tests::generation_drift_after_publication_retains_bound_intent_and_reports_uncertain ... ok
test mechanism::deploy::bound_effect_tests::bound_sidecar_copied_to_another_binding_is_refused_by_no_create_view ... ok
test mechanism::deploy::bound_effect_tests::protected_bound_deploy_changed_request_conflicts_before_provider ... ok
test mechanism::deploy::bound_effect_tests::protected_bound_deploy_replays_verified_receipt_without_second_effect ... ok
test mechanism::deploy::bound_effect_tests::protected_bound_deploy_missing_result_is_uncertain_and_never_reapplies ... ok
test mechanism::deploy::bound_effect_tests::two_admitted_homes_publish_physical_builtin_and_distinct_bound_receipts ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 963 filtered out; finished in 0.05s

```

bridge-regression.log

```text
running 10 tests
test mechanism::deploy::bridge_tests::pending_c1_reservation_without_bridge_never_acquires_destination ... ok
test mechanism::deploy::bridge_tests::public_authored_deploy_refuses_profile_scope_generation_and_source_drift_before_effect ... ok
test mechanism::deploy::bridge_tests::public_post_ready_generation_drift_keeps_run_and_partial_effect_truth ... ok
test mechanism::deploy::bridge_tests::cold_deploy_attach_retains_interrupted_partial_without_provider ... ok
test mechanism::deploy::bridge_tests::protected_interruption_after_first_effect_keeps_intent_and_never_reapplies ... ok
test mechanism::deploy::bridge_tests::cold_deploy_reissued_same_path_after_settings_replacement_refuses ... ok
test mechanism::deploy::bridge_tests::public_damaged_result_attaches_as_uncertain_without_redeploy ... ok
test mechanism::deploy::bridge_tests::cold_deploy_attach_recovers_completed_with_reissued_held_roots ... ok
test mechanism::deploy::bridge_tests::public_authored_fixed_deploy_submit_and_replay_keep_one_owner_run ... ok
test mechanism::deploy::bridge_tests::copied_receipt_sidecar_and_result_cannot_complete_another_bound_home ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 960 filtered out; finished in 0.09s

```

provider-regression.log

```text
running 5 tests
test mechanism::vibebin::bound_provider::tests::first_bound_plan_refuses_an_unowned_existing_pointer ... ok
test mechanism::vibebin::bound_provider::tests::bound_plan_refuses_a_resolved_artifact_outside_its_pinned_record ... ok
test mechanism::vibebin::bound_provider::tests::preplan_view_refuses_apply_before_any_destination_write ... ok
test mechanism::vibebin::bound_provider::tests::bound_plan_and_observation_use_pinned_destination_and_refuse_effects ... ok
test mechanism::vibebin::bound_provider::tests::locked_bound_apply_publishes_payload_then_launcher_then_pointer_and_checkpoints ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 965 filtered out; finished in 0.01s

```

## Actual failure excerpts

cleanup-compile.log

```text
error[E0599]: no associated function or constant named `default` found for struct `mechanism::deploy::model::ClientExecutables` in the current scope
   --> crates/vibe-lifecycle/src/mechanism/deploy/bridge/cleanup.rs:127:102
    |
127 |         user_home: locked.user_home().root_path(), clients: &super::super::model::ClientExecutables::default(),
    |                                                                                                      ^^^^^^^ associated function or constant not found in `mechanism::deploy::model::ClientExecutables`
    |
   ::: crates/vibe-lifecycle/src/mechanism/deploy/model.rs:166:1
    |
166 | pub struct ClientExecutables {
    | ---------------------------- associated function or constant `default` not found for this struct
    |
    = help: items from traits can only be used if the trait is implemented and in scope
    = note: the following trait defines an item `default`, perhaps you need to implement it:
            candidate #1: `std::default::Default`

warning: unused import: `DeployProvider`
 --> crates/vibe-lifecycle/src/mechanism/deploy/bridge/cleanup.rs:8:24
  |
8 | use crate::mechanism::{DeployProvider, DeployTargetRequest, EffectClass};
  |                        ^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

For more information about this error, try `rustc --explain E0599`.
warning: `vibe-lifecycle` (lib test) generated 1 warning
error: could not compile `vibe-lifecycle` (lib test) due to 1 previous error; 1 warning emitted
```

cleanup-check.log

```text
running 1 test
test mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_derivative_submission_updates_the_same_bound_owner ... FAILED

failures:

---- mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_derivative_submission_updates_the_same_bound_owner stdout ----

thread 'mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_derivative_submission_updates_the_same_bound_owner' (3671740) panicked at crates/vibe-lifecycle/src/mechanism/deploy/derivative/m_10_d_2_cases.rs:532:71:
called `Result::unwrap()` on an `Err` value: Refused("bound deployment of `local-helper` under `sha256:9c0f3b7de2f72654692bddba15c23708c51e3ebb9520439707cbc82e0391359a` may have published resources: lock-resources.json operation binding differs from the deployment state's expected binding; the intent is retained and recovery requires fresh host admission")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    mechanism::deploy::derivative::m_10_d_2_cases::ordinary_host_derivative_submission_updates_the_same_bound_owner

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 967 filtered out; finished in 0.24s

error: test failed, to rerun pass `-p vibe-lifecycle --lib`
```

derivative-final-3.log

```text
error[E0277]: `?` couldn't convert the error to `BoundDeployBridgeError`
   --> crates/vibe-lifecycle/src/mechanism/deploy/bridge/cleanup.rs:299:61
    |
240 | ) -> Result<Vec<String>> {
    |      ------------------- expected `BoundDeployBridgeError` because of this
...
299 |         let observed = provider.verify(&request, &resources)?;
    |                                 ----------------------------^ the trait `From<mechanism::error::MechanismError>` is not implemented for `BoundDeployBridgeError`
    |                                 |
    |                                 this can't be annotated with `?` because it has type `Result<_, mechanism::error::MechanismError>`
    |
note: `BoundDeployBridgeError` needs to implement `From<mechanism::error::MechanismError>`
   --> crates/vibe-lifecycle/src/mechanism/deploy/bridge.rs:36:1
    |
 36 | pub(crate) enum BoundDeployBridgeError {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
    = help: `BoundDeployBridgeError` implements trait `From<T>`:
              From<ScratchError>
              From<lease::LifecycleLeaseError>
              From<mechanism::deploy::error::DeployError>
              From<serde_json::Error>
              From<submission::SubmissionError>

For more information about this error, try `rustc --explain E0277`.
error: could not compile `vibe-lifecycle` (lib test) due to 1 previous error
```

## Literal formatting failure locations

```text
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs:571:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:10:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:83:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:92:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:234:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs:100:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs:2:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:34:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:184:
Diff in /fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs:340:
```

## Artifact content and retained local logs

Patch SHA256 (content checksum only): 07ec50f0e0f5cb56972e681384a781cff0439c1a1ce030665bd0bc1b572e1790; 161499 bytes. Patch includes all 18 tracked dirty paths and all 5 untracked source files, including useful prior work. No Git mutation.

- baseline-proof.log: 149 bytes, SHA256 77bd9394cf114a0b894de14fe3e52bcd687540c04225b5bc6a810c35d3ef0708. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- bound-regression.log: 20985 bytes, SHA256 39b6367d1765cae08708a3da041bb3d9db767f1fdebb8e517081a26b621d495e. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- bridge-regression.log: 21198 bytes, SHA256 74f57d6ce31ae45f2a2524fcee74bc21d5cc0c328ea8d7419a8e2c2635a90a36. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- cleanup-check-2.log: 20266 bytes, SHA256 28926204d549a2465276d2a2ca949f379236d69d1a410673d31781395db17a51. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- cleanup-check.log: 21265 bytes, SHA256 89db76d9dedbaf8bd1be49e6914f1370cfb7c5c985145df2e6cd92725976ea68. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- cleanup-compile.log: 19667 bytes, SHA256 2152920076f739e543a167d4cc5abb7a9c37ddc7d329438d62a15d81ce570e72. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- cleanup-recovery-check.log: 20276 bytes, SHA256 dd3f6636f9a9b2b74bc9adf2f46511114d420afdc83a15be4f97ca2c710458af. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- derivative-final-2.log: 21569 bytes, SHA256 ceac676fe9c77e964eea139cb64c1556a5af89fc7fc61783f42f3af8fe2a8b52. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- derivative-final-3.log: 19618 bytes, SHA256 931933d9190ca97f037bbc7ee32697810d45a3f07a543fa4d4b4bdfd6b43557d. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- derivative-final-4.log: 21569 bytes, SHA256 77d51241cc433133badda1799f855e5168e155a218a589180ca4c77e86c25b7e. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- derivative-final.log: 21428 bytes, SHA256 d73c9aec81a119702737b812ef9ab08e5b105137e4d6f5060270b3a4fd7a07bb. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- diff-check-final.log: 0 bytes, SHA256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- diff-check.log: 0 bytes, SHA256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- globalfmt-final.log: 6742 bytes, SHA256 0830a5205329534a498c9a4c6343785d774c13180297fb64e076b6f1d03538db. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- globalfmt.log: 6742 bytes, SHA256 0830a5205329534a498c9a4c6343785d774c13180297fb64e076b6f1d03538db. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- ordinary-check.log: 20378 bytes, SHA256 44cbcd2aa28e470a0b7e6ea9874f113d0071f35923465a7268ae3a1058ca29cf. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- ordinary-final.log: 20678 bytes, SHA256 fc64db32ba93b0ead4b18f47e64a9e15519879ec596af3b2414f28dcfc1a45dc. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- ordinary-join.log: 21912 bytes, SHA256 65e206c06469ce55a75e76ef37e9f5f7f90161f0f604d4dfa047da5ade5cf885. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- provider-regression.log: 20658 bytes, SHA256 d8ee3b1a23e6b58b2008f927fe1eae76eea2e782609865839927b0a2c6882cd8. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- recovery-check.log: 20390 bytes, SHA256 6ff1a1099e79f35022ee0c986fd7afd8e7f749ae92214e6ce0b0a9582dc56723. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
- xml-check.log: 162 bytes, SHA256 de385408bd3c54c95afd137b83a5036aade6663085dc46935c61b1ddd4a514aa. Retained under /tmp/vibevm-m10d2-continuation-l92lb22s.
