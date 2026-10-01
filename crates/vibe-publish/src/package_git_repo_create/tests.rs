use super::*;
use crate::package_git_approval::tests::{World, binary, limits};
use crate::package_git_plan::{CreateDecision, classify_create};
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteSnapshot {
    repository: RepositoryObservation,
    refs: RefSnapshot,
}

pub(crate) trait FakeCreatePort {
    fn observe(&mut self) -> Result<RemoteSnapshot, String>;
    fn create(
        &mut self,
        repository: &HostedRepository,
        creation: &CreationOptions,
    ) -> Result<RepositoryIdentity, String>;
}

/// Test-only dispatch. Production has no RepoCreator, GitHub client or write
/// port connected to this owner.
pub(crate) fn drive_fake(
    world: &World,
    approval_id: &str,
    port: &mut impl FakeCreatePort,
) -> CreateInspection {
    let proof = match proved(
        &world.home,
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        Some(&world.source),
        None,
        &world.binary,
        limits(),
        approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run = proof.run_id.as_str();
    let (repository, creation) = match create_shape(&proof) {
        Ok(value) => value,
        Err(status) => return status,
    };
    match inspect_cold(
        &world.home,
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        Some(&world.source),
        None,
        &world.binary,
        limits(),
        approval_id,
    ) {
        CreateInspection::NotStarted { .. } => {}
        other => return other,
    }
    let baseline = RemoteSnapshot {
        repository: proof.plan.before_repo.clone(),
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
            "pre-effect repository/refs differ from approved baseline",
        );
    }
    let pending = expected_pending(&proof, repository, creation);
    let pending_bytes = match journal::publish(&world.lease, run, PENDING, &pending) {
        Ok(value) => value,
        Err(e) => {
            return recovery(
                Some(run),
                format!("create Pending publication uncertain: {e:#}"),
            );
        }
    };
    if !matches!(journal::read_bytes(&world.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "create Pending changed before dispatch");
    }
    let immediate = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("last pre-observation: {e}")),
    };
    if immediate != baseline {
        return recovery(
            Some(run),
            "remote changed after Pending before create; no dispatch",
        );
    }
    let reproved = match proved(
        &world.home,
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        Some(&world.source),
        None,
        &world.binary,
        limits(),
        approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    if reproved != proof
        || !matches!(journal::read_bytes(&world.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(
            Some(run),
            "local Ready/approval/Pending changed before create",
        );
    }
    let returned = match port.create(repository, creation) {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("create callback uncertain: {e}")),
    };
    let after = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("first post-observation: {e}")),
    };
    let after_again = match port.observe() {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("second post-observation: {e}")),
    };
    if after != after_again || after.refs != empty_refs() {
        return recovery(
            Some(run),
            "created repository/refs changed during post-observation",
        );
    }
    if !matches!(
        classify_create(&proof.plan, true, Some(&returned), &after.repository),
        Ok(CreateDecision::Landed(_))
    ) {
        return recovery(
            Some(run),
            "create response and independent repository identity differ",
        );
    }
    if !matches!(journal::read_bytes(&world.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "create Pending changed before Landed");
    }
    let landed = RepoCreateLanded {
        format: REPO_CREATE_FORMAT.into(),
        run_id: run.into(),
        pending_sha256: sha(&pending_bytes),
        returned_identity: returned,
        observed_repo: after.repository,
        observed_refs: after.refs,
    };
    if let Err(e) = journal::publish(&world.lease, run, LANDED, &landed) {
        return recovery(Some(run), format!("create Landed uncertain: {e:#}"));
    }
    inspect_cold(
        &world.home,
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        Some(&world.source),
        None,
        &world.binary,
        limits(),
        approval_id,
    )
}

pub(crate) struct Fake {
    repository: HostedRepository,
    observed: RemoteSnapshot,
    observations: usize,
    calls: usize,
    lose_response: bool,
    wrong_returned_id: bool,
    foreign_after_pending: bool,
    foreign_before_pending: bool,
    post_refs_drift: bool,
    tamper_on_third_observation: Option<std::path::PathBuf>,
}

impl Fake {
    pub(crate) fn new(world: &World) -> Self {
        let (_, proof) = package_git_operation::ready_proof_cold(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "approval",
            Some(&world.source),
        )
        .unwrap();
        let plan = proof.unwrap().plan;
        let RepositoryTarget::HostedCreate { repository, .. } = plan.target else {
            panic!("hosted fixture")
        };
        Self {
            repository,
            observed: RemoteSnapshot {
                repository: RepositoryObservation::Absent,
                refs: empty_refs(),
            },
            observations: 0,
            calls: 0,
            lose_response: false,
            wrong_returned_id: false,
            foreign_after_pending: false,
            foreign_before_pending: false,
            post_refs_drift: false,
            tamper_on_third_observation: None,
        }
    }
    fn desired(&self) -> RepositoryIdentity {
        RepositoryIdentity::Hosted {
            repository: self.repository.clone(),
            id: 7,
        }
    }
}

