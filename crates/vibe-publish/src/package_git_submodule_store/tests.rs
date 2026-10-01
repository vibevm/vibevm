use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_gitlink::{self, CensusLimits, GitIndexCensus};
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_payload::{PayloadLimits, capture_source_identity};
use crate::package_git_plan::{
    CommitMetadata, CreationOptions, GitPerson, HostedRepository, PackageCoordinate,
    PackageGitPlan, RawRef, RefSnapshot, RepositoryObservation, RepositoryTarget, StagedCommit,
    StagedObjects, TagMetadata, TagRef,
};
use vibe_lifecycle::{JournalAdmission, SubmissionReservations};

fn binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .find_map(|dir| dir.join(name).canonicalize().ok().filter(|p| p.is_file()))
}
fn git(binary: &Path, cwd: &Path, args: &[&str]) -> String {
    let out = Command::new(binary)
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
        .env("PATH", binary.parent().unwrap())
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
fn limits() -> SubmoduleLimits {
    SubmoduleLimits {
        census: CensusLimits {
            max_index_bytes: 16 * 1024,
            max_probe_bytes: 4096,
            max_gitlinks: 4,
            max_path_bytes: 512,
        },
        max_gitdir_path_bytes: 4096,
        max_gitfile_bytes: 4096,
        max_git_output_bytes: 4096,
        max_config_bytes: 4096,
        max_pack_entries: 16,
        max_commit_bytes: 4096,
        max_tree_bytes: 4096,
        max_blob_bytes: 4096,
        max_total_raw_bytes: 16 * 1024,
        max_files: 12,
        max_directories: 12,
        max_children_per_tree: 12,
        max_depth: 5,
        max_path_bytes: 512,
    }
}
fn expected_tree(binary: &Path, source: &Project, oid_len: usize) -> String {
    let root = tempfile::tempdir().unwrap();
    if oid_len == 64 {
        git(
            binary,
            root.path(),
            &["init", "-q", "--object-format=sha256"],
        );
    } else {
        git(binary, root.path(), &["init", "-q"]);
    }
    fs::create_dir_all(root.path().join("vendor/dep")).unwrap();
    fs::copy(
        source.root_path().join("vibe.toml"),
        root.path().join("vibe.toml"),
    )
    .unwrap();
    fs::copy(
        source.root_path().join("vendor/dep/data.txt"),
        root.path().join("vendor/dep/data.txt"),
    )
    .unwrap();
    git(binary, root.path(), &["add", "-A"]);
    git(binary, root.path(), &["write-tree"])
}
fn plan(source: &Project, tree: String, oid_len: usize) -> PackageGitPlan {
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
        target: RepositoryTarget::HostedCreate {
            repository: HostedRepository {
                host: "github.com".into(),
                organization: "vibespecs".into(),
                name: "org.vibevm.wal".into(),
            },
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
        flattened_submodules_sha256: format!("sha256:{}", "b".repeat(64)),
        oid_hex_len: oid_len,
        before_repo: RepositoryObservation::Absent,
        before_refs: RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
        staged: StagedObjects {
            git_tree_oid: tree,
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
struct World {
    root: tempfile::TempDir,
    _source_root: tempfile::TempDir,
    source: Project,
    approved_gitdir: Project,
    candidate: GitlinkCandidate,
    lease: LifecycleLease,
    namespace: String,
    scope: SubmissionScope,
    run_id: String,
    binary: PathBuf,
}
fn world(binary: &Path, oid_len: usize) -> World {
    let source_root = tempfile::tempdir().unwrap();
    if oid_len == 64 {
        git(
            binary,
            source_root.path(),
            &["init", "-q", "--object-format=sha256"],
        );
    } else {
        git(binary, source_root.path(), &["init", "-q"]);
    }
    fs::write(
        source_root.path().join("vibe.toml"),
        b"[package]\nkind='flow'\ngroup='org.vibevm'\nname='wal'\nversion='1.1.0'\n",
    )
    .unwrap();
    let checkout = source_root.path().join("vendor/dep");
    fs::create_dir_all(&checkout).unwrap();
    if oid_len == 64 {
        git(binary, &checkout, &["init", "-q", "--object-format=sha256"]);
    } else {
        git(binary, &checkout, &["init", "-q"]);
    }
    git(
        binary,
        &checkout,
        &["config", "user.email", "fixture@local"],
    );
    git(binary, &checkout, &["config", "user.name", "fixture"]);
    fs::write(checkout.join("data.txt"), b"selected Gitlink bytes\n").unwrap();
    git(binary, &checkout, &["add", "data.txt"]);
    git(binary, &checkout, &["commit", "-qm", "selected submodule"]);
    let commit = git(binary, &checkout, &["rev-parse", "HEAD"]);
    git(binary, source_root.path(), &["add", "vibe.toml"]);
    let row = format!("160000,{commit},vendor/dep");
    git(
        binary,
        source_root.path(),
        &["update-index", "--add", "--cacheinfo", &row],
    );
    let source = Project::open(source_root.path()).unwrap();
    let GitIndexCensus::PopulatedCandidate { links, .. } =
        package_git_gitlink::census(&source, binary, limits().census)
    else {
        panic!("Gitlink absent")
    };
    let candidate = links.into_iter().find(|r| r.path == "vendor/dep").unwrap();
    let approved_gitdir = Project::open(&checkout.join(".git")).unwrap();
    let selected_plan = plan(&source, expected_tree(binary, &source, oid_len), oid_len);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = SubmissionScope {
        context: "gitlink-store-fixture".into(),
        target: "package-git:github.com/vibespecs/org.vibevm.wal".into(),
        principal: "owner".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let token = namespace.id().to_owned();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &scope,
        "link",
        &source,
        &selected_plan,
        payload_limits(),
    )
    .unwrap() else {
        panic!("payload not staged")
    };
    assert!(matches!(
        package_git_tree::stage_tree_pending(&lease, &token, &scope, "link", Some(&source), binary)
            .unwrap(),
        TreeInspection::TreeCompleteButNotReady { .. }
    ));
    World {
        root,
        _source_root: source_root,
        source,
        approved_gitdir,
        candidate,
        lease,
        namespace: token,
        scope,
        run_id,
        binary: binary.into(),
    }
}

#[test]
fn one_gitlink_import_lands_and_cold_survives_source_loss() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local Gitlink store fixture skipped");
        return;
    };
    let world = world(&binary, 40);
    let first = stage_one_pending(
        &world.lease,
        &world.namespace,
        &world.scope,
        "link",
        &world.source,
        &world.approved_gitdir,
        &world.candidate,
        &binary,
        limits(),
    );
    let LinkStoreInspection::ByteStoreCompleteButNotReady {
        run_id,
        path_sha256,
        source_reobserved: true,
    } = first
    else {
        panic!("Gitlink byte store did not land: {first:?}")
    };
    assert_eq!(run_id, world.run_id);
    let (_, landed_name) = suffixes(&path_sha256);
    let landed_before = journal::read_bytes(&world.lease, &run_id, &landed_name)
        .unwrap()
        .unwrap();
    assert!(matches!(
        stage_one_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "link",
            &world.source,
            &world.approved_gitdir,
            &world.candidate,
            &binary,
            limits()
        ),
        LinkStoreInspection::ByteStoreCompleteButNotReady { .. }
    ));
    assert_eq!(
        journal::read_bytes(&world.lease, &run_id, &landed_name)
            .unwrap()
            .unwrap(),
        landed_before
    );
    fs::rename(
        world.source.root_path().join("vendor/dep"),
        world.source.root_path().join("retired-checkout"),
    )
    .unwrap();
    assert!(matches!(
        inspect_one_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "link",
            &world.candidate.path,
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending
        ),
        LinkStoreInspection::ByteStoreCompleteButNotReady {
            source_reobserved: false,
            ..
        }
    ));
}

