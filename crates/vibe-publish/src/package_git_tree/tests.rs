use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::package_git_operation::prepare_pending;
use crate::package_git_payload::{PayloadLimits, stage_local_payload};
use crate::package_git_plan::{
    CommitMetadata, GitPerson, PackageCoordinate, PackageGitPlan, RawRef, RefSnapshot,
    RepositoryIdentity, RepositoryObservation, RepositoryTarget, StagedCommit, StagedObjects,
    TagMetadata, TagRef,
};
use vibe_lifecycle::{JournalAdmission, SubmissionReservations, SubmissionScope};

fn git_binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter_map(|dir| {
            let path = dir.join(name);
            path.canonicalize()
                .ok()
                .filter(|resolved| resolved.is_file())
        })
        .next()
}
fn limits() -> PayloadLimits {
    PayloadLimits {
        max_files: 12,
        max_directories: 12,
        max_children_per_directory: 12,
        max_depth: 5,
        max_path_bytes: 512,
        max_file_bytes: 4096,
        max_total_bytes: 16 * 1024,
        max_manifest_bytes: 16 * 1024,
    }
}
fn source() -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    fs::create_dir(root.path().join("docs")).unwrap();
    fs::write(root.path().join("docs/Привет.md"), b"hello\n").unwrap();
    let project = Project::open(root.path()).unwrap();
    (root, project)
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "tree-host".into(),
        target: format!("package-git-direct:sha256:{}", "9".repeat(64)),
        principal: "owner".into(),
    }
}
fn plan(payload_sha256: String, tree_oid: String) -> PackageGitPlan {
    plan_with_oid_len(payload_sha256, tree_oid, 40)
}
fn plan_with_oid_len(payload_sha256: String, tree_oid: String, oid_len: usize) -> PackageGitPlan {
    let remote_identity_sha256 = format!("sha256:{}", "9".repeat(64));
    let person = GitPerson {
        name: "Oleg Chirukhin".into(),
        email: "oleg@example.test".into(),
        timestamp_seconds: 1_700_000_000,
        timezone_offset_minutes: 180,
    };
    let mut plan = PackageGitPlan {
        epoch: 2,
        package: PackageCoordinate {
            kind: "flow".into(),
            group: "org.vibevm".into(),
            name: "wal".into(),
            version: "1.1.0".into(),
            mutable: true,
        },
        version_tag_ref: "refs/tags/v1.1.0".into(),
        target: RepositoryTarget::DirectGitExisting {
            remote_identity_sha256: remote_identity_sha256.clone(),
        },
        payload_sha256,
        flattened_submodules_sha256: format!("sha256:{}", "b".repeat(64)),
        oid_hex_len: oid_len,
        before_repo: RepositoryObservation::Present(RepositoryIdentity::Direct {
            remote_identity_sha256,
        }),
        before_refs: RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
        staged: StagedObjects {
            git_tree_oid: tree_oid,
            commit: StagedCommit::New {
                oid: "d".repeat(oid_len),
                parent_oid: None,
            },
            annotated_tag_oid: Some("e".repeat(oid_len)),
            tag_target_commit_oid: "d".repeat(oid_len),
            commit_metadata: Some(CommitMetadata {
                author: person.clone(),
                committer: person.clone(),
                message: "Release wal@1.1.0\n".into(),
            }),
            tag_metadata: Some(TagMetadata {
                tagger: person,
                message: "wal@1.1.0\n".into(),
            }),
        },
    };
    plan.derive_staged_object_ids().unwrap();
    plan
}
fn expected_tree(source: &Project, binary: &Path) -> (String, String) {
    expected_tree_with_oid_len(source, binary, 40)
}
fn expected_tree_with_oid_len(source: &Project, binary: &Path, oid_len: usize) -> (String, String) {
    let payload_root = tempfile::tempdir().unwrap();
    let payload = Project::open(payload_root.path()).unwrap();
    let manifest = stage_local_payload(source, &payload, limits()).unwrap();
    let objects_root = tempfile::tempdir().unwrap();
    let objects = Project::open(objects_root.path()).unwrap();
    let git = LocalGit::new(binary, &objects, oid_len).unwrap();
    git.init().unwrap();
    let rows: Vec<_> = manifest
        .files
        .iter()
        .map(|file| {
            git.hash_file(&payload, file, limits().max_file_bytes)
                .unwrap()
        })
        .collect();
    (manifest.tree_sha256, git.make_tree(&rows).unwrap())
}

#[test]
fn selected_sha256_git_format_is_probed_before_retained_staging() {
    let Some(binary) = git_binary() else {
        eprintln!("skipping local Git tree test: Git absent");
        return;
    };
    let (_source_root, source) = source();
    if !LocalGit::probe_sha256(&binary).unwrap() {
        return; // Older host Git: the production path returns typed Unavailable.
    }
    let (payload_sha, tree_oid) = expected_tree_with_oid_len(&source, &binary, 64);
    let selected_plan = plan_with_oid_len(payload_sha, tree_oid.clone(), 64);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    prepare_pending(
        &lease,
        &namespace,
        &selected,
        "sha256",
        &source,
        &selected_plan,
        limits(),
    )
    .unwrap();
    assert!(matches!(
        stage_tree_pending(&lease, namespace.id(), &selected, "sha256", Some(&source), &binary)
            .unwrap(),
        TreeInspection::TreeCompleteButNotReady { root_tree_oid, .. } if root_tree_oid == tree_oid
    ));
}

