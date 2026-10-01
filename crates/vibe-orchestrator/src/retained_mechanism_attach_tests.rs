//! Cross-crate host proof: cold inspection needs no ritual plan or provider.

use vibe_lifecycle::retained_mechanism::{
    BuildInspection, DeployColdHostFacts, DeployInspection, PackageInspection,
    ReadOnlyDeployAdmission, RetainedInspection, RetainedMechanismError,
    inspect_bound_vibe_bin_deploy, inspect_fixed_cargo_build, inspect_fixed_static_file_package,
};
use vibe_lifecycle::{
    LifecycleLease, ReservationDecision, SubmissionError, SubmissionReservations, SubmissionScope,
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

#[test]
fn host_can_reopen_lease_and_attach_pending_package_and_deploy_without_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let scope = SubmissionScope {
        context: "host-context".into(),
        target: "instance-a".into(),
        principal: "owner".into(),
    };
    let token = {
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        for (key, run_id) in [
            ("fixed-package", "run-package"),
            ("bound-deploy", "run-deploy"),
        ] {
            assert_eq!(
                SubmissionReservations::reserve(
                    &lease,
                    &namespace,
                    &scope,
                    key,
                    b"canonical request",
                    b"canonical plan",
                    run_id,
                )
                .unwrap(),
                ReservationDecision::Created {
                    domain_run_id: run_id.into()
                }
            );
        }
        namespace.id().to_owned()
    };
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let root_entries = || {
        let mut entries: Vec<_> = std::fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        entries.sort();
        entries
    };
    let before = root_entries();
    let admission = ReadOnlyDeployAdmission::reissue(DeployColdHostFacts {
        host_context_id: &scope.context,
        instance_ref: &scope.target,
        selected_generation_ref: Some("generation-a"),
        principal_ref: &scope.principal,
        admission_epoch: 1,
        binding_nonce: "nonce-a",
        allow_alias_resolution: false,
        target_abi: None,
        target_libc: None,
        project_root: root.path(),
        settings_root: settings.path(),
        user_home: home.path(),
    })
    .unwrap();

    // Reservation-only evidence requires recovery; it proves no completed effect.
    assert_eq!(
        inspect_fixed_static_file_package(&lease, &token, &scope, "fixed-package").unwrap(),
        PackageInspection::RecoveryRequired {
            run_id: "run-package".into()
        }
    );
    assert_eq!(
        inspect_bound_vibe_bin_deploy(&lease, &token, &scope, "bound-deploy", &admission).unwrap(),
        DeployInspection::RecoveryRequired {
            run_id: "run-deploy".into()
        }
    );
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..scope.clone()
    };
    assert!(matches!(
        inspect_fixed_static_file_package(&lease, &token, &foreign, "fixed-package"),
        Err(RetainedMechanismError::Submission(
            SubmissionError::ForeignScope
        ))
    ));
    assert!(matches!(
        inspect_bound_vibe_bin_deploy(&lease, &token, &foreign, "bound-deploy", &admission),
        Err(RetainedMechanismError::Refused(reason))
            if reason == "bound deploy bridge refused: cold deploy grant differs from trusted scope"
    ));
    assert_eq!(
        inspect_fixed_static_file_package(&lease, &token, &scope, "absent").unwrap(),
        PackageInspection::Absent
    );
    assert_eq!(
        inspect_bound_vibe_bin_deploy(&lease, &token, &scope, "absent", &admission).unwrap(),
        DeployInspection::Absent
    );
    assert_eq!(
        root_entries(),
        before,
        "inspection must not create output paths"
    );
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("Cargo.lock").exists());
    assert!(!settings.path().join("bin").exists());
    assert!(!settings.path().join("opt/bin").exists());
    assert!(!settings.path().join("store").exists());
    assert_eq!(std::fs::read_dir(settings.path()).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
}
