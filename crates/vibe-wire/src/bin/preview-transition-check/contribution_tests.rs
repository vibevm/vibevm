//! Scratch-Git cases for selected contribution proof boundaries.

use super::tests::Fixture;
use super::*;
use vibe_wire::generated::preview::e1::release_impact::{
    ImplementationSubject, ImplementationSubjectKind, ReleaseImpact,
};
use vibe_wire::generated::preview::e1::transition::{
    CaptureRef, ContributionScope, PendingCapture, PendingCaptureState, RecordDigest,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CONTRIBUTION-SCOPE-PROOF");

impl Fixture {
    pub(super) fn commit_product(&self, name: &str, bytes: &[u8]) -> String {
        fs::write(self.root.join(name), bytes).unwrap();
        self.git(&["add", name]);
        self.commit()
    }

    pub(super) fn commit(&self) -> String {
        self.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture contribution",
        ]);
        self.git(&["rev-parse", "HEAD"])
    }

    pub(super) fn snapshot(&self, id: &str) -> CaptureRef {
        let mut capture: vibe_wire::generated::preview::e1::capture::Capture =
            serde_json::from_slice(&fs::read(self.release.join("before.json")).unwrap()).unwrap();
        capture.capture_id = id.into();
        capture.repository_commit_sha1 = self.git(&["rev-parse", "HEAD"]);
        capture.repository_tree_sha1 = self.git(&["rev-parse", "HEAD^{tree}"]);
        capture.packages[0].git_tree_sha1 = self.git(&["rev-parse", "HEAD:pkg"]);
        capture.packages[0].manifest_sha256 =
            capture::sha256(&fs::read(self.root.join("pkg/vibe.toml")).unwrap());
        capture.files[0].git_blob_sha1 = self.git(&["rev-parse", "HEAD:cli.rs"]);
        capture.files[0].sha256 = capture::sha256(&fs::read(self.root.join("cli.rs")).unwrap());
        capture.surfaces[0].source_sha256 = capture.files[0].sha256.clone();
        self.write_capture(&capture, &format!("{id}.json"))
    }

    pub(super) fn after(&mut self, reference: CaptureRef) {
        self.transition.dp2 = PendingCapture {
            state: PendingCaptureState::Candidate,
            capture_id: Some(reference.capture_id),
            source: Some(reference.source),
            capture_file_sha256: Some(reference.capture_file_sha256),
            commit_sha1: Some(reference.commit_sha1),
            git_tree_sha1: Some(reference.git_tree_sha1),
        };
    }

    pub(super) fn impact(
        &self,
        id: &str,
        subject: &str,
        before: &str,
        after: &str,
        paths: &[&str],
    ) -> ReleaseImpact {
        let mut record: ReleaseImpact = serde_json::from_str(include_str!(
            "../../../../../formats/corpora/release-impact/e1/valid/internal-refactor.json"
        ))
        .unwrap();
        record.change_id = id.into();
        record.before_capture_id = before.into();
        record.after_capture_id = after.into();
        record.implementation_subjects[0].identity = subject.into();
        record.acceptance.review_subject = subject.into();
        record.acceptance.review_evidence = "proof/checked.json".into();
        record.affected_paths = Some(paths.iter().map(|path| (*path).into()).collect());
        record.evidence[0].subject_id = subject.into();
        record.evidence[0].source = "proof/checked.json".into();
        fs::create_dir_all(self.root.join("proof")).unwrap();
        fs::write(
            self.root.join("proof/checked.json"),
            b"retained checked output\n",
        )
        .unwrap();
        record.evidence[0].sha256 = Some(capture::sha256(b"retained checked output\n"));
        record
    }

    pub(super) fn select(&mut self, records: &[ReleaseImpact]) {
        let mut bindings = Vec::new();
        for record in records {
            let relative = vibe_core::layout::current_specs_root()
                .join("changes")
                .join(&record.change_id)
                .join("release-impact.json");
            let path = self.root.join(&relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let bytes = serde_json::to_vec(record).unwrap();
            fs::write(path, &bytes).unwrap();
            bindings.push(RecordDigest {
                change_id: record.change_id.clone(),
                sha256: capture::sha256(&bytes),
                required_claim_ids: Some(vec!["source-diff-reviewed".into()]),
            });
        }
        bindings.sort_by(|a, b| a.change_id.cmp(&b.change_id));
        self.transition.change_records = bindings
            .iter()
            .map(|record| record.change_id.clone())
            .collect();
        self.transition.contribution_scope = Some(ContributionScope {
            before_capture_id: self.transition.dp1.capture_id.clone(),
            after_capture_id: self.transition.dp2.capture_id.clone().unwrap(),
            record_digests: bindings,
        });
    }

    pub(super) fn contribution_report(&self) -> CheckReport {
        let path = self.release.join("transition.json");
        fs::write(&path, serde_json::to_vec(&self.transition).unwrap()).unwrap();
        run(&CheckRequest {
            schema: 1,
            transition: path.to_str().unwrap().into(),
            mode: CheckRequestMode::Contribution,
            stage: None,
            format: vibe_wire::generated::preview::e1::check_request::CheckRequestFormat::Json,
        })
    }
}

