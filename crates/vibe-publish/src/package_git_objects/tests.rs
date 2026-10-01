use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_operation::journal::NoGitlinksEvidence;
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_payload::{PayloadLimits, capture_source_identity};
use crate::package_git_plan::{
    CommitMetadata, CreationOptions, GitPerson, HostedRepository, PackageCoordinate,
    PackageGitPlan, RawRef, RefSnapshot, RepositoryIdentity, RepositoryObservation,
    RepositoryTarget, StagedCommit, StagedObjects, TagMetadata, TagRef,
};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_tree::{self, TreeInspection};
use vibe_lifecycle::{JournalAdmission, SubmissionReservations};

fn binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
        dir.join(name)
            .canonicalize()
            .ok()
            .filter(|path| path.is_file())
    })
}
fn git(binary: &Path, cwd: &Path, args: &[&str]) -> String {
    let output = Command::new(binary)
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
        output.status.success(),
        "local Git fixture {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
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
fn repository() -> HostedRepository {
    HostedRepository {
        host: "github.com".into(),
        organization: "vibespecs".into(),
        name: "org.vibevm.wal".into(),
    }
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "objects-fixture".into(),
        target: "package-git:github.com/vibespecs/org.vibevm.wal".into(),
        principal: "owner".into(),
    }
}
fn source() -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let source = Project::open(root.path()).unwrap();
    (root, source)
}
fn tree_oid(binary: &Path, source: &Project, oid_len: usize) -> String {
    let root = tempfile::tempdir().unwrap();
    fs::copy(
        source.root_path().join("vibe.toml"),
        root.path().join("vibe.toml"),
    )
    .unwrap();
    if oid_len == 64 {
        git(
            binary,
            root.path(),
            &["init", "-q", "--object-format=sha256"],
        );
    } else {
        git(binary, root.path(), &["init", "-q"]);
    }
    git(binary, root.path(), &["add", "vibe.toml"]);
    git(binary, root.path(), &["write-tree"])
}
fn plan(source: &Project, tree: String, existing_parent: bool, oid_len: usize) -> PackageGitPlan {
    let person = GitPerson {
        name: "Oleg Chirukhin".into(),
        email: "oleg@example.test".into(),
        timestamp_seconds: 1_700_000_000,
        timezone_offset_minutes: 180,
    };
    let parent = existing_parent.then(|| "a".repeat(oid_len));
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
        target: RepositoryTarget::HostedCreate {
            repository: repository(),
            creation: CreationOptions {
                description: None,
                homepage: None,
                default_branch: "main".into(),
                auto_init: false,
            },
        },
        payload_sha256: capture_source_identity(source, payload_limits())
            .unwrap()
            .tree_sha256,
        flattened_submodules_sha256: package_git_provenance::digest(
            &NoGitlinksEvidence::NoGitRepository,
        )
        .unwrap(),
        oid_hex_len: oid_len,
        before_repo: if existing_parent {
            RepositoryObservation::Present(RepositoryIdentity::Hosted {
                repository: repository(),
                id: 72,
            })
        } else {
            RepositoryObservation::Absent
        },
        before_refs: RefSnapshot {
            main: parent
                .as_ref()
                .map_or(RawRef::Absent, |oid| RawRef::Present { oid: oid.clone() }),
            version_tag: TagRef::Absent,
        },
        staged: StagedObjects {
            git_tree_oid: tree,
            commit: StagedCommit::New {
                oid: "d".repeat(oid_len),
                parent_oid: parent,
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
struct World {
    _root: tempfile::TempDir,
    _source_root: tempfile::TempDir,
    source: Project,
    lease: LifecycleLease,
    token: String,
    selected: SubmissionScope,
    run_id: String,
    plan: PackageGitPlan,
}
fn world(binary: &Path, existing_parent: bool, oid_len: usize) -> World {
    let (source_root, source) = source();
    let plan = plan(
        &source,
        tree_oid(binary, &source, oid_len),
        existing_parent,
        oid_len,
    );
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let token = namespace.id().to_owned();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "objects",
        &source,
        &plan,
        payload_limits(),
    )
    .unwrap() else {
        panic!("payload stage failed")
    };
    assert!(matches!(
        package_git_tree::stage_tree_pending(
            &lease,
            &token,
            &selected,
            "objects",
            Some(&source),
            binary
        )
        .unwrap(),
        TreeInspection::TreeCompleteButNotReady { .. }
    ));
    assert!(matches!(
        package_git_provenance::retain_no_gitlinks_pending(
            &lease,
            &token,
            &selected,
            "objects",
            &source,
            binary,
            census_limits()
        ),
        ProvenanceInspection::NoGitlinksRetainedButNotReady { .. }
    ));
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "objects")
            .unwrap()
            .unwrap()
            .journal_admission,
        JournalAdmission::Pending
    );
    World {
        _root: root,
        _source_root: source_root,
        source,
        lease,
        token,
        selected,
        run_id,
        plan,
    }
}

