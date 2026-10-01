use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_operation::journal::NoGitlinksEvidence;
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_payload::{PayloadLimits, capture_source_identity};
use crate::package_git_plan::{
    CommitMetadata, CreationOptions, GitPerson, HostedRepository, PackageCoordinate,
    PackageGitPlan, RefSnapshot, StagedObjects, TagMetadata,
};
use crate::package_git_provenance::{self, ProvenanceInspection};

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
fn limits() -> ReadyLimits {
    ReadyLimits {
        census: CensusLimits {
            max_index_bytes: 8192,
            max_probe_bytes: 4096,
            max_gitlinks: 4,
            max_path_bytes: 256,
        },
        parent: None,
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
fn source() -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let source = Project::open(root.path()).unwrap();
    (root, source)
}
fn tree_oid(binary: &Path, source: &Project) -> String {
    let root = tempfile::tempdir().unwrap();
    fs::copy(
        source.root_path().join("vibe.toml"),
        root.path().join("vibe.toml"),
    )
    .unwrap();
    git(binary, root.path(), &["init", "-q"]);
    git(binary, root.path(), &["add", "vibe.toml"]);
    git(binary, root.path(), &["write-tree"])
}
fn plan(source: &Project, tree: String) -> PackageGitPlan {
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
        flattened_submodules_sha256: crate::package_git_provenance::digest(
            &NoGitlinksEvidence::NoGitRepository,
        )
        .unwrap(),
        oid_hex_len: 40,
        before_repo: RepositoryObservation::Absent,
        before_refs: RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
        staged: StagedObjects {
            git_tree_oid: tree,
            commit: StagedCommit::New {
                oid: "d".repeat(40),
                parent_oid: None,
            },
            annotated_tag_oid: Some("e".repeat(40)),
            tag_target_commit_oid: "d".repeat(40),
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
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    run_id: String,
    plan: PackageGitPlan,
    binary: PathBuf,
}
fn world(binary: &Path) -> World {
    let (source_root, source) = source();
    let plan = plan(&source, tree_oid(binary, &source));
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = SubmissionScope {
        context: "ready-fixture".into(),
        target: "package-git:github.com/vibespecs/org.vibevm.wal".into(),
        principal: "owner".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &scope,
        "ready",
        &source,
        &plan,
        payload_limits(),
    )
    .unwrap() else {
        panic!("payload not staged")
    };
    assert!(matches!(
        package_git_tree::stage_tree_pending(
            &lease,
            namespace.id(),
            &scope,
            "ready",
            Some(&source),
            binary
        )
        .unwrap(),
        TreeInspection::TreeCompleteButNotReady { .. }
    ));
    assert!(matches!(
        package_git_provenance::retain_no_gitlinks_pending(
            &lease,
            namespace.id(),
            &scope,
            "ready",
            &source,
            binary,
            limits().census
        ),
        ProvenanceInspection::NoGitlinksRetainedButNotReady { .. }
    ));
    assert!(matches!(
        package_git_objects::stage_parentless_objects_pending(
            &lease,
            namespace.id(),
            &scope,
            "ready",
            &source,
            binary,
            limits().census
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
    World {
        root,
        _source_root: source_root,
        source,
        lease,
        namespace,
        scope,
        run_id,
        plan,
        binary: binary.into(),
    }
}

#[test]
fn parentless_ready_seal_survives_cold_reopen_and_is_not_effect_permission() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; ready fixture skipped");
        return;
    };
    let world = world(&binary);
    let result = seal_local_ready(
        &world.lease,
        world.namespace.id(),
        &world.scope,
        "ready",
        &world.source,
        None,
        &binary,
        limits(),
    );
    let ReadyInspection::ReadyForApproval {
        run_id,
        seal_sha256,
    } = result
    else {
        panic!("Ready seal failed: {result:?}")
    };
    assert_eq!(run_id, world.run_id);
    assert_eq!(
        SubmissionReservations::journal_admission(
            &world.lease,
            &world.namespace,
            &world.scope,
            "ready",
            &run_id
        )
        .unwrap(),
        JournalAdmission::Ready
    );
    let bytes = journal::read_bytes(&world.lease, &run_id, SEAL)
        .unwrap()
        .unwrap();
    assert_eq!(sha(&bytes), seal_sha256);
    let seal: journal::ReadySeal = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(seal.format, journal::READY_FORMAT);
    assert_eq!(
        seal.provenance,
        journal::ReadyProvenance::NoGitlinks {
            pending_sha256: sha(
                &journal::read_bytes(&world.lease, &run_id, "provenance-pending")
                    .unwrap()
                    .unwrap()
            ),
            landed_sha256: sha(&journal::read_bytes(&world.lease, &run_id, "provenance")
                .unwrap()
                .unwrap()),
        }
    );
    assert!(matches!(
        prepare_pending(&world.lease, &world.namespace, &world.scope, "ready", &world.source, &world.plan, payload_limits()).unwrap(),
        PendingInspection::ReadyRetainedNeedsColdProof { run_id: same } if same == run_id
    ));
    assert_eq!(
        journal::read_bytes(&world.lease, &run_id, SEAL)
            .unwrap()
            .unwrap(),
        bytes
    );
    let World {
        root,
        lease,
        namespace,
        scope,
        run_id,
        ..
    } = world;
    drop(lease);
    let reopened = LifecycleLease::acquire(root.path()).unwrap();
    assert!(
        matches!(inspect_ready_seal_cold(&reopened, namespace.id(), &scope, "ready", None, None, &binary, limits()), ReadyInspection::ReadyForApproval { run_id: same, .. } if same == run_id)
    );
}

#[test]
fn crash_after_seal_before_c1_ready_resumes_same_run_without_reseal() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; ready fixture skipped");
        return;
    };
    let world = world(&binary);
    let first = seal_inner(
        &world.lease,
        world.namespace.id(),
        &world.scope,
        "ready",
        &world.source,
        None,
        &binary,
        limits(),
        || true,
    );
    assert!(
        matches!(first, ReadyInspection::SealRetainedC1Pending { .. }),
        "{first:?}"
    );
    assert_eq!(
        SubmissionReservations::journal_admission(
            &world.lease,
            &world.namespace,
            &world.scope,
            "ready",
            &world.run_id
        )
        .unwrap(),
        JournalAdmission::Pending
    );
    let before = journal::read_bytes(&world.lease, &world.run_id, SEAL)
        .unwrap()
        .unwrap();
    let second = seal_local_ready(
        &world.lease,
        world.namespace.id(),
        &world.scope,
        "ready",
        &world.source,
        None,
        &binary,
        limits(),
    );
    assert!(
        matches!(second, ReadyInspection::ReadyForApproval { .. }),
        "{second:?}"
    );
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, SEAL)
            .unwrap()
            .unwrap(),
        before
    );
}

