use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::*;
use crate::package_git_operation::journal::NoGitlinksEvidence;
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_parent_source;
use crate::package_git_payload::{PayloadLimits, capture_source_identity};
use crate::package_git_plan::{
    CommitMetadata, GitPerson, PackageCoordinate, RawRef, RefSnapshot, RepositoryIdentity,
    RepositoryObservation, RepositoryTarget, StagedObjects, TagMetadata,
};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_tree::{self, TreeInspection};
use vibe_lifecycle::{JournalAdmission, SubmissionReservations};

pub(crate) fn binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
        dir.join(name)
            .canonicalize()
            .ok()
            .filter(|path| path.is_file())
    })
}
fn git(binary: &Path, cwd: &Path, args: &[&str], input: &[u8]) -> String {
    let mut child = Command::new(binary)
        .current_dir(cwd)
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("LC_ALL", "C")
        .env("HOME", cwd)
        .env("USERPROFILE", cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "local fixture Git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
pub(crate) fn limits() -> ParentLimits {
    ParentLimits {
        source: SourceLimits {
            max_path_bytes: 4096,
            max_probe_bytes: 4096,
            max_refs_bytes: 8192,
        },
        census: CensusLimits {
            max_index_bytes: 8192,
            max_probe_bytes: 4096,
            max_gitlinks: 4,
            max_path_bytes: 256,
        },
        max_ref_output_bytes: 8192,
        max_reachable_output_bytes: 16 * 1024,
        max_objects: 32,
        max_object_bytes: 4096,
        max_total_bytes: 32 * 4096,
        max_parent_commit_bytes: 4096,
        max_pack_entries: 32,
        max_config_bytes: 4096,
    }
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
fn package_source(contents: &[u8]) -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), contents).unwrap();
    let project = Project::open(root.path()).unwrap();
    (root, project)
}
pub(crate) struct Origin {
    _root: tempfile::TempDir,
    pub(crate) project: Project,
    pub(crate) main: String,
    pub(crate) previous: String,
    pub(crate) tag: String,
    blob: String,
    tree: String,
}
fn seed_origin(binary: &Path, contents: &[u8]) -> Origin {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("origin.git");
    fs::create_dir(&path).unwrap();
    git(binary, &path, &["init", "--bare", "-q"], b"");
    let bare = path.to_str().unwrap();
    let blob = git(
        binary,
        &path,
        &["--git-dir", bare, "hash-object", "-w", "--stdin"],
        contents,
    );
    let tree_input = format!("100644 blob {blob}\tvibe.toml\n");
    let tree = git(
        binary,
        &path,
        &["--git-dir", bare, "mktree"],
        tree_input.as_bytes(),
    );
    let first = format!(
        "tree {tree}\nauthor Fixture <fixture@example.test> 1700000000 +0000\ncommitter Fixture <fixture@example.test> 1700000000 +0000\n\nfirst\n"
    );
    let previous = git(
        binary,
        &path,
        &[
            "--git-dir",
            bare,
            "hash-object",
            "-t",
            "commit",
            "-w",
            "--stdin",
        ],
        first.as_bytes(),
    );
    let second = format!(
        "tree {tree}\nparent {previous}\nauthor Fixture <fixture@example.test> 1700000001 +0000\ncommitter Fixture <fixture@example.test> 1700000001 +0000\n\nsecond\n"
    );
    let main = git(
        binary,
        &path,
        &[
            "--git-dir",
            bare,
            "hash-object",
            "-t",
            "commit",
            "-w",
            "--stdin",
        ],
        second.as_bytes(),
    );
    git(
        binary,
        &path,
        &["--git-dir", bare, "update-ref", "refs/heads/main", &main],
        b"",
    );
    let tag_body = format!(
        "object {main}\ntype commit\ntag v1.1.0\ntagger Fixture <fixture@example.test> 1700000001 +0000\n\nfixture tag\n"
    );
    let tag = git(
        binary,
        &path,
        &[
            "--git-dir",
            bare,
            "hash-object",
            "-t",
            "tag",
            "-w",
            "--stdin",
        ],
        tag_body.as_bytes(),
    );
    git(
        binary,
        &path,
        &["--git-dir", bare, "update-ref", "refs/tags/v1.1.0", &tag],
        b"",
    );
    Origin {
        _root: root,
        project: Project::open(&path).unwrap(),
        main,
        previous,
        tag,
        blob,
        tree,
    }
}
fn staged_tree(binary: &Path, package: &Project) -> String {
    let root = tempfile::tempdir().unwrap();
    fs::copy(
        package.root_path().join("vibe.toml"),
        root.path().join("vibe.toml"),
    )
    .unwrap();
    git(binary, root.path(), &["init", "-q"], b"");
    git(binary, root.path(), &["add", "vibe.toml"], b"");
    git(binary, root.path(), &["write-tree"], b"")
}
fn plan(binary: &Path, package: &Project, origin: &Origin, reused_main: bool) -> PackageGitPlan {
    let identity =
        package_git_parent_source::local_bare_identity_digest(&origin.project, limits().source)
            .unwrap();
    let person = GitPerson {
        name: "Oleg Chirukhin".into(),
        email: "oleg@example.test".into(),
        timestamp_seconds: 1_700_000_002,
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
            remote_identity_sha256: identity.clone(),
        },
        payload_sha256: capture_source_identity(package, payload_limits())
            .unwrap()
            .tree_sha256,
        flattened_submodules_sha256: package_git_provenance::digest(
            &NoGitlinksEvidence::NoGitRepository,
        )
        .unwrap(),
        oid_hex_len: 40,
        before_repo: RepositoryObservation::Present(RepositoryIdentity::Direct {
            remote_identity_sha256: identity,
        }),
        before_refs: RefSnapshot {
            main: RawRef::Present {
                oid: origin.main.clone(),
            },
            version_tag: TagRef::Present {
                raw_oid: origin.tag.clone(),
                peeled_commit_oid: origin.main.clone(),
            },
        },
        staged: StagedObjects {
            git_tree_oid: staged_tree(binary, package),
            commit: if reused_main {
                StagedCommit::ReusedMain {
                    oid: origin.main.clone(),
                }
            } else {
                StagedCommit::New {
                    oid: "d".repeat(40),
                    parent_oid: Some(origin.main.clone()),
                }
            },
            annotated_tag_oid: if reused_main {
                None
            } else {
                Some("e".repeat(40))
            },
            tag_target_commit_oid: origin.main.clone(),
            commit_metadata: if reused_main {
                None
            } else {
                Some(CommitMetadata {
                    author: person.clone(),
                    committer: person.clone(),
                    message: "Release wal@1.1.0\n".into(),
                })
            },
            tag_metadata: if reused_main {
                None
            } else {
                Some(TagMetadata {
                    tagger: person,
                    message: "wal@1.1.0\n".into(),
                })
            },
        },
    };
    plan.derive_staged_object_ids().unwrap();
    plan
}
pub(crate) struct World {
    pub(crate) root: tempfile::TempDir,
    _package_root: tempfile::TempDir,
    pub(crate) package: Project,
    pub(crate) origin: Origin,
    pub(crate) lease: LifecycleLease,
    pub(crate) token: String,
    pub(crate) scope: SubmissionScope,
    pub(crate) run_id: String,
    pub(crate) plan: PackageGitPlan,
}
pub(crate) fn world(binary: &Path, reused_main: bool) -> World {
    let contents: &[u8] = if reused_main {
        b"[package]\nname='old'\n"
    } else {
        b"[package]\nname='new'\n"
    };
    let origin = seed_origin(binary, b"[package]\nname='old'\n");
    let (package_root, package) = package_source(contents);
    let plan = plan(binary, &package, &origin, reused_main);
    assert_eq!(reused_main, plan.staged.git_tree_oid == origin.tree);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = SubmissionScope {
        context: "parent-fixture".into(),
        target: format!(
            "package-git-direct:{}",
            package_git_parent_source::local_bare_identity_digest(&origin.project, limits().source)
                .unwrap()
        ),
        principal: "owner".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let token = namespace.id().to_owned();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &scope,
        "parent",
        &package,
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
            &scope,
            "parent",
            Some(&package),
            binary
        )
        .unwrap(),
        TreeInspection::TreeCompleteButNotReady { .. }
    ));
    assert!(matches!(
        package_git_provenance::retain_no_gitlinks_pending(
            &lease,
            &token,
            &scope,
            "parent",
            &package,
            binary,
            limits().census
        ),
        ProvenanceInspection::NoGitlinksRetainedButNotReady { .. }
    ));
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &scope, "parent")
            .unwrap()
            .unwrap()
            .journal_admission,
        JournalAdmission::Pending
    );
    World {
        root,
        _package_root: package_root,
        package,
        origin,
        lease,
        token,
        scope,
        run_id,
        plan,
    }
}

