use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_payload::{PayloadLimits, capture_source_identity, stage_local_payload};
use crate::package_git_plan::{
    CommitMetadata, CreationOptions, GitPerson, HostedRepository, PackageCoordinate,
    PackageGitPlan, RawRef, RefSnapshot, RepositoryObservation, RepositoryTarget, StagedCommit,
    StagedObjects, TagMetadata, TagRef,
};
use crate::package_git_submodule;
use crate::package_git_submodule_store::{self, LinkStoreInspection};
use crate::package_git_tree::TreeInspection;
use vibe_lifecycle::SubmissionReservations;

pub(crate) fn binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .find_map(|dir| dir.join(name).canonicalize().ok().filter(|p| p.is_file()))
}
pub(crate) fn git(binary: &Path, cwd: &Path, args: &[&str]) -> String {
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
        "local fixture Git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
pub(crate) fn payload_limits() -> PayloadLimits {
    PayloadLimits {
        max_files: 16,
        max_directories: 16,
        max_children_per_directory: 16,
        max_depth: 5,
        max_path_bytes: 512,
        max_file_bytes: 4096,
        max_total_bytes: 16 * 4096,
        max_manifest_bytes: 16 * 4096,
    }
}
pub(crate) fn limits() -> SubmoduleLimits {
    SubmoduleLimits {
        census: crate::package_git_gitlink::CensusLimits {
            max_index_bytes: 32 * 1024,
            max_probe_bytes: 4096,
            max_gitlinks: 8,
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
        max_total_raw_bytes: 16 * 4096,
        max_files: 16,
        max_directories: 16,
        max_children_per_tree: 16,
        max_depth: 5,
        max_path_bytes: 512,
    }
}
fn selected_tree(binary: &Path, source: &Project) -> String {
    let root = tempfile::tempdir().unwrap();
    git(binary, root.path(), &["init", "-q"]);
    fs::write(
        root.path().join("vibe.toml"),
        fs::read(source.root_path().join("vibe.toml")).unwrap(),
    )
    .unwrap();
    for name in ["a", "b"] {
        fs::create_dir_all(root.path().join(format!("vendor/{name}"))).unwrap();
        fs::write(
            root.path().join(format!("vendor/{name}/data.txt")),
            fs::read(source.root_path().join(format!("vendor/{name}/data.txt"))).unwrap(),
        )
        .unwrap();
    }
    git(binary, root.path(), &["add", "-A"]);
    git(binary, root.path(), &["write-tree"])
}
fn plan(source: &Project, digest: String, tree: String) -> PackageGitPlan {
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
        flattened_submodules_sha256: digest,
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
pub(crate) struct World {
    pub(crate) root: tempfile::TempDir,
    _source_root: tempfile::TempDir,
    pub(crate) source: Project,
    pub(crate) approved: Vec<Project>,
    pub(crate) lease: LifecycleLease,
    pub(crate) namespace: String,
    pub(crate) scope: SubmissionScope,
    pub(crate) run_id: String,
    pub(crate) digest: String,
    pub(crate) binary: PathBuf,
}
impl World {
    pub(crate) fn held(&self) -> Vec<ApprovedGitdir<'_>> {
        ["vendor/a", "vendor/b"]
            .into_iter()
            .zip(&self.approved)
            .map(|(path, gitdir)| ApprovedGitdir { path, gitdir })
            .collect()
    }
    pub(crate) fn stage_links(&self, count: usize) {
        let GitIndexCensus::PopulatedCandidate { links, .. } =
            package_git_gitlink::census(&self.source, &self.binary, limits().census)
        else {
            panic!("index unavailable")
        };
        for (link, gitdir) in links.iter().zip(&self.approved).take(count) {
            assert!(matches!(
                package_git_submodule_store::stage_one_pending(
                    &self.lease,
                    &self.namespace,
                    &self.scope,
                    "aggregate",
                    &self.source,
                    gitdir,
                    link,
                    &self.binary,
                    limits()
                ),
                LinkStoreInspection::ByteStoreCompleteButNotReady { .. }
            ));
        }
    }
}
pub(crate) fn world(binary: &Path) -> World {
    let source_root = tempfile::tempdir().unwrap();
    git(binary, source_root.path(), &["init", "-q"]);
    fs::write(
        source_root.path().join("vibe.toml"),
        b"[package]\nkind='flow'\ngroup='org.vibevm'\nname='wal'\nversion='1.1.0'\n",
    )
    .unwrap();
    let mut approved = Vec::new();
    for name in ["a", "b"] {
        let checkout = source_root.path().join(format!("vendor/{name}"));
        fs::create_dir_all(&checkout).unwrap();
        git(binary, &checkout, &["init", "-q"]);
        git(
            binary,
            &checkout,
            &["config", "user.email", "fixture@local"],
        );
        git(binary, &checkout, &["config", "user.name", "fixture"]);
        fs::write(checkout.join("data.txt"), format!("link {name}\n")).unwrap();
        git(binary, &checkout, &["add", "data.txt"]);
        git(binary, &checkout, &["commit", "-qm", "submodule"]);
        let oid = git(binary, &checkout, &["rev-parse", "HEAD"]);
        let row = format!("160000,{oid},vendor/{name}");
        git(
            binary,
            source_root.path(),
            &["update-index", "--add", "--cacheinfo", &row],
        );
        approved.push(Project::open(&checkout.join(".git")).unwrap());
    }
    git(binary, source_root.path(), &["add", "vibe.toml"]);
    let source = Project::open(source_root.path()).unwrap();
    let pre_root = tempfile::tempdir().unwrap();
    let pre_vault = Project::open(pre_root.path()).unwrap();
    let pre_manifest = stage_local_payload(&source, &pre_vault, payload_limits()).unwrap();
    let index = package_git_gitlink::census(&source, binary, limits().census);
    let GitIndexCensus::PopulatedCandidate { links, .. } = &index else {
        panic!("pre-C1 full index absent")
    };
    let comparisons: Vec<_> = links
        .iter()
        .zip(&approved)
        .map(|(link, gitdir)| {
            package_git_submodule::compare_one(
                &source,
                &pre_vault,
                &pre_manifest,
                link,
                gitdir,
                binary,
                limits(),
            )
        })
        .collect();
    let digest = semantic_digest(&index, &comparisons).unwrap();
    let selected_plan = plan(&source, digest.clone(), selected_tree(binary, &source));
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = SubmissionScope {
        context: "populated-aggregate-fixture".into(),
        target: "package-git:github.com/vibespecs/org.vibevm.wal".into(),
        principal: "owner".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 600_000).unwrap();
    let token = namespace.id().to_owned();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &scope,
        "aggregate",
        &source,
        &selected_plan,
        payload_limits(),
    )
    .unwrap() else {
        panic!("C1 payload not staged")
    };
    assert!(matches!(
        package_git_tree::stage_tree_pending(
            &lease,
            &token,
            &scope,
            "aggregate",
            Some(&source),
            binary
        )
        .unwrap(),
        TreeInspection::TreeCompleteButNotReady { .. }
    ));
    World {
        root,
        _source_root: source_root,
        source,
        approved,
        lease,
        namespace: token,
        scope,
        run_id,
        digest,
        binary: binary.into(),
    }
}

#[test]
fn two_link_aggregate_lands_and_cold_survives_source_loss() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; aggregate fixture skipped");
        return;
    };
    let world = world(&binary);
    world.stage_links(2);
    let approved = world.held();
    let expected = AggregateInspection::AggregateCompleteButNotReady {
        run_id: world.run_id.clone(),
        flattened_submodules_sha256: world.digest.clone(),
        source_reobserved: true,
    };
    assert_eq!(
        retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits()
        ),
        expected
    );
    let bytes = journal::read_bytes(&world.lease, &world.run_id, LANDED)
        .unwrap()
        .unwrap();
    assert_eq!(
        retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits()
        ),
        expected
    );
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, LANDED)
            .unwrap()
            .unwrap(),
        bytes
    );
    fs::rename(
        world.source.root_path().join("vendor/a"),
        world.source.root_path().join("retired-a"),
    )
    .unwrap();
    assert_eq!(
        inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending
        ),
        AggregateInspection::AggregateCompleteButNotReady {
            run_id: world.run_id.clone(),
            flattened_submodules_sha256: world.digest.clone(),
            source_reobserved: false
        }
    );
}

