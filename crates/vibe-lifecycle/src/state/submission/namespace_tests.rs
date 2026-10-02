//! Read-only namespace validation, including the missing-pointer issue fence.
use super::*;

fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "stable-host".into(),
        target: "project".into(),
        principal: "fixture".into(),
    }
}

#[test]
fn namespace_metadata_and_issue_fence_survive_reopen_without_writes() {
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    assert!(!SubmissionReservations::namespace_exists_for_scope(&lease, &scope).unwrap());
    let token = SubmissionReservations::issue_at(&lease, &scope, 100, 200).unwrap();
    let before = std::fs::read(root.path().join(FILE)).unwrap();
    drop(lease);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let token = SubmissionNamespace::from_host_token(token.id().into()).unwrap();
    assert!(SubmissionReservations::namespace_exists_for_scope(&lease, &scope).unwrap());
    assert_eq!(
        SubmissionReservations::validate_namespace_at(&lease, &token, &scope, 101).unwrap(),
        (300, false)
    );
    assert_eq!(
        SubmissionReservations::validate_namespace_at(&lease, &token, &scope, 300).unwrap(),
        (300, true)
    );
    assert_eq!(std::fs::read(root.path().join(FILE)).unwrap(), before);
}

#[test]
fn namespace_validation_refuses_unissued_foreign_and_clock_rollback() {
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let token = SubmissionReservations::issue_at(&lease, &scope, 100, 200).unwrap();
    let forged =
        SubmissionNamespace::from_host_token(format!("{:016x}-{}", 99, "0".repeat(32))).unwrap();
    assert!(matches!(
        SubmissionReservations::validate_namespace_at(&lease, &forged, &scope, 101),
        Err(SubmissionError::UnknownNamespace)
    ));
    for field in ["context", "target", "principal"] {
        let mut foreign = scope.clone();
        match field {
            "context" => foreign.context.push('x'),
            "target" => foreign.target.push('x'),
            _ => foreign.principal.push('x'),
        }
        assert!(matches!(
            SubmissionReservations::validate_namespace_at(&lease, &token, &foreign, 101),
            Err(SubmissionError::ForeignScope)
        ));
        assert!(!SubmissionReservations::namespace_exists_for_scope(&lease, &foreign).unwrap());
    }
    assert!(matches!(
        SubmissionReservations::validate_namespace_at(&lease, &token, &scope, 99),
        Err(SubmissionError::InvalidInput(_))
    ));
}

#[test]
fn retired_namespace_metadata_retains_reserved_key_and_never_renews() {
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let token = SubmissionReservations::issue_at(&lease, &scope, 100, 200).unwrap();
    SubmissionReservations::reserve_at(
        &lease, &token, &scope, 101, "key", b"request", b"plan", "run",
    )
    .unwrap();
    SubmissionReservations::retire_namespace(&lease, &token, &scope).unwrap();
    assert_eq!(
        SubmissionReservations::validate_namespace_at(&lease, &token, &scope, 301).unwrap(),
        (300, true)
    );
    assert!(SubmissionReservations::namespace_exists_for_scope(&lease, &scope).unwrap());
    assert_eq!(
        SubmissionReservations::inspect_existing_key_at(&lease, &token, &scope, 301, "key")
            .unwrap()
            .unwrap()
            .domain_run_id,
        "run"
    );
}

#[test]
fn malformed_index_and_invalid_scope_do_not_report_first_issue() {
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    std::fs::write(root.path().join(FILE), b"{}").unwrap();
    assert!(matches!(
        SubmissionReservations::namespace_exists_for_scope(&lease, &scope()),
        Err(SubmissionError::InvalidIndex(_))
    ));
    let mut invalid = scope();
    invalid.context.clear();
    assert!(matches!(
        SubmissionReservations::namespace_exists_for_scope(&lease, &invalid),
        Err(SubmissionError::InvalidInput(_))
    ));
}
