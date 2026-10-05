//! Explicit trusted host fixtures; no ambient OS-user authority is claimed.
use super::*;
use std::fs;

pub(super) fn facts() -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: true,
        approve_exact_local_install: false,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: true,
    }
}
pub(super) fn service(root: &std::path::Path) -> CliGrantService {
    if !root.join("vibe.toml").exists() {
        fs::write(
            root.join("vibe.toml"),
            "[project]\nname = 'grant-fixture'\nversion = '1.1.0'\n",
        )
        .unwrap();
    }
    CliGrantService::from_trusted_host(
        Arc::new(Project::open(root).unwrap()),
        "physical-target".into(),
        "fixture-principal".into(),
        TargetOs::Linux,
        TrustedPolicySource::from_host_identifier("fixture-host-policy".into()).unwrap(),
        facts(),
    )
    .unwrap()
}
fn pointer(service: &CliGrantService) -> Pointer {
    serde_json::from_slice(&fs::read(service.project.root_path().join(&service.path)).unwrap())
        .unwrap()
}
fn write_pointer(service: &CliGrantService, value: &Pointer) {
    fs::write(
        service.project.root_path().join(&service.path),
        serde_json::to_vec(value).unwrap(),
    )
    .unwrap();
}
fn namespace(service: &CliGrantService) -> SubmissionNamespace {
    let GrantState::Issued { namespace, .. } = pointer(service).grant else {
        panic!("issued fixture")
    };
    SubmissionNamespace::from_host_token(namespace).unwrap()
}
fn index(root: &std::path::Path) -> Vec<u8> {
    fs::read(root.join(".vibe/machine-submissions.json")).unwrap()
}
fn assert_refuses_both(service: &CliGrantService) {
    assert!(service.reattach().is_err());
    assert!(service.issue(60_000).is_err());
}

#[test]
fn independent_instances_recover_exact_namespace_scope_and_do_not_write_c1() {
    let root = tempfile::tempdir().unwrap();
    let first = service(root.path());
    let host = first.issue(60_000).unwrap();
    assert_eq!(host.readonly_scope(), &first.scope);
    let retained = pointer(&first);
    let before = index(root.path());
    drop(host);
    drop(first);
    let second = service(root.path());
    let host = second.reattach().unwrap();
    assert_eq!(host.readonly_scope(), &second.scope);
    assert_eq!(pointer(&second), retained);
    assert_eq!(index(root.path()), before);
    assert!(matches!(
        second.issue(60_000),
        Err(CliGrantError::AlreadyIssued)
    ));
}

#[test]
fn missing_pointer_after_issue_never_mints_replacement_or_deletes_c1() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    service.issue(60_000).unwrap();
    let before = index(root.path());
    fs::remove_file(root.path().join(&service.path)).unwrap();
    assert!(matches!(
        service.reattach(),
        Err(CliGrantError::MissingPointer)
    ));
    assert!(matches!(
        service.issue(60_000),
        Err(CliGrantError::MissingPointer)
    ));
    assert_eq!(index(root.path()), before);
}

#[test]
fn pending_issue_fence_blocks_even_without_a_c1_namespace() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let lease = service.lease().unwrap();
    service
        .publish(&lease, &service.pointer(GrantState::Issuing))
        .unwrap();
    drop(lease);
    assert!(matches!(
        service.issue(60_000),
        Err(CliGrantError::IssuanceIncomplete)
    ));
    assert!(matches!(
        service.reattach(),
        Err(CliGrantError::IssuanceIncomplete)
    ));
    assert!(!root.path().join(".vibe/machine-submissions.json").exists());
}

// SafeFS consumes a hook once. Re-arm across the Issuing and policy writes
// to reach the final binding publication rather than a preceding checkpoint.
fn corrupt_issued_publication(path: std::path::PathBuf) {
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |_, name| {
        let issued = path.file_name() == Some(std::ffi::OsStr::new(name))
            && matches!(
                serde_json::from_slice::<Pointer>(&fs::read(&path).unwrap())
                    .unwrap()
                    .grant,
                GrantState::Issued { .. }
            );
        if issued {
            fs::write(&path, b"corrupt-final-pointer").unwrap();
        } else {
            corrupt_issued_publication(path.clone());
        }
    })));
}