fn local() -> (Fixture, ReleaseImpact) {
    let mut fixture = Fixture::new();
    let subject = fixture.commit_product("cli.rs", b"enum Cli { Install, Check }\n");
    fixture.after(fixture.snapshot("after"));
    let record = fixture.impact("change-local", &subject, "before", "after", &["cli.rs"]);
    fixture.select(std::slice::from_ref(&record));
    (fixture, record)
}

fn failed(report: &CheckReport, code: &str) {
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
fn local_candidate_has_exact_byte_and_git_proof_without_semantic_acceptance() {
    let (fixture, _) = local();
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    assert!(
        report
            .checked_claims
            .contains(&"selected-local-git-coverage".into())
    );
    assert_eq!(report.findings.len(), 3);
    assert!(
        report
            .findings
            .iter()
            .all(|finding| matches!(finding.state, FindingState::Pending))
    );
    assert_eq!(
        serde_json::to_vec(&report).unwrap(),
        serde_json::to_vec(&fixture.contribution_report()).unwrap()
    );
}

#[test]
fn missing_scope_digest_and_evidence_have_precise_open_or_failed_results() {
    let mut fixture = Fixture::new();
    fixture.transition.proof.contributions_verified = true;
    let report = fixture.contribution_report();
    assert!(matches!(report.verdict, CheckReportVerdict::Pending));
    assert_eq!(report.findings[0].code, "contribution-scope-missing");
    for defect in [
        "binding-digest",
        "evidence-digest",
        "evidence-missing",
        "evidence-unavailable",
        "evidence-unverified",
        "evidence-failed",
        "paths-unbound",
    ] {
        let (mut fixture, mut record) = local();
        match defect {
            "binding-digest" => {}
            "evidence-digest" => record.evidence[0].sha256 = None,
            "evidence-missing" => fs::remove_file(fixture.root.join("proof/checked.json")).unwrap(),
            "evidence-unavailable" => record.evidence[0].verdict =
                vibe_wire::generated::preview::e1::release_impact::EvidenceRefVerdict::Unavailable,
            "evidence-unverified" => record.evidence[0].verdict =
                vibe_wire::generated::preview::e1::release_impact::EvidenceRefVerdict::Unverified,
            "evidence-failed" => {
                record.evidence[0].verdict =
                    vibe_wire::generated::preview::e1::release_impact::EvidenceRefVerdict::Failed
            }
            _ => record.affected_paths = None,
        }
        fixture.select(&[record]);
        if defect == "binding-digest" {
            fixture
                .transition
                .contribution_scope
                .as_mut()
                .unwrap()
                .record_digests[0]
                .sha256 = "0".repeat(64);
        }
        let report = fixture.contribution_report();
        if defect == "binding-digest" {
            failed(&report, "record-digest");
        } else if defect == "evidence-failed" {
            failed(&report, "evidence-failed");
        } else {
            assert!(
                matches!(report.verdict, CheckReportVerdict::Pending),
                "{defect}: {report:?}"
            );
        }
    }
}

#[test]
fn dangling_or_rejected_inputs_and_wrong_evidence_refuse() {
    for defect in [
        "record",
        "capture",
        "relation",
        "rejected",
        "record-id",
        "evidence-bytes",
        "evidence-subject",
        "record-window",
    ] {
        let (mut fixture, mut record) = local();
        let expected = match defect {
            "record" => "record-read",
            "capture" => "capture-unresolved",
            "relation" => "record-relation",
            "rejected" => "record-rejected",
            "record-id" => "record-identity",
            "evidence-bytes" => "evidence-digest",
            "evidence-subject" => "evidence-subject",
            _ => "commit-outside-record",
        };
        match defect {
            "capture" => record.after_capture_id = "unknown".into(),
            "relation" => record.relations.push(
                vibe_wire::generated::preview::e1::release_impact::Relation {
                    kind:
                        vibe_wire::generated::preview::e1::release_impact::RelationKind::Continues,
                    change_id: "unknown-change".into(),
                },
            ),
            "rejected" => {
                record.acceptance.state =
                    vibe_wire::generated::preview::e1::release_impact::AcceptanceState::Rejected
            }
            "evidence-bytes" => {
                fs::write(fixture.root.join("proof/checked.json"), b"changed").unwrap()
            }
            "evidence-subject" => record.evidence[0].subject_id = "unrelated".into(),
            "record-window" => record.before_capture_id = "after".into(),
            _ => {}
        }
        fixture.select(&[record]);
        let record_path = fixture
            .root
            .join(vibe_core::layout::current_specs_root())
            .join("changes/change-local/release-impact.json");
        if defect == "record" {
            fs::remove_file(&record_path).unwrap();
        }
        if defect == "record-id" {
            let mut record: ReleaseImpact =
                serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
            record.change_id = "different-id".into();
            let bytes = serde_json::to_vec(&record).unwrap();
            fs::write(&record_path, &bytes).unwrap();
            fixture
                .transition
                .contribution_scope
                .as_mut()
                .unwrap()
                .record_digests[0]
                .sha256 = capture::sha256(&bytes);
        }
        failed(&fixture.contribution_report(), expected);
    }
}

#[test]
fn actual_commits_paths_and_new_files_cannot_be_omitted_or_invented() {
    for defect in [
        "commit",
        "path",
        "new-file",
        "invented",
        "outside",
        "empty-bindings",
    ] {
        let (mut fixture, mut record) = local();
        let expected = match defect {
            "commit" | "empty-bindings" => "commit-uncovered",
            "path" | "new-file" => "paths-uncovered",
            "invented" => "paths-invented",
            _ => "commit-outside-record",
        };
        match defect {
            "commit" => {
                fixture.commit_product("new.rs", b"new\n");
                fixture.after(fixture.snapshot("after"));
            }
            "new-file" => {
                fs::write(fixture.root.join("new.rs"), b"new\n").unwrap();
                fixture.git(&["add", "new.rs"]);
                let subject = fixture.commit();
                record.implementation_subjects.push(ImplementationSubject {
                    kind: ImplementationSubjectKind::Commit,
                    identity: subject,
                });
                fixture.after(fixture.snapshot("after"));
            }
            "path" => record.affected_paths = Some(vec![]),
            "invented" => record.affected_paths = Some(vec!["cli.rs".into(), "invented.rs".into()]),
            "outside" => {
                record.implementation_subjects[0].identity =
                    fixture.transition.dp1.commit_sha1.clone()
            }
            _ => {}
        }
        if defect == "empty-bindings" {
            fixture.select(&[]);
        } else {
            fixture.select(&[record]);
        }
        failed(&fixture.contribution_report(), expected);
    }
}

#[test]
fn one_to_many_coverage_and_revert_net_noop_are_not_delivered_impact() {
    let (mut fixture, mut first) = local();
    let mut second = first.clone();
    second.change_id = "change-second".into();
    fixture.select(&[first.clone(), second]);
    assert!(matches!(
        fixture.contribution_report().verdict,
        CheckReportVerdict::Passed
    ));
    let middle = fixture.snapshot("middle");
    let revert = fixture.commit_product("cli.rs", b"enum Cli { Install }\n");
    fixture.after(fixture.snapshot("after"));
    fixture.transition.captures = Some(vec![middle]);
    first.after_capture_id = "middle".into();
    let mut second = fixture.impact("change-revert", &revert, "middle", "after", &["cli.rs"]);
    second.relations.push(
        vibe_wire::generated::preview::e1::release_impact::Relation {
            kind: vibe_wire::generated::preview::e1::release_impact::RelationKind::Reverts,
            change_id: first.change_id.clone(),
        },
    );
    fixture.select(&[first, second]);
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    assert_eq!(
        fixture.transition.dp1.git_tree_sha1,
        fixture.transition.dp2.git_tree_sha1.clone().unwrap()
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "semantic-review-later")
    );
}