#[test]
fn sha256_gitlink_import_keeps_selected_object_format() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local Gitlink store fixture skipped");
        return;
    };
    if !LocalGit::probe_sha256(&binary).unwrap() {
        return;
    }
    let world = world(&binary, 64);
    let result = stage_one_pending(
        &world.lease,
        &world.namespace,
        &world.scope,
        "link",
        &world.source,
        &world.approved_gitdir,
        &world.candidate,
        &binary,
        limits(),
    );
    assert!(
        matches!(
            result,
            LinkStoreInspection::ByteStoreCompleteButNotReady { .. }
        ),
        "{result:?}"
    );
    let store = journal::gitlink_store(
        &world.lease,
        &world.run_id,
        &path_sha(&world.candidate.path),
        false,
    )
    .unwrap()
    .unwrap();
    let git = LocalGit::new(&binary, &store, 64).unwrap();
    assert!(git.object_present(&world.candidate.indexed_oid).unwrap());
    assert!(matches!(
        inspect_one_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "link",
            &world.candidate.path,
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending
        ),
        LinkStoreInspection::ByteStoreCompleteButNotReady {
            source_reobserved: false,
            ..
        }
    ));
}

#[test]
fn missing_retained_blob_refuses_source_less_cold() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local Gitlink store fixture skipped");
        return;
    };
    let world = world(&binary, 40);
    assert!(matches!(
        stage_one_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "link",
            &world.source,
            &world.approved_gitdir,
            &world.candidate,
            &binary,
            limits()
        ),
        LinkStoreInspection::ByteStoreCompleteButNotReady { .. }
    ));
    let checkout = world.source.root_path().join("vendor/dep");
    let blob = git(&binary, &checkout, &["rev-parse", "HEAD:data.txt"]);
    let store = journal::gitlink_store(
        &world.lease,
        &world.run_id,
        &path_sha(&world.candidate.path),
        false,
    )
    .unwrap()
    .unwrap();
    let loose = store
        .root_path()
        .join(format!("objects/{}/{}", &blob[..2], &blob[2..]));
    fs::remove_file(loose).unwrap();
    assert!(matches!(
        inspect_one_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "link",
            &world.candidate.path,
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending
        ),
        LinkStoreInspection::Uncertain { .. }
    ));
}

