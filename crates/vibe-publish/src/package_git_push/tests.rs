use super::*;
use crate::package_git_approval::tests::{World, binary, limits};
use crate::package_git_repo_create::tests::{
    Fake as CreateFake, drive_fake as drive_create, linked_world,
};
use std::fs;

pub(crate) use super::driver::{
    AtomicPushPort as FakePushPort, PushPortError, PushRequest, Snapshot,
};

/// Held test-only inputs for the same owner driver on HostedCreate and DirectGitExisting.
pub(crate) struct FakeRun<'a> {
    pub(crate) home: &'a ApprovalHome,
    pub(crate) lease: &'a LifecycleLease,
    pub(crate) namespace: &'a SubmissionNamespace,
    pub(crate) scope: &'a SubmissionScope,
    pub(crate) key: &'a str,
    pub(crate) package_source: &'a Project,
    pub(crate) local_bare_source: Option<&'a Project>,
    pub(crate) binary: &'a Path,
    pub(crate) limits: ReadyLimits,
}

pub(crate) struct ProfileRun<'a> {
    pub(crate) home: &'a ApprovalHome,
    pub(crate) lease: &'a LifecycleLease,
    pub(crate) namespace: &'a SubmissionNamespace,
    pub(crate) scope: &'a SubmissionScope,
    pub(crate) key: &'a str,
    pub(crate) package_source: &'a Project,
    pub(crate) binary: &'a Path,
    pub(crate) profile: ReadyProfile<'a>,
}

/// Test-only transport. Production retains only cold inspection and no Git
/// or provider write port is reachable from a public call.
pub(crate) fn drive_fake(
    world: &World,
    approval_id: &str,
    port: &mut impl FakePushPort,
) -> PushInspection {
    drive_fake_run(
        &FakeRun {
            home: &world.home,
            lease: &world.lease,
            namespace: &world.namespace,
            scope: &world.scope,
            key: "approval",
            package_source: &world.source,
            local_bare_source: None,
            binary: &world.binary,
            limits: limits(),
        },
        approval_id,
        port,
    )
}

pub(crate) fn drive_fake_run(
    input: &FakeRun<'_>,
    approval_id: &str,
    port: &mut impl FakePushPort,
) -> PushInspection {
    drive_fake_profile(
        &ProfileRun {
            home: input.home,
            lease: input.lease,
            namespace: input.namespace,
            scope: input.scope,
            key: input.key,
            package_source: input.package_source,
            binary: input.binary,
            profile: ReadyProfile::NoGitlinks {
                limits: input.limits,
                local_bare_source: input.local_bare_source,
            },
        },
        approval_id,
        port,
    )
}

pub(crate) fn drive_fake_profile(
    input: &ProfileRun<'_>,
    approval_id: &str,
    port: &mut impl FakePushPort,
) -> PushInspection {
    super::driver::drive(
        &super::driver::PushRun {
            home: input.home,
            lease: input.lease,
            namespace: input.namespace,
            scope: input.scope,
            key: input.key,
            package_source: input.package_source,
            git_binary: input.binary,
            profile: input.profile,
            approval_id,
        },
        port,
    )
}

pub(crate) fn ready_world(binary: &Path) -> (World, String) {
    let (world, id) = linked_world(binary);
    let mut creator = CreateFake::new(&world);
    assert!(matches!(
        drive_create(&world, &id, &mut creator),
        CreateInspection::Landed {
            repository_id: 7,
            ..
        }
    ));
    (world, id)
}

pub(crate) struct Fake {
    observed: Snapshot,
    calls: usize,
    observations: usize,
    lose_reply: bool,
    replace_objects_on_push: bool,
    wrong_receipt: bool,
    third_state_before_pending: bool,
    third_state_after_pending: bool,
    third_state_after_push: bool,
    tamper_on_third_observation: Option<std::path::PathBuf>,
}
impl Fake {
    pub(crate) fn new(world: &World, id: &str) -> Self {
        let proof = proved(
            &world.home,
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            Some(&world.source),
            None,
            &world.binary,
            limits(),
            id,
        )
        .unwrap();
        Self {
            observed: Snapshot {
                repository: RepositoryObservation::Present(proof.repository),
                refs: proof.plan.before_refs,
            },
            calls: 0,
            observations: 0,
            lose_reply: false,
            replace_objects_on_push: false,
            wrong_receipt: false,
            third_state_before_pending: false,
            third_state_after_pending: false,
            third_state_after_push: false,
            tamper_on_third_observation: None,
        }
    }
    pub(crate) fn calls(&self) -> usize {
        self.calls
    }
    pub(crate) fn lose_next_reply(&mut self) {
        self.lose_reply = true;
    }
    pub(crate) fn drift_remote_refs(&mut self) {
        self.observed.refs.main = crate::package_git_plan::RawRef::Present {
            oid: "a".repeat(40),
        };
    }
}
impl FakePushPort for Fake {
    fn observe(
        &mut self,
        repository: &RepositoryIdentity,
        version_tag_ref: &str,
    ) -> Result<Snapshot, PushPortError> {
        (|| -> Result<Snapshot, String> {
            assert_eq!(
                self.observed.repository,
                RepositoryObservation::Present(repository.clone())
            );
            assert_eq!(version_tag_ref, "refs/tags/v1.1.0");
            self.observations += 1;
            if self.observations == 3
                && let Some(path) = &self.tamper_on_third_observation
            {
                fs::write(path, b"not-json").map_err(|e| e.to_string())?;
            }
            if (self.third_state_before_pending && self.observations == 2)
                || (self.third_state_after_pending && self.observations == 3)
                || (self.third_state_after_push && self.observations == 4)
            {
                let mut foreign = self.observed.clone();
                foreign.refs.main = crate::package_git_plan::RawRef::Present {
                    oid: "a".repeat(40),
                };
                return Ok(foreign);
            }
            Ok(self.observed.clone())
        })()
        .map_err(|_| PushPortError::Uncertain)
    }
    fn atomic_push(&mut self, request: PushRequest<'_>) -> Result<PushReceipt, PushPortError> {
        (|| -> Result<PushReceipt, String> {
            assert!(request.objects.root_dir().is_ok());
            self.calls += 1;
            assert_eq!(
                self.observed.repository,
                RepositoryObservation::Present(request.repository.clone())
            );
            assert_eq!(&self.observed.refs, request.before_refs);
            assert_eq!(request.version_tag_ref, "refs/tags/v1.1.0");
            assert_eq!(request.commit_oid.len(), 40);
            assert_eq!(request.tag_oid.len(), 40);
            self.observed.refs = request.desired_refs.clone();
            if self.replace_objects_on_push {
                let root = request.objects.root_path();
                fs::rename(root, root.with_extension("displaced")).map_err(|e| e.to_string())?;
                fs::create_dir(root).map_err(|e| e.to_string())?;
            }
            if self.lose_reply {
                Err("injected response loss".into())
            } else {
                Ok(PushReceipt {
                    main_update: if self.wrong_receipt {
                        RefUpdateKind::FastForward
                    } else {
                        RefUpdateKind::New
                    },
                    tag_update: RefUpdateKind::New,
                })
            }
        })()
        .map_err(|_| PushPortError::Uncertain)
    }
}

