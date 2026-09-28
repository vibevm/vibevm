use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use sha2::{Digest, Sha256};
use vibe_wire::generated::preview::e1::capture::Capture;
use vibe_wire::generated::preview::e1::check_request::CheckRequestFormat;
use vibe_wire::generated::preview::e1::transition::{
    CaptureRef, PendingCapture, PendingCaptureState, PendingDocumentationState,
};

use super::*;

static NEXT_SCRATCH: AtomicUsize = AtomicUsize::new(0);

pub(super) struct Fixture {
    pub(super) root: PathBuf,
    pub(super) release: PathBuf,
    pub(super) transition: Transition,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

impl Fixture {
    pub(super) fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .env("HOME", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root.join("empty-git-config"))
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?}: {:?}",
            args,
            output.stderr
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }

    pub(super) fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "preview-proof-{}-{}",
            std::process::id(),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("home")).unwrap();
        fs::write(root.join("empty-git-config"), b"").unwrap();
        fs::create_dir(root.join("pkg")).unwrap();
        fs::write(root.join("pkg/vibe.toml"), b"name = 'example'\n").unwrap();
        fs::write(root.join("cli.rs"), b"enum Cli { Install }\n").unwrap();
        let release = root.join("release");
        fs::create_dir(&release).unwrap();
        let transition: Transition = serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-transition/e1/valid/captured.json"
        ))
        .unwrap();
        let mut fixture = Self {
            root,
            release,
            transition,
        };
        fixture.git(&["init", "--quiet"]);
        fixture.git(&["add", "pkg/vibe.toml", "cli.rs"]);
        fixture.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ]);
        let commit = fixture.git(&["rev-parse", "HEAD"]);
        let tree = fixture.git(&["rev-parse", "HEAD^{tree}"]);
        fs::write(
            fixture.release.join("raw.json"),
            b"{\"observation\":\"source\"}",
        )
        .unwrap();
        let capture: Capture = serde_json::from_value(serde_json::json!({
            "schema": 1, "capture_id": "before", "repository_commit_sha1": commit,
            "repository_tree_sha1": tree, "source_record": "raw.json",
            "source_record_sha256": sha256(b"{\"observation\":\"source\"}"), "scope": "tracked-commit",
            "packages": [{"coordinate": "example@1.0.0", "kind": "flow", "slot": "pkg",
                "git_tree_sha1": fixture.git(&["rev-parse", "HEAD:pkg"]),
                "manifest_sha256": sha256(b"name = 'example'\n")}],
            "files": [{"path": "cli.rs", "sha256": sha256(b"enum Cli { Install }\n"),
                "git_blob_sha1": fixture.git(&["rev-parse", "HEAD:cli.rs"])}],
            "surfaces": [{"kind": "cli", "source": "cli.rs", "source_sha256": sha256(b"enum Cli { Install }\n"),
                "evidence_level": "source-declared", "identifiers": ["Install"]}],
            "excluded_from_product_delta": [], "toolchains": [], "artifact_state": "unavailable",
            "documentation_input_state": "pending-owner-binding"
        })).unwrap();
        fixture.transition.dp1 = fixture.write_capture(&capture, "before.json");
        fixture
    }

    pub(super) fn write_capture(&self, capture: &Capture, source: &str) -> CaptureRef {
        let bytes = serde_json::to_vec(capture).unwrap();
        fs::write(self.release.join(source), &bytes).unwrap();
        CaptureRef {
            capture_id: capture.capture_id.clone(),
            source: source.into(),
            capture_file_sha256: sha256(&bytes),
            commit_sha1: capture.repository_commit_sha1.clone(),
            git_tree_sha1: capture.repository_tree_sha1.clone(),
        }
    }

    fn candidate(&mut self) {
        let mut capture: Capture =
            serde_json::from_slice(&fs::read(self.release.join("before.json")).unwrap()).unwrap();
        capture.capture_id = "after".into();
        let reference = self.write_capture(&capture, "after.json");
        self.transition.dp2 = PendingCapture {
            state: PendingCaptureState::Candidate,
            capture_id: Some(reference.capture_id),
            source: Some(reference.source),
            capture_file_sha256: Some(reference.capture_file_sha256),
            commit_sha1: Some(reference.commit_sha1),
            git_tree_sha1: Some(reference.git_tree_sha1),
        };
    }

    fn report(&self) -> CheckReport {
        let path = self.release.join("transition.json");
        fs::write(&path, serde_json::to_vec(&self.transition).unwrap()).unwrap();
        run(&CheckRequest {
            schema: 1,
            transition: path.to_str().unwrap().into(),
            mode: CheckRequestMode::Capture,
            stage: None,
            format: CheckRequestFormat::Json,
        })
    }
}