#[test]
fn final_pointer_publication_uncertainty_leaves_one_namespace_and_blocks_reissue() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let path = root.path().join(&service.path);
    corrupt_issued_publication(path.clone());
    let result = service.issue(60_000);
    vibe_safefs::arm_before_publish_verify(None);
    assert!(matches!(
        result,
        Err(CliGrantError::UncertainPublication(_))
    ));
    let before = index(root.path());
    assert_refuses_both(&service);
    fs::remove_file(path).unwrap();
    assert!(matches!(
        service.issue(60_000),
        Err(CliGrantError::MissingPointer)
    ));
    assert_eq!(index(root.path()), before);
    let value: serde_json::Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(value["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn initial_pointer_publication_uncertainty_never_reaches_c1() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let path = root.path().join(&service.path);
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |_, name| {
        if path.file_name() == Some(std::ffi::OsStr::new(name)) {
            fs::write(&path, b"uncertain").unwrap();
        }
    })));
    let result = service.issue(60_000);
    vibe_safefs::arm_before_publish_verify(None);
    assert!(matches!(
        result,
        Err(CliGrantError::UncertainPublication(_))
    ));
    assert_refuses_both(&service);
    assert!(!root.path().join(".vibe/machine-submissions.json").exists());
}

#[test]
fn malformed_oversized_unknown_field_and_unknown_version_pointers_refuse() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    service.issue(60_000).unwrap();
    let original = pointer(&service);
    let before = index(root.path());
    let encoded = serde_json::to_value(&original).unwrap();
    let mut top_unknown = encoded.clone();
    top_unknown["approval"] = true.into();
    let mut scope_unknown = encoded.clone();
    scope_unknown["scope"]["outer_session"] = "copied".into();
    let mut grant_unknown = encoded.clone();
    grant_unknown["grant"]["result"] = "forged".into();
    let mut version = encoded;
    version["schema"] = 2.into();
    for bytes in [
        b"broken".to_vec(),
        vec![b'x'; CAP + 1],
        serde_json::to_vec(&top_unknown).unwrap(),
        serde_json::to_vec(&scope_unknown).unwrap(),
        serde_json::to_vec(&grant_unknown).unwrap(),
        serde_json::to_vec(&version).unwrap(),
    ] {
        fs::write(root.path().join(&service.path), bytes).unwrap();
        assert_refuses_both(&service);
        assert_eq!(index(root.path()), before);
    }
}

#[test]
fn foreign_pointer_root_scope_source_and_os_refuse_without_replacement() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    service.issue(60_000).unwrap();
    let original = pointer(&service);
    let before = index(root.path());
    for changed in ["root", "context", "target", "principal", "source", "os"] {
        let mut foreign = original.clone();
        match changed {
            "root" => foreign.root_token.push('x'),
            "context" => foreign.scope.context.push('x'),
            "target" => foreign.scope.target.push('x'),
            "principal" => foreign.scope.principal.push('x'),
            "source" => foreign.policy_source.push('x'),
            _ => foreign.target_os = "windows".into(),
        }
        write_pointer(&service, &foreign);
        assert!(matches!(
            service.reattach(),
            Err(CliGrantError::ForeignBinding)
        ));
        assert!(matches!(
            service.issue(60_000),
            Err(CliGrantError::ForeignBinding)
        ));
        assert_eq!(index(root.path()), before);
    }
}

#[test]
fn changed_host_authority_cannot_reattach_a_copied_pointer() {
    let root = tempfile::tempdir().unwrap();
    let original = service(root.path());
    original.issue(60_000).unwrap();
    let bytes = fs::read(root.path().join(&original.path)).unwrap();
    for (target, principal, os, source) in [
        (
            "other",
            "fixture-principal",
            TargetOs::Linux,
            "fixture-host-policy",
        ),
        (
            "physical-target",
            "other",
            TargetOs::Linux,
            "fixture-host-policy",
        ),
        (
            "physical-target",
            "fixture-principal",
            TargetOs::Windows,
            "fixture-host-policy",
        ),
        (
            "physical-target",
            "fixture-principal",
            TargetOs::Linux,
            "other",
        ),
    ] {
        let foreign = CliGrantService::from_trusted_host(
            Arc::new(Project::open(root.path()).unwrap()),
            target.into(),
            principal.into(),
            os,
            TrustedPolicySource::from_host_identifier(source.into()).unwrap(),
            facts(),
        )
        .unwrap();
        fs::write(root.path().join(&foreign.path), &bytes).unwrap();
        assert_refuses_both(&foreign);
    }
}

#[test]
fn copied_pointer_and_index_to_foreign_root_refuse() {
    let root = tempfile::tempdir().unwrap();
    let original = service(root.path());
    original.issue(60_000).unwrap();
    let foreign_root = tempfile::tempdir().unwrap();
    let foreign = service(foreign_root.path());
    fs::create_dir_all(foreign_root.path().join(".vibe/cli-host-grants")).unwrap();
    fs::copy(
        root.path().join(&original.path),
        foreign_root.path().join(&foreign.path),
    )
    .unwrap();
    fs::write(
        foreign_root.path().join(".vibe/machine-submissions.json"),
        index(root.path()),
    )
    .unwrap();
    assert_refuses_both(&foreign);
}

