use super::local_bare_tests::LocalBarePort;
use super::*;
use crate::package_git_provenance_populated::tests::limits as populated_limits;
use crate::package_git_repo_create::tests::{
    drive_fake_run as drive_create,
    profile_tests::{ProfileFake, input as create_input, linked},
};

pub(crate) fn push_input<'a>(
    world: &'a crate::package_git_provenance_populated::tests::World,
    home: &'a ApprovalHome,
    namespace: &'a SubmissionNamespace,
    approved: Option<&'a [crate::package_git_provenance_populated::ApprovedGitdir<'a>]>,
) -> ProfileRun<'a> {
    ProfileRun {
        home,
        lease: &world.lease,
        namespace,
        scope: &world.scope,
        key: "aggregate",
        package_source: &world.source,
        binary: &world.binary,
        profile: ReadyProfile::ParentlessPopulated {
            limits: populated_limits(),
            approved_gitdirs: approved,
        },
    }
}

pub(crate) fn created() -> Option<(
    crate::package_git_provenance_populated::tests::World,
    tempfile::TempDir,
    ApprovalHome,
    SubmissionNamespace,
    String,
)> {
    let (world, settings, home, namespace, id) = linked()?;
    let approved = world.held();
    let mut creator = ProfileFake::new(&world);
    let result = drive_create(
        &create_input(&world, &home, &namespace, Some(&approved)),
        &id,
        &mut creator,
    );
    assert!(
        matches!(
            result,
            CreateInspection::Landed {
                repository_id: 7,
                ..
            }
        ),
        "{result:?}"
    );
    assert_eq!(creator.calls(), 1);
    Some((world, settings, home, namespace, id))
}

#[test]
fn populated_create_then_local_atomic_push_lands_and_cold_attaches() {
    let Some((world, _settings, home, namespace, id)) = created() else {
        return;
    };
    let approved = world.held();
    let full = push_input(&world, &home, &namespace, Some(&approved));
    let partial = push_input(&world, &home, &namespace, Some(&approved[..1]));
    let mut local = LocalBarePort::new_profile(&full, &id);
    let refused = drive_fake_profile(&partial, &id, &mut local);
    assert!(
        matches!(refused, PushInspection::RecoveryRequired { .. }),
        "{refused:?}"
    );
    assert_eq!(local.calls(), 0);
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, PENDING)
            .unwrap()
            .is_none()
    );

    let landed = drive_fake_profile(&full, &id, &mut local);
    assert!(
        matches!(landed, PushInspection::Landed { .. }),
        "{landed:?}"
    );
    assert_eq!(local.calls(), 1);
    let cold = inspect_cold_profile(
        &home,
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        None,
        &world.binary,
        ReadyProfile::ParentlessPopulated {
            limits: populated_limits(),
            approved_gitdirs: None,
        },
        &id,
    );
    assert!(matches!(cold, PushInspection::Landed { .. }), "{cold:?}");
    assert!(matches!(
        drive_fake_profile(&full, &id, &mut local),
        PushInspection::Landed { .. }
    ));
    assert_eq!(local.calls(), 1);
}

pub(crate) struct LostReply(pub(crate) LocalBarePort);
impl FakePushPort for LostReply {
    fn observe(&mut self) -> Result<Snapshot, String> {
        self.0.observe()
    }
    fn atomic_push(&mut self, request: PushRequest<'_>) -> Result<PushReceipt, String> {
        let _actual = self.0.atomic_push(request)?;
        Err("injected lost reply after local atomic push".into())
    }
}

#[test]
fn populated_lost_atomic_reply_retains_pending_without_replay() {
    let Some((world, _settings, home, namespace, id)) = created() else {
        return;
    };
    let approved = world.held();
    let input = push_input(&world, &home, &namespace, Some(&approved));
    let mut local = LostReply(LocalBarePort::new_profile(&input, &id));
    let result = drive_fake_profile(&input, &id, &mut local);
    assert!(
        matches!(result, PushInspection::RecoveryRequired { .. }),
        "{result:?}"
    );
    assert_eq!(local.0.calls(), 1);
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, PENDING)
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, LANDED)
            .unwrap()
            .is_none()
    );
    let desired = proved_profile(
        &home,
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        Some(&world.source),
        &world.binary,
        input.profile,
        &id,
    )
    .unwrap()
    .desired_refs;
    assert_eq!(local.0.exact_refs(), desired);
    assert!(matches!(
        drive_fake_profile(&input, &id, &mut local),
        PushInspection::PendingRecoveryRequired { .. }
    ));
    assert_eq!(local.0.calls(), 1);
}

#[test]
fn populated_desired_oid_race_has_no_attributed_landed() {
    let Some((world, _settings, home, namespace, id)) = created() else {
        return;
    };
    let approved = world.held();
    let input = push_input(&world, &home, &namespace, Some(&approved));
    let mut local = LocalBarePort::new_profile(&input, &id);
    local.race_desired_main();
    let result = drive_fake_profile(&input, &id, &mut local);
    assert!(
        matches!(result, PushInspection::RecoveryRequired { .. }),
        "{result:?}"
    );
    assert_eq!(local.calls(), 1);
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, PENDING)
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, LANDED)
            .unwrap()
            .is_none()
    );
}
