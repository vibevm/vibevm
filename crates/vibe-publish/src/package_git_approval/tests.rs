use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_gitlink::CensusLimits;
use crate::package_git_objects::{self, ObjectInspection};
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_payload::{PayloadLimits, capture_source_identity};
use crate::package_git_plan::{
    CommitMetadata, CreationOptions, GitPerson, HostedRepository, PackageCoordinate, RawRef,
    RefSnapshot, RepositoryIdentity, StagedCommit, StagedObjects, TagMetadata, TagRef,
};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_tree::{self, TreeInspection};

fn binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .find_map(|dir| dir.join(name).canonicalize().ok().filter(|p| p.is_file()))
}

fn git(binary: &Path, cwd: &Path, args: &[&str]) -> String {
    let result = Command::new(binary)
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
        .env("PATH", binary.parent().unwrap())
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "local fixture Git {args:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().into()
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

struct World {
    _source_root: tempfile::TempDir,
    root: tempfile::TempDir,
    settings_root: tempfile::TempDir,
    source: Project,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    run_id: String,
    binary: PathBuf,
    home: ApprovalHome,
}

impl World {
    fn new(binary: &Path) -> Self {
        let source_root = tempfile::tempdir().unwrap();
        fs::write(
            source_root.path().join("vibe.toml"),
            b"[package]\nname='wal'\n",
        )
        .unwrap();
        let source = Project::open(source_root.path()).unwrap();
        let scratch = tempfile::tempdir().unwrap();
        fs::copy(
            source.root_path().join("vibe.toml"),
            scratch.path().join("vibe.toml"),
        )
        .unwrap();
        git(binary, scratch.path(), &["init", "-q"]);
        git(binary, scratch.path(), &["add", "vibe.toml"]);
        let tree = git(binary, scratch.path(), &["write-tree"]);
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
            payload_sha256: capture_source_identity(&source, payload_limits())
                .unwrap()
                .tree_sha256,
            flattened_submodules_sha256: package_git_provenance::digest(
                &journal::NoGitlinksEvidence::NoGitRepository,
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
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = SubmissionScope {
            context: "package-approval-fixture".into(),
            target: "package-git:github.com/vibespecs/org.vibevm.wal".into(),
            principal: "owner".into(),
        };
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
            &lease,
            &namespace,
            &scope,
            "approval",
            &source,
            &plan,
            payload_limits(),
        )
        .unwrap() else {
            panic!("local payload not retained")
        };
        assert!(matches!(
            package_git_tree::stage_tree_pending(
                &lease,
                namespace.id(),
                &scope,
                "approval",
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
                "approval",
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
                "approval",
                &source,
                binary,
                limits().census
            ),
            ObjectInspection::ObjectsCompleteButNotReady { .. }
        ));
        assert!(matches!(
            package_git_ready::seal_local_ready(
                &lease,
                namespace.id(),
                &scope,
                "approval",
                &source,
                None,
                binary,
                limits()
            ),
            ReadyInspection::ReadyForApproval { .. }
        ));
        let settings_root = tempfile::tempdir().unwrap();
        fs::create_dir_all(settings_root.path().join(HOME).join(APPROVAL_DIR)).unwrap();
        let home = ApprovalHome::open_selected(settings_root.path()).unwrap();
        let world = Self {
            _source_root: source_root,
            root,
            settings_root,
            source,
            lease,
            namespace,
            scope,
            run_id,
            binary: binary.into(),
            home,
        };
        world.write_record(&world.record());
        world
    }

    fn record(&self) -> ApprovalRecord {
        let bytes = journal::read_bytes(&self.lease, &self.run_id, "ready")
            .unwrap()
            .unwrap();
        let seal: journal::ReadySeal = serde_json::from_slice(&bytes).unwrap();
        let (_, proof) = package_git_operation::ready_proof_cold(
            &self.lease,
            self.namespace.id(),
            &self.scope,
            "approval",
            Some(&self.source),
        )
        .unwrap();
        let plan = proof.unwrap().plan;
        ApprovalRecord {
            format: FORMAT.into(),
            approval_id: "human-approval-1".into(),
            approver: "Oleg Chirukhin".into(),
            issued_at_unix: 1,
            expires_at_unix: 20,
            semantic_plan_sha256: seal.semantic_plan_sha256.clone(),
            c1_plan_sha256: seal.c1_plan_sha256.clone(),
            request_sha256: seal.request_sha256.clone(),
            ready_seal_sha256: sha(&bytes),
            namespace_id: self.namespace.id().into(),
            run_id: self.run_id.clone(),
            key_sha256: seal.key_sha256,
            context: self.scope.context.clone(),
            target: self.scope.target.clone(),
            principal: self.scope.principal.clone(),
            workspace_root_token: seal.lease_root_token,
            settings_root_token: self.home.settings.root_correlation_token().unwrap(),
            repository: plan.target.clone(),
            effects: effects(&plan).unwrap(),
        }
    }

    fn write_record(&self, record: &ApprovalRecord) {
        let path = self
            .settings_root
            .path()
            .join(HOME)
            .join(APPROVAL_DIR)
            .join(approval_name(&record.semantic_plan_sha256).unwrap());
        fs::write(path, serde_json::to_vec(record).unwrap()).unwrap();
    }

    fn load(&self) -> Result<VerifiedApproval, ApprovalError> {
        self.home.load_at(
            &self.lease,
            &self.namespace,
            &self.scope,
            "approval",
            Some(&self.source),
            None,
            &self.binary,
            limits(),
            10,
        )
    }

    fn redeem(
        &self,
        verified: &VerifiedApproval,
        now: u64,
        lose_reply: bool,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        self.home.redeem_at(
            &self.lease,
            &self.namespace,
            &self.scope,
            "approval",
            Some(&self.source),
            None,
            &self.binary,
            limits(),
            verified,
            Some(now),
            lose_reply,
        )
    }

    fn marker_path(&self, id: &str) -> PathBuf {
        self.settings_root
            .path()
            .join(HOME)
            .join(REDEEMED)
            .join(marker_name(id))
    }
}

#[test]
fn exact_owner_record_redeems_once_and_cold_survives_expiry() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    let receipt = world.redeem(&verified, 10, false).unwrap();
    assert_eq!(receipt.run_id, world.run_id);
    assert!(matches!(
        world.redeem(&verified, 10, false),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    let expected = ColdExpectation {
        approval_id: &verified.record.approval_id,
        approval_record_sha256: &receipt.approval_record_sha256,
        marker_sha256: &receipt.marker_sha256,
    };
    assert_eq!(
        world
            .home
            .inspect_redemption_cold(
                &world.lease,
                &world.namespace,
                &world.scope,
                "approval",
                None,
                None,
                &binary,
                limits(),
                &expected
            )
            .unwrap(),
        receipt
    );
    // No time check on historical marker inspection: the fixture expiry is 20.
    assert!(world.root.path().exists());
}

#[test]
fn expired_between_load_and_last_prepublication_boundary_leaves_no_marker() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    assert!(matches!(
        world.redeem(&verified, 20, false),
        Err(ApprovalError::Refused(_))
    ));
    assert!(!world.marker_path(&verified.record.approval_id).exists());
}

#[test]
fn foreign_repo_scope_root_and_noncanonical_record_refuse_before_marker() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let original = world.record();
    let mut altered = original.clone();
    altered.target = "foreign".into();
    world.write_record(&altered);
    assert!(world.load().is_err());
    altered = original.clone();
    altered.workspace_root_token = sha(b"foreign");
    world.write_record(&altered);
    assert!(world.load().is_err());
    altered = original.clone();
    altered.repository = RepositoryTarget::DirectGitExisting {
        remote_identity_sha256: sha(b"foreign"),
    };
    world.write_record(&altered);
    assert!(world.load().is_err());
    world.write_record(&original);
    let path = world
        .settings_root
        .path()
        .join(HOME)
        .join(APPROVAL_DIR)
        .join(approval_name(&original.semantic_plan_sha256).unwrap());
    let mut spaced = serde_json::to_vec(&original).unwrap();
    spaced.push(b' ');
    fs::write(path, spaced).unwrap();
    assert!(world.load().is_err());
    assert!(!world.marker_path(&original.approval_id).exists());
}