#[test]
fn missing_one_link_never_creates_aggregate_pending() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; aggregate fixture skipped");
        return;
    };
    let world = world(&binary);
    world.stage_links(1);
    let approved = world.held();
    assert!(matches!(
        retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits()
        ),
        AggregateInspection::NotReady { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, PENDING)
            .unwrap()
            .is_none()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, LANDED)
            .unwrap()
            .is_none()
    );
}

#[test]
fn full_index_drift_after_pending_stays_uncertain_without_landed() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; aggregate fixture skipped");
        return;
    };
    let world = world(&binary);
    world.stage_links(2);
    let approved = world.held();
    let status = retain_inner(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        &world.source,
        &approved,
        &binary,
        limits(),
        || {
            let changed = format!("160000,{},vendor/b", "a".repeat(40));
            git(
                &binary,
                world.source.root_path(),
                &["update-index", "--add", "--cacheinfo", &changed],
            );
        },
    );
    assert!(
        matches!(status, AggregateInspection::Uncertain { .. }),
        "{status:?}"
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, PENDING)
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, LANDED)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending
        ),
        AggregateInspection::Uncertain { .. }
    ));
}

#[test]
fn tampered_aggregate_landed_refuses_historical_cold() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; aggregate fixture skipped");
        return;
    };
    let world = world(&binary);
    world.stage_links(2);
    let approved = world.held();
    assert!(matches!(
        retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits()
        ),
        AggregateInspection::AggregateCompleteButNotReady { .. }
    ));
    fs::write(
        world.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-{LANDED}.json",
            world.run_id
        )),
        b"not-json",
    )
    .unwrap();
    assert!(matches!(
        inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending
        ),
        AggregateInspection::Uncertain { .. }
    ));
}

