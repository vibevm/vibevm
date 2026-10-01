use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_payload::{PayloadLimits, capture_source_identity};
use crate::package_git_plan::{
    PackageCoordinate, PackageGitPlan, RawRef, RefSnapshot, RepositoryIdentity,
    RepositoryObservation, RepositoryTarget, StagedCommit, StagedObjects, TagRef,
};
use vibe_lifecycle::{JournalAdmission, SubmissionReservations};

fn git_binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .find_map(|dir| dir.join(name).canonicalize().ok().filter(|p| p.is_file()))
}
fn run_git(binary: &Path, cwd: &Path, args: &[&str]) -> String {
    let out = Command::new(binary)
        .current_dir(cwd)
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .env("HOME", cwd)
        .env("USERPROFILE", cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "local Git fixture {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn payload_limits() -> PayloadLimits {
    PayloadLimits {
        max_files: 8,
        max_directories: 8,
        max_children_per_directory: 8,
        max_depth: 4,
        max_path_bytes: 256,
        max_file_bytes: 4096,
        max_total_bytes: 8192,
        max_manifest_bytes: 8192,
    }
}
fn census_limits() -> CensusLimits {
    CensusLimits {
        max_index_bytes: 8192,
        max_probe_bytes: 4096,
        max_gitlinks: 4,
        max_path_bytes: 256,
    }
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "provenance-fixture".into(),
        target: format!("package-git-direct:sha256:{}", "9".repeat(64)),
        principal: "owner".into(),
    }
}
fn plan(source: &Project, tree_oid: String, provenance_digest: String) -> PackageGitPlan {
    let remote = format!("sha256:{}", "9".repeat(64));
    PackageGitPlan {
        package: PackageCoordinate {
            kind: "flow".into(),
            group: "org.vibevm".into(),
            name: "wal".into(),
            version: "1.1.0".into(),
            mutable: true,
        },
        version_tag_ref: "refs/tags/v1.1.0".into(),
        target: RepositoryTarget::DirectGitExisting {
            remote_identity_sha256: remote.clone(),
        },
        payload_sha256: capture_source_identity(source, payload_limits())
            .unwrap()
            .tree_sha256,
        flattened_submodules_sha256: provenance_digest,
        oid_hex_len: 40,
        before_repo: RepositoryObservation::Present(RepositoryIdentity::Direct {
            remote_identity_sha256: remote,
        }),
        before_refs: RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
        staged: StagedObjects {
            git_tree_oid: tree_oid,
            commit: StagedCommit::New {
                oid: "d".repeat(40),
                parent_oid: None,
            },
            annotated_tag_oid: Some("e".repeat(40)),
            tag_target_commit_oid: "d".repeat(40),
        },
    }
}
fn source() -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let source = Project::open(root.path()).unwrap();
    (root, source)
}
fn expected_tree(binary: &Path, source: &Project) -> String {
    let tmp = tempfile::tempdir().unwrap();
    fs::copy(
        source.root_path().join("vibe.toml"),
        tmp.path().join("vibe.toml"),
    )
    .unwrap();
    run_git(binary, tmp.path(), &["init", "-q"]);
    run_git(binary, tmp.path(), &["add", "vibe.toml"]);
    run_git(binary, tmp.path(), &["write-tree"])
}
fn stage_world(
    binary: &Path,
    source: &Project,
    digest: String,
) -> (
    tempfile::TempDir,
    LifecycleLease,
    String,
    SubmissionScope,
    String,
) {
    let tree_oid = expected_tree(binary, source);
    let selected_plan = plan(source, tree_oid, digest);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let token = namespace.id().to_owned();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "proof",
        source,
        &selected_plan,
        payload_limits(),
    )
    .unwrap() else {
        panic!("payload not staged")
    };
    assert!(matches!(
        package_git_tree::stage_tree_pending(
            &lease,
            &token,
            &selected,
            "proof",
            Some(source),
            binary
        )
        .unwrap(),
        TreeInspection::TreeCompleteButNotReady { .. }
    ));
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "proof")
            .unwrap()
            .unwrap()
            .journal_admission,
        JournalAdmission::Pending
    );
    (root, lease, token, selected, run_id)
}