#[test]
fn pending_dp2_and_valid_candidate_have_exact_scoped_capture_proof() {
    let mut fixture = Fixture::new();
    let report = fixture.report();
    assert!(matches!(report.verdict, CheckReportVerdict::Passed));
    assert_eq!(report.checked_claims.len(), 6);
    assert!(
        report
            .checked_claims
            .iter()
            .all(|claim| claim.starts_with("dp1:"))
    );
    fixture.candidate();
    let report = fixture.report();
    assert!(matches!(report.verdict, CheckReportVerdict::Passed));
    assert_eq!(report.checked_claims.len(), 12);
    assert_eq!(report.subject_ids, ["after", "before"]);
    assert!(
        report
            .checked_claims
            .iter()
            .any(|claim| claim == "dp2:repository-tree")
    );
    // An exact no-op may reference the same immutable capture twice.
    fixture.transition.dp2 = PendingCapture {
        state: PendingCaptureState::Candidate,
        capture_id: Some(fixture.transition.dp1.capture_id.clone()),
        source: Some(fixture.transition.dp1.source.clone()),
        capture_file_sha256: Some(fixture.transition.dp1.capture_file_sha256.clone()),
        commit_sha1: Some(fixture.transition.dp1.commit_sha1.clone()),
        git_tree_sha1: Some(fixture.transition.dp1.git_tree_sha1.clone()),
    };
    let report = fixture.report();
    assert!(matches!(report.verdict, CheckReportVerdict::Passed));
    assert_eq!(report.subject_ids, ["before"]);
    assert_eq!(report.checked_claims.len(), 12);
}

#[test]
fn dp2_missing_changed_and_wrong_binding_are_failures() {
    for defect in ["missing", "changed", "binding", "raw"] {
        let mut fixture = Fixture::new();
        fixture.candidate();
        match defect {
            "missing" => fs::remove_file(fixture.release.join("after.json")).unwrap(),
            "changed" => fs::write(fixture.release.join("after.json"), b"{}").unwrap(),
            "binding" => fixture.transition.dp2.capture_id = Some("wrong".into()),
            _ => {
                fs::write(
                    fixture.release.join("after-raw.json"),
                    b"changed observation",
                )
                .unwrap();
                let mut capture: Capture =
                    serde_json::from_slice(&fs::read(fixture.release.join("after.json")).unwrap())
                        .unwrap();
                capture.source_record = "after-raw.json".into();
                let reference = fixture.write_capture(&capture, "after.json");
                fixture.transition.dp2.capture_file_sha256 = Some(reference.capture_file_sha256);
            }
        }
        let report = fixture.report();
        assert!(
            matches!(report.verdict, CheckReportVerdict::Failed),
            "{defect}"
        );
        assert_eq!(report.findings[0].subject, "dp2");
        let expected = match defect {
            "missing" => "capture-read",
            "changed" => "capture-digest",
            "binding" => "capture-binding",
            _ => "raw-capture-digest",
        };
        assert_eq!(report.findings[0].code, expected);
    }
}

#[test]
fn supplied_intermediate_capture_requires_exact_retained_evidence() {
    let mut fixture = Fixture::new();
    let mut capture: Capture =
        serde_json::from_slice(&fs::read(fixture.release.join("before.json")).unwrap()).unwrap();
    capture.capture_id = "intermediate".into();
    let reference = fixture.write_capture(&capture, "intermediate.json");
    fixture.transition.captures = Some(vec![reference]);
    let report = fixture.report();
    assert!(matches!(report.verdict, CheckReportVerdict::Passed));
    assert_eq!(report.subject_ids, ["before", "intermediate"]);
    assert!(
        report
            .checked_claims
            .iter()
            .any(|claim| claim == "catalogue:intermediate:repository-tree")
    );
    fs::remove_file(fixture.release.join("intermediate.json")).unwrap();
    let report = fixture.report();
    assert!(matches!(report.verdict, CheckReportVerdict::Failed));
    assert_eq!(report.findings[0].code, "capture-read");
    assert_eq!(report.findings[0].subject, "catalogue:intermediate");
}

