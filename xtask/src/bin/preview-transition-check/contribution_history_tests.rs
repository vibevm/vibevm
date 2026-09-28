//! Cumulative immutable records and explicit current-proof selection.

use super::tests::Fixture;
use super::*;
use vibe_wire::generated::preview::e1::release_impact::{
    EvidenceRefVerdict, ImplementationSubject, ImplementationSubjectKind, ReleaseImpact,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CONTRIBUTION-HISTORY-SELECTION");

fn cumulative() -> (Fixture, ReleaseImpact, String, String) {
    let mut fixture = Fixture::new();
    let historical = fixture.commit_product("old.rs", b"historical source\n");
    let frontier = fixture.snapshot("frontier");
    let current = fixture.commit_product("cli.rs", b"enum Cli { Install, Check }\n");
    fixture.after(fixture.snapshot("after"));
    fixture.transition.captures = Some(vec![frontier]);
    let mut record = fixture.impact(
        "change-cumulative",
        &current,
        "before",
        "after",
        &["cli.rs", "old.rs"],
    );
    record.implementation_subjects.push(ImplementationSubject {
        kind: ImplementationSubjectKind::Commit,
        identity: historical.clone(),
    });
    record.evidence[0].claim_id = "current-proof".into();
    for (claim, verdict) in [
        ("old-failed", EvidenceRefVerdict::Failed),
        ("old-unavailable", EvidenceRefVerdict::Unavailable),
        ("native-future", EvidenceRefVerdict::Unavailable),
    ] {
        let mut history = record.evidence[0].clone();
        history.claim_id = claim.into();
        history.verdict = verdict;
        history.subject_id = historical.clone();
        history.source = format!("proof/{claim}.json");
        history.sha256 = None;
        record.evidence.push(history);
    }
    select(&mut fixture, &record, Some(vec!["current-proof".into()]));
    (fixture, record, historical, current)
}

fn select(fixture: &mut Fixture, record: &ReleaseImpact, claims: Option<Vec<String>>) {
    fixture.select(std::slice::from_ref(record));
    let scope = fixture.transition.contribution_scope.as_mut().unwrap();
    scope.before_capture_id = "frontier".into();
    scope.record_digests[0].required_claim_ids = claims;
}

fn assert_failed(report: &CheckReport, code: &str) {
    assert!(
        matches!(report.verdict, CheckReportVerdict::Failed),
        "{report:?}"
    );
    assert!(
        report.findings.iter().any(|finding| finding.code == code),
        "{report:?}"
    );
}

#[test]
fn cumulative_history_is_retained_without_becoming_current_coverage_or_proof() {
    let (fixture, _, historical, current) = cumulative();
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    assert!(
        report
            .checked_claims
            .contains(&format!("historical-commit-paths:{historical}"))
    );
    assert!(
        !report
            .checked_claims
            .contains(&format!("git-commit-paths:{historical}"))
    );
    assert!(
        report
            .checked_claims
            .contains(&format!("git-commit-paths:{current}"))
    );
    assert!(
        report
            .checked_claims
            .iter()
            .any(|claim| claim == "evidence-bytes:change-cumulative:current-proof")
    );
    assert!(
        !report
            .checked_claims
            .iter()
            .any(|claim| claim.contains("old-failed")
                || claim.contains("old-unavailable")
                || claim.contains("native-future"))
    );
    assert_eq!(report.findings.len(), 3);
}

#[test]
fn required_claim_selection_cannot_be_missing_empty_ambiguous_or_obsolete() {
    for defect in [
        "selection-absent",
        "selection-empty",
        "claim-absent",
        "ambiguous",
        "historical-subject",
    ] {
        let (mut fixture, mut record, historical, _) = cumulative();
        let claims = match defect {
            "selection-absent" => None,
            "selection-empty" => Some(vec![]),
            "claim-absent" => Some(vec!["omitted-proof".into()]),
            _ => Some(vec!["current-proof".into()]),
        };
        if defect == "ambiguous" {
            let mut other = record.evidence[0].clone();
            other.source = "proof/alternate.json".into();
            record.evidence.push(other);
        }
        if defect == "historical-subject" {
            record.evidence[0].subject_id = historical;
        }
        select(&mut fixture, &record, claims);
        let report = fixture.contribution_report();
        match defect {
            "selection-absent" => {
                assert!(matches!(report.verdict, CheckReportVerdict::Pending));
                assert!(
                    report
                        .findings
                        .iter()
                        .any(|finding| finding.code == "required-claims-missing")
                );
            }
            "selection-empty" => assert_failed(&report, "transition-meaning"),
            "claim-absent" => assert_failed(&report, "evidence-claim-missing"),
            "ambiguous" => assert_failed(&report, "evidence-claim-ambiguous"),
            _ => assert_failed(&report, "evidence-subject"),
        }
    }
}

#[test]
fn required_failed_or_unavailable_history_cannot_hide_behind_current_pass() {
    for (claim, failed) in [("old-failed", true), ("old-unavailable", false)] {
        let (mut fixture, record, _, _) = cumulative();
        select(
            &mut fixture,
            &record,
            Some(vec!["current-proof".into(), claim.into()]),
        );
        let report = fixture.contribution_report();
        if failed {
            assert_failed(&report, "evidence-failed");
        } else {
            assert!(
                matches!(report.verdict, CheckReportVerdict::Pending),
                "{report:?}"
            );
            assert!(
                report
                    .findings
                    .iter()
                    .any(|finding| finding.code == "evidence-unavailable"
                        && matches!(finding.state, FindingState::Unavailable))
            );
        }
    }
}

#[test]
fn historical_only_record_paths_are_valid_beside_a_current_record() {
    let (mut fixture, _, historical, current) = cumulative();
    let mut old = fixture.impact(
        "change-history",
        &historical,
        "before",
        "frontier",
        &["old.rs"],
    );
    old.evidence[0].claim_id = "scope-review".into();
    old.evidence[0].subject_id = "frontier".into();
    let now = fixture.impact("change-current", &current, "frontier", "after", &["cli.rs"]);
    fixture.select(&[old, now]);
    let scope = fixture.transition.contribution_scope.as_mut().unwrap();
    scope.before_capture_id = "frontier".into();
    scope
        .record_digests
        .iter_mut()
        .find(|binding| binding.change_id == "change-history")
        .unwrap()
        .required_claim_ids = Some(vec!["scope-review".into()]);
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
}

#[test]
fn future_or_out_of_record_subjects_and_current_omissions_still_fail() {
    for defect in ["future", "old-bound", "current-omitted"] {
        let (mut fixture, mut record, historical, current) = cumulative();
        let expected = match defect {
            "future" => {
                let future = fixture.commit_product("future.rs", b"future source\n");
                let reference = fixture.snapshot("future");
                fixture
                    .transition
                    .captures
                    .as_mut()
                    .unwrap()
                    .push(reference);
                record.after_capture_id = "future".into();
                record.implementation_subjects.push(ImplementationSubject {
                    kind: ImplementationSubjectKind::Commit,
                    identity: future,
                });
                "commit-outside-scope"
            }
            "old-bound" => {
                record.before_capture_id = "frontier".into();
                "commit-outside-record"
            }
            _ => {
                record
                    .implementation_subjects
                    .retain(|subject| subject.identity != current);
                record.evidence[0].subject_id = "frontier".into();
                "commit-uncovered"
            }
        };
        assert!(
            record
                .implementation_subjects
                .iter()
                .any(|subject| subject.identity == historical)
        );
        select(&mut fixture, &record, Some(vec!["current-proof".into()]));
        assert_failed(&fixture.contribution_report(), expected);
    }
}