#[test]
fn no_git_repository_retains_proof_and_cold_distinguishes_historical_source() {
    let Some(binary) = git_binary() else {
        eprintln!("Git absent; local fixture skipped");
        return;
    };
    let (_source_root, source) = source();
    let proof_digest = digest(&NoGitlinksEvidence::NoGitRepository).unwrap();
    let (_root, lease, token, selected, run_id) =
        stage_world(&binary, &source, proof_digest.clone());
    let expected = ProvenanceInspection::NoGitlinksRetainedButNotReady {
        run_id: run_id.clone(),
        flattened_submodules_sha256: proof_digest.clone(),
        source_reobserved: true,
    };
    assert_eq!(
        retain_no_gitlinks_pending(
            &lease,
            &token,
            &selected,
            "proof",
            &source,
            &binary,
            census_limits()
        ),
        expected
    );
    let receipt = journal::read_bytes(&lease, &run_id, "provenance")
        .unwrap()
        .unwrap();
    assert_eq!(
        retain_no_gitlinks_pending(
            &lease,
            &token,
            &selected,
            "proof",
            &source,
            &binary,
            census_limits()
        ),
        expected
    );
    assert_eq!(
        journal::read_bytes(&lease, &run_id, "provenance")
            .unwrap()
            .unwrap(),
        receipt
    );
    drop(lease);
    let reopened = LifecycleLease::acquire(_root.path()).unwrap();
    assert_eq!(
        inspect_no_gitlinks_cold(
            &reopened,
            &token,
            &selected,
            "proof",
            Some(&source),
            &binary,
            census_limits()
        ),
        expected
    );
    assert_eq!(
        inspect_no_gitlinks_cold(
            &reopened,
            &token,
            &selected,
            "proof",
            None,
            &binary,
            census_limits()
        ),
        ProvenanceInspection::NoGitlinksRetainedButNotReady {
            run_id: run_id.clone(),
            flattened_submodules_sha256: proof_digest,
            source_reobserved: false
        }
    );
    fs::remove_file(_root.path().join(format!(
        ".vibe/package-git-operations/{run_id}.package-git-provenance-pending.json"
    )))
    .unwrap();
    assert!(matches!(
        inspect_no_gitlinks_cold(
            &reopened,
            &token,
            &selected,
            "proof",
            None,
            &binary,
            census_limits()
        ),
        ProvenanceInspection::Uncertain { .. }
    ));
}

#[test]
fn no_gitlinks_index_drift_or_tampered_receipt_refuses_current_proof() {
    let Some(binary) = git_binary() else {
        eprintln!("Git absent; local fixture skipped");
        return;
    };
    let (source_root, source) = source();
    run_git(&binary, source.root_path(), &["init", "-q"]);
    run_git(&binary, source.root_path(), &["add", "vibe.toml"]);
    let NoGitlinksEvidence::NoGitlinks { .. } = evidence(
        package_git_gitlink::census(&source, &binary, census_limits()),
        "fixture",
    )
    .unwrap() else {
        panic!()
    };
    let evidence = evidence(
        package_git_gitlink::census(&source, &binary, census_limits()),
        "fixture",
    )
    .unwrap();
    let proof_digest = digest(&evidence).unwrap();
    let (root, lease, token, selected, run_id) = stage_world(&binary, &source, proof_digest);
    assert!(matches!(
        retain_no_gitlinks_pending(
            &lease,
            &token,
            &selected,
            "proof",
            &source,
            &binary,
            census_limits()
        ),
        ProvenanceInspection::NoGitlinksRetainedButNotReady { .. }
    ));
    let receipt_path = root.path().join(format!(
        ".vibe/package-git-operations/{run_id}.package-git-provenance.json"
    ));
    let original = fs::read(&receipt_path).unwrap();
    fs::write(&receipt_path, b"not-json").unwrap();
    assert!(matches!(
        inspect_no_gitlinks_cold(
            &lease,
            &token,
            &selected,
            "proof",
            Some(&source),
            &binary,
            census_limits()
        ),
        ProvenanceInspection::Uncertain { .. }
    ));
    fs::write(&receipt_path, original).unwrap();
    fs::write(source_root.path().join("foreign.txt"), b"new index row").unwrap();
    run_git(&binary, source.root_path(), &["add", "foreign.txt"]);
    assert!(matches!(
        inspect_no_gitlinks_cold(
            &lease,
            &token,
            &selected,
            "proof",
            Some(&source),
            &binary,
            census_limits()
        ),
        ProvenanceInspection::Uncertain { .. }
    ));
    assert!(matches!(
        inspect_no_gitlinks_cold(
            &lease,
            &token,
            &selected,
            "proof",
            None,
            &binary,
            census_limits()
        ),
        ProvenanceInspection::NoGitlinksRetainedButNotReady {
            source_reobserved: false,
            ..
        }
    ));
}

