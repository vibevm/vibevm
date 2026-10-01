//! Cross-crate host proof: cold inspection needs no ritual plan or provider.

use vibe_lifecycle::retained_mechanism::{
    BuildInspection, RetainedInspection, inspect_fixed_cargo_build,
};
use vibe_lifecycle::{
    LifecycleLease, ReservationDecision, SubmissionReservations, SubmissionScope,
};

#[test]
fn host_can_reopen_lease_and_attach_pending_fixed_build_without_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let scope = SubmissionScope {
        context: "host-context".into(),
        target: "root".into(),
        principal: "owner".into(),
    };
    let token = {
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        assert!(matches!(
            SubmissionReservations::reserve(
                &lease,
                &namespace,
                &scope,
                "fixed-build",
                b"canonical request",
                b"canonical plan",
                "run-a",
            )
            .unwrap(),
            ReservationDecision::Created { .. }
        ));
        namespace.id().to_owned()
    };
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        inspect_fixed_cargo_build(&lease, &token, &scope, "fixed-build").unwrap(),
        BuildInspection::RecoveryRequired {
            run_id: "run-a".into()
        }
    );
    assert_eq!(
        inspect_fixed_cargo_build(&lease, &token, &scope, "absent").unwrap(),
        RetainedInspection::Absent
    );
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("Cargo.lock").exists());
}
