use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use vibe_publish::publication_effect_plan::CapturedPublicationRemote;
use vibe_publish::publication_subject::{
    PublicationSubjectExpectations, require_apply_ready_default_smoke,
};
use vibe_publish::release_manifest::{
    BundleDistributionManifest, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DistributionAsset,
    DistributionComponent, DistributionComponentName, DistributionSourceArchive,
    PlatformDistributionFragment, SUPPORTED_DISTRIBUTION_TARGETS,
};
use vibe_safefs::Project;
use vibe_wire::generated::distribution_smoke_report::{
    DistributionSmokeReport, DistributionSmokeTargetStatus,
};
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedLatestRelease,
    PublicationObservedLatestReleasePresent, PublicationObservedRelease,
    PublicationObservedReleaseAbsent, PublicationObservedReleasePresent, PublicationObservedTag,
    PublicationObservedTagAbsent, PublicationObservedTagPresent, PublicationPartition,
    PublicationPartitionReleaseAction,
};
use vibe_wire::generated::publication_subject::{
    PublicationSmoke, PublicationSmokeProbe, PublicationSmokeProbeStatus,
};

use super::{
    PreparedSubjectCapture, PublicationVisibilityChoice, PublishedReplacementChoice,
    capture_prepared_subject, capture_prepared_subject_at_identity, plan_captured_subject,
};
use crate::dist::archive::{ArchiveEntry, read_zip, write_stored_zip, write_zip};
use crate::dist::smoke::prepare::prepare_local_with;

#[path = "tests_installer_bytes.rs"]
mod tests_installer_bytes;
#[path = "tests_plan_local.rs"]
mod tests_plan_local;
#[path = "tests_smoke.rs"]
mod tests_smoke;
use crate::dist::snapshot;

