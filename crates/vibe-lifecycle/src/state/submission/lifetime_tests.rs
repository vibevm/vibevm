//! C.1-only cold lifetime evidence. The disposable markers below prove the
//! Ready handshake; they are not Build, Package, Deploy or Publish journals.

use super::*;

struct Retained {
    token: String,
    scope: SubmissionScope,
    request: Vec<u8>,
    plan: Vec<u8>,
    run_id: String,
    reservation_ref: String,
}

#[test]
fn four_domain_coordinates_survive_lease_exit_and_reopen_without_effect_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let mut retained = Vec::new();
    for (domain, principal) in [
        ("build", "builder"),
        ("package", "packager"),
        ("deploy", "deployer"),
        ("publication", "publisher"),
    ] {
        let scope = SubmissionScope {
            context: "cold-owner-context".into(),
            target: format!("fixture-{domain}-instance"),
            principal: principal.into(),
        };
        let namespace = SubmissionReservations::issue_at(&lease, &scope, 10, 1_000).unwrap();
        let run_id = format!("fixture-run-{domain}");
        let request = format!("{domain}-request-v1").into_bytes();
        let plan = format!("{domain}-plan-v1").into_bytes();
        assert_eq!(
            SubmissionReservations::reserve_at(
                &lease, &namespace, &scope, 11, "same-key", &request, &plan, &run_id,
            )
            .unwrap(),
            ReservationDecision::Created {
                domain_run_id: run_id.clone()
            },
        );

        // A C.1 Ready bit is set only after a domain owner proves its own
        // durable intent. This tiny test marker proves that handshake only;
        // no provider, process, destination or remote adapter is called.
        let home = lease
            .project()
            .ensure_subproject(&[".vibe", "c1-lifetime-fixture"])
            .unwrap();
        let dir = home.root_dir().unwrap();
        let marker = format!("{run_id}.intent.json");
        let bytes = format!("{{\"run_id\":\"{run_id}\",\"domain\":\"{domain}\"}}").into_bytes();
        home.publish_new_in(&dir, &marker, &bytes).unwrap();
        assert_eq!(
            home.read_file_bounded_in(&dir, &marker, 256).unwrap(),
            Some(bytes)
        );
        SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope, "same-key", &run_id)
            .unwrap();
        assert_eq!(
            SubmissionReservations::journal_admission(
                &lease, &namespace, &scope, "same-key", &run_id
            )
            .unwrap(),
            JournalAdmission::Ready
        );
        let reservation_ref = SubmissionReservations::reservation_ref(
            &lease, &namespace, &scope, "same-key", &run_id,
        )
        .unwrap();
        retained.push(Retained {
            token: namespace.id().into(),
            scope,
            request,
            plan,
            run_id,
            reservation_ref,
        });
    }
    let index_before = std::fs::read(root.path().join(FILE)).unwrap();
    drop(lease);

    // Reopen only the durable root and host-retained opaque namespace tokens.
    // The original namespace values and in-process lock owner are gone.
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    for saved in &retained {
        let namespace = SubmissionNamespace::from_host_token(saved.token.clone()).unwrap();
        let inspection = SubmissionReservations::inspect_existing_request_at(
            &lease,
            &namespace,
            &saved.scope,
            12,
            "same-key",
            &saved.request,
        )
        .unwrap();
        assert_eq!(
            inspection,
            ExistingSubmissionInspection::Reserved {
                domain_run_id: saved.run_id.clone(),
                journal_admission: JournalAdmission::Ready,
                reservation_ref: Some(saved.reservation_ref.clone()),
                retired: false,
            }
        );
        assert_eq!(
            SubmissionReservations::inspect_existing_key_at(
                &lease,
                &namespace,
                &saved.scope,
                12,
                "same-key"
            )
            .unwrap(),
            Some(ExistingKeyInspection {
                domain_run_id: saved.run_id.clone(),
                request_sha256: format!("sha256:{}", digest(&saved.request)),
                plan_sha256: format!("sha256:{}", digest(&saved.plan)),
                journal_admission: JournalAdmission::Ready,
                reservation_ref: Some(saved.reservation_ref.clone()),
                retired: false,
            })
        );
        assert!(
            SubmissionReservations::inspect_existing_key_at(
                &lease,
                &namespace,
                &saved.scope,
                12,
                "missing-key"
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            SubmissionReservations::inspect_reserved_plan_sha256(
                &lease,
                &namespace,
                &saved.scope,
                "same-key",
                &saved.run_id
            )
            .unwrap(),
            format!("sha256:{}", digest(&saved.plan))
        );
        assert_eq!(
            SubmissionReservations::reservation_ref(
                &lease,
                &namespace,
                &saved.scope,
                "same-key",
                &saved.run_id
            )
            .unwrap(),
            saved.reservation_ref
        );

        let mut foreign_principal = saved.scope.clone();
        foreign_principal.principal.push_str("-foreign");
        assert!(matches!(
            SubmissionReservations::inspect_existing_request_at(
                &lease,
                &namespace,
                &foreign_principal,
                12,
                "same-key",
                &saved.request
            ),
            Err(SubmissionError::ForeignScope)
        ));
        assert!(matches!(
            SubmissionReservations::inspect_existing_key_at(
                &lease,
                &namespace,
                &foreign_principal,
                12,
                "same-key"
            ),
            Err(SubmissionError::ForeignScope)
        ));
        let mut foreign_instance = saved.scope.clone();
        foreign_instance.target.push_str("-foreign");
        assert!(matches!(
            SubmissionReservations::inspect_existing_request_at(
                &lease,
                &namespace,
                &foreign_instance,
                12,
                "same-key",
                &saved.request
            ),
            Err(SubmissionError::ForeignScope)
        ));
        assert!(matches!(
            SubmissionReservations::inspect_existing_request_at(
                &lease,
                &namespace,
                &saved.scope,
                12,
                "same-key",
                b"changed-request"
            ),
            Err(SubmissionError::KeyConflict)
        ));
    }
    assert_eq!(
        std::fs::read(root.path().join(FILE)).unwrap(),
        index_before,
        "cold inspection must not rewrite the C.1 index"
    );
}
