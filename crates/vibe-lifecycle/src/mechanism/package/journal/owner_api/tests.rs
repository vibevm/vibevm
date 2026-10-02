use super::*;
use crate::SubmissionReservations;
use crate::mechanism::package::journal::{StaticFileJournalError, StaticFileJournalInspection};

fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "owner-api-context".into(),
        target: "owner-api-target".into(),
        principal: "owner-api-principal".into(),
    }
}

#[test]
fn fixed_owner_api_copies_once_replays_and_cold_attaches() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.bin"), b"fixed bytes").unwrap();
    let scope = scope();
    let (namespace_id, first) = {
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let first = submit_fixed_static_file_package(
            &lease,
            &namespace,
            &scope,
            "copy",
            "2026-10-03T00:00:00Z",
        )
        .unwrap();
        assert!(matches!(
            first,
            StaticFileJournalInspection::Completed { .. }
        ));
        let replay = submit_fixed_static_file_package(
            &lease,
            &namespace,
            &scope,
            "copy",
            "2026-10-03T00:00:00Z",
        )
        .unwrap();
        assert_eq!(replay, first);
        (namespace.id().to_owned(), first)
    };
    assert_eq!(
        std::fs::read(
            root.path()
                .join("target/vibe-package/launcher/launcher.bin")
        )
        .unwrap(),
        b"fixed bytes"
    );
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        inspect_fixed_static_file_package(&lease, &namespace_id, &scope, "copy").unwrap(),
        Some(first)
    );
}

#[test]
fn fixed_owner_api_source_conflict_and_foreign_scope_refuse_without_recopy() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.bin"), b"initial").unwrap();
    let scope = scope();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let first = submit_fixed_static_file_package(
        &lease,
        &namespace,
        &scope,
        "copy",
        "2026-10-03T00:00:00Z",
    )
    .unwrap();
    assert!(matches!(
        first,
        StaticFileJournalInspection::Completed { .. }
    ));
    assert!(
        submit_fixed_static_file_package(
            &lease,
            &namespace,
            &scope,
            "copy",
            "2026-10-04T00:00:00Z",
        )
        .is_err()
    );
    std::fs::write(root.path().join("source.bin"), b"changed").unwrap();
    assert!(
        submit_fixed_static_file_package(
            &lease,
            &namespace,
            &scope,
            "copy",
            "2026-10-03T00:00:00Z",
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(
            root.path()
                .join("target/vibe-package/launcher/launcher.bin")
        )
        .unwrap(),
        b"initial"
    );
    let foreign = SubmissionScope {
        principal: "other".into(),
        ..scope.clone()
    };
    assert!(matches!(
        inspect_fixed_static_file_package(&lease, namespace.id(), &foreign, "copy"),
        Err(StaticFileJournalError::Submission(
            crate::SubmissionError::ForeignScope
        ))
    ));
}

#[test]
fn fixed_owner_api_foreign_root_refuses_before_effect() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.bin"), b"source").unwrap();
    std::fs::write(other.path().join("source.bin"), b"other").unwrap();
    let scope = scope();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let other_lease = LifecycleLease::acquire(other.path()).unwrap();
    assert!(
        submit_fixed_static_file_package(
            &other_lease,
            &namespace,
            &scope,
            "copy",
            "2026-10-03T00:00:00Z",
        )
        .is_err()
    );
    assert!(
        !other
            .path()
            .join("target/vibe-package/launcher/launcher.bin")
            .exists()
    );
}