#[test]
fn parentless_commit_and_tag_are_retained_and_cold_verified_without_ready() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local object fixture skipped");
        return;
    };
    let world = world(&binary, false, 40);
    let expected = ObjectInspection::ObjectsCompleteButNotReady {
        run_id: world.run_id.clone(),
        source_reobserved: true,
    };
    assert_eq!(
        stage_parentless_objects_pending(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            &world.source,
            &binary,
            census_limits()
        ),
        expected
    );
    let receipt = journal::read_bytes(&world.lease, &world.run_id, "objects")
        .unwrap()
        .unwrap();
    let row: ObjectReceipt = serde_json::from_slice(&receipt).unwrap();
    assert_eq!(row.format, journal::OBJECT_FORMAT);
    assert!(matches!(
        row.provenance,
        ObjectProvenance::NoGitlinks { .. }
    ));
    assert_eq!(
        stage_parentless_objects_pending(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            &world.source,
            &binary,
            census_limits()
        ),
        expected
    );
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .unwrap(),
        receipt
    );
    assert_eq!(
        inspect_objects_cold(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            None,
            &binary,
            census_limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady {
            run_id: world.run_id.clone(),
            source_reobserved: false
        }
    );
    let World {
        _root: root,
        lease,
        token,
        selected,
        run_id,
        ..
    } = world;
    drop(lease);
    let reopened = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        inspect_objects_cold(
            &reopened,
            &token,
            &selected,
            "objects",
            None,
            &binary,
            census_limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady {
            run_id: run_id.clone(),
            source_reobserved: false
        }
    );
    fs::remove_file(root.path().join(format!(
        ".vibe/package-git-operations/{run_id}.package-git-objects.json"
    )))
    .unwrap();
    assert!(matches!(
        inspect_objects_cold(
            &reopened,
            &token,
            &selected,
            "objects",
            None,
            &binary,
            census_limits()
        ),
        ObjectInspection::Uncertain { .. }
    ));
}

#[test]
fn tampered_commit_object_or_missing_receipt_is_uncertain_without_restage() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local object fixture skipped");
        return;
    };
    let world = world(&binary, false, 40);
    assert!(matches!(
        stage_parentless_objects_pending(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            &world.source,
            &binary,
            census_limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
    let oid = &world.plan.staged.tag_target_commit_oid;
    let object = world._root.path().join(format!(
        ".vibe/package-git-operations/{}.objects/objects/{}/{}",
        world.run_id,
        &oid[..2],
        &oid[2..]
    ));
    fs::remove_file(object).unwrap();
    assert!(matches!(
        inspect_objects_cold(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            Some(&world.source),
            &binary,
            census_limits()
        ),
        ObjectInspection::Uncertain { .. }
    ));
    assert!(matches!(
        stage_parentless_objects_pending(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            &world.source,
            &binary,
            census_limits()
        ),
        ObjectInspection::Uncertain { .. }
    ));
}

#[test]
fn existing_parent_is_unavailable_before_any_commit_or_tag_write() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local object fixture skipped");
        return;
    };
    let world = world(&binary, true, 40);
    assert!(matches!(
        stage_parentless_objects_pending(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            &world.source,
            &binary,
            census_limits()
        ),
        ObjectInspection::Unavailable { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .is_none()
    );
    let objects = journal::objects(&world.lease, &world.run_id, false)
        .unwrap()
        .unwrap();
    let git = LocalGit::new(&binary, &objects, world.plan.oid_hex_len).unwrap();
    assert!(
        !git.object_present(&world.plan.staged.tag_target_commit_oid)
            .unwrap()
    );
    assert!(
        !git.object_present(world.plan.staged.annotated_tag_oid.as_deref().unwrap())
            .unwrap()
    );
}

#[test]
fn parentless_sha256_commit_and_tag_use_selected_git_object_format() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local object fixture skipped");
        return;
    };
    if !LocalGit::probe_sha256(&binary).unwrap() {
        return; // Older host Git; production tree stage reports Unavailable.
    }
    let world = world(&binary, false, 64);
    assert_eq!(
        stage_parentless_objects_pending(
            &world.lease,
            &world.token,
            &world.selected,
            "objects",
            &world.source,
            &binary,
            census_limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady {
            run_id: world.run_id.clone(),
            source_reobserved: true
        }
    );
    let receipt = journal::read::<ObjectReceipt>(&world.lease, &world.run_id, "objects")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.oid_hex_len, 64);
    assert_eq!(receipt.commit_oid.len(), 64);
    assert_eq!(receipt.tag_oid.len(), 64);
}

#[path = "tests_populated.rs"]
mod tests_populated;
