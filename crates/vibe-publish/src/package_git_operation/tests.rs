use std::fs;

use super::*;
use crate::package_git_plan::{
    PackageCoordinate, RawRef, RefSnapshot, RepositoryIdentity, RepositoryObservation,
    RepositoryTarget, StagedCommit, StagedObjects, TagRef,
};

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
fn plan(source: &Project) -> PackageGitPlan {
    let tree = capture_source_identity(source, limits())
        .unwrap()
        .tree_sha256;
    let remote_identity_sha256 = format!("sha256:{}", "9".repeat(64));
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
            remote_identity_sha256: remote_identity_sha256.clone(),
        },
        payload_sha256: tree,
        flattened_submodules_sha256: format!("sha256:{}", "b".repeat(64)),
        oid_hex_len: 40,
        before_repo: RepositoryObservation::Present(RepositoryIdentity::Direct {
            remote_identity_sha256,
        }),
        before_refs: RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
        staged: StagedObjects {
            git_tree_oid: "c".repeat(40),
            commit: StagedCommit::New {
                oid: "d".repeat(40),
                parent_oid: None,
            },
            annotated_tag_oid: Some("e".repeat(40)),
            tag_target_commit_oid: "d".repeat(40),
        },
    }
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "host-package-git".into(),
        target: format!("package-git-direct:sha256:{}", "9".repeat(64)),
        principal: "owner".into(),
    }
}

#[test]
fn one_c1_pending_run_retains_exact_local_vault_but_never_becomes_ready() {
    let (_source_root, source) = source();
    let plan = plan(&source);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let first = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "package-key",
        &source,
        &plan,
        limits(),
    )
    .unwrap();
    let PendingInspection::VaultCompleteButNotReady {
        run_id,
        tree_sha256,
    } = first
    else {
        panic!("local vault incomplete")
    };
    assert_eq!(tree_sha256, plan.payload_sha256);
    let row =
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "package-key")
            .unwrap()
            .unwrap();
    assert_eq!(row.domain_run_id, run_id);
    assert_eq!(row.journal_admission, JournalAdmission::Pending);
    let vault = root
        .path()
        .join(format!(".vibe/package-git-operations/{run_id}.payloads"));
    assert_eq!(
        fs::read(vault.join("payload/docs/Привет.md")).unwrap(),
        b"hello\n"
    );
    assert!(vault.join("complete.json").is_file());
    assert!(
        root.path()
            .join(format!(
                ".vibe/package-git-operations/{run_id}.package-git-stage.json"
            ))
            .is_file()
    );
    let before = fs::read(vault.join("complete.json")).unwrap();
    assert_eq!(
        prepare_pending(
            &lease,
            &namespace,
            &selected,
            "package-key",
            &source,
            &plan,
            limits()
        )
        .unwrap(),
        PendingInspection::VaultCompleteButNotReady {
            run_id: run_id.clone(),
            tree_sha256
        }
    );
    assert_eq!(
        fs::read(vault.join("complete.json")).unwrap(),
        before,
        "duplicate restaged payload"
    );
    let token = namespace.id().to_owned();
    drop(lease);
    let reopened = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        inspect_pending_cold(&reopened, &token, &selected, "package-key", Some(&source)).unwrap(),
        PendingInspection::VaultCompleteButNotReady {
            run_id,
            tree_sha256: plan.payload_sha256
        }
    );
}

#[test]
fn interruption_after_intent_stays_pending_and_duplicate_does_not_stage() {
    let (_source_root, source) = source();
    let plan = plan(&source);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let first = prepare_pending_inner(
        &lease,
        &namespace,
        &selected,
        "stopped",
        &source,
        &plan,
        limits(),
        || true,
    )
    .unwrap();
    let PendingInspection::PendingIncomplete { run_id } = first else {
        panic!()
    };
    assert!(
        root.path()
            .join(format!(
                ".vibe/package-git-operations/{run_id}.package-git-intent.json"
            ))
            .is_file()
    );
    assert!(
        !root
            .path()
            .join(format!(".vibe/package-git-operations/{run_id}.payloads"))
            .exists()
    );
    assert_eq!(
        prepare_pending(
            &lease,
            &namespace,
            &selected,
            "stopped",
            &source,
            &plan,
            limits()
        )
        .unwrap(),
        PendingInspection::PendingIncomplete {
            run_id: run_id.clone()
        }
    );
    assert!(
        !root
            .path()
            .join(format!(".vibe/package-git-operations/{run_id}.payloads"))
            .exists()
    );
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "stopped")
            .unwrap()
            .unwrap()
            .journal_admission,
        JournalAdmission::Pending
    );
}

#[test]
fn changed_source_plan_or_scope_never_restages_or_grants_ready() {
    let (source_root, source) = source();
    let plan = plan(&source);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let first = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "drift",
        &source,
        &plan,
        limits(),
    )
    .unwrap();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = first else {
        panic!()
    };
    let mut changed = plan.clone();
    changed.version_tag_ref = "refs/tags/release-1.1.0".into();
    assert!(
        prepare_pending(
            &lease,
            &namespace,
            &selected,
            "drift",
            &source,
            &changed,
            limits()
        )
        .is_err()
    );
    fs::write(source_root.path().join("docs/Привет.md"), b"foreign\n").unwrap();
    assert!(
        matches!(inspect_pending_cold(&lease, namespace.id(), &selected, "drift", Some(&source)).unwrap(),
        PendingInspection::Uncertain { run_id: seen, .. } if seen == run_id)
    );
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..selected.clone()
    };
    assert!(
        inspect_pending_cold(&lease, namespace.id(), &foreign, "drift", Some(&source)).is_err()
    );
    assert_eq!(
        inspect_pending_cold(
            &lease,
            namespace.id(),
            &selected,
            "wrong-key",
            Some(&source)
        )
        .unwrap(),
        PendingInspection::Absent
    );
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "drift")
            .unwrap()
            .unwrap()
            .journal_admission,
        JournalAdmission::Pending
    );
}