fn run_git(root: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    crate::dist::scrub_release_credentials(&mut command);
    let output = command.output().expect("fixture git starts");
    assert!(
        output.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn smoke() -> PublicationSmoke {
    PublicationSmoke {
        policy_id: "distribution-smoke/e1".into(),
        policy_epoch: 1,
        probes: SUPPORTED_DISTRIBUTION_TARGETS
            .into_iter()
            .map(|target| PublicationSmokeProbe {
                probe_id: "launch".into(),
                target: target.into(),
                status: PublicationSmokeProbeStatus::Skipped,
                tested_sha256: None,
                runner_ref: None,
                evidence_ref: Some(
                    "reason=not-executed-in-capture-fixture;host=fixture/none".into(),
                ),
            })
            .collect(),
    }
}

struct World {
    repo: tempfile::TempDir,
    out: PathBuf,
}

impl World {
    fn new() -> Self {
        let repo = tempfile::tempdir().unwrap();
        run_git(repo.path(), &["init", "--quiet", "--initial-branch=main"]);
        run_git(repo.path(), &["config", "user.name", "Fixture"]);
        run_git(
            repo.path(),
            &["config", "user.email", "fixture@example.invalid"],
        );
        std::fs::write(
            repo.path().join("Cargo.toml"),
            b"[workspace]\n[workspace.package]\nversion = \"1.1.0\"\n",
        )
        .unwrap();
        std::fs::write(repo.path().join("Cargo.lock"), b"committed lock bytes\n").unwrap();
        let installers = repo.path().join("distribution/install");
        std::fs::create_dir_all(&installers).unwrap();
        std::fs::write(installers.join("install.sh"), b"committed bash\n").unwrap();
        std::fs::write(installers.join("install.ps1"), b"committed powershell\n").unwrap();
        std::fs::write(installers.join("install.cmd"), b"committed cmd\n").unwrap();
        run_git(
            repo.path(),
            &["add", "Cargo.toml", "Cargo.lock", "distribution"],
        );
        run_git(repo.path(), &["commit", "--quiet", "-m", "fixture source"]);
        let identity = snapshot::committed_identity(repo.path(), true).unwrap();
        let out = repo.path().join("dist");
        std::fs::create_dir(&out).unwrap();
        for target in SUPPORTED_DISTRIBUTION_TARGETS {
            let suffix = if target == "x86_64-pc-windows-msvc" {
                ".exe"
            } else {
                ""
            };
            let vibe = format!("vibe component {target}").into_bytes();
            let index = format!("index component {target}").into_bytes();
            let source_zip = write_stored_zip(&[ArchiveEntry {
                name: "Cargo.lock".into(),
                bytes: b"source bytes".to_vec(),
                mode: 0o644,
            }])
            .unwrap();
            let manifest = BundleDistributionManifest::new(
                semver::Version::new(1, 1, 0),
                identity.commit.clone(),
                target,
                vec![
                    DistributionComponent {
                        name: DistributionComponentName::Vibe,
                        path: format!("vibe{suffix}"),
                        size: vibe.len() as u64,
                        digest: sha(&vibe),
                    },
                    DistributionComponent {
                        name: DistributionComponentName::VibeIndex,
                        path: format!("vibe-index{suffix}"),
                        size: index.len() as u64,
                        digest: sha(&index),
                    },
                ],
                DistributionSourceArchive {
                    path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.into(),
                    size: source_zip.len() as u64,
                    digest: sha(&source_zip),
                    tree_oid: identity.tree.clone(),
                },
            )
            .unwrap();
            let bundle = write_zip(&[
                ArchiveEntry {
                    name: format!("vibe{suffix}"),
                    bytes: vibe.clone(),
                    mode: 0o755,
                },
                ArchiveEntry {
                    name: format!("vibe-index{suffix}"),
                    bytes: index,
                    mode: 0o755,
                },
                ArchiveEntry {
                    name: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.into(),
                    bytes: source_zip,
                    mode: 0o644,
                },
                ArchiveEntry {
                    name: "LICENSE.md".into(),
                    bytes: b"fixture license".to_vec(),
                    mode: 0o644,
                },
                ArchiveEntry {
                    name: "README.md".into(),
                    bytes: b"fixture readme".to_vec(),
                    mode: 0o644,
                },
                ArchiveEntry {
                    name: "DISTRIBUTION.json".into(),
                    bytes: manifest.to_json_bytes().unwrap(),
                    mode: 0o644,
                },
            ])
            .unwrap();
            let bundle_name = format!("vibevm-1.1.0-{target}.zip");
            let bootstrap_name = format!("vibe-bootstrap-{target}{suffix}");
            let fragment = PlatformDistributionFragment::new(
                DistributionAsset {
                    name: bundle_name.clone(),
                    size: bundle.len() as u64,
                    digest: sha(&bundle),
                },
                DistributionAsset {
                    name: bootstrap_name.clone(),
                    size: vibe.len() as u64,
                    digest: sha(&vibe),
                },
                manifest,
            )
            .unwrap();
            std::fs::write(out.join(bundle_name), bundle).unwrap();
            std::fs::write(out.join(bootstrap_name), vibe).unwrap();
            std::fs::write(
                out.join(format!("vibevm-1.1.0-{target}.fragment.json")),
                fragment.to_json_bytes().unwrap(),
            )
            .unwrap();
        }
        Self { repo, out }
    }

    fn capture(&self) -> anyhow::Result<PreparedSubjectCapture> {
        capture_prepared_subject(self.repo.path(), &self.out, smoke())
    }
}

fn output_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

fn absent_remote() -> CapturedPublicationRemote {
    CapturedPublicationRemote {
        generation_ref: "fixture-remote-absent".into(),
        tag: PublicationObservedTag::Absent(Box::new(PublicationObservedTagAbsent {})),
        release: PublicationObservedRelease::Absent(Box::new(PublicationObservedReleaseAbsent {})),
        observed_latest_release: PublicationObservedLatestRelease::Present(Box::new(
            PublicationObservedLatestReleasePresent {
                release_id: "101".into(),
                tag: "v1.0.1".into(),
            },
        )),
    }
}

fn old_draft_remote(commit: &str, release_id: &str, generation: &str) -> CapturedPublicationRemote {
    CapturedPublicationRemote {
        generation_ref: generation.into(),
        tag: PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
            reference: "refs/tags/v1.1.0".into(),
            object_kind: "commit".into(),
            object_sha: commit.into(),
        })),
        observed_latest_release: PublicationObservedLatestRelease::Present(Box::new(
            PublicationObservedLatestReleasePresent {
                release_id: "101".into(),
                tag: "v1.0.1".into(),
            },
        )),
        release: PublicationObservedRelease::Present(Box::new(PublicationObservedReleasePresent {
            release_id: release_id.into(),
            generation_ref: generation.into(),
            tag: "v1.1.0".into(),
            draft: true,
            prerelease: false,
            target_commitish: commit.into(),
            name: Some("vibevm 1.1.0 prior draft".into()),
            body: Some("Prior mutable draft.".into()),
            inventory_complete: true,
            assets: vec![PublicationObservedAsset {
                asset_id: "7".into(),
                name: "old-bundle.zip".into(),
                size_bytes: "3".into(),
                sha256: sha(b"old"),
            }],
        })),
    }
}

