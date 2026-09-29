# Foundation report integration fixes

Writer stopped. Changes only migration_application/report.rs (281lines) and validation_tests.rs (496lines). Both under600; targeted rustfmt EXIT0. Earlier other foundation APIs/owners unchanged.

1. Reserved project-stage residue is reportable only when its exact leaf equals the existing identity owner’s Apply/Restore stage name for this report’s established intent. Ordinary parent components are validated through split_relative with a harmless dummy leaf. Unproven ownership stays unproven; reporting grants no removal or path authority. Other stages, reserved parent names, escaped/absolute paths, missing intent and store-location stages refuse. Ordinary residue still uses the existing path law.

2. NoOp can retain a historical completed TargetPublication with ValidatedAfter or TargetRestore with ApprovedBefore. Existing validation still requires intent/all3planner IDs/participating barrier, and refuses pending/uncertain effects. This permits truthful verified replay without claiming a new target write; runtime tests own no-new-write proof.

3. ApprovedBefore/ValidatedAfter always require all3planner identities. Unavailable preflight with null identities remains valid.

4. ReportBuilder::into_report now returns Result<OperationReport, ReportValidationFailure>. Internal failure carries public report:OperationReport and fault:ApplicationValidationError; it is not Serialize or new wire. An invalid construction retains all original effects/warnings/residue/error. Runtime was notified to resolve expected uncertain outcomes before validation and preserve the failure report, never replace it with blank preflight or fabricate successful autocorrection.

Four exact tests bound before compilation and run individually with --locked --offline -p vibe-workspace --lib, jobs4/sharedtarget, -- --exact:

- migration_application::validation_tests::expected_stage_residue_is_reportable_without_namespace_authority
- migration_application::validation_tests::no_op_replay_retains_only_coherent_completed_target_effects
- migration_application::validation_tests::recognized_target_bytes_require_all_planner_identities
- migration_application::validation_tests::report_validation_failure_retains_all_observed_evidence

All4invocations EXIT0, each1passed/0failed/557filtered. Raw separate logs under /tmp/vibevm-next-M13-D1-domain-checks/: report-stage.log, report-replay.log, report-identities.log, report-retention.log. Earlier8pass evidence retained for unchanged cases; no library check repeat, broad test family, source/schema/generated/Cargo/Git/public/user-path changes. Root retains review/integration/acceptance.