#[test]
fn occupied_cross_domain_marker_lost_reply_and_tamper_require_recovery() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    fs::create_dir_all(world.settings_root.path().join(HOME).join(REDEEMED)).unwrap();
    let marker_path = world.marker_path(&verified.record.approval_id);
    fs::write(&marker_path, b"distribution-marker").unwrap();
    assert!(matches!(
        world.redeem(&verified, 10, false),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    assert_eq!(fs::read(&marker_path).unwrap(), b"distribution-marker");
    fs::remove_file(&marker_path).unwrap();
    assert!(matches!(
        world.redeem(&verified, 10, true),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    assert!(matches!(
        world.redeem(&verified, 10, false),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    let bytes = fs::read(&marker_path).unwrap();
    let expected = ColdExpectation {
        approval_id: &verified.record.approval_id,
        approval_record_sha256: &verified.record_sha256,
        marker_sha256: &sha(&bytes),
    };
    assert!(
        world
            .home
            .inspect_redemption_cold(
                &world.lease,
                &world.namespace,
                &world.scope,
                "approval",
                None,
                None,
                &binary,
                limits(),
                &expected
            )
            .is_ok()
    );
    fs::write(&marker_path, b"tampered").unwrap();
    assert!(matches!(
        world.home.inspect_redemption_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            None,
            None,
            &binary,
            limits(),
            &expected
        ),
        Err(ApprovalError::RecoveryRequired(_))
    ));
}

#[test]
fn existing_hosted_repository_is_approved_for_atomic_push_only() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let (_, proof) = package_git_operation::ready_proof_cold(
        &world.lease,
        world.namespace.id(),
        &world.scope,
        "approval",
        Some(&world.source),
    )
    .unwrap();
    let mut plan = proof.unwrap().plan;
    let RepositoryTarget::HostedCreate { repository, .. } = &plan.target else {
        panic!("fixture target is not hosted")
    };
    plan.before_repo = RepositoryObservation::Present(RepositoryIdentity::Hosted {
        repository: repository.clone(),
        id: 7,
    });
    let parent = "a".repeat(40);
    plan.before_refs.main = RawRef::Present {
        oid: parent.clone(),
    };
    plan.staged.commit = StagedCommit::New {
        oid: "d".repeat(40),
        parent_oid: Some(parent),
    };
    plan.derive_staged_object_ids().unwrap();
    assert_eq!(
        effects(&plan).unwrap(),
        EffectSet {
            create_repository: false,
            atomic_push_main_and_version_tag: true,
        }
    );
    if let RepositoryObservation::Present(RepositoryIdentity::Hosted { id, .. }) =
        &mut plan.before_repo
    {
        *id = 0;
    }
    assert!(effects(&plan).is_err());
}

#[test]
fn replaced_settings_path_after_load_refuses_redemption() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    let original = world.settings_root.path().to_path_buf();
    let moved = original.with_extension("held-old");
    fs::rename(&original, &moved).unwrap();
    fs::create_dir(&original).unwrap();
    assert!(matches!(
        world.redeem(&verified, 10, false),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    assert!(!world.marker_path(&verified.record.approval_id).exists());
}

#[test]
fn damaged_ready_seal_after_load_requires_recovery_before_marker() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    fs::write(
        world.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-ready.json",
            world.run_id
        )),
        b"not-json",
    )
    .unwrap();
    assert!(matches!(
        world.redeem(&verified, 10, false),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    assert!(!world.marker_path(&verified.record.approval_id).exists());
}

#[path = "link_tests.rs"]
mod link_tests;
