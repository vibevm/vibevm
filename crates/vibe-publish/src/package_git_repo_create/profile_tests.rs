use super::*;
use crate::package_git_approval::prepared_populated_for_test as prepared;
use crate::package_git_provenance_populated::ApprovedGitdir;
use crate::package_git_ready::profile::ReadyProfile;

fn linked() -> Option<(
    crate::package_git_provenance_populated::tests::World,
    tempfile::TempDir,
    ApprovalHome,
    SubmissionNamespace,
    String,
)> {
    let (world, settings, home, namespace) = prepared()?;
    let approved = world.held();
    let verified = home
        .load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            crate::package_git_provenance_populated::tests::limits(),
        )
        .unwrap();
    assert!(matches!(
        home.link_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            crate::package_git_provenance_populated::tests::limits(),
            &verified
        ),
        LinkInspection::Landed { .. }
    ));
    Some((
        world,
        settings,
        home,
        namespace,
        verified.authority_id().into(),
    ))
}

struct ProfileFake {
    repository: HostedRepository,
    observed: RemoteSnapshot,
    observations: usize,
    calls: usize,
    lose_response: bool,
    foreign_third_observation: bool,
    tamper_on_third: Option<std::path::PathBuf>,
}
impl ProfileFake {
    fn new(world: &crate::package_git_provenance_populated::tests::World) -> Self {
        let (_, proof) = package_git_operation::ready_proof_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
        )
        .unwrap();
        let RepositoryTarget::HostedCreate { repository, .. } = proof.unwrap().plan.target else {
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
            foreign_third_observation: false,
            tamper_on_third: None,
        }
    }
}
impl FakeCreatePort for ProfileFake {
    fn observe(&mut self) -> Result<RemoteSnapshot, String> {
        self.observations += 1;
        if self.observations == 3 {
            if let Some(path) = &self.tamper_on_third {
                fs::write(path, b"not-json").map_err(|e| e.to_string())?;
            }
            if self.foreign_third_observation {
                return Ok(RemoteSnapshot {
                    repository: RepositoryObservation::Present(RepositoryIdentity::Hosted {
                        repository: self.repository.clone(),
                        id: 99,
                    }),
                    refs: empty_refs(),
                });
            }
        }
        Ok(self.observed.clone())
    }
    fn create(
        &mut self,
        repository: &HostedRepository,
        creation: &CreationOptions,
    ) -> Result<RepositoryIdentity, String> {
        assert_eq!(repository, &self.repository);
        assert!(!creation.auto_init);
        self.calls += 1;
        let identity = RepositoryIdentity::Hosted {
            repository: self.repository.clone(),
            id: 7,
        };
        self.observed.repository = RepositoryObservation::Present(identity.clone());
        if self.lose_response {
            Err("injected lost callback".into())
        } else {
            Ok(identity)
        }
    }
}

fn input<'a>(
    world: &'a crate::package_git_provenance_populated::tests::World,
    home: &'a ApprovalHome,
    namespace: &'a SubmissionNamespace,
    approved: Option<&'a [ApprovedGitdir<'a>]>,
) -> FakeRun<'a> {
    FakeRun {
        home,
        lease: &world.lease,
        namespace,
        scope: &world.scope,
        key: "aggregate",
        package_source: &world.source,
        git_binary: &world.binary,
        profile: ReadyProfile::ParentlessPopulated {
            limits: crate::package_git_provenance_populated::tests::limits(),
            approved_gitdirs: approved,
        },
    }
}

#[test]
fn populated_ready_and_link_drive_one_fake_create_then_cold_attach() {
    let Some((world, _settings, home, namespace, id)) = linked() else {
        return;
    };
    let approved = world.held();
    let mut fake = ProfileFake::new(&world);
    let status = drive_fake_run(
        &input(&world, &home, &namespace, Some(&approved)),
        &id,
        &mut fake,
    );
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
        inspect_cold_profile(
            &home,
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            &world.binary,
            ReadyProfile::ParentlessPopulated {
                limits: crate::package_git_provenance_populated::tests::limits(),
                approved_gitdirs: None,
            },
            &id
        ),
        CreateInspection::Landed {
            repository_id: 7,
            ..
        }
    ));
    assert!(matches!(
        drive_fake_run(
            &input(&world, &home, &namespace, Some(&approved)),
            &id,
            &mut fake
        ),
        CreateInspection::Landed { .. }
    ));
    assert_eq!(fake.calls, 1);
}

#[test]
fn populated_lost_create_reply_keeps_pending_even_when_repo_appears() {
    let Some((world, _settings, home, namespace, id)) = linked() else {
        return;
    };
    let approved = world.held();
    let mut fake = ProfileFake::new(&world);
    fake.lose_response = true;
    assert!(matches!(
        drive_fake_run(
            &input(&world, &home, &namespace, Some(&approved)),
            &id,
            &mut fake
        ),
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
        drive_fake_run(
            &input(&world, &home, &namespace, Some(&approved)),
            &id,
            &mut fake
        ),
        CreateInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 1);
}

#[test]
fn incomplete_or_foreign_gitdir_refuses_before_create_pending() {
    let Some((world, _settings, home, namespace, id)) = linked() else {
        return;
    };
    let approved = world.held();
    let foreign_root = tempfile::tempdir().unwrap();
    let foreign = Project::open(foreign_root.path()).unwrap();
    let wrong = [
        ApprovedGitdir {
            path: "vendor/a",
            gitdir: &foreign,
        },
        ApprovedGitdir {
            path: "vendor/b",
            gitdir: &world.approved[1],
        },
    ];
    for selected in [Some(&approved[..1]), Some(&wrong[..]), None] {
        let mut fake = ProfileFake::new(&world);
        assert!(matches!(
            drive_fake_run(&input(&world, &home, &namespace, selected), &id, &mut fake),
            CreateInspection::RecoveryRequired { .. }
        ));
        assert_eq!(fake.calls, 0);
    }
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, PENDING)
            .unwrap()
            .is_none()
    );
}

#[test]
fn changed_link_or_third_state_after_pending_never_invokes_create() {
    let Some((world, _settings, home, namespace, id)) = linked() else {
        return;
    };
    let approved = world.held();
    let mut fake = ProfileFake::new(&world);
    fake.foreign_third_observation = true;
    assert!(matches!(
        drive_fake_run(
            &input(&world, &home, &namespace, Some(&approved)),
            &id,
            &mut fake
        ),
        CreateInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 0);
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

    let Some((world, _settings, home, namespace, id)) = linked() else {
        return;
    };
    let approved = world.held();
    let mut fake = ProfileFake::new(&world);
    fake.tamper_on_third = Some(world.root.path().join(format!(
        ".vibe/package-git-operations/{}.package-git-approval-link.json",
        world.run_id
    )));
    assert!(matches!(
        drive_fake_run(
            &input(&world, &home, &namespace, Some(&approved)),
            &id,
            &mut fake
        ),
        CreateInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fake.calls, 0);
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
