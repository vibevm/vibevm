use super::*;
use tempfile::TempDir;

fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "context".into(),
        target: "target".into(),
        principal: "principal".into(),
    }
}
fn source() -> TrustedPolicySource {
    TrustedPolicySource::from_host_identifier("host-config".into()).unwrap()
}
fn facts() -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: true,
        allow_detach: false,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: false,
    }
}
fn attached(root: &TempDir) -> PolicyOwner {
    PolicyOwner::attach_readonly(Arc::new(Project::open(root.path()).unwrap()), scope()).unwrap()
}
fn initialized() -> (TempDir, LifecycleLease, PolicySnapshot) {
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let snapshot =
        PolicyOwner::initialize_under_lease(&lease, &scope(), source(), facts()).unwrap();
    (root, lease, snapshot)
}
fn bytes(root: &TempDir) -> Vec<u8> {
    std::fs::read(root.path().join(record::path(&scope()))).unwrap()
}
fn replace(root: &TempDir, value: &[u8]) {
    std::fs::write(root.path().join(record::path(&scope())), value).unwrap();
}

#[test]
fn readonly_missing_creates_no_infrastructure() {
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(
        PolicyOwner::attach_readonly(Arc::new(Project::open(root.path()).unwrap()), scope()),
        Err(PolicyError::Unavailable)
    ));
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn independent_owners_restart_and_startup_idempotence() {
    let (root, lease, snapshot) = initialized();
    let before = bytes(&root);
    let a = attached(&root);
    let b = attached(&root);
    assert_eq!(a.current_under_lease(&lease).unwrap(), snapshot);
    assert_eq!(b.current_under_lease(&lease).unwrap(), snapshot);
    assert_eq!(
        PolicyOwner::initialize_under_lease(&lease, &scope(), source(), facts()).unwrap(),
        snapshot
    );
    assert_eq!(bytes(&root), before);
    drop(lease);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        attached(&root).current_under_lease(&lease).unwrap(),
        snapshot
    );
    assert!(!root.path().join(".vibe/runs").exists());
}

#[test]
fn update_advances_revision_and_stale_actor_refuses_without_publication() {
    let (root, lease, first) = initialized();
    let a = attached(&root);
    let b = attached(&root);
    let next = a
        .update_under_lease(&lease, first.generation(), source(), facts())
        .unwrap();
    assert_ne!(first.generation(), next.generation());
    let rec: Record = serde_json::from_slice(&bytes(&root)).unwrap();
    assert_eq!(rec.revision, 2);
    let before = bytes(&root);
    assert_eq!(
        b.update_under_lease(&lease, first.generation(), source(), facts()),
        Err(PolicyError::StaleGeneration)
    );
    assert_eq!(before, bytes(&root));
    assert_eq!(b.current_under_lease(&lease).unwrap(), next);
    let mut changed = facts();
    changed.allow_detach = true;
    let third = b
        .update_under_lease(&lease, next.generation(), source(), changed.clone())
        .unwrap();
    assert_ne!(third.generation(), next.generation());
    assert_eq!(third.facts(), &changed);
    let fourth = a
        .update_under_lease(
            &lease,
            third.generation(),
            TrustedPolicySource::from_host_identifier("second-source".into()).unwrap(),
            changed,
        )
        .unwrap();
    assert_ne!(third.generation(), fourth.generation());
    assert_eq!(fourth.source().identifier(), "second-source");
}

#[test]
fn startup_disagreement_preserves_bytes() {
    let (root, lease, _) = initialized();
    let before = bytes(&root);
    let mut changed = facts();
    changed.allow_detach = true;
    assert_eq!(
        PolicyOwner::initialize_under_lease(&lease, &scope(), source(), changed),
        Err(PolicyError::StartupDisagreement)
    );
    assert_eq!(
        PolicyOwner::initialize_under_lease(
            &lease,
            &scope(),
            TrustedPolicySource::from_host_identifier("other".into()).unwrap(),
            facts()
        ),
        Err(PolicyError::StartupDisagreement)
    );
    assert_eq!(before, bytes(&root));
}

#[test]
fn malformed_records_refuse_startup_attach_and_update_without_overwrite() {
    let (root, lease, first) = initialized();
    let owner = attached(&root);
    let original = String::from_utf8(bytes(&root)).unwrap();
    let variants = vec![
        b"{".to_vec(),
        original.replacen('{', "{\"unknown\":true,", 1).into_bytes(),
        original.replacen('{', "{\"schema\":1,", 1).into_bytes(),
        original
            .replace("\"context\":", "\"extra\":1,\"context\":")
            .into_bytes(),
        original
            .replace("\"context\":", "\"context\":\"duplicate\",\"context\":")
            .into_bytes(),
        original
            .replace(
                "\"approve_captured_script\":",
                "\"extra\":1,\"approve_captured_script\":",
            )
            .into_bytes(),
        original
            .replace(
                "\"approve_captured_script\":",
                "\"approve_captured_script\":true,\"approve_captured_script\":",
            )
            .into_bytes(),
        original
            .replace("\"revision\":1", "\"revision\":0")
            .into_bytes(),
        original
            .replace("\"schema\":1", "\"schema\":2")
            .into_bytes(),
        original.replace("host-config", " ").into_bytes(),
        original.replace("sha256:", "invalid:").into_bytes(),
        vec![b' '; record::CAP + 1],
    ];
    for invalid in variants {
        replace(&root, &invalid);
        assert!(PolicyOwner::initialize_under_lease(&lease, &scope(), source(), facts()).is_err());
        assert!(
            PolicyOwner::attach_readonly(Arc::new(Project::open(root.path()).unwrap()), scope())
                .is_err()
        );
        assert!(
            owner
                .update_under_lease(&lease, first.generation(), source(), facts())
                .is_err()
        );
        assert_eq!(bytes(&root), invalid);
    }
}

