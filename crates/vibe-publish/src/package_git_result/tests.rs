use std::fs;

use super::*;
use crate::package_git_approval::tests::{binary, limits};
use crate::package_git_push::tests::{Fake, drive_fake, ready_world};

#[test]
fn complete_fake_chain_seals_once_and_cold_attaches_after_eof_and_remote_drift() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut remote = Fake::new(&world, &id);
    assert!(matches!(
        drive_fake(&world, &id, &mut remote),
        PushInspection::Landed { .. }
    ));
    assert_eq!(remote.calls(), 1);
    assert!(matches!(
        inspect_cold(
            &world.home,
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            None,
            None,
            &binary,
            limits(),
            &id
        ),
        ResultInspection::ReadyToSeal { .. }
    ));
    let result = seal_result(
        &world.home,
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        None,
        None,
        &binary,
        limits(),
        &id,
    );
    let ResultInspection::Completed {
        run_id,
        result_sha256,
    } = result
    else {
        panic!("result not completed: {result:?}")
    };
    assert_eq!(run_id, world.run_id);
    assert_eq!(
        sha(&journal::read_bytes(&world.lease, &run_id, RESULT)
            .unwrap()
            .unwrap()),
        result_sha256
    );
    remote.drift_remote_refs();
    assert_eq!(remote.calls(), 1);
    let lease = world.lease;
    drop(lease);
    let reopened = LifecycleLease::acquire(world.root.path()).unwrap();
    assert!(
        matches!(inspect_cold(&world.home, &reopened, &world.namespace, &world.scope,
        "approval", None, None, &binary, limits(), &id), ResultInspection::Completed { run_id: same, .. } if same == run_id)
    );
    assert!(matches!(
        seal_result(
            &world.home,
            &reopened,
            &world.namespace,
            &world.scope,
            "approval",
            None,
            None,
            &binary,
            limits(),
            &id
        ),
        ResultInspection::Completed { .. }
    ));
}

#[test]
fn push_pending_lost_reply_is_retained_partial_not_completed() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut remote = Fake::new(&world, &id);
    remote.lose_next_reply();
    assert!(matches!(
        drive_fake(&world, &id, &mut remote),
        PushInspection::RecoveryRequired { .. }
    ));
    let result = inspect_cold(
        &world.home,
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        None,
        None,
        &binary,
        limits(),
        &id,
    );
    let ResultInspection::PartialRecovery {
        run_id,
        push_pending_sha256,
    } = result
    else {
        panic!("lost push not partial: {result:?}")
    };
    assert_eq!(run_id, world.run_id);
    assert_eq!(
        sha(
            &journal::read_bytes(&world.lease, &run_id, "atomic-push-pending")
                .unwrap()
                .unwrap()
        ),
        push_pending_sha256
    );
    assert!(
        journal::read_bytes(&world.lease, &run_id, RESULT)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        seal_result(
            &world.home,
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            None,
            None,
            &binary,
            limits(),
            &id
        ),
        ResultInspection::PartialRecovery { .. }
    ));
    remote.drift_remote_refs();
    assert_eq!(remote.calls(), 1);
}

#[test]
fn lost_result_reply_cold_recovers_without_second_publication() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut remote = Fake::new(&world, &id);
    assert!(matches!(
        drive_fake(&world, &id, &mut remote),
        PushInspection::Landed { .. }
    ));
    assert!(matches!(
        seal_inner(
            &world.home,
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            None,
            None,
            &binary,
            limits(),
            &id,
            true
        ),
        ResultInspection::RecoveryRequired { .. }
    ));
    let retained = journal::read_bytes(&world.lease, &world.run_id, RESULT)
        .unwrap()
        .unwrap();
    assert!(matches!(
        inspect_cold(
            &world.home,
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            None,
            None,
            &binary,
            limits(),
            &id
        ),
        ResultInspection::Completed { .. }
    ));
    assert!(matches!(
        seal_result(
            &world.home,
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            None,
            None,
            &binary,
            limits(),
            &id
        ),
        ResultInspection::Completed { .. }
    ));
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, RESULT)
            .unwrap()
            .unwrap(),
        retained
    );
    assert_eq!(remote.calls(), 1);
}

#[test]
fn tampered_result_or_push_landed_refuses_historical_completion() {
    let Some(binary) = binary() else { return };
    for suffix in [RESULT, "atomic-push"] {
        let (world, id) = ready_world(&binary);
        let mut remote = Fake::new(&world, &id);
        assert!(matches!(
            drive_fake(&world, &id, &mut remote),
            PushInspection::Landed { .. }
        ));
        assert!(matches!(
            seal_result(
                &world.home,
                &world.lease,
                &world.namespace,
                &world.scope,
                "approval",
                None,
                None,
                &binary,
                limits(),
                &id
            ),
            ResultInspection::Completed { .. }
        ));
        fs::write(
            world.root.path().join(format!(
                ".vibe/package-git-operations/{}.package-git-{suffix}.json",
                world.run_id
            )),
            b"not-json",
        )
        .unwrap();
        assert!(matches!(
            inspect_cold(
                &world.home,
                &world.lease,
                &world.namespace,
                &world.scope,
                "approval",
                None,
                None,
                &binary,
                limits(),
                &id
            ),
            ResultInspection::RecoveryRequired { .. }
        ));
    }
}

#[path = "direct_tests.rs"]
mod direct_tests;

#[path = "profile_tests.rs"]
mod profile_tests;