#[test]
fn unissued_or_foreign_namespace_and_changed_expiry_refuse() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    service.issue(60_000).unwrap();
    let original = pointer(&service);
    let lease = service.lease().unwrap();
    let mut foreign_scope = service.scope.clone();
    foreign_scope.context.push('x');
    let foreign = SubmissionReservations::issue_namespace(&lease, &foreign_scope, 60_000).unwrap();
    drop(lease);
    for token in [
        format!("{:016x}-{}", 99, "0".repeat(32)),
        foreign.id().into(),
    ] {
        let mut forged = original.clone();
        let GrantState::Issued { namespace, .. } = &mut forged.grant else {
            unreachable!()
        };
        *namespace = token;
        write_pointer(&service, &forged);
        assert!(matches!(
            service.reattach(),
            Err(CliGrantError::Namespace(
                SubmissionError::UnknownNamespace | SubmissionError::ForeignScope
            ))
        ));
        assert!(service.issue(60_000).is_err());
    }
    let mut forged = original;
    let GrantState::Issued { expires_at_ms, .. } = &mut forged.grant else {
        unreachable!()
    };
    *expires_at_ms += 1;
    write_pointer(&service, &forged);
    assert!(matches!(
        service.reattach(),
        Err(CliGrantError::InvalidPointer(_))
    ));
}

#[test]
fn changed_current_policy_requires_current_facts_and_does_not_restore_pointer_policy() {
    let root = tempfile::tempdir().unwrap();
    let original = service(root.path());
    original.issue(60_000).unwrap();
    let retained = pointer(&original);
    let lease = original.lease().unwrap();
    let owner = original.current_owner(&lease).unwrap();
    let old = owner.current_under_lease(&lease).unwrap();
    let mut changed = facts();
    changed.approve_captured_script = false;
    let new = owner
        .update_under_lease(
            &lease,
            old.generation(),
            original.source.clone(),
            changed.clone(),
        )
        .unwrap();
    assert_ne!(old.generation(), new.generation());
    drop(lease);
    assert!(matches!(
        original.reattach(),
        Err(CliGrantError::Policy(PolicyError::StartupDisagreement))
    ));
    let mut current = service(root.path());
    current.facts = changed;
    current.reattach().unwrap();
    assert_eq!(pointer(&current), retained);
    let lease = current.lease().unwrap();
    let snapshot = owner.current_under_lease(&lease).unwrap();
    owner
        .update_under_lease(
            &lease,
            snapshot.generation(),
            TrustedPolicySource::from_host_identifier("other-source".into()).unwrap(),
            current.facts.clone(),
        )
        .unwrap();
    drop(lease);
    assert!(matches!(
        current.reattach(),
        Err(CliGrantError::Policy(PolicyError::StartupDisagreement))
    ));
}

#[test]
fn expired_and_retired_grants_reattach_for_recovery_but_never_dispatch_new_keys() {
    for retire in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let service = service(root.path());
        service.issue(if retire { 60_000 } else { 1 }).unwrap();
        let namespace = namespace(&service);
        let lease = service.lease().unwrap();
        if retire {
            SubmissionReservations::retire_namespace(&lease, &namespace, &service.scope).unwrap();
        }
        drop(lease);
        if !retire {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        service.reattach().unwrap();
        let lease = service.lease().unwrap();
        assert!(
            SubmissionReservations::validate_namespace(&lease, &namespace, &service.scope)
                .unwrap()
                .1
        );
        assert_eq!(
            SubmissionReservations::reserve(
                &lease,
                &namespace,
                &service.scope,
                "new",
                b"r",
                b"p",
                "run"
            )
            .unwrap(),
            vibe_lifecycle::ReservationDecision::Retired {
                domain_run_id: None
            }
        );
        drop(lease);
        assert!(matches!(
            service.issue(60_000),
            Err(CliGrantError::AlreadyIssued)
        ));
    }
}

#[test]
fn concurrent_first_issue_has_one_namespace_and_later_reattaches() {
    let root = tempfile::tempdir().unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut children = Vec::new();
    for _ in 0..2 {
        let service = service(root.path());
        let barrier = barrier.clone();
        children.push(std::thread::spawn(move || {
            barrier.wait();
            service.issue(60_000)
        }));
    }
    barrier.wait();
    let results: Vec<_> = children
        .into_iter()
        .map(|child| child.join().unwrap())
        .collect();
    assert!(results.iter().filter(|result| result.is_ok()).count() <= 1);
    assert!(results.iter().all(|result| matches!(
        result,
        Ok(_)
            | Err(CliGrantError::Lease(LifecycleLeaseError::Busy { .. }))
            | Err(CliGrantError::AlreadyIssued)
            | Err(CliGrantError::Host(FixedDecisionError::Infrastructure(_)))
    )));
    let index: serde_json::Value = serde_json::from_slice(&index(root.path())).unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    service(root.path()).reattach().unwrap();
}