#[test]
fn alternate_catalogue_location_is_verified_even_for_a_known_endpoint_identity() {
    let mut fixture = Fixture::new();
    let mut alternate = fixture.transition.dp1.clone();
    alternate.source = "alternate.json".into();
    fs::copy(
        fixture.release.join("before.json"),
        fixture.release.join("alternate.json"),
    )
    .unwrap();
    fixture.transition.captures = Some(vec![alternate]);
    assert!(matches!(
        fixture.report().verdict,
        CheckReportVerdict::Passed
    ));
    fs::write(fixture.release.join("alternate.json"), b"{}").unwrap();
    let report = fixture.report();
    assert!(matches!(report.verdict, CheckReportVerdict::Failed));
    assert_eq!(report.findings[0].code, "capture-digest");
    assert_eq!(report.findings[0].subject, "catalogue:before");
}

#[test]
fn asserted_final_and_documentation_labels_cannot_suppress_pending_capture_findings() {
    let mut fixture = Fixture::new();
    fixture.candidate();
    fixture.transition.documentation.state = PendingDocumentationState::Verified;
    fixture.transition.documentation.capture_id = Some("docs-exact".into());
    fixture.transition.documentation.source = Some("unverified-docs.json".into());
    fixture.transition.documentation.capture_file_sha256 = Some("a".repeat(64));
    fixture.transition.proof.documentation_verified = true;
    fixture.transition.proof.final_product_panel_passed = true;
    let report = fixture.report();
    assert!(matches!(report.verdict, CheckReportVerdict::Passed));
    assert_eq!(report.findings.len(), 2);
    assert!(
        report
            .findings
            .iter()
            .all(|finding| matches!(finding.state, FindingState::Pending))
    );
    assert_eq!(report.findings[0].code, "documentation-proof-later");
    assert_eq!(report.findings[1].code, "final-product-proof-later");
    assert_eq!(
        report.findings[0].reason,
        "Capture mode does not verify documentation proof."
    );
    assert_eq!(
        report.findings[1].reason,
        "Capture mode does not verify the final product panel."
    );
}

#[test]
fn malformed_transition_report_does_not_echo_input_value() {
    let fixture = Fixture::new();
    let path = fixture.release.join("malformed.json");
    fs::write(&path, br#"{"schema":"credential-like-marker"}"#).unwrap();
    let report = run(&CheckRequest {
        schema: 1,
        transition: path.to_str().unwrap().into(),
        mode: CheckRequestMode::Capture,
        stage: None,
        format: CheckRequestFormat::Json,
    });
    assert!(matches!(report.verdict, CheckReportVerdict::Failed));
    assert_eq!(report.findings[0].code, "transition-shape");
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("credential-like-marker")
    );
}

#[cfg(unix)]
#[test]
fn capture_and_raw_symlinks_cannot_escape_release_root() {
    use std::os::unix::fs::symlink;
    for kind in ["capture", "raw", "ancestor", "inside"] {
        let mut fixture = Fixture::new();
        let relative = if kind == "raw" {
            "raw.json"
        } else {
            "before.json"
        };
        let inside = fixture.release.join(relative);
        let outside = if kind == "inside" {
            fixture.release.join("alias.json")
        } else {
            fixture.root.join("outside.json")
        };
        fs::rename(&inside, &outside).unwrap();
        if kind == "ancestor" {
            symlink(&fixture.root, fixture.release.join("alias-dir")).unwrap();
            fixture.transition.dp1.source = "alias-dir/outside.json".into();
        } else {
            symlink(&outside, &inside).unwrap();
        }
        let report = fixture.report();
        assert!(matches!(report.verdict, CheckReportVerdict::Failed));
        assert_eq!(report.findings[0].code, "evidence-read-refused");
        assert_eq!(report.findings[0].subject, "dp1");
        assert!(!report.findings[0].reason.contains("observation\":\"source"));
    }
}
