use std::fs;
use std::path::PathBuf;

use vibe_publish::publication_subject::{
    PublicationSubjectExpectations, require_apply_ready_default_smoke, validate_publication_subject,
};
use vibe_wire::generated::distribution_smoke_report::{
    DistributionSmokeReport, DistributionSmokeTargetStatus,
};
use vibe_wire::generated::publication_subject::{PublicationSmokeProbeStatus, PublicationSubject};

use super::super::report_store::StoredSmokeReport;
use super::super::{NativeProbeRecord, StagedBundle, run_native_probe};
use super::{prepare_report, summary_from_stored_report};

fn subject() -> PublicationSubject {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../formats/corpora/publication_subject/e1/valid/complete-five-targets.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn records() -> Vec<NativeProbeRecord> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../formats/corpora/distribution_smoke_report/e1/valid/five-unavailable.json");
    let report: DistributionSmokeReport = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    report
        .targets
        .into_iter()
        .map(|row| NativeProbeRecord {
            target: row.target,
            host: row.host,
            status: match row.status {
                DistributionSmokeTargetStatus::Passed => PublicationSmokeProbeStatus::Passed,
                DistributionSmokeTargetStatus::Failed => PublicationSmokeProbeStatus::Failed,
                DistributionSmokeTargetStatus::Skipped => PublicationSmokeProbeStatus::Skipped,
                DistributionSmokeTargetStatus::Unavailable => {
                    PublicationSmokeProbeStatus::Unavailable
                }
            },
            bundle_digest: row.bundle_sha256,
            tested_sha256: row.tested_sha256,
            runner_ref: row.runner_ref,
            executable_sha256: row.executable_sha256,
            reason: row.reason,
            stdout: row.stdout,
            stderr: row.stderr,
            exit_code: row.exit_code,
            elapsed_ms: row.elapsed_ms.parse().unwrap(),
            timed_out: row.timed_out,
            output_truncated: row.output_truncated,
        })
        .collect()
}

fn stored(digest: &str) -> StoredSmokeReport {
    StoredSmokeReport {
        digest: digest.to_string(),
        relative_path: format!("smoke/e1/{}.json", digest.strip_prefix("sha256:").unwrap()),
    }
}

#[test]
fn five_unavailable_claims_join_canonically_and_remain_not_ready() {
    let mut reversed = records();
    reversed.reverse();
    let subject = subject();
    let prepared = prepare_report(&subject, reversed).unwrap();
    let report: DistributionSmokeReport = serde_json::from_slice(&prepared.bytes).unwrap();
    assert_eq!(report.targets.len(), 5);
    assert_eq!(report.targets[0].target, "x86_64-pc-windows-msvc");
    assert_eq!(report.targets[4].target, "aarch64-apple-darwin");
    let summary =
        summary_from_stored_report(&subject, &prepared, &stored(&prepared.digest)).unwrap();
    assert!(
        summary
            .probes
            .iter()
            .all(|row| row.status == PublicationSmokeProbeStatus::Unavailable)
    );
    assert!(
        summary
            .probes
            .iter()
            .all(|row| row.tested_sha256.is_none() && row.runner_ref.is_none())
    );
    assert!(summary.probes.iter().all(|row| {
        row.evidence_ref
            .as_ref()
            .unwrap()
            .contains(&prepared.digest)
    }));
    let mut joined = subject;
    joined.smoke = summary;
    let expected = PublicationSubjectExpectations {
        version: "1.1.0",
        repository: "vibevm/vibevm",
        smoke_policy_id: "distribution-smoke/e1",
        smoke_policy_epoch: 1,
    };
    validate_publication_subject(&joined, &expected).unwrap();
    assert!(require_apply_ready_default_smoke(&joined, &expected).is_err());
}

#[test]
fn missing_duplicate_wrong_target_or_bundle_refuses() {
    let subject = subject();
    let mut missing = records();
    missing.pop();
    assert!(prepare_report(&subject, missing).is_err());
    let mut duplicate = records();
    duplicate[4].target = duplicate[0].target.clone();
    assert!(prepare_report(&subject, duplicate).is_err());
    let mut wrong = records();
    wrong[2].target = "foreign-target".into();
    assert!(prepare_report(&subject, wrong).is_err());
    let mut wrong_bundle = records();
    wrong_bundle[2].bundle_digest = format!("sha256:{}", "f".repeat(64));
    assert!(prepare_report(&subject, wrong_bundle).is_err());
}

#[test]
fn invalid_measured_output_or_duration_refuses() {
    let subject = subject();
    let mut wrong_output = records();
    wrong_output[2].stdout = "vibe 1.1.0\n".into();
    assert!(prepare_report(&subject, wrong_output).is_err());
    let mut too_long = records();
    too_long[2].elapsed_ms = 60_001;
    assert!(prepare_report(&subject, too_long).is_err());
    let mut false_timeout = records();
    false_timeout[2].timed_out = true;
    assert!(prepare_report(&subject, false_timeout).is_err());
    let mut false_pass = records();
    false_pass[2].status = PublicationSmokeProbeStatus::Passed;
    false_pass[2].tested_sha256 = Some(subject.targets[2].bundle.sha256.clone());
    false_pass[2].runner_ref = Some("system-native/linux/x86_64/gnu".into());
    false_pass[2].executable_sha256 = Some(subject.targets[2].components[0].sha256.clone());
    false_pass[2].exit_code = Some(0);
    false_pass[2].reason = "exact staged native executable returned vibe 1.1.0".into();
    false_pass[2].stdout = "wrong output\n".into();
    assert!(prepare_report(&subject, false_pass).is_err());
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn real_os_child_fixture_joins_as_passed_without_becoming_release_evidence() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    let mut subject = subject();
    let script = b"#!/bin/sh\nprintf 'vibe 1.1.0\\n'\n";
    let executable_sha = vibe_publish::sha256_digest(script);
    subject.targets[2].components[0].sha256 = executable_sha.clone();
    subject.targets[2].components[0].size_bytes = script.len().to_string();
    subject.targets[2].bootstrap.sha256 = executable_sha.clone();
    subject.targets[2].bootstrap.size_bytes = script.len().to_string();
    let hold = tempfile::tempdir().unwrap();
    let home = hold.path().to_path_buf();
    fs::create_dir(home.join("bin")).unwrap();
    let executable = home.join("bin/vibe");
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let stage = StagedBundle {
        hold,
        target: "x86_64-unknown-linux-gnu".into(),
        bundle_digest: subject.targets[2].bundle.sha256.clone(),
        home,
        executable,
        executable_size: script.len() as u64,
        executable_digest: executable_sha,
    };
    let real_child = run_native_probe(&stage, Duration::from_secs(2));
    assert_eq!(
        real_child.status,
        PublicationSmokeProbeStatus::Passed,
        "{real_child:?}"
    );
    let mut claims = records();
    claims[2] = real_child;
    let prepared = prepare_report(&subject, claims).unwrap();
    let summary =
        summary_from_stored_report(&subject, &prepared, &stored(&prepared.digest)).unwrap();
    assert_eq!(
        summary.probes[2].status,
        PublicationSmokeProbeStatus::Passed
    );
    assert_eq!(
        summary.probes[2].tested_sha256.as_deref(),
        Some(stage.bundle_digest.as_str())
    );
    assert!(summary.probes[2].runner_ref.is_some());
    assert!(
        summary.probes[2]
            .evidence_ref
            .as_ref()
            .unwrap()
            .starts_with("report=sha256:")
    );
    // This unit fixture never entered the persisted production capture route.
}