#[test]
fn local_bare_parent_closure_lands_and_cold_rechecks_without_fetch() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, false);
    let first = stage_local_parent_pending(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        &world.origin.project,
        &binary,
        limits(),
    );
    let ParentInspection::ClosureCompleteButNotReady {
        run_id,
        closure_sha256,
        source_reobserved: true,
    } = first
    else {
        panic!("parent closure did not land: {first:?}")
    };
    assert_eq!(run_id, world.run_id);
    let receipt = journal::read::<ParentReceipt>(&world.lease, &run_id, "parent")
        .unwrap()
        .unwrap();
    assert!(receipt.closure_objects >= 5);
    assert_eq!(receipt.parent_oid, world.origin.main);
    assert_eq!(receipt.closure_sha256, closure_sha256);
    let before = journal::read_bytes(&world.lease, &run_id, "parent")
        .unwrap()
        .unwrap();
    assert!(matches!(
        stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ParentInspection::ClosureCompleteButNotReady { .. }
    ));
    assert_eq!(
        journal::read_bytes(&world.lease, &run_id, "parent")
            .unwrap()
            .unwrap(),
        before
    );
    assert!(matches!(
        inspect_parent_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            limits()
        ),
        ParentInspection::ClosureCompleteButNotReady {
            source_reobserved: false,
            ..
        }
    ));
    let object_root = world
        .root
        .path()
        .join(format!(".vibe/package-git-operations/{run_id}.objects"));
    let config = fs::read_to_string(object_root.join("config")).unwrap();
    assert!(!config.contains("[remote "));
    assert!(!object_root.join("FETCH_HEAD").exists());
}