#[test]
fn exact_metadata_maintenance_is_optional_but_other_docs_are_product_paths() {
    for extra in [false, true] {
        let (mut fixture, mut record) = local();
        let product = fixture.snapshot("product");
        record.after_capture_id = "product".into();
        fixture.select(&[record]);
        let record_path = vibe_core::layout::current_specs_root()
            .join("changes/change-local/release-impact.json")
            .to_string_lossy()
            .replace('\\', "/");
        fixture.git(&["add", &record_path]);
        if extra {
            fs::write(fixture.root.join("docs.xml"), b"changed prose\n").unwrap();
            fixture.git(&["add", "docs.xml"]);
        }
        fixture.commit();
        fixture.after(fixture.snapshot("after"));
        fixture.transition.captures = Some(vec![product]);
        let report = fixture.contribution_report();
        if extra {
            failed(&report, "commit-uncovered");
        } else {
            assert!(
                matches!(report.verdict, CheckReportVerdict::Passed),
                "{report:?}"
            );
        }
    }
}

#[test]
fn unsupported_subjects_stay_pending_and_never_hide_local_coverage_failure() {
    for kind in [
        ImplementationSubjectKind::Content,
        ImplementationSubjectKind::ExternalCommit,
    ] {
        let (mut fixture, mut record) = local();
        record.implementation_subjects.push(ImplementationSubject {
            kind: kind.clone(),
            identity: "external-source".into(),
        });
        fixture.select(&[record.clone()]);
        let report = fixture.contribution_report();
        assert!(
            matches!(report.verdict, CheckReportVerdict::Pending),
            "{report:?}"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| matches!(finding.state, FindingState::Unsupported))
        );
        record.implementation_subjects.remove(0);
        fixture.select(&[record]);
        failed(&fixture.contribution_report(), "commit-uncovered");
    }
}