#[test]
fn pre_c1_semantic_digest_is_order_independent_but_complete() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; aggregate fixture skipped");
        return;
    };
    let world = world(&binary);
    let index = package_git_gitlink::census(&world.source, &binary, limits().census);
    let GitIndexCensus::PopulatedCandidate { links, .. } = &index else {
        panic!()
    };
    let (_, proof) = package_git_operation::pending_proof_cold(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        Some(&world.source),
    )
    .unwrap();
    let proof = proof.unwrap();
    let mut comparisons: Vec<_> = links
        .iter()
        .zip(&world.approved)
        .map(|(link, gitdir)| {
            package_git_submodule::compare_one(
                &world.source,
                &proof.vault,
                &proof.manifest,
                link,
                gitdir,
                &binary,
                limits(),
            )
        })
        .collect();
    assert_eq!(semantic_digest(&index, &comparisons).unwrap(), world.digest);
    comparisons.reverse();
    assert_eq!(semantic_digest(&index, &comparisons).unwrap(), world.digest);
    assert!(semantic_digest(&index, &comparisons[..1]).is_err());
    comparisons.push(comparisons[0].clone());
    assert!(semantic_digest(&index, &comparisons).is_err());
    let mut reordered_index = index.clone();
    let GitIndexCensus::PopulatedCandidate { links, .. } = &mut reordered_index else {
        panic!()
    };
    links.reverse();
    assert!(semantic_digest(&reordered_index, &comparisons[..2]).is_err());
    let mut duplicate_index = index.clone();
    let GitIndexCensus::PopulatedCandidate { links, .. } = &mut duplicate_index else {
        panic!()
    };
    links[1] = links[0].clone();
    assert!(semantic_digest(&duplicate_index, &comparisons[..2]).is_err());
}

#[path = "tests_ready.rs"]
mod tests_ready;

#[path = "tests_failure.rs"]
mod tests_failure;