impl FakeCreatePort for Fake {
    fn observe(&mut self) -> Result<RemoteSnapshot, String> {
        self.observations += 1;
        if self.observations == 3
            && let Some(path) = &self.tamper_on_third_observation
        {
            fs::write(path, b"not-json").map_err(|e| e.to_string())?;
        }
        if (self.foreign_before_pending && self.observations == 2)
            || (self.foreign_after_pending && self.observations == 3)
        {
            return Ok(RemoteSnapshot {
                repository: RepositoryObservation::Present(RepositoryIdentity::Hosted {
                    repository: self.repository.clone(),
                    id: 99,
                }),
                refs: empty_refs(),
            });
        }
        Ok(self.observed.clone())
    }
    fn create(
        &mut self,
        repository: &HostedRepository,
        creation: &CreationOptions,
    ) -> Result<RepositoryIdentity, String> {
        assert_eq!(repository, &self.repository);
        assert_eq!(creation.default_branch, "main");
        self.calls += 1;
        self.observed.repository = RepositoryObservation::Present(self.desired());
        if self.post_refs_drift {
            self.observed.refs.main = RawRef::Present {
                oid: "a".repeat(40),
            };
        }
        if self.lose_response {
            return Err("injected response loss".into());
        }
        if self.wrong_returned_id {
            return Ok(RepositoryIdentity::Hosted {
                repository: self.repository.clone(),
                id: 8,
            });
        }
        Ok(self.desired())
    }
}

pub(crate) fn linked_world(binary: &Path) -> (World, String) {
    let world = World::new(binary);
    let verified = world.load().unwrap();
    assert!(matches!(
        world.link(&verified),
        LinkInspection::Landed { .. }
    ));
    let id = verified.authority_id().to_owned();
    (world, id)
}

#[test]
fn success_requires_pending_callback_and_independent_exact_id_observation() {
    let Some(binary) = binary() else { return };
    let (world, id) = linked_world(&binary);
    let mut fake = Fake::new(&world);
    let status = drive_fake(&world, &id, &mut fake);
    assert!(
        matches!(
            status,
            CreateInspection::Landed {
                repository_id: 7,
                ..
            }
        ),
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
        CreateInspection::Landed {
            repository_id: 7,
            ..
        }
    ));
    let again = drive_fake(&world, &id, &mut fake);
    assert!(matches!(again, CreateInspection::Landed { .. }));
    assert_eq!(fake.calls, 1);
}

#[test]
fn response_loss_keeps_pending_even_if_matching_repo_appears() {
    let Some(binary) = binary() else { return };
    let (world, id) = linked_world(&binary);
    let mut fake = Fake::new(&world);
    fake.lose_response = true;
    assert!(matches!(
        drive_fake(&world, &id, &mut fake),
        CreateInspection::RecoveryRequired { .. }
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
        CreateInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 1);
}

#[test]
fn before_or_after_pending_drift_refuses_without_create() {
    let Some(binary) = binary() else { return };
    for after_pending in [false, true] {
        let (world, id) = linked_world(&binary);
        let mut fake = Fake::new(&world);
        fake.foreign_before_pending = !after_pending;
        fake.foreign_after_pending = after_pending;
        assert!(matches!(
            drive_fake(&world, &id, &mut fake),
            CreateInspection::RecoveryRequired { .. }
        ));
        assert_eq!(fake.calls, 0);
        assert_eq!(
            journal::read_bytes(&world.lease, &world.run_id, PENDING)
                .unwrap()
                .is_some(),
            after_pending
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, LANDED)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn foreign_callback_id_or_refs_after_create_never_land() {
    let Some(binary) = binary() else { return };
    for wrong_id in [false, true] {
        let (world, id) = linked_world(&binary);
        let mut fake = Fake::new(&world);
        fake.wrong_returned_id = wrong_id;
        fake.post_refs_drift = !wrong_id;
        assert!(matches!(
            drive_fake(&world, &id, &mut fake),
            CreateInspection::RecoveryRequired { .. }
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
    }
}

#[test]
fn missing_or_tampered_approval_link_refuses_before_create_pending() {
    let Some(binary) = binary() else { return };
    let unlinked = World::new(&binary);
    let approval_id = unlinked.load().unwrap().authority_id().to_owned();
    let mut fake = Fake::new(&unlinked);
    assert!(matches!(
        drive_fake(&unlinked, &approval_id, &mut fake),
        CreateInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 0);
    assert!(
        journal::read_bytes(&unlinked.lease, &unlinked.run_id, PENDING)
            .unwrap()
            .is_none()
    );

    let (tampered, id) = linked_world(&binary);
    fs::write(
        tampered.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-approval-link.json",
            tampered.run_id
        )),
        b"not-json",
    )
    .unwrap();
    let mut fake = Fake::new(&tampered);
    assert!(matches!(
        drive_fake(&tampered, &id, &mut fake),
        CreateInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 0);
    assert!(
        journal::read_bytes(&tampered.lease, &tampered.run_id, PENDING)
            .unwrap()
            .is_none()
    );
}

#[test]
fn local_authority_drift_during_last_remote_observation_prevents_create() {
    let Some(binary) = binary() else { return };
    for suffix in ["approval-link", "ready"] {
        let (world, id) = linked_world(&binary);
        let mut fake = Fake::new(&world);
        fake.tamper_on_third_observation = Some(world.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-{suffix}.json",
            world.run_id
        )));
        assert!(matches!(
            drive_fake(&world, &id, &mut fake),
            CreateInspection::RecoveryRequired { .. }
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
}