#[test]
fn reused_main_noop_requires_exact_fetched_tree_and_peeled_tag() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, true);
    assert!(matches!(
        stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ParentInspection::ClosureCompleteButNotReady { .. }
    ));
    let receipt = journal::read::<ParentReceipt>(&world.lease, &world.run_id, "parent")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.parent_tree_oid, world.plan.staged.git_tree_oid);
    assert_eq!(receipt.parent_tree_oid, world.origin.tree);
    assert!(matches!(
        crate::package_git_objects::stage_existing_main_objects_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        crate::package_git_objects::ObjectInspection::Unavailable { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .is_none()
    );
}

#[test]
fn moved_source_refs_after_fetch_leave_pending_only_and_no_retry() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, false);
    let source = world.origin.project.root_path().to_str().unwrap();
    let moved = stage_inner(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        &world.origin.project,
        &binary,
        limits(),
        || {
            git(
                &binary,
                world.origin.project.root_path(),
                &[
                    "--git-dir",
                    source,
                    "update-ref",
                    "refs/heads/main",
                    &world.origin.previous,
                ],
                b"",
            );
        },
    );
    assert!(matches!(moved, ParentInspection::Uncertain { .. }));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "parent-pending")
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "parent")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        inspect_parent_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            limits()
        ),
        ParentInspection::Uncertain { .. }
    ));
    assert!(matches!(
        stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ParentInspection::Uncertain { .. }
    ));
}

#[path = "tests_failure.rs"]
mod tests_failure;