#[test]
fn exact_fake_atomic_push_lands_once_and_cold_attaches() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut fake = Fake::new(&world, &id);
    let status = drive_fake(&world, &id, &mut fake);
    assert!(
        matches!(status, PushInspection::Landed { .. }),
        "{status:?}"
    );
    assert_eq!(fake.calls, 1);
    assert_eq!(fake.observations, 5);
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
        PushInspection::Landed { .. }
    ));
    assert!(matches!(
        drive_fake(&world, &id, &mut fake),
        PushInspection::Landed { .. }
    ));
    assert_eq!(fake.calls, 1);
}

#[test]
fn lost_callback_with_desired_refs_keeps_pending_and_never_replays() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut fake = Fake::new(&world, &id);
    fake.lose_reply = true;
    assert!(matches!(
        drive_fake(&world, &id, &mut fake),
        PushInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 1);
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
    assert!(matches!(
        drive_fake(&world, &id, &mut fake),
        PushInspection::PendingRecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 1);
}

#[test]
fn third_state_before_or_after_pending_or_push_never_lands() {
    let Some(binary) = binary() else { return };
    for phase in 0..3 {
        let (world, id) = ready_world(&binary);
        let mut fake = Fake::new(&world, &id);
        fake.third_state_before_pending = phase == 0;
        fake.third_state_after_pending = phase == 1;
        fake.third_state_after_push = phase == 2;
        assert!(matches!(
            drive_fake(&world, &id, &mut fake),
            PushInspection::RecoveryRequired { .. }
        ));
        assert_eq!(fake.calls, if phase == 2 { 1 } else { 0 });
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, LANDED)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn local_authority_drift_after_pending_prevents_push_dispatch() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut fake = Fake::new(&world, &id);
    fake.tamper_on_third_observation = Some(world.root.path().join(format!(
        ".vibe/package-git-operations/{}.package-git-repo-create.json",
        world.run_id
    )));
    assert!(matches!(
        drive_fake(&world, &id, &mut fake),
        PushInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 0);
    assert_eq!(fake.observations, 3);
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

#[test]
fn replaced_retained_object_root_during_callback_never_lands_or_replays() {
    let Some(binary) = binary() else { return };
    for lose_reply in [false, true] {
        let (world, id) = ready_world(&binary);
        let mut fake = Fake::new(&world, &id);
        fake.replace_objects_on_push = true;
        fake.lose_reply = lose_reply;
        let result = drive_fake(&world, &id, &mut fake);
        assert!(
            matches!(result, PushInspection::RecoveryRequired { ref reason, .. }
            if reason == "retained object root changed during push"),
            "{result:?}"
        );
        assert_eq!(fake.calls, 1);
        assert_eq!(fake.observations, 3);
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
        assert!(matches!(
            drive_fake(&world, &id, &mut fake),
            PushInspection::RecoveryRequired { .. }
        ));
        assert_eq!(fake.calls, 1);
    }
}

#[test]
fn incorrect_actual_update_kind_with_desired_refs_never_lands() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut fake = Fake::new(&world, &id);
    fake.wrong_receipt = true;
    assert!(matches!(
        drive_fake(&world, &id, &mut fake),
        PushInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 1);
    assert_eq!(fake.observations, 3);
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
    assert!(matches!(
        drive_fake(&world, &id, &mut fake),
        PushInspection::PendingRecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 1);
}

#[path = "local_bare_tests.rs"]
pub(crate) mod local_bare_tests;

#[path = "profile_tests.rs"]
pub(crate) mod profile_tests;
