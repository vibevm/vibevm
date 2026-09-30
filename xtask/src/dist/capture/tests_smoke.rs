use super::{World, sha};
use crate::dist::run_prepare_local_into;
use crate::dist::smoke::prepare::{SmokeSelection, prepare_local_with_selection};
use vibe_publish::SUPPORTED_DISTRIBUTION_TARGETS;
use vibe_publish::publication_subject::{
    PublicationSubjectExpectations, require_apply_ready_default_smoke,
};
use vibe_safefs::Project;
use vibe_wire::generated::distribution_smoke_report::{
    DistributionSmokeReport, DistributionSmokeTargetStatus,
};

#[test]
fn scoped_skip_creates_new_immutable_report_without_a_false_launch() {
    let world = World::new();
    let output = Project::open(&world.out).unwrap();
    let default = prepare_local_with_selection(
        world.repo.path(),
        &output,
        &SmokeSelection::default(),
        || Ok(()),
    )
    .unwrap();
    let selected = SmokeSelection::new(
        vec!["x86_64-unknown-linux-gnu".into()],
        Some("operator requested deferral".into()),
    )
    .unwrap();
    let skipped =
        prepare_local_with_selection(world.repo.path(), &output, &selected, || Ok(())).unwrap();
    assert_ne!(default.report.digest, skipped.report.digest);
    assert_eq!(
        output
            .read_file_bounded(&default.report.relative_path, 256 * 1024)
            .unwrap()
            .unwrap(),
        default.report_bytes
    );
    assert_eq!(
        output
            .read_file_bounded(&skipped.report.relative_path, 256 * 1024)
            .unwrap()
            .unwrap(),
        skipped.report_bytes
    );
    let report: DistributionSmokeReport = serde_json::from_slice(&skipped.report_bytes).unwrap();
    assert_eq!(report.targets.len(), 5);
    assert_eq!(report.targets[2].target, "x86_64-unknown-linux-gnu");
    assert_eq!(
        report.targets[2].status,
        DistributionSmokeTargetStatus::Skipped
    );
    assert!(
        report.targets[2]
            .reason
            .contains("operator requested deferral")
    );
    assert!(report.targets[2].tested_sha256.is_none());
    assert!(report.targets[2].runner_ref.is_none());
    assert!(
        report
            .targets
            .iter()
            .all(|row| row.status != DistributionSmokeTargetStatus::Passed)
    );
    let expected = PublicationSubjectExpectations {
        version: "1.1.0",
        repository: "vibevm/vibevm",
        smoke_policy_id: "distribution-smoke/e1",
        smoke_policy_epoch: 1,
    };
    assert!(require_apply_ready_default_smoke(&skipped.capture.subject, &expected).is_err());
}

#[test]
fn skip_cannot_bypass_damaged_bundle_or_make_a_report() {
    let world = World::new();
    let target = SUPPORTED_DISTRIBUTION_TARGETS[2];
    let bundle = world.out.join(format!("vibevm-1.1.0-{target}.zip"));
    std::fs::write(&bundle, b"damaged ZIP").unwrap();
    let output = Project::open(&world.out).unwrap();
    let selected = SmokeSelection::new(
        vec![target.into()],
        Some("operator requested deferral".into()),
    )
    .unwrap();
    let error = prepare_local_with_selection(world.repo.path(), &output, &selected, || Ok(()))
        .err()
        .unwrap();
    assert!(
        format!("{error:#}").contains("bundle") || format!("{error:#}").contains("distribution")
    );
    assert!(!world.out.join("smoke").exists());
}

#[test]
fn local_cli_writer_emits_exact_persisted_registered_report_bytes() {
    let world = World::new();
    let mut stdout = Vec::new();
    run_prepare_local_into(
        world.repo.path(),
        "1.1.0",
        &world.out,
        vec!["x86_64-unknown-linux-gnu".into()],
        Some("explicit local skip".into()),
        &mut stdout,
    )
    .unwrap();
    let report: DistributionSmokeReport = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(report.targets.len(), 5);
    assert_eq!(
        report.targets[2].status,
        DistributionSmokeTargetStatus::Skipped
    );
    let digest = sha(&stdout);
    let path = world
        .out
        .join("smoke/e1")
        .join(format!("{}.json", digest.strip_prefix("sha256:").unwrap()));
    assert_eq!(std::fs::read(path).unwrap(), stdout);
}