#[test]
fn committed_source_and_five_local_assets_join_without_output_writes() {
    let world = World::new();
    let before = output_bytes(&world.out);
    let first = world.capture().unwrap();
    let second = world.capture().unwrap();
    assert_eq!(first.subject_digest, second.subject_digest);
    assert_eq!(first.aggregate_bytes, second.aggregate_bytes);
    assert_eq!(first.identity, second.identity);
    assert_eq!(first.subject.source.commit, first.identity.commit);
    assert_eq!(first.subject.source.tree, first.identity.tree);
    assert_eq!(
        first.subject.source.cargo_lock.sha256,
        sha(b"committed lock bytes\n")
    );
    assert_eq!(
        first.subject.installers[0].asset.sha256,
        sha(b"committed bash\n")
    );
    assert_eq!(first.subject.targets.len(), 5);
    assert_eq!(output_bytes(&world.out), before);
}

#[test]
fn absent_remote_plan_uses_exact_captured_subject_assets_without_writes() {
    let world = World::new();
    let output_before = output_bytes(&world.out);
    let source_before = std::fs::read(world.repo.path().join("Cargo.lock")).unwrap();
    let capture = world.capture().unwrap();
    let subject_digest = capture.subject_digest.clone();
    let prepared = plan_captured_subject(
        capture,
        &absent_remote(),
        PublicationVisibilityChoice::RetainDraft,
        PublishedReplacementChoice::Refuse,
    )
    .unwrap();
    assert_eq!(prepared.plan.subject_sha256, subject_digest);
    assert_eq!(prepared.plan.target.host, "github.com");
    assert_eq!(prepared.plan.target.owner, "vibevm");
    assert_eq!(prepared.plan.target.repository, "vibevm");
    assert_eq!(prepared.plan.partitions.len(), 10);
    for (index, target) in prepared.capture.subject.targets.iter().enumerate() {
        let PublicationPartition::PlatformAssets(step) = &prepared.plan.partitions[index + 2]
        else {
            panic!("platform partition {index} missing")
        };
        assert_eq!(step.target, target.target);
        for (effect, asset) in
            step.assets
                .iter()
                .zip([&target.bundle, &target.bootstrap, &target.fragment])
        {
            assert_eq!(effect.name, asset.name);
            assert_eq!(effect.desired_sha256, asset.sha256);
            assert_eq!(effect.desired_size_bytes, asset.size_bytes);
        }
    }
    let PublicationPartition::FinalAssets(final_assets) = &prepared.plan.partitions[8] else {
        panic!("final asset partition missing")
    };
    for (effect, asset) in final_assets.assets.iter().zip(
        std::iter::once(&prepared.capture.subject.aggregate).chain(
            prepared
                .capture
                .subject
                .installers
                .iter()
                .map(|row| &row.asset),
        ),
    ) {
        assert_eq!(effect.name, asset.name);
        assert_eq!(effect.desired_sha256, asset.sha256);
    }
    assert_eq!(output_bytes(&world.out), output_before);
    assert_eq!(
        std::fs::read(world.repo.path().join("Cargo.lock")).unwrap(),
        source_before
    );
}