#[test]
fn missing_or_tampered_seal_and_changed_vault_require_recovery() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; ready fixture skipped");
        return;
    };
    for defect in ["missing", "tampered", "vault", "tree", "objects"] {
        let world = world(&binary);
        assert!(matches!(
            seal_local_ready(
                &world.lease,
                world.namespace.id(),
                &world.scope,
                "ready",
                &world.source,
                None,
                &binary,
                limits()
            ),
            ReadyInspection::ReadyForApproval { .. }
        ));
        let seal = world.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-ready.json",
            world.run_id
        ));
        match defect {
            "missing" => fs::remove_file(seal).unwrap(),
            "tampered" => fs::write(seal, b"not-json").unwrap(),
            "vault" => fs::write(
                world.root.path().join(format!(
                    ".vibe/package-git-operations/{}.payloads/payload/vibe.toml",
                    world.run_id
                )),
                b"drift",
            )
            .unwrap(),
            "tree" | "objects" => fs::write(
                world.root.path().join(format!(
                    ".vibe/package-git-operations/{}.package-git-{defect}.json",
                    world.run_id
                )),
                b"not-json",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        assert!(matches!(
            inspect_ready_seal_cold(
                &world.lease,
                world.namespace.id(),
                &world.scope,
                "ready",
                None,
                None,
                &binary,
                limits()
            ),
            ReadyInspection::RecoveryRequired { .. }
        ));
    }
}

#[test]
fn foreign_scope_or_key_cannot_read_ready_seal() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; ready fixture skipped");
        return;
    };
    let world = world(&binary);
    assert!(matches!(
        seal_local_ready(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "ready",
            &world.source,
            None,
            &binary,
            limits()
        ),
        ReadyInspection::ReadyForApproval { .. }
    ));
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..world.scope.clone()
    };
    assert!(matches!(
        inspect_ready_seal_cold(
            &world.lease,
            world.namespace.id(),
            &foreign,
            "ready",
            None,
            None,
            &binary,
            limits()
        ),
        ReadyInspection::RecoveryRequired { .. }
    ));
    assert!(!matches!(
        inspect_ready_seal_cold(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "foreign-key",
            None,
            None,
            &binary,
            limits()
        ),
        ReadyInspection::ReadyForApproval { .. }
    ));
}

#[test]
fn historical_e1_ready_seal_never_decodes_or_upgrades_as_e2() {
    let Some(binary) = binary() else { return };
    let world = world(&binary);
    assert!(matches!(
        seal_local_ready(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "ready",
            &world.source,
            None,
            &binary,
            limits()
        ),
        ReadyInspection::ReadyForApproval { .. }
    ));
    let bytes = journal::read_bytes(&world.lease, &world.run_id, SEAL)
        .unwrap()
        .unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    old["format"] = serde_json::Value::String("vibevm.package-git-ready-seal/e1".into());
    old.as_object_mut().unwrap().remove("provenance");
    old["no_gitlinks_pending_sha256"] = serde_json::Value::String(sha(b"old pending"));
    old["no_gitlinks_landed_sha256"] = serde_json::Value::String(sha(b"old landed"));
    let old_bytes = serde_json::to_vec(&old).unwrap();
    let path = world.root.path().join(format!(
        ".vibe/package-git-operations/{}.package-git-ready.json",
        world.run_id
    ));
    fs::write(&path, &old_bytes).unwrap();
    assert!(matches!(
        inspect_ready_seal_cold(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "ready",
            None,
            None,
            &binary,
            limits()
        ),
        ReadyInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fs::read(path).unwrap(), old_bytes);
}