#[test]
fn overflow_and_deleted_record_refuse_without_recovery() {
    let (root, lease, _) = initialized();
    let owner = attached(&root);
    let mut rec: Record = serde_json::from_slice(&bytes(&root)).unwrap();
    rec.revision = u64::MAX;
    replace(&root, &serde_json::to_vec(&rec).unwrap());
    let before = bytes(&root);
    let current = owner.current_under_lease(&lease).unwrap();
    assert_eq!(
        owner.update_under_lease(&lease, current.generation(), source(), facts()),
        Err(PolicyError::RevisionOverflow)
    );
    assert_eq!(before, bytes(&root));
    std::fs::remove_file(root.path().join(record::path(&scope()))).unwrap();
    assert_eq!(
        owner.current_under_lease(&lease),
        Err(PolicyError::Unavailable)
    );
    assert_eq!(
        owner.update_under_lease(&lease, current.generation(), source(), facts()),
        Err(PolicyError::Unavailable)
    );
    assert!(!root.path().join(record::path(&scope())).exists());
}

#[test]
fn foreign_scope_copied_root_and_foreign_lease_refuse() {
    let (root, lease, first) = initialized();
    let owner = attached(&root);
    let before = bytes(&root);
    let mut foreign = scope();
    foreign.principal = "other".into();
    std::fs::write(root.path().join(record::path(&foreign)), &before).unwrap();
    assert!(matches!(
        PolicyOwner::attach_readonly(
            Arc::new(Project::open(root.path()).unwrap()),
            foreign.clone()
        ),
        Err(PolicyError::ForeignScope)
    ));
    assert_eq!(
        PolicyOwner::initialize_under_lease(&lease, &foreign, source(), facts()),
        Err(PolicyError::ForeignScope)
    );
    let other = tempfile::tempdir().unwrap();
    let other_lease = LifecycleLease::acquire(other.path()).unwrap();
    std::fs::create_dir_all(other.path().join(".vibe/machine-policy")).unwrap();
    std::fs::write(other.path().join(record::path(&scope())), &before).unwrap();
    assert!(matches!(
        PolicyOwner::attach_readonly(Arc::new(Project::open(other.path()).unwrap()), scope()),
        Err(PolicyError::ForeignScope)
    ));
    assert_eq!(
        PolicyOwner::initialize_under_lease(&other_lease, &scope(), source(), facts()),
        Err(PolicyError::ForeignScope)
    );
    assert_eq!(
        owner.current_under_lease(&other_lease),
        Err(PolicyError::ForeignScope)
    );
    assert_eq!(
        owner.update_under_lease(&other_lease, first.generation(), source(), facts()),
        Err(PolicyError::ForeignScope)
    );
    assert_eq!(bytes(&root), before);
}

#[test]
fn publication_fault_remains_uncertain_while_fresh_attach_observes_candidate() {
    let (root, lease, first) = initialized();
    let owner = attached(&root);
    let path = record::path(&scope());
    let mut changed = facts();
    changed.allow_detach = true;
    vibe_safefs::fail_after_publish(Some(&path));
    let result = owner.update_under_lease(&lease, first.generation(), source(), changed.clone());
    vibe_safefs::fail_after_publish(None);
    assert!(matches!(result, Err(PolicyError::UncertainPublication(_))));
    let actual = attached(&root).current_under_lease(&lease).unwrap();
    assert_ne!(actual.generation(), first.generation());
    assert_eq!(actual.facts(), &changed);
    assert_eq!(
        owner.update_under_lease(&lease, first.generation(), source(), facts()),
        Err(PolicyError::StaleGeneration)
    );
}