#[test]
fn tampered_retained_bytes_do_not_produce_vault_complete_status() {
    let (_source_root, source) = source();
    let plan = plan(&source);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "tamper",
        &source,
        &plan,
        limits(),
    )
    .unwrap() else {
        panic!()
    };
    let vault_file = root.path().join(format!(
        ".vibe/package-git-operations/{run_id}.payloads/payload/vibe.toml"
    ));
    fs::write(vault_file, b"tampered").unwrap();
    assert!(matches!(
        inspect_pending_cold(&lease, namespace.id(), &selected, "tamper", Some(&source)).unwrap(),
        PendingInspection::Uncertain { .. }
    ));
}

#[test]
fn intent_tampered_between_cold_proof_passes_is_typed_uncertain() {
    let (_source_root, source) = source();
    let plan = plan(&source);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "mid-proof",
        &source,
        &plan,
        limits(),
    )
    .unwrap() else {
        panic!("fixture must begin as a complete Pending vault")
    };
    let (status, proof) = pending_proof_cold_inner(
        &lease,
        namespace.id(),
        &selected,
        "mid-proof",
        Some(&source),
        |observed_run| {
            assert_eq!(observed_run, run_id);
            fs::write(
                root.path().join(format!(
                    ".vibe/package-git-operations/{observed_run}.package-git-intent.json"
                )),
                b"not-json",
            )
            .unwrap();
        },
    )
    .unwrap();
    assert!(proof.is_none());
    assert!(matches!(status, PendingInspection::Uncertain { run_id: got, .. } if got == run_id));
}

#[test]
fn cold_refuses_c1_bound_malformed_plan_even_with_matching_record_hashes() {
    let (_source_root, source) = source();
    let plan = plan(&source);
    let selected_source = capture_source_identity(&source, limits()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let request = journal::Request {
        format: journal::REQUEST_FORMAT.into(),
        semantic_plan_sha256: plan.digest().unwrap(),
        source_root_token: selected_source.root_token.clone(),
        source_tree_sha256: selected_source.tree_sha256.clone(),
        skipped_gitmodules: selected_source.skipped_gitmodules,
        lease_root_token: lease
            .package_git_journal_project()
            .root_correlation_token()
            .unwrap(),
        scope_target: selected.target.clone(),
        limits: limits(),
    };
    let request_bytes = serde_json::to_vec(&request).unwrap();
    let bad_plan = b"{\"bogus\":true}";
    let run_id = vibe_lifecycle::process::propose_run_id(lease.root()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &selected,
            "forged",
            &request_bytes,
            bad_plan,
            &run_id
        )
        .unwrap(),
        ReservationDecision::Created { .. }
    ));
    let intent = journal::Intent {
        format: journal::INTENT_FORMAT.into(),
        run_id: run_id.clone(),
        request_json: String::from_utf8(request_bytes.clone()).unwrap(),
        plan_json: String::from_utf8(bad_plan.to_vec()).unwrap(),
        request_sha256: sha(&request_bytes),
        c1_plan_sha256: sha(bad_plan),
        semantic_plan_sha256: request.semantic_plan_sha256,
        source_root_token: request.source_root_token,
        source_tree_sha256: request.source_tree_sha256,
        skipped_gitmodules: request.skipped_gitmodules,
        lease_root_token: request.lease_root_token,
        scope_context: selected.context.clone(),
        scope_target: selected.target.clone(),
        scope_principal: selected.principal.clone(),
        key_sha256: sha(b"forged"),
        limits: limits(),
    };
    journal::publish(&lease, &run_id, "intent", &intent).unwrap();
    assert_eq!(
        inspect_pending_cold(&lease, namespace.id(), &selected, "forged", Some(&source)).unwrap(),
        PendingInspection::RecoveryRequired { run_id }
    );
}

#[cfg(unix)]
#[test]
fn copied_same_bytes_at_replaced_source_root_is_cold_uncertain() {
    let parent = tempfile::tempdir().unwrap();
    let source_path = parent.path().join("source");
    let retired = parent.path().join("retired-source");
    fs::create_dir(&source_path).unwrap();
    fs::write(source_path.join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let source = Project::open(&source_path).unwrap();
    let plan = plan(&source);
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let PendingInspection::VaultCompleteButNotReady { run_id, .. } = prepare_pending(
        &lease,
        &namespace,
        &selected,
        "root-drift",
        &source,
        &plan,
        limits(),
    )
    .unwrap() else {
        panic!()
    };
    fs::rename(&source_path, &retired).unwrap();
    fs::create_dir(&source_path).unwrap();
    fs::write(source_path.join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    assert!(
        matches!(inspect_pending_cold(&lease, namespace.id(), &selected, "root-drift", Some(&source)).unwrap(),
        PendingInspection::Uncertain { run_id: observed, .. } if observed == run_id)
    );
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "root-drift")
            .unwrap()
            .unwrap()
            .journal_admission,
        JournalAdmission::Pending
    );
}
