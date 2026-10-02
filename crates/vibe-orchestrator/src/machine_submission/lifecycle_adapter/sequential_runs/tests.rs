use super::*;
#[test]
fn unidentified_and_unreserved_headers_refuse_without_writes() {
    for previous in ["", "11111111111111111111111111111111"] {
        let dir = tempfile::tempdir().unwrap();
        let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
        let store = LifecycleStateStore::begin(
            lease.clone(),
            "validate".into(),
            vec!["validate".into()],
            "2026-10-02T00:00:00Z".into(),
            previous.into(),
            ".".into(),
            false,
        )
        .unwrap();
        drop(store);
        let path = dir.path().join(LifecycleStateStore::FILE);
        let before = std::fs::read(&path).unwrap();
        assert!(inspect_previous_terminal(&lease, "22222222222222222222222222222222").is_err());
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

#[test]
fn inverse_c1_correlation_is_readonly_and_never_promotes_pending() {
    let dir = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(dir.path()).unwrap();
    let scope = SubmissionScope {
        context: "test".into(),
        target: "root".into(),
        principal: "host".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
    let run = "11111111111111111111111111111111";
    SubmissionReservations::reserve(
        &lease, &namespace, &scope, "first", b"request", b"plan", run,
    )
    .unwrap();
    let index = dir.path().join(".vibe/machine-submissions.json");
    let before = std::fs::read(&index).unwrap();
    let pending = SubmissionReservations::inspect_existing_run(&lease, run)
        .unwrap()
        .unwrap();
    assert_eq!(pending.journal_admission, JournalAdmission::Pending);
    assert!(pending.reservation_ref.is_none());
    assert_eq!(std::fs::read(&index).unwrap(), before);
    SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope, "first", run).unwrap();
    let before = std::fs::read(&index).unwrap();
    let ready = SubmissionReservations::inspect_existing_run(&lease, run)
        .unwrap()
        .unwrap();
    assert_eq!(
        ready.reservation_ref.as_deref(),
        Some(
            SubmissionReservations::reservation_ref(&lease, &namespace, &scope, "first", run)
                .unwrap()
                .as_str()
        )
    );
    assert!(
        SubmissionReservations::inspect_existing_run(&lease, "22222222222222222222222222222222")
            .unwrap()
            .is_none()
    );
    assert_eq!(std::fs::read(index).unwrap(), before);
}