#[test]
fn identities_are_bounded_and_scope_digest_is_length_delimited() {
    for invalid in [
        "".to_owned(),
        " ".into(),
        " source".into(),
        "source ".into(),
        "a\nb".into(),
        "x".repeat(257),
    ] {
        assert!(TrustedPolicySource::from_host_identifier(invalid).is_err());
    }
    assert!(TrustedPolicySource::from_host_identifier("é".repeat(128)).is_ok());
    assert!(TrustedPolicySource::from_host_identifier("é".repeat(129)).is_err());
    let a = SubmissionScope {
        context: "ab".into(),
        target: "c".into(),
        principal: "p".into(),
    };
    let b = SubmissionScope {
        context: "a".into(),
        target: "bc".into(),
        principal: "p".into(),
    };
    assert_ne!(record::path(&a), record::path(&b));
    let root = tempfile::tempdir().unwrap();
    let mut invalid = scope();
    invalid.context = "\n".into();
    assert!(
        PolicyOwner::attach_readonly(Arc::new(Project::open(root.path()).unwrap()), invalid)
            .is_err()
    );
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn generation_binds_every_durable_coordinate_without_local_issuer() {
    let (root, _lease, first) = initialized();
    let original: Record = serde_json::from_slice(&bytes(&root)).unwrap();
    assert_eq!(original.snapshot(), first);
    let mut changed = original.clone();
    changed.source = "other-source".into();
    assert_ne!(changed.snapshot().generation(), first.generation());
    changed = original.clone();
    let mut alternate = facts();
    alternate.allow_detach = true;
    changed.facts = record::Facts::from_host(alternate);
    assert_ne!(changed.snapshot().generation(), first.generation());
    changed = original.clone();
    changed.revision += 1;
    assert_ne!(changed.snapshot().generation(), first.generation());
    // Exact scope and root/owner coordinates also contribute to generation.
    let encoded = String::from_utf8(bytes(&root)).unwrap();
    for altered in [
        encoded.replace("\"context\":\"context\"", "\"context\":\"other-context\""),
        encoded.replace("\"target\":\"target\"", "\"target\":\"other-target\""),
        encoded.replace(
            "\"principal\":\"principal\"",
            "\"principal\":\"other-principal\"",
        ),
        encoded.replace("sha256:", "other-root:"),
    ] {
        let altered: Record = serde_json::from_str(&altered).unwrap();
        assert_ne!(altered.snapshot().generation(), first.generation());
    }
    let other_owner = Record::new(
        &scope(),
        &owner_token(&root),
        source(),
        facts(),
        root.path(),
    )
    .unwrap();
    assert_ne!(other_owner.snapshot().generation(), first.generation());
}
fn owner_token(root: &TempDir) -> String {
    Project::open(root.path())
        .unwrap()
        .root_correlation_token()
        .unwrap()
}

#[test]
fn scope_filename_is_a_portable_lowercase_hex_component() {
    let path = record::path(&scope());
    let component = path.strip_prefix(".vibe/machine-policy/").unwrap();
    let hex = component.strip_suffix(".json").unwrap();
    assert_eq!(hex.len(), 64);
    assert!(
        hex.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    );
    assert!(
        !component
            .chars()
            .any(|c| c == ':' || c == '/' || c == '\\' || c.is_control())
    );
}

#[test]
fn checkpoint_assessment_distinguishes_weak_support_from_uncertainty() {
    use std::io::ErrorKind;
    use vibe_safefs::{DirectorySync, Published};
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join(".vibe/machine-policy");
    let checkpoint = |kind| DirectorySync {
        directory: parent.clone(),
        durability: kind,
    };
    let mut write = DurableWrite {
        published: Published::default(),
        file_synced: true,
        parent: DirectoryDurability::Synced,
        directory_syncs: vec![checkpoint(DirectoryDurability::Synced)],
    };
    assert!(assess_checkpoints(&write, &parent).unwrap().is_empty());
    for kind in [
        DirectoryDurability::JournalRecoverable,
        DirectoryDurability::Unsupported(ErrorKind::Unsupported),
    ] {
        write.parent = kind;
        write.directory_syncs = vec![checkpoint(kind)];
        let warnings = assess_checkpoints(&write, &parent).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains(&format!("{kind:?}")));
        assert!(warnings[0].contains(&parent.display().to_string()));
    }
    let uncertain = |write: &DurableWrite| {
        assert!(matches!(
            assess_checkpoints(write, &parent),
            Err(PolicyError::UncertainPublication(_))
        ));
    };
    write.parent = DirectoryDurability::Failed(ErrorKind::Other);
    write.directory_syncs = vec![checkpoint(write.parent)];
    uncertain(&write);
    write.parent = DirectoryDurability::Synced;
    write.directory_syncs.clear();
    uncertain(&write);
    write.directory_syncs = vec![checkpoint(write.parent)];
    write.file_synced = false;
    uncertain(&write);
    write.file_synced = true;
    let created = parent.clone();
    write.published.created_directories.push(created.clone());
    uncertain(&write); // Missing the new directory entry's parent checkpoint.
    write.directory_syncs.insert(
        0,
        DirectorySync {
            directory: created.parent().unwrap().to_owned(),
            durability: DirectoryDurability::Failed(ErrorKind::PermissionDenied),
        },
    );
    uncertain(&write); // Earlier checkpoints matter even when the final one synced.
    write.directory_syncs[0].durability = DirectoryDurability::Unsupported(ErrorKind::Unsupported);
    assert_eq!(assess_checkpoints(&write, &parent).unwrap().len(), 1);
    write.directory_syncs[0].durability = DirectoryDurability::Synced;
    assert!(assess_checkpoints(&write, &parent).unwrap().is_empty());
    write.parent = DirectoryDurability::JournalRecoverable;
    uncertain(&write); // Summary and final checkpoint must agree.
}
