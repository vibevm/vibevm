#![cfg(test)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#BOOTSTRAP-STAGE-READINESS");

use super::*;
use std::collections::BTreeMap;
use vibe_wire::generated::preview::e1::check_request::CheckRequestFormat;
use vibe_wire::generated::preview::e1::transition::{
    ContributionScope, DocumentationCaptureRef, PendingCapture, PendingCaptureState,
    PendingDocumentationState, PendingGuideState, RecordDigest, SourceDigestRef,
};

fn transition() -> Transition {
    serde_json::from_str(include_str!(
        "../../../../formats/corpora/preview-transition/e1/valid/captured.json"
    ))
    .expect("existing registered transition fixture")
}

fn request(mode: CheckRequestMode, stage: Option<CheckRequestStage>) -> CheckRequest {
    CheckRequest {
        schema: 1,
        transition: "/unopened-checkout/release/transition.json".into(),
        mode,
        stage,
        format: CheckRequestFormat::Json,
        repository_roots: None,
    }
}

fn report(request: &CheckRequest, transition: &Transition) -> CheckReport {
    let mut report = empty_report(report_mode(&request.mode), report_stage(&request.stage));
    if let Err(error) = readiness::check(request, transition, &mut report) {
        panic!("unexpected {}: {}", error.code, error.reason);
    }
    report
}

fn finding<'a>(report: &'a CheckReport, code: &str) -> &'a Finding {
    report
        .findings
        .iter()
        .find(|finding| finding.code == code)
        .unwrap_or_else(|| panic!("missing {code}"))
}

fn complete_declarations(transition: &mut Transition) {
    let before = transition.dp1.clone();
    transition.dp2 = PendingCapture {
        state: PendingCaptureState::Sealed,
        capture_id: Some(before.capture_id),
        source: Some(before.source),
        capture_file_sha256: Some(before.capture_file_sha256),
        commit_sha1: Some(before.commit_sha1),
        git_tree_sha1: Some(before.git_tree_sha1),
    };
    transition.documentation.state = PendingDocumentationState::Verified;
    transition.documentation.capture_id = Some("docs-current".into());
    transition.documentation.source = Some("unverified-current.json".into());
    transition.documentation.capture_file_sha256 = Some("a".repeat(64));
    transition.documentation_base = Some(DocumentationCaptureRef {
        capture_id: "docs-base".into(),
        source: "unverified-base.json".into(),
        capture_file_sha256: "b".repeat(64),
    });
    transition.documentation_map = Some(SourceDigestRef {
        source: "unverified-map.json".into(),
        sha256: "c".repeat(64),
    });
    transition.migration_guide.state = PendingGuideState::Verified;
    transition.migration_guide.source = Some("unverified-guide.md".into());
    transition.migration_guide.source_sha256 = Some("d".repeat(64));
    transition.proof.capture_validated = true;
    transition.proof.contributions_verified = true;
    transition.proof.documentation_verified = true;
    transition.proof.migration_rehearsed = true;
    transition.proof.final_product_panel_passed = true;
    transition
        .validate_preview()
        .expect("declarations are shape-valid, not verified");
}

fn stages() -> Vec<(CheckRequestMode, Option<CheckRequestStage>, &'static str)> {
    vec![
        (
            CheckRequestMode::Bindings,
            Some(CheckRequestStage::Roles),
            "bindings-roles-owner-unavailable",
        ),
        (
            CheckRequestMode::Bindings,
            Some(CheckRequestStage::Linked),
            "bindings-linked-migration-owner-unavailable",
        ),
        (
            CheckRequestMode::Documentation,
            Some(CheckRequestStage::Updated),
            "documentation-reviewed-delta-unverified",
        ),
        (
            CheckRequestMode::Documentation,
            Some(CheckRequestStage::Verified),
            "documentation-current-verification-unverified",
        ),
        (
            CheckRequestMode::Release,
            Some(CheckRequestStage::Reviewed),
            "release-net-reconciliation-unverified",
        ),
        (
            CheckRequestMode::Release,
            Some(CheckRequestStage::Verified),
            "release-rehearsal-unverified",
        ),
        (
            CheckRequestMode::Retired,
            None,
            "retired-scaffold-free-candidate-unverified",
        ),
    ]
}

#[test]
fn each_late_stage_reports_its_own_due_prerequisites_and_exact_stage() {
    for (mode, stage, expected) in stages() {
        let request = request(mode, stage);
        let result = report(&request, &transition());
        assert!(matches!(result.verdict, CheckReportVerdict::Pending));
        assert_eq!(result.mode, report_mode(&request.mode));
        assert_eq!(result.stage, report_stage(&request.stage));
        assert!(result.checked_claims.is_empty());
        finding(&result, expected);
        assert!(
            result
                .findings
                .iter()
                .all(|finding| finding.code != "stage-pending")
        );
    }
}