#[test]
fn mid_import_source_drift_leaves_pending_only_and_never_reimports() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local Gitlink store fixture skipped");
        return;
    };
    let world = world(&binary, 40);
    let data = world.source.root_path().join("vendor/dep/data.txt");
    let result = stage_inner(
        &world.lease,
        &world.namespace,
        &world.scope,
        "link",
        &world.source,
        &world.approved_gitdir,
        &world.candidate,
        &binary,
        limits(),
        |count| {
            if count == 1 {
                fs::write(&data, b"changed mid-import\n").unwrap();
            }
        },
    );
    assert!(
        matches!(result, LinkStoreInspection::Uncertain { .. }),
        "{result:?}"
    );
    let (pending_name, landed_name) = suffixes(&path_sha(&world.candidate.path));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, &pending_name)
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, &landed_name)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        inspect_one_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "link",
            &world.candidate.path,
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending
        ),
        LinkStoreInspection::Uncertain { .. }
    ));
    assert!(matches!(
        stage_one_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "link",
            &world.source,
            &world.approved_gitdir,
            &world.candidate,
            &binary,
            limits()
        ),
        LinkStoreInspection::Uncertain { .. }
    ));
}

#[test]
fn uninitialized_gitlink_refuses_before_per_link_pending() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local Gitlink store fixture skipped");
        return;
    };
    let world = world(&binary, 40);
    fs::remove_dir_all(world.source.root_path().join("vendor/dep/.git")).unwrap();
    let status = stage_one_pending(
        &world.lease,
        &world.namespace,
        &world.scope,
        "link",
        &world.source,
        &world.approved_gitdir,
        &world.candidate,
        &binary,
        limits(),
    );
    assert!(
        matches!(status, LinkStoreInspection::Unavailable { .. }),
        "{status:?}"
    );
    let (pending_name, _) = suffixes(&path_sha(&world.candidate.path));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, &pending_name)
            .unwrap()
            .is_none()
    );
}
