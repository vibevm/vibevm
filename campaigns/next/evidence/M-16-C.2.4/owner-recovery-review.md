# M-16-C.2.4 owner recovery review (candidate)

This is a source and focused-test map for the current local `next` branch. It
does not accept M-16-C.2.4, authorize public 1.1.0 publication, or claim a
single post-effect integration test across all four mechanisms.

| Owner | Retained effect / failure evidence | Cold attach and limit |
| --- | --- | --- |
| Build | `build/journal/tests.rs`: `one_real_offline_cargo_build_retains_verified_result_and_never_respawns` counts the Cargo command effects; `crash_after_cargo_apply_before_record_is_partial_and_never_respawns` observes target/cache presence and no artifact record after an injected crash, without another spawn. | `cold_build_attach_recovers_completed_without_execution_inputs_or_spawn` reopens a lease and recovers the same result; a replaced same-path root is refused by `copied_build_c1_and_capsule_at_replaced_same_path_refuse_cold_attach`. Fixed offline Cargo only. |
| Package | `package/journal/tests.rs`: `fixed_copy_retains_verified_result_and_duplicate_never_reapplies` counts one copy; `interruption_after_copy_before_record_is_uncertain_and_never_retries` and `artifact_record_without_owner_result_never_completes_or_recopies` retain partial truth. | `completed_record_or_output_drift_downgrades_inspection_without_copy` rejects a changed result/output. Fixed static-file package only. |
| Deploy | `deploy/bridge_tests.rs`: `cold_deploy_attach_recovers_completed_with_reissued_held_roots` retains a verified receipt; `cold_deploy_attach_retains_interrupted_partial_without_provider` retains an uncertain run after an effect boundary. | Cold attach requires reissued host admission and held physical roots; `cold_deploy_reissued_same_path_after_settings_replacement_refuses` checks replacement. Fixed local UserProfile/ProducedToolRuntime route only. |
| Publication | `calls/driver_tests.rs` and `calls/asset_driver_tests.rs` retain per-call Pending on lost DELETE/POST/PATCH replies without replay. `calls/terminal_tests.rs` proves terminal chains of 42 calls for a new draft and 43 for prior-draft replacement; missing/tampered terminal records and EOF remain uncertain. | The production cold inspector rechecks C1, approval, vault and every call record before `CallChainCompleted`. This is private owner machinery: no public or CLI GitHub publication route is enabled. |

`calls/terminal_tests.rs::one_lease_reopens_four_distinct_domain_keys_without_inventing_owner_effects`
proves one root can retain four distinct C1 identities without a daemon. On a
new lease, Publication has a real private terminal result; Build, Package and
Deploy deliberately have **only C1 reservations**, so each reports
`RecoveryRequired`. It also checks foreign scope, wrong keys and no new remote
reads. It does **not** prove a quartet of completed or post-effect partial
domain journals under one lease. The protected Build/Package/Deploy executors
are crate-private; exporting them solely for a test would widen the product
boundary without a corresponding host capability. Their post-effect behavior
is covered by the separate domain tests above.

Remaining before child acceptance: join a fixed host-controlled publication
invocation to the private owner without exposing an unavailable adapter; review
API exit and parked/one-writer behavior against the same trusted host boundary;
and record native platform evidence or explicit unavailable results. The
delete-first replacement of a **published current latest** release remains
unavailable until a pre-delete fallback can be proven. No public Git push,
release or package publication is part of this review.