#[test]
fn producer_complete_flags_and_verified_labels_cannot_establish_owner_results() {
    let mut transition = transition();
    complete_declarations(&mut transition);
    for (mode, stage, expected) in stages() {
        let result = report(&request(mode, stage), &transition);
        assert!(matches!(result.verdict, CheckReportVerdict::Pending));
        assert!(result.checked_claims.is_empty());
        finding(&result, expected);
    }
}

#[test]
fn linked_names_native_roles_predecessor_and_actual_domain_joins() {
    let role = report(
        &request(CheckRequestMode::Bindings, Some(CheckRequestStage::Roles)),
        &transition(),
    );
    assert!(matches!(
        finding(&role, "bindings-roles-owner-unavailable").state,
        FindingState::Unsupported
    ));
    finding(&role, "bindings-linked-proof-later");
    assert!(
        !role
            .findings
            .iter()
            .any(|finding| finding.code == "bindings-linked-migration-owner-unavailable")
    );
    let linked = report(
        &request(CheckRequestMode::Bindings, Some(CheckRequestStage::Linked)),
        &transition(),
    );
    for code in [
        "bindings-roles-owner-unavailable",
        "bindings-linked-capability-owner-unavailable",
        "bindings-linked-migration-owner-unavailable",
    ] {
        assert!(matches!(
            finding(&linked, code).state,
            FindingState::Unsupported
        ));
    }
    finding(&linked, "bindings-documentation-release-proof-later");
}

#[test]
fn bindings_and_release_predecessors_require_selected_records_and_scope() {
    for (mode, stage) in [
        (CheckRequestMode::Bindings, Some(CheckRequestStage::Roles)),
        (CheckRequestMode::Bindings, Some(CheckRequestStage::Linked)),
        (CheckRequestMode::Release, Some(CheckRequestStage::Reviewed)),
        (CheckRequestMode::Release, Some(CheckRequestStage::Verified)),
        (CheckRequestMode::Retired, None),
    ] {
        let request = request(mode, stage);
        let missing = report(&request, &transition());
        finding(&missing, "bindings-record-selection-missing");
        finding(&missing, "bindings-contribution-scope-missing");
        assert!(matches!(
            finding(&missing, "bindings-roles-owner-unavailable").state,
            FindingState::Unsupported
        ));

        let mut selected = transition();
        complete_declarations(&mut selected);
        selected.change_records = vec!["change-example".into()];
        selected.contribution_scope = Some(ContributionScope {
            before_capture_id: selected.dp1.capture_id.clone(),
            after_capture_id: selected.dp1.capture_id.clone(),
            record_digests: vec![RecordDigest {
                change_id: "change-example".into(),
                sha256: "e".repeat(64),
                required_claim_ids: Some(vec!["selected-claim".into()]),
            }],
        });
        let present = report(&request, &selected);
        finding(&present, "bindings-record-selection-unverified");
        finding(&present, "bindings-contribution-scope-unverified");
        assert!(!present.findings.iter().any(|finding| matches!(
            finding.code.as_str(),
            "bindings-record-selection-missing" | "bindings-contribution-scope-missing"
        )));
        assert!(matches!(present.verdict, CheckReportVerdict::Pending));
        assert!(present.checked_claims.is_empty());
        assert!(matches!(
            finding(&present, "bindings-roles-owner-unavailable").state,
            FindingState::Unsupported
        ));
    }
}

#[test]
fn missing_documentation_subjects_differ_from_selected_unverified_declarations() {
    for stage in [CheckRequestStage::Updated, CheckRequestStage::Verified] {
        let request = request(CheckRequestMode::Documentation, Some(stage));
        let missing = report(&request, &transition());
        for code in [
            "documentation-current-capture-missing",
            "documentation-map-missing",
            "documentation-base-capture-missing",
            "product-candidate-capture-missing",
        ] {
            finding(&missing, code);
        }
        let mut selected = transition();
        complete_declarations(&mut selected);
        let present = report(&request, &selected);
        for code in [
            "documentation-current-capture-unverified",
            "documentation-map-unverified",
            "documentation-base-capture-unverified",
            "product-candidate-capture-unverified",
        ] {
            finding(&present, code);
        }
        assert!(
            !present
                .findings
                .iter()
                .any(|finding| finding.code.ends_with("-missing"))
        );
        assert!(matches!(present.verdict, CheckReportVerdict::Pending));
    }
}

#[test]
fn release_reviewed_keeps_actual_rehearsal_as_later_proof() {
    let reviewed = report(
        &request(CheckRequestMode::Release, Some(CheckRequestStage::Reviewed)),
        &transition(),
    );
    finding(&reviewed, "migration-guide-selection-missing");
    finding(&reviewed, "release-guidance-review-unverified");
    finding(&reviewed, "release-rehearsal-proof-later");
    assert!(
        !reviewed
            .findings
            .iter()
            .any(|finding| finding.code == "release-rehearsal-unverified")
    );
    let verified = report(
        &request(CheckRequestMode::Release, Some(CheckRequestStage::Verified)),
        &transition(),
    );
    finding(&verified, "release-reviewed-predecessor-unverified");
    finding(&verified, "release-rehearsal-unverified");
    finding(&verified, "release-retirement-proof-later");
}

