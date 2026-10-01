use super::*;
use crate::package_git_approval::tests::{World, binary, limits};
use crate::package_git_operation;
use std::fs;

use super::driver::{CreatePort as FakeCreatePort, CreatePortError, RemoteSnapshot};
pub(crate) struct FakeRun<'a> {
    pub(crate) home: &'a ApprovalHome,
    pub(crate) lease: &'a LifecycleLease,
    pub(crate) namespace: &'a SubmissionNamespace,
    pub(crate) scope: &'a SubmissionScope,
    pub(crate) key: &'a str,
    pub(crate) package_source: &'a Project,
    pub(crate) git_binary: &'a Path,
    pub(crate) profile: ReadyProfile<'a>,
}

/// Test-only dispatch. Production has no RepoCreator, GitHub client or write
/// port connected to this owner.
pub(crate) fn drive_fake(
    world: &World,
    approval_id: &str,
    port: &mut impl FakeCreatePort,
) -> CreateInspection {
    drive_fake_run(
        &FakeRun {
            home: &world.home,
            lease: &world.lease,
            namespace: &world.namespace,
            scope: &world.scope,
            key: "approval",
            package_source: &world.source,
            git_binary: &world.binary,
            profile: ReadyProfile::NoGitlinks {
                limits: limits(),
                local_bare_source: None,
            },
        },
        approval_id,
        port,
    )
}

pub(crate) fn drive_fake_run(
    input: &FakeRun<'_>,
    approval_id: &str,
    port: &mut impl FakeCreatePort,
) -> CreateInspection {
    driver::drive(
        &driver::CreateRun {
            home: input.home,
            lease: input.lease,
            namespace: input.namespace,
            scope: input.scope,
            key: input.key,
            package_source: input.package_source,
            git_binary: input.git_binary,
            profile: input.profile,
            approval_id,
        },
        port,
    )
}

pub(crate) struct Fake {
    repository: HostedRepository,
    version_tag_ref: String,
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
            version_tag_ref: plan.version_tag_ref,
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
    fn observe(
        &mut self,
        repository: &HostedRepository,
        version_tag_ref: &str,
    ) -> Result<RemoteSnapshot, CreatePortError> {
        assert_eq!(repository, &self.repository);
        assert_eq!(version_tag_ref, self.version_tag_ref);
        self.observations += 1;
        if self.observations == 3
            && let Some(path) = &self.tamper_on_third_observation
        {
            fs::write(path, b"not-json").map_err(|_| CreatePortError::Uncertain)?;
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
    ) -> Result<RepositoryIdentity, CreatePortError> {
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
            return Err(CreatePortError::Uncertain);
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

#[path = "profile_tests.rs"]
pub(crate) mod profile_tests;
