use super::*;

#[test]
fn cold_static_file_attach_recovers_completed_without_execution_or_recopy() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"exact source").unwrap();
    let trusted = scope();
    let copies = AtomicUsize::new(0);
    let (token, original) = {
        let world = empty_world();
        let registry = registry(&world);
        let routes = MechanismRoutes::default();
        let targets = [target()];
        let selected = execution(root.path(), &targets, &registry, &routes);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let counted = || {
            copies.fetch_add(1, Ordering::SeqCst);
        };
        let result = execute_fixed_static_file_with_hook(
            &lease,
            &namespace,
            &trusted,
            "cold-copy",
            &selected,
            &counted,
        )
        .unwrap();
        assert!(matches!(
            result,
            StaticFileJournalInspection::Completed { .. }
        ));
        (namespace.id().to_owned(), result)
    };
    assert_eq!(copies.load(Ordering::SeqCst), 1);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let cold = inspect_fixed_static_file_cold(&lease, &token, &trusted, "cold-copy")
        .unwrap()
        .unwrap();
    assert_eq!(cold, original);
    assert_eq!(copies.load(Ordering::SeqCst), 1);
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..trusted.clone()
    };
    assert!(matches!(
        inspect_fixed_static_file_cold(&lease, &token, &foreign, "cold-copy"),
        Err(StaticFileJournalError::Submission(
            crate::SubmissionError::ForeignScope
        ))
    ));
    let StaticFileJournalInspection::Completed { run_id, .. } = original else {
        unreachable!()
    };
    let capsule = root
        .path()
        .join(".vibe/package-operations")
        .join(format!("{run_id}.static-file-input.json"));
    let original_capsule = std::fs::read(&capsule).unwrap();
    std::fs::write(&capsule, b"{\"tampered\":true}").unwrap();
    let malformed = inspect_fixed_static_file_cold(&lease, &token, &trusted, "cold-copy")
        .unwrap()
        .unwrap();
    assert!(
        matches!(malformed, StaticFileJournalInspection::Uncertain { run_id: same, partial }
        if same == run_id && matches!(partial.artifact_record, ObservedFile::Present { .. }))
    );
    let mut changed: serde_json::Value = serde_json::from_slice(&original_capsule).unwrap();
    changed["request"] = serde_json::json!("{}");
    std::fs::write(&capsule, serde_json::to_vec(&changed).unwrap()).unwrap();
    let tampered = inspect_fixed_static_file_cold(&lease, &token, &trusted, "cold-copy")
        .unwrap()
        .unwrap();
    assert!(
        matches!(tampered, StaticFileJournalInspection::Uncertain { run_id: same, .. }
        if same == run_id)
    );
    std::fs::remove_file(&capsule).unwrap();
    let missing = inspect_fixed_static_file_cold(&lease, &token, &trusted, "cold-copy")
        .unwrap()
        .unwrap();
    assert!(
        matches!(missing, StaticFileJournalInspection::Uncertain { run_id: same, .. }
        if same == run_id)
    );
    assert_eq!(copies.load(Ordering::SeqCst), 1);
}

#[test]
fn cold_static_file_attach_retains_ready_without_result_as_uncertain() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source").unwrap();
    let trusted = scope();
    let (token, run_id) = {
        let world = empty_world();
        let registry = registry(&world);
        let routes = MechanismRoutes::default();
        let targets = [target()];
        let selected = execution(root.path(), &targets, &registry, &routes);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let first = reserve_fixed_static_file_intent(
            &lease,
            &namespace,
            &trusted,
            "cold-pending",
            &selected,
        )
        .unwrap();
        let StaticFileJournalInspection::IntentReady { run_id } = first else {
            panic!("fixed intent must be ready")
        };
        (namespace.id().to_owned(), run_id)
    };
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let cold = inspect_fixed_static_file_cold(&lease, &token, &trusted, "cold-pending")
        .unwrap()
        .unwrap();
    let StaticFileJournalInspection::Uncertain {
        run_id: seen,
        partial,
    } = cold
    else {
        panic!("ready intent without result must remain uncertain")
    };
    assert_eq!(seen, run_id);
    assert_eq!(partial.output_file, ObservedFile::Absent);
    assert_eq!(partial.artifact_record, ObservedFile::Absent);
    assert!(!root.path().join(OUTPUT_DIR).exists());
}

#[test]
fn warm_static_file_replay_refuses_changed_capsule_without_copy() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let trusted = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
    let StaticFileJournalInspection::IntentReady { run_id } =
        reserve_fixed_static_file_intent(&lease, &namespace, &trusted, "capsule-drift", &selected)
            .unwrap()
    else {
        panic!("fixed intent must be ready")
    };
    let capsule = root
        .path()
        .join(".vibe/package-operations")
        .join(format!("{run_id}.static-file-input.json"));
    std::fs::write(capsule, b"{\"tampered\":true}").unwrap();
    let seen =
        reserve_fixed_static_file_intent(&lease, &namespace, &trusted, "capsule-drift", &selected)
            .unwrap();
    assert!(matches!(
        seen,
        StaticFileJournalInspection::Uncertain { .. }
    ));
    assert!(!root.path().join(OUTPUT_DIR).exists());
}

#[test]
fn copied_package_c1_and_capsule_at_replaced_same_path_refuse_cold_attach() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source").unwrap();
    let trusted = scope();
    let (token, run_id) = {
        let world = empty_world();
        let registry = registry(&world);
        let routes = MechanismRoutes::default();
        let targets = [target()];
        let selected = execution(root.path(), &targets, &registry, &routes);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let StaticFileJournalInspection::IntentReady { run_id } =
            reserve_fixed_static_file_intent(&lease, &namespace, &trusted, "copied", &selected)
                .unwrap()
        else {
            panic!("fixed intent must be Ready")
        };
        (namespace.id().to_owned(), run_id)
    };
    let current = root.path().to_path_buf();
    let archived = current.with_extension("cold-old");
    std::fs::rename(&current, &archived).unwrap();
    std::fs::create_dir_all(current.join(".vibe/package-operations")).unwrap();
    std::fs::copy(
        archived.join(".vibe/machine-submissions.json"),
        current.join(".vibe/machine-submissions.json"),
    )
    .unwrap();
    for name in [
        format!("{run_id}.static-file-intent.json"),
        format!("{run_id}.static-file-input.json"),
    ] {
        std::fs::copy(
            archived.join(".vibe/package-operations").join(&name),
            current.join(".vibe/package-operations").join(name),
        )
        .unwrap();
    }
    {
        let lease = LifecycleLease::acquire(&current).unwrap();
        assert!(matches!(
            inspect_fixed_static_file_cold(&lease, &token, &trusted, "copied").unwrap(),
            Some(StaticFileJournalInspection::Uncertain { run_id: seen, .. }) if seen == run_id
        ));
        assert!(!current.join(OUTPUT_DIR).exists());
    }
    std::fs::remove_dir_all(&current).unwrap();
    std::fs::rename(archived, current).unwrap();
}