#[test]
fn old_draft_remote_identity_and_visibility_change_pure_plan_digest() {
    let world = World::new();
    let output_before = output_bytes(&world.out);
    let capture = world.capture().unwrap();
    let commit = capture.identity.commit.clone();
    let first = plan_captured_subject(
        capture,
        &old_draft_remote(&commit, "41", "generation-a"),
        PublicationVisibilityChoice::RetainDraft,
        PublishedReplacementChoice::Refuse,
    )
    .unwrap();
    let PublicationObservedRelease::Present(observed) = &first.plan.observed_release else {
        panic!("complete old draft was not retained")
    };
    assert_eq!(observed.release_id, "41");
    let PublicationPartition::Release(release) = &first.plan.partitions[1] else {
        panic!("release partition missing")
    };
    assert_eq!(release.action, PublicationPartitionReleaseAction::Replace);
    let different_id = plan_captured_subject(
        world.capture().unwrap(),
        &old_draft_remote(&commit, "42", "generation-a"),
        PublicationVisibilityChoice::RetainDraft,
        PublishedReplacementChoice::Refuse,
    )
    .unwrap();
    let different_generation = plan_captured_subject(
        world.capture().unwrap(),
        &old_draft_remote(&commit, "41", "generation-b"),
        PublicationVisibilityChoice::RetainDraft,
        PublishedReplacementChoice::Refuse,
    )
    .unwrap();
    let different_visibility = plan_captured_subject(
        world.capture().unwrap(),
        &old_draft_remote(&commit, "41", "generation-a"),
        PublicationVisibilityChoice::Publish,
        PublishedReplacementChoice::Refuse,
    )
    .unwrap();
    assert_ne!(first.plan_digest, different_id.plan_digest);
    assert_ne!(first.plan_digest, different_generation.plan_digest);
    assert_ne!(first.plan_digest, different_visibility.plan_digest);
    for changed in [&different_id, &different_generation, &different_visibility] {
        assert_eq!(
            first.capture.identity.commit,
            changed.capture.identity.commit
        );
        assert_eq!(first.capture.identity.tree, changed.capture.identity.tree);
    }
    assert_eq!(output_bytes(&world.out), output_before);
}

#[test]
fn published_mutable_release_requires_explicit_plan_replacement_choice() {
    let world = World::new();
    let output_before = output_bytes(&world.out);
    let capture = world.capture().unwrap();
    let mut remote = old_draft_remote(&capture.identity.commit, "51", "published-generation");
    let PublicationObservedRelease::Present(release) = &mut remote.release else {
        unreachable!()
    };
    release.draft = false;
    assert!(
        plan_captured_subject(
            capture,
            &remote,
            PublicationVisibilityChoice::RetainDraft,
            PublishedReplacementChoice::Refuse,
        )
        .is_err()
    );
    let selected = plan_captured_subject(
        world.capture().unwrap(),
        &remote,
        PublicationVisibilityChoice::RetainDraft,
        PublishedReplacementChoice::PlanReplacement,
    )
    .unwrap();
    assert_eq!(selected.capture.subject.version, "1.1.0");
    let PublicationObservedRelease::Present(observed) = &selected.plan.observed_release else {
        panic!("selected published release identity missing")
    };
    assert_eq!(observed.release_id, "51");
    assert!(!observed.draft);
    let PublicationPartition::Release(step) = &selected.plan.partitions[1] else {
        panic!("replacement partition missing")
    };
    assert_eq!(step.action, PublicationPartitionReleaseAction::Replace);
    assert_eq!(output_bytes(&world.out), output_before);
}

#[test]
fn dirty_tracked_lock_refuses_instead_of_rebinding_subject() {
    let world = World::new();
    let baseline = world.capture().unwrap();
    std::fs::write(world.repo.path().join("Cargo.lock"), b"dirty lock bytes\n").unwrap();
    let error = world.capture().err().expect("dirty tracked lock refuses");
    assert!(
        error.to_string().contains("fully committed HEAD"),
        "{error:#}"
    );
    assert_eq!(
        baseline.subject.source.cargo_lock.sha256,
        sha(b"committed lock bytes\n")
    );
}