#[test]
fn retained_nested_unicode_tree_matches_plan_and_survives_cold_attach() {
    let Some(binary) = git_binary() else {
        eprintln!("skipping local Git tree test: Git absent");
        return;
    };
    let (_source_root, source) = source();
    let (payload_sha, tree_oid) = expected_tree(&source, &binary);
    let plan = plan(payload_sha, tree_oid.clone());
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    assert!(matches!(
        prepare_pending(
            &lease,
            &namespace,
            &selected,
            "tree",
            &source,
            &plan,
            limits()
        )
        .unwrap(),
        PendingInspection::VaultCompleteButNotReady { .. }
    ));
    let result = stage_tree_pending(
        &lease,
        namespace.id(),
        &selected,
        "tree",
        Some(&source),
        &binary,
    )
    .unwrap();
    let TreeInspection::TreeCompleteButNotReady {
        run_id,
        root_tree_oid,
    } = result
    else {
        panic!("tree did not stage: {result:?}")
    };
    assert_eq!(root_tree_oid, tree_oid);
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "tree")
            .unwrap()
            .unwrap()
            .journal_admission,
        JournalAdmission::Pending
    );
    let objects = root
        .path()
        .join(format!(".vibe/package-git-operations/{run_id}.objects"));
    let config = fs::read_to_string(objects.join("config")).unwrap();
    assert!(!config.contains("remote ") && !config.contains("token"));
    let receipt = journal::read::<TreeReceipt>(&lease, &run_id, "tree")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.files.len(), 2);
    assert!(
        receipt
            .files
            .iter()
            .any(|file| file.path == "docs/Привет.md")
    );
    let before = journal::read_bytes(&lease, &run_id, "tree")
        .unwrap()
        .unwrap();
    assert_eq!(
        stage_tree_pending(
            &lease,
            namespace.id(),
            &selected,
            "tree",
            Some(&source),
            &binary
        )
        .unwrap(),
        TreeInspection::TreeCompleteButNotReady {
            run_id: run_id.clone(),
            root_tree_oid: tree_oid.clone()
        }
    );
    assert_eq!(
        journal::read_bytes(&lease, &run_id, "tree")
            .unwrap()
            .unwrap(),
        before
    );
    let token = namespace.id().to_owned();
    drop(lease);
    let reopened = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        inspect_tree_cold(&reopened, &token, &selected, "tree", Some(&source), &binary).unwrap(),
        TreeInspection::TreeCompleteButNotReady {
            run_id,
            root_tree_oid: tree_oid
        }
    );
}

#[test]
fn removed_blob_and_foreign_expected_tree_cannot_be_complete_or_restaged() {
    let Some(binary) = git_binary() else {
        eprintln!("skipping local Git tree test: Git absent");
        return;
    };
    let (_source_root, source) = source();
    let (payload_sha, tree_oid) = expected_tree(&source, &binary);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let wrong = plan(payload_sha.clone(), "f".repeat(40));
    prepare_pending(
        &lease,
        &namespace,
        &selected,
        "wrong-tree",
        &source,
        &wrong,
        limits(),
    )
    .unwrap();
    assert!(matches!(
        stage_tree_pending(
            &lease,
            namespace.id(),
            &selected,
            "wrong-tree",
            Some(&source),
            &binary
        )
        .unwrap(),
        TreeInspection::Uncertain { .. }
    ));
    assert!(
        matches!(
            stage_tree_pending(
                &lease,
                namespace.id(),
                &selected,
                "wrong-tree",
                Some(&source),
                &binary
            )
            .unwrap(),
            TreeInspection::Uncertain { .. }
        ),
        "partial object root must not be restaged"
    );

    let valid = plan(payload_sha, tree_oid);
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "changed-blob",
        &source,
        &valid,
        limits(),
    )
    .unwrap() else {
        panic!()
    };
    assert!(matches!(
        stage_tree_pending(
            &lease,
            namespace.id(),
            &selected,
            "changed-blob",
            Some(&source),
            &binary
        )
        .unwrap(),
        TreeInspection::TreeCompleteButNotReady { .. }
    ));
    let receipt = journal::read::<TreeReceipt>(&lease, &run_id, "tree")
        .unwrap()
        .unwrap();
    let oid = &receipt.files[0].blob_oid;
    let object = root.path().join(format!(
        ".vibe/package-git-operations/{run_id}.objects/objects/{}/{}",
        &oid[..2],
        &oid[2..]
    ));
    fs::remove_file(object).unwrap();
    assert!(matches!(
        inspect_tree_cold(
            &lease,
            namespace.id(),
            &selected,
            "changed-blob",
            Some(&source),
            &binary
        )
        .unwrap(),
        TreeInspection::Uncertain { .. }
    ));
}