#[test]
fn retired_distinguishes_candidate_eligibility_and_later_attestation() {
    let mut selected = transition();
    complete_declarations(&mut selected);
    let result = report(&request(CheckRequestMode::Retired, None), &selected);
    assert!(matches!(result.verdict, CheckReportVerdict::Pending));
    assert_eq!(result.stage, None);
    finding(&result, "retired-scaffold-free-candidate-unverified");
    finding(
        &result,
        "retired-permanent-reference-product-proof-unverified",
    );
    finding(&result, "retired-attestation-after-candidate");
}

#[test]
fn unrelated_success_cannot_discharge_native_rehearsal_or_final_subjects() {
    let mut selected = transition();
    complete_declarations(&mut selected);
    for (mode, stage, required) in [
        (
            CheckRequestMode::Bindings,
            Some(CheckRequestStage::Roles),
            "bindings-roles-owner-unavailable",
        ),
        (
            CheckRequestMode::Release,
            Some(CheckRequestStage::Verified),
            "release-rehearsal-unverified",
        ),
        (
            CheckRequestMode::Retired,
            None,
            "retired-permanent-reference-product-proof-unverified",
        ),
    ] {
        let request = request(mode, stage);
        let mut report = empty_report(report_mode(&request.mode), report_stage(&request.stage));
        report.verdict = CheckReportVerdict::Passed;
        report.checked_claims = vec!["unrelated-subject:successful-observation".into()];
        assert!(readiness::check(&request, &selected, &mut report).is_ok());
        assert!(matches!(report.verdict, CheckReportVerdict::Pending));
        assert_eq!(
            report.checked_claims,
            ["unrelated-subject:successful-observation"]
        );
        finding(&report, required);
    }
}

#[test]
fn owning_validator_refusals_precede_readiness_diagnostics() {
    let selected_request = request(CheckRequestMode::Release, Some(CheckRequestStage::Reviewed));
    let mut selected = transition();
    selected.dp2.capture_id = Some("credential-like-input-value".into());
    let mut report = empty_report(
        report_mode(&selected_request.mode),
        report_stage(&selected_request.stage),
    );
    let error = match readiness::check(&selected_request, &selected, &mut report) {
        Ok(()) => panic!("contradictory pending capture was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "transition-meaning");
    assert!(!error.reason.contains("credential-like-input-value"));
    assert!(report.findings.is_empty());
    let invalid = request(CheckRequestMode::Bindings, None);
    assert!(readiness::check(&invalid, &transition(), &mut report).is_err());
    assert!(report.findings.is_empty());
    let mut escaping = transition();
    escaping.documentation_map = Some(SourceDigestRef {
        source: "../unselected-map.json".into(),
        sha256: "c".repeat(64),
    });
    assert!(readiness::check(&selected_request, &escaping, &mut report).is_err());
    assert!(report.findings.is_empty());
}

#[test]
fn diagnostics_are_stable_across_retrieval_roots_without_path_or_time_output() {
    let mut selected = transition();
    complete_declarations(&mut selected);
    let mut first = request(CheckRequestMode::Release, Some(CheckRequestStage::Verified));
    first.transition = "/checkout/one/release/transition.json".into();
    first.repository_roots = Some(BTreeMap::from([(
        "manual".into(),
        "/checkout/one/docs".into(),
    )]));
    let mut second = first.clone();
    second.transition = "/checkout/two/release/transition.json".into();
    second.repository_roots = Some(BTreeMap::from([(
        "manual".into(),
        "/checkout/two/docs".into(),
    )]));
    let left = serde_json::to_string(&report(&first, &selected)).unwrap();
    let right = serde_json::to_string(&report(&second, &selected)).unwrap();
    assert_eq!(left, right);
    assert!(!left.contains("/checkout/"));
    assert!(!left.contains("timestamp"));
}

#[test]
fn later_unverified_declarations_do_not_invalidate_earlier_actual_capture_proof() {
    let mut fixture = tests::Fixture::new();
    complete_declarations(&mut fixture.transition);
    let path = fixture.release.join("transition.json");
    fs::write(&path, serde_json::to_vec(&fixture.transition).unwrap()).unwrap();
    let result = run(&CheckRequest {
        transition: path.to_str().unwrap().into(),
        ..request(CheckRequestMode::Capture, None)
    });
    assert!(matches!(result.verdict, CheckReportVerdict::Passed));
    assert!(
        result
            .checked_claims
            .iter()
            .any(|claim| claim == "dp1:repository-tree")
    );
    assert!(
        result
            .findings
            .iter()
            .all(|finding| matches!(finding.state, FindingState::Pending))
    );
    finding(&result, "documentation-proof-later");
    finding(&result, "final-product-proof-later");
    assert!(
        !result
            .findings
            .iter()
            .any(|finding| finding.code.starts_with("bindings-roles"))
    );
}