#[test]
fn root_replacement_refuses_retained_and_new_service_even_with_copied_records() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    fs::create_dir(&root).unwrap();
    let retained = service(&root);
    retained.issue(60_000).unwrap();
    let bytes = fs::read(root.join(&retained.path)).unwrap();
    let index = index(&root);
    fs::rename(&root, parent.path().join("old-root")).unwrap();
    fs::create_dir_all(root.join(".vibe/cli-host-grants")).unwrap();
    fs::write(root.join(&retained.path), bytes).unwrap();
    fs::write(root.join(".vibe/machine-submissions.json"), index).unwrap();
    assert_refuses_both(&retained);
    assert_refuses_both(&service(&root));
}

#[test]
fn invalid_authority_and_nonfinite_ttl_refuse_before_issuance() {
    let root = tempfile::tempdir().unwrap();
    for target in ["", " padded", "control\n", &"x".repeat(257)] {
        assert!(matches!(
            CliGrantService::from_trusted_host(
                Arc::new(Project::open(root.path()).unwrap()),
                target.into(),
                "principal".into(),
                TargetOs::Linux,
                TrustedPolicySource::from_host_identifier("source".into()).unwrap(),
                facts()
            ),
            Err(CliGrantError::InvalidAuthority)
        ));
    }
    let service = service(root.path());
    for ttl in [0, MAX_TTL_MS + 1, u64::MAX] {
        assert!(matches!(service.issue(ttl), Err(CliGrantError::InvalidTtl)));
    }
    assert!(matches!(
        service.reattach(),
        Err(CliGrantError::MissingPointer)
    ));
    assert!(!root.path().join(&service.path).exists());
    assert!(!root.path().join(".vibe/machine-submissions.json").exists());
}

#[test]
fn durability_evidence_requires_all_synced_checkpoints_and_exact_parent_paths() {
    let root = tempfile::tempdir().unwrap();
    let project = Project::open(root.path()).unwrap();
    let write = project
        .write_atomic_durable("fresh/record", b"fixture")
        .unwrap();
    let parent = root.path().join("fresh");
    require_durable(&write, &parent).unwrap();
    let mut unsynced = write.clone();
    unsynced.file_synced = false;
    assert!(require_durable(&unsynced, &parent).is_err());
    let mut missing = write.clone();
    missing.directory_syncs.remove(0);
    assert!(require_durable(&missing, &parent).is_err());
    let mut wrong_path = write.clone();
    wrong_path.directory_syncs[0].directory = parent.clone();
    assert!(require_durable(&wrong_path, &parent).is_err());
    for kind in [
        DirectoryDurability::JournalRecoverable,
        DirectoryDurability::Unsupported(std::io::ErrorKind::Unsupported),
        DirectoryDurability::Failed(std::io::ErrorKind::Other),
    ] {
        let mut weak = write.clone();
        weak.directory_syncs[0].durability = kind;
        assert!(matches!(
            require_durable(&weak, &parent),
            Err(CliGrantError::UncertainPublication(_))
        ));
    }
}

#[test]
fn missing_or_corrupt_current_policy_is_never_initialized_by_reattach() {
    for corrupt in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let service = service(root.path());
        service.issue(60_000).unwrap();
        let before = index(root.path());
        let record = fs::read_dir(root.path().join(".vibe/machine-policy"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        if corrupt {
            fs::write(&record, b"corrupt").unwrap();
        } else {
            fs::remove_file(&record).unwrap();
        }
        assert!(matches!(
            service.reattach(),
            Err(CliGrantError::Policy(
                PolicyError::Unavailable | PolicyError::InvalidRecord(_)
            ))
        ));
        assert!(matches!(
            service.issue(60_000),
            Err(CliGrantError::AlreadyIssued)
        ));
        if corrupt {
            assert_eq!(fs::read(&record).unwrap(), b"corrupt");
        } else {
            assert!(!record.exists());
        }
        assert_eq!(index(root.path()), before);
    }
}

#[test]
fn host_construction_refusal_retains_one_grant_for_explicit_manifest_repair() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    fs::remove_file(root.path().join("vibe.toml")).unwrap();
    assert!(matches!(
        service.issue(60_000),
        Err(CliGrantError::Host(FixedDecisionError::Infrastructure(_)))
    ));
    let retained = pointer(&service);
    let before = index(root.path());
    assert!(matches!(
        service.issue(60_000),
        Err(CliGrantError::AlreadyIssued)
    ));
    assert!(service.reattach().is_err());
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname = 'repaired'\nversion = '1.1.0'\n",
    )
    .unwrap();
    service.reattach().unwrap();
    assert_eq!(pointer(&service), retained);
    assert_eq!(index(root.path()), before);
}
