use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use vibe_publish::release_manifest::{
    BundleDistributionManifest, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DistributionAsset,
    DistributionComponent, DistributionComponentName, DistributionSourceArchive,
    PlatformDistributionFragment, SUPPORTED_DISTRIBUTION_TARGETS,
};
use vibe_wire::generated::publication_subject::{
    PublicationSmoke, PublicationSmokeProbe, PublicationSmokeProbeStatus,
};

use super::{PreparedSubjectCapture, capture_prepared_subject};
use crate::dist::archive::{ArchiveEntry, read_zip, write_stored_zip, write_zip};
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
                evidence_ref: None,
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