#[test]
fn populated_and_uninitialized_gitlinks_never_retain_no_links_receipt() {
    let candidate = GitIndexCensus::PopulatedCandidate {
        repository_root_token: "root".into(),
        package_prefix: "".into(),
        index_sha256: format!("sha256:{}", "a".repeat(64)),
        links: Vec::new(),
    };
    assert_eq!(
        evidence(candidate, "run"),
        Err(ProvenanceInspection::NeedsCommitTreeProof {
            run_id: "run".into()
        })
    );
    let uninit = GitIndexCensus::Uninitialized {
        path: "vendor/dep".into(),
        indexed_oid: "a".repeat(40),
    };
    assert!(matches!(
        evidence(uninit, "run"),
        Err(ProvenanceInspection::Unavailable { .. })
    ));
}

#[test]
fn index_change_at_receipt_boundary_is_uncertain_and_never_recaptured() {
    let Some(binary) = git_binary() else {
        eprintln!("Git absent; local fixture skipped");
        return;
    };
    let (_source_root, source) = source();
    run_git(&binary, source.root_path(), &["init", "-q"]);
    run_git(&binary, source.root_path(), &["add", "vibe.toml"]);
    let selected_evidence = evidence(
        package_git_gitlink::census(&source, &binary, census_limits()),
        "fixture",
    )
    .unwrap();
    let (root, lease, token, selected, run_id) =
        stage_world(&binary, &source, digest(&selected_evidence).unwrap());
    let first = retain_inner(
        &lease,
        &token,
        &selected,
        "proof",
        &source,
        &binary,
        census_limits(),
        || {
            let blob = run_git(&binary, source.root_path(), &["hash-object", "vibe.toml"]);
            let row = format!("100644,{blob},phantom.txt");
            run_git(
                &binary,
                source.root_path(),
                &["update-index", "--add", "--cacheinfo", &row],
            );
        },
    );
    assert!(matches!(first, ProvenanceInspection::Uncertain { .. }));
    let pending = journal::read_bytes(&lease, &run_id, "provenance-pending")
        .unwrap()
        .expect("Pending publication happened before the second index observation");
    assert!(
        journal::read_bytes(&lease, &run_id, "provenance")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        inspect_no_gitlinks_cold(
            &lease,
            &token,
            &selected,
            "proof",
            None,
            &binary,
            census_limits()
        ),
        ProvenanceInspection::Uncertain { .. }
    ));
    assert!(matches!(
        retain_no_gitlinks_pending(
            &lease,
            &token,
            &selected,
            "proof",
            &source,
            &binary,
            census_limits()
        ),
        ProvenanceInspection::Uncertain { .. }
    ));
    assert_eq!(
        journal::read_bytes(&lease, &run_id, "provenance-pending").unwrap(),
        Some(pending)
    );
    assert!(
        journal::read_bytes(&lease, &run_id, "provenance")
            .unwrap()
            .is_none()
    );
    assert!(
        root.path()
            .join(format!(
                ".vibe/package-git-operations/{run_id}.package-git-provenance-pending.json"
            ))
            .is_file()
    );
}

#[test]
fn foreign_no_gitlinks_plan_digest_does_not_create_a_receipt() {
    let Some(binary) = git_binary() else {
        eprintln!("Git absent; local fixture skipped");
        return;
    };
    let (_source_root, source) = source();
    let (_root, lease, token, selected, run_id) =
        stage_world(&binary, &source, format!("sha256:{}", "a".repeat(64)));
    assert!(matches!(
        retain_no_gitlinks_pending(
            &lease,
            &token,
            &selected,
            "proof",
            &source,
            &binary,
            census_limits()
        ),
        ProvenanceInspection::Uncertain { .. }
    ));
    assert!(
        journal::read_bytes(&lease, &run_id, "provenance")
            .unwrap()
            .is_none()
    );
}