#[cfg(unix)]
#[test]
fn passing_evidence_symlink_is_refused() {
    let (fixture, _) = local();
    let path = fixture.root.join("proof/checked.json");
    let moved = fixture.root.join("outside.json");
    fs::rename(&path, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &path).unwrap();
    failed(&fixture.contribution_report(), "evidence-read-refused");
}

#[test]
fn empty_exact_noop_and_captured_nonproduct_scope_need_no_invented_records() {
    let mut fixture = Fixture::new();
    fixture.after(fixture.transition.dp1.clone());
    fixture.select(&[]);
    assert!(matches!(
        fixture.contribution_report().verdict,
        CheckReportVerdict::Passed
    ));

    let mut fixture = Fixture::new();
    let mut before: vibe_wire::generated::preview::e1::capture::Capture =
        serde_json::from_slice(&fs::read(fixture.release.join("before.json")).unwrap()).unwrap();
    before.excluded_from_product_delta.push(
        vibe_wire::generated::preview::e1::capture::ScopeExclusion {
            path: "temporary/".into(),
            reason: "Captured non-product scratch scope.".into(),
        },
    );
    fixture.transition.dp1 = fixture.write_capture(&before, "before.json");
    fs::create_dir(fixture.root.join("temporary")).unwrap();
    fixture.commit_product("temporary/note.xml", b"scratch\n");
    fixture.after(fixture.snapshot("after"));
    fixture.select(&[]);
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
}

#[test]
fn tracked_deletion_is_part_of_the_covered_first_parent_delta() {
    let mut fixture = Fixture::new();
    fixture.commit_product("gone.rs", b"before deletion\n");
    fixture.transition.dp1 = fixture.snapshot("before");
    fixture.git(&["rm", "--quiet", "gone.rs"]);
    let subject = fixture.commit();
    fixture.after(fixture.snapshot("after"));
    let record = fixture.impact("change-delete", &subject, "before", "after", &["gone.rs"]);
    fixture.select(&[record]);
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
}

#[test]
fn honest_excluded_paths_can_be_declared_for_matching_commits() {
    let mut fixture = Fixture::new();
    let mut before: vibe_wire::generated::preview::e1::capture::Capture =
        serde_json::from_slice(&fs::read(fixture.release.join("before.json")).unwrap()).unwrap();
    before.excluded_from_product_delta.push(
        vibe_wire::generated::preview::e1::capture::ScopeExclusion {
            path: "temporary/".into(),
            reason: "Captured non-product scratch scope.".into(),
        },
    );
    fixture.transition.dp1 = fixture.write_capture(&before, "before.json");
    fs::create_dir(fixture.root.join("temporary")).unwrap();
    let excluded = fixture.commit_product("temporary/note.xml", b"scratch\n");
    let product = fixture.commit_product("cli.rs", b"enum Cli { Install, Check }\n");
    fixture.after(fixture.snapshot("after"));
    let mut record = fixture.impact(
        "change-many",
        &product,
        "before",
        "after",
        &["cli.rs", "temporary/note.xml"],
    );
    record.implementation_subjects.push(ImplementationSubject {
        kind: ImplementationSubjectKind::Commit,
        identity: excluded,
    });
    fixture.select(&[record]);
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
}
