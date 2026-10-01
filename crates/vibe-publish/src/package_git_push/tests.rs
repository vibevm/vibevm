use super::*;
use crate::package_git_approval::tests::{World, binary, limits};
use crate::package_git_repo_create::tests::{
    Fake as CreateFake, drive_fake as drive_create, linked_world,
};
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) repository: RepositoryObservation,
    pub(crate) refs: RefSnapshot,
}

#[derive(Debug)]
pub(crate) struct PushRequest<'a> {
    pub(crate) repository: &'a RepositoryIdentity,
    pub(crate) before_refs: &'a RefSnapshot,
    pub(crate) desired_refs: &'a RefSnapshot,
    pub(crate) version_tag_ref: &'a str,
    pub(crate) commit_oid: &'a str,
    pub(crate) tag_oid: &'a str,
}

pub(crate) trait FakePushPort {
    fn observe(&mut self) -> Result<Snapshot, String>;
    fn atomic_push(&mut self, request: PushRequest<'_>) -> Result<PushReceipt, String>;
}

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
    let proof = match proved(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        Some(input.package_source),
        input.local_bare_source,
        input.binary,
        input.limits,
        approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run = proof.run();
    match inspect_cold(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        Some(input.package_source),
        input.local_bare_source,
        input.binary,
        input.limits,
        approval_id,
    ) {
        PushInspection::NotStarted { .. } => {}
        other => return other,
    }
    let baseline = Snapshot {
        repository: RepositoryObservation::Present(proof.repository.clone()),
        refs: proof.plan.before_refs.clone(),
    };
    let first = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("first pre-observation: {e}")),
    };
    let second = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("second pre-observation: {e}")),
    };
    if first != baseline || second != baseline {
        return recovery(
            Some(run),
            "raw repository/main/tag preconditions differ before Pending",
        );
    }
    let pending = expected_pending(&proof);
    let pending_bytes = match journal::publish(input.lease, run, PENDING, &pending) {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("atomic-push Pending uncertain: {e:#}")),
    };
    if !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "atomic-push Pending changed before dispatch");
    }
    let immediate = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("last pre-observation: {e}")),
    };
    if immediate != baseline {
        return recovery(
            Some(run),
            "raw refs changed after Pending before atomic push",
        );
    }
    let reproved = match proved(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        Some(input.package_source),
        input.local_bare_source,
        input.binary,
        input.limits,
        approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    if reproved != proof
        || !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(
            Some(run),
            "local Ready/approval/repo-create/Pending changed before push",
        );
    }
    let request = PushRequest {
        repository: &proof.repository,
        before_refs: &proof.plan.before_refs,
        desired_refs: &proof.desired_refs,
        version_tag_ref: &proof.plan.version_tag_ref,
        commit_oid: &proof.seal.selected_commit_oid,
        tag_oid: &proof.seal.selected_tag_oid,
    };
    let receipt = match port.atomic_push(request) {
        Ok(value) if exact_updates(&proof.plan, value) => value,
        Ok(_) => {
            return recovery(
                Some(run),
                "atomic-push callback did not prove both exact ref updates",
            );
        }
        Err(e) => return recovery(Some(run), format!("atomic-push callback uncertain: {e}")),
    };
    let after = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("first post-observation: {e}")),
    };
    let after_again = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("second post-observation: {e}")),
    };
    let desired = Snapshot {
        repository: RepositoryObservation::Present(proof.repository.clone()),
        refs: proof.desired_refs.clone(),
    };
    if after != desired || after_again != desired {
        return recovery(
            Some(run),
            "atomic push post-observation is not exact desired refs",
        );
    }
    if !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "atomic-push Pending changed before Landed");
    }
    let landed = AtomicPushLanded {
        format: ATOMIC_PUSH_FORMAT.into(),
        run_id: run.into(),
        pending_sha256: sha(&pending_bytes),
        callback_exit_zero: true,
        main_update: receipt.main_update,
        tag_update: receipt.tag_update,
        observed_repository: after.repository,
        observed_refs: after.refs,
    };
    if let Err(e) = journal::publish(input.lease, run, LANDED, &landed) {
        return recovery(Some(run), format!("atomic-push Landed uncertain: {e:#}"));
    }
    inspect_cold(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        None,
        None, // The selected Direct bare refs just moved; prove historical bytes.
        input.binary,
        input.limits,
        approval_id,
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
    fn observe(&mut self) -> Result<Snapshot, String> {
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
    }
    fn atomic_push(&mut self, request: PushRequest<'_>) -> Result<PushReceipt, String> {
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
        if self.lose_reply {
            Err("injected response loss".into())
        } else {
            Ok(PushReceipt {
                main_update: RefUpdateKind::New,
                tag_update: RefUpdateKind::New,
            })
        }
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

#[path = "local_bare_tests.rs"]
pub(crate) mod local_bare_tests;
