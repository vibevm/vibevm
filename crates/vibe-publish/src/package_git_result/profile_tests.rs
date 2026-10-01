use super::*;
use crate::package_git_provenance_populated::tests::limits as populated_limits;
use crate::package_git_push::tests::{
    drive_fake_profile,
    local_bare_tests::LocalBarePort,
    profile_tests::{LostReply, created, push_input},
};
use crate::package_git_ready::profile::ReadyProfile;

fn historical() -> ReadyProfile<'static> {
    ReadyProfile::ParentlessPopulated {
        limits: populated_limits(),
        approved_gitdirs: None,
    }
}

#[test]
fn populated_create_push_terminal_lost_result_reply_cold_recovers_after_eof_and_drift() {
    let Some((world, _settings, home, namespace, id)) = created() else {
        return;
    };
    let approved = world.held();
    let input = push_input(&world, &home, &namespace, Some(&approved));
    let mut local = LocalBarePort::new_profile(&input, &id);
    let push = drive_fake_profile(&input, &id, &mut local);
    assert!(matches!(push, PushInspection::Landed { .. }), "{push:?}");
    assert_eq!(local.calls(), 1);
    let ready = inspect_cold_profile(
        &home,
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        None,
        &world.binary,
        historical(),
        &id,
    );
    assert!(
        matches!(ready, ResultInspection::ReadyToSeal { .. }),
        "{ready:?}"
    );
    let lost_reply = seal_inner_profile(
        &home,
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        None,
        &world.binary,
        historical(),
        &id,
        true,
    );
    assert!(
        matches!(lost_reply, ResultInspection::RecoveryRequired { .. }),
        "{lost_reply:?}"
    );
    let retained = journal::read_bytes(&world.lease, &world.run_id, RESULT)
        .unwrap()
        .unwrap();
    assert!(matches!(
        inspect_cold_profile(
            &home,
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            &world.binary,
            historical(),
            &id,
        ),
        ResultInspection::Completed { .. }
    ));
    local.drift_tag_after_push();
    drop(local);
    drop(input);
    drop(approved);
    let lease = world.lease;
    drop(lease);
    let reopened = LifecycleLease::acquire(world.root.path()).unwrap();
    let cold = inspect_cold_profile(
        &home,
        &reopened,
        &namespace,
        &world.scope,
        "aggregate",
        None,
        &world.binary,
        historical(),
        &id,
    );
    assert!(
        matches!(cold, ResultInspection::Completed { .. }),
        "{cold:?}"
    );
    assert_eq!(
        journal::read_bytes(&reopened, &world.run_id, RESULT)
            .unwrap()
            .unwrap(),
        retained
    );
}

#[test]
fn populated_lost_push_reply_is_historical_partial_never_completed() {
    let Some((world, _settings, home, namespace, id)) = created() else {
        return;
    };
    let approved = world.held();
    let input = push_input(&world, &home, &namespace, Some(&approved));
    let mut local = LostReply(LocalBarePort::new_profile(&input, &id));
    let push = drive_fake_profile(&input, &id, &mut local);
    assert!(
        matches!(push, PushInspection::RecoveryRequired { .. }),
        "{push:?}"
    );
    assert_eq!(local.0.calls(), 1);
    let partial = inspect_cold_profile(
        &home,
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        None,
        &world.binary,
        historical(),
        &id,
    );
    let ResultInspection::PartialRecovery {
        run_id,
        push_pending_sha256,
    } = partial
    else {
        panic!("lost push was not partial: {partial:?}");
    };
    assert_eq!(run_id, world.run_id);
    assert_eq!(
        push_pending_sha256,
        sha(
            &journal::read_bytes(&world.lease, &run_id, "atomic-push-pending")
                .unwrap()
                .unwrap()
        )
    );
    assert!(
        journal::read_bytes(&world.lease, &run_id, RESULT)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        seal_result_profile(
            &home,
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            &world.binary,
            historical(),
            &id,
        ),
        ResultInspection::PartialRecovery { .. }
    ));
    assert_eq!(local.0.calls(), 1);
}