#[test]
fn changed_bundle_and_missing_fragment_refuse_without_mutating_output() {
    for change in ["bundle", "missing"] {
        let world = World::new();
        let target = SUPPORTED_DISTRIBUTION_TARGETS[0];
        let path = if change == "bundle" {
            world.out.join(format!("vibevm-1.1.0-{target}.zip"))
        } else {
            world
                .out
                .join(format!("vibevm-1.1.0-{target}.fragment.json"))
        };
        if change == "bundle" {
            std::fs::write(&path, b"changed bundle bytes").unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
        let before = output_bytes(&world.out);
        assert!(world.capture().is_err(), "{change}");
        assert_eq!(output_bytes(&world.out), before);
    }
}

#[test]
fn local_smoke_preparation_persists_five_honest_rows_and_report_bound_subject() {
    let world = World::new();
    let output = Project::open(&world.out).unwrap();
    let prepared = prepare_local_with(world.repo.path(), &output, || Ok(())).unwrap();
    let bytes = output
        .read_file_bounded(&prepared.report.relative_path, 256 * 1024)
        .unwrap()
        .unwrap();
    assert_eq!(sha(&bytes), prepared.report.digest);
    let report: DistributionSmokeReport = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report.targets.len(), 5);
    assert_eq!(
        report
            .targets
            .iter()
            .map(|row| row.target.as_str())
            .collect::<Vec<_>>(),
        SUPPORTED_DISTRIBUTION_TARGETS
    );
    // World bundles contain fixture text, not a runnable vibe binary. Even
    // Linux-native rows may not claim a successful installed launch.
    assert!(
        report
            .targets
            .iter()
            .all(|row| row.status == DistributionSmokeTargetStatus::Unavailable)
    );
    assert!(
        report
            .targets
            .iter()
            .all(|row| row.tested_sha256.is_none() && row.runner_ref.is_none())
    );
    assert!(prepared.capture.subject.smoke.probes.iter().all(|row| {
        row.evidence_ref
            .as_ref()
            .unwrap()
            .contains(&prepared.report.digest)
    }));
    let expected = PublicationSubjectExpectations {
        version: "1.1.0",
        repository: "vibevm/vibevm",
        smoke_policy_id: "distribution-smoke/e1",
        smoke_policy_epoch: 1,
    };
    assert!(require_apply_ready_default_smoke(&prepared.capture.subject, &expected).is_err());
}

#[test]
fn changed_asset_between_smoke_report_and_final_capture_refuses() {
    let world = World::new();
    let output = Project::open(&world.out).unwrap();
    let target = SUPPORTED_DISTRIBUTION_TARGETS[0];
    let bundle = world.out.join(format!("vibevm-1.1.0-{target}.zip"));
    let error = prepare_local_with(world.repo.path(), &output, || {
        std::fs::write(&bundle, b"changed after report")?;
        Ok(())
    })
    .err()
    .expect("changed bundle must refuse final capture");
    assert!(
        format!("{error:#}").contains("bundle") || format!("{error:#}").contains("distribution")
    );
}

#[test]
fn dirty_source_refuses_before_missing_output_is_opened() {
    let world = World::new();
    std::fs::write(
        world.repo.path().join("Cargo.lock"),
        b"dirty tracked lock\n",
    )
    .unwrap();
    std::fs::remove_dir_all(&world.out).unwrap();
    let error = capture_prepared_subject(world.repo.path(), &world.out, smoke())
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains("fully committed HEAD"));
}

#[test]
fn missing_or_corrupt_embedded_source_archive_refuses_even_when_outer_digest_is_updated() {
    for change in ["missing", "corrupt"] {
        let world = World::new();
        let target = SUPPORTED_DISTRIBUTION_TARGETS[0];
        let bundle_path = world.out.join(format!("vibevm-1.1.0-{target}.zip"));
        let fragment_path = world
            .out
            .join(format!("vibevm-1.1.0-{target}.fragment.json"));
        let mut entries = read_zip(&std::fs::read(&bundle_path).unwrap()).unwrap();
        if change == "missing" {
            entries.retain(|entry| entry.name != DISTRIBUTION_SOURCE_ARCHIVE_FILENAME);
        } else {
            entries
                .iter_mut()
                .find(|entry| entry.name == DISTRIBUTION_SOURCE_ARCHIVE_FILENAME)
                .unwrap()
                .bytes = b"wrong source archive".to_vec();
        }
        let rebuilt = write_zip(&entries).unwrap();
        let mut fragment =
            PlatformDistributionFragment::from_json_slice(&std::fs::read(&fragment_path).unwrap())
                .unwrap();
        fragment.asset.size = rebuilt.len() as u64;
        fragment.asset.digest = sha(&rebuilt);
        std::fs::write(&bundle_path, rebuilt).unwrap();
        std::fs::write(&fragment_path, fragment.to_json_bytes().unwrap()).unwrap();
        let before = output_bytes(&world.out);
        let error = world
            .capture()
            .err()
            .expect("embedded source damage refuses");
        assert!(
            format!("{error:#}").contains("vibevm-source.zip")
                || format!("{error:#}").contains("source archive"),
            "{change}: {error:#}"
        );
        assert_eq!(output_bytes(&world.out), before);
    }
}
