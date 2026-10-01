use super::*;

fn oid_hex(ch: char) -> String {
    ch.to_string().repeat(40)
}
fn sha(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}
fn hosted() -> HostedRepository {
    HostedRepository {
        host: "github.com".into(),
        organization: "vibespecs".into(),
        name: "org.vibevm.wal".into(),
    }
}
fn hosted_identity(id: u64) -> RepositoryIdentity {
    RepositoryIdentity::Hosted {
        repository: hosted(),
        id,
    }
}
fn absent() -> PackageGitPlan {
    PackageGitPlan {
        package: PackageCoordinate {
            kind: "flow".into(),
            group: "org.vibevm".into(),
            name: "wal".into(),
            version: "1.1.0".into(),
            mutable: true,
        },
        version_tag_ref: "refs/tags/v1.1.0".into(),
        target: RepositoryTarget::HostedCreate {
            repository: hosted(),
            creation: CreationOptions {
                description: Some("WAL flow".into()),
                homepage: None,
                default_branch: "main".into(),
                auto_init: false,
            },
        },
        payload_sha256: sha('a'),
        flattened_submodules_sha256: sha('b'),
        oid_hex_len: 40,
        before_repo: RepositoryObservation::Absent,
        before_refs: RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
        staged: StagedObjects {
            git_tree_oid: oid_hex('c'),
            commit: StagedCommit::New {
                oid: oid_hex('d'),
                parent_oid: None,
            },
            annotated_tag_oid: Some(oid_hex('e')),
            tag_target_commit_oid: oid_hex('d'),
        },
    }
}
fn existing() -> PackageGitPlan {
    let mut plan = absent();
    plan.before_repo = RepositoryObservation::Present(hosted_identity(72));
    plan.before_refs = RefSnapshot {
        main: RawRef::Present { oid: oid_hex('1') },
        version_tag: TagRef::Present {
            raw_oid: oid_hex('2'),
            peeled_commit_oid: oid_hex('1'),
        },
    };
    plan.staged.commit = StagedCommit::New {
        oid: oid_hex('d'),
        parent_oid: Some(oid_hex('1')),
    };
    plan
}
fn direct() -> PackageGitPlan {
    let mut plan = existing();
    let remote_identity_sha256 = sha('9'); // credential-free selected bare-Git identity
    plan.target = RepositoryTarget::DirectGitExisting {
        remote_identity_sha256: remote_identity_sha256.clone(),
    };
    plan.before_repo = RepositoryObservation::Present(RepositoryIdentity::Direct {
        remote_identity_sha256,
    });
    plan
}

#[test]
fn exact_payload_target_refs_objects_and_options_change_digest() {
    let plan = existing();
    let baseline = plan.digest().unwrap();
    let mut variants = Vec::new();
    let mut changed = plan.clone();
    changed.version_tag_ref = "refs/tags/release-1.1.0".into();
    variants.push(changed);
    let mut changed = plan.clone();
    changed.payload_sha256 = sha('f');
    variants.push(changed);
    let mut changed = plan.clone();
    changed.flattened_submodules_sha256 = sha('f');
    variants.push(changed);
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { repository, .. } = &mut changed.target else {
        panic!()
    };
    repository.name.push_str("-next");
    changed.before_repo = RepositoryObservation::Present(RepositoryIdentity::Hosted {
        repository: repository.clone(),
        id: 72,
    });
    variants.push(changed);
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { creation, .. } = &mut changed.target else {
        panic!()
    };
    creation.description = Some("changed".into());
    variants.push(changed);
    let mut changed = plan.clone();
    changed.before_refs.main = RawRef::Present { oid: oid_hex('3') };
    changed.staged.commit = StagedCommit::New {
        oid: oid_hex('d'),
        parent_oid: Some(oid_hex('3')),
    };
    variants.push(changed);
    let mut changed = plan.clone();
    changed.staged.annotated_tag_oid = Some(oid_hex('4'));
    variants.push(changed);
    for candidate in variants {
        assert_ne!(candidate.digest().unwrap(), baseline);
    }
    assert_ne!(direct().digest().unwrap(), baseline);
    assert_eq!(plan.digest().unwrap(), baseline);
}

#[test]
fn strict_scope_oid_parent_and_credential_surfaces_refuse() {
    let plan = existing();
    let mut invalids = Vec::new();
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { repository, .. } = &mut changed.target else {
        panic!()
    };
    repository.host = "https://github.com".into();
    invalids.push(changed);
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { repository, .. } = &mut changed.target else {
        panic!()
    };
    repository.organization = "foreign/org".into();
    invalids.push(changed);
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { repository, .. } = &mut changed.target else {
        panic!()
    };
    repository.name = "user:secret@github.com".into();
    invalids.push(changed);
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { creation, .. } = &mut changed.target else {
        panic!()
    };
    creation.homepage = Some("https://user:secret@example.test".into());
    invalids.push(changed);
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { creation, .. } = &mut changed.target else {
        panic!()
    };
    creation.homepage = Some("https://example.test/?token=secret".into());
    invalids.push(changed);
    let mut changed = plan.clone();
    let RepositoryTarget::HostedCreate { creation, .. } = &mut changed.target else {
        panic!()
    };
    creation.auto_init = true;
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.version_tag_ref = "refs/tags/v1.1.1".into();
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.package.mutable = false;
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.package.version = "01.1.0".into();
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.package.name = "wal/escape".into();
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.oid_hex_len = 39;
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.staged.annotated_tag_oid = Some("A".repeat(40));
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.staged.tag_target_commit_oid = oid_hex('3');
    invalids.push(changed);
    let mut changed = plan.clone();
    changed.staged.commit = StagedCommit::New {
        oid: oid_hex('d'),
        parent_oid: Some(oid_hex('3')),
    };
    invalids.push(changed);
    let mut changed = absent();
    changed.before_refs.main = RawRef::Present { oid: oid_hex('1') };
    invalids.push(changed);
    let mut changed = direct();
    changed.before_repo = RepositoryObservation::Absent;
    invalids.push(changed);
    let mut changed = direct();
    changed.before_repo = RepositoryObservation::Present(RepositoryIdentity::Direct {
        remote_identity_sha256: sha('8'),
    });
    invalids.push(changed);
    for candidate in invalids {
        assert!(
            candidate.validate().is_err(),
            "unexpectedly valid: {candidate:?}"
        );
    }
    let mut sha256_repo = absent();
    sha256_repo.oid_hex_len = 64;
    sha256_repo.staged.git_tree_oid = "c".repeat(64);
    sha256_repo.staged.commit = StagedCommit::New {
        oid: "d".repeat(64),
        parent_oid: None,
    };
    sha256_repo.staged.annotated_tag_oid = Some("e".repeat(64));
    sha256_repo.staged.tag_target_commit_oid = "d".repeat(64);
    assert!(sha256_repo.validate().is_ok());
}

#[test]
fn hosted_create_requires_retained_response_and_exact_independent_identity() {
    let plan = absent();
    assert_eq!(
        classify_create(&plan, false, None, &RepositoryObservation::Absent).unwrap(),
        CreateDecision::NeedCreate
    );
    let appeared = RepositoryObservation::Present(hosted_identity(72));
    assert_eq!(
        classify_create(&plan, true, None, &appeared).unwrap(),
        CreateDecision::Uncertain
    );
    assert_eq!(
        classify_create(&plan, true, None, &RepositoryObservation::Absent).unwrap(),
        CreateDecision::Uncertain
    );
    assert_eq!(
        classify_create(
            &plan,
            true,
            Some(&hosted_identity(72)),
            &RepositoryObservation::Absent
        )
        .unwrap(),
        CreateDecision::Uncertain
    );
    assert_eq!(
        classify_create(
            &plan,
            true,
            Some(&hosted_identity(72)),
            &RepositoryObservation::Present(hosted_identity(73))
        )
        .unwrap(),
        CreateDecision::RecoveryRequired
    );
    let landed = classify_create(&plan, true, Some(&hosted_identity(72)), &appeared).unwrap();
    let CreateDecision::Landed(ready) = landed else {
        panic!("exact hosted create should land")
    };
    assert_eq!(ready.identity(), &hosted_identity(72));
    let present = existing();
    assert_eq!(
        classify_create(
            &present,
            false,
            None,
            &RepositoryObservation::Present(hosted_identity(72))
        )
        .unwrap(),
        CreateDecision::AlreadyPresent(RepoReady {
            identity: hosted_identity(72)
        })
    );
    assert_eq!(
        classify_create(
            &present,
            false,
            None,
            &RepositoryObservation::Present(hosted_identity(73))
        )
        .unwrap(),
        CreateDecision::RecoveryRequired
    );
    assert!(classify_create(&plan, false, Some(&hosted_identity(72)), &appeared).is_err());
    let foreign = RepositoryIdentity::Hosted {
        repository: HostedRepository {
            organization: "other".into(),
            ..hosted()
        },
        id: 72,
    };
    assert_eq!(
        classify_create(&plan, true, Some(&foreign), &appeared).unwrap(),
        CreateDecision::RecoveryRequired
    );
    assert_eq!(
        classify_create(
            &plan,
            true,
            Some(&hosted_identity(72)),
            &RepositoryObservation::Present(foreign.clone())
        )
        .unwrap(),
        CreateDecision::RecoveryRequired
    );
    assert_eq!(
        classify_create(&plan, true, None, &RepositoryObservation::Present(foreign)).unwrap(),
        CreateDecision::RecoveryRequired
    );
}

#[test]
fn direct_existing_bare_identity_has_no_create_call_or_numeric_id() {
    let plan = direct();
    plan.validate().unwrap();
    let ready = match classify_create(&plan, false, None, &plan.before_repo).unwrap() {
        CreateDecision::AlreadyPresent(ready) => ready,
        _ => panic!(),
    };
    assert!(
        matches!(ready.identity(), RepositoryIdentity::Direct { remote_identity_sha256 } if remote_identity_sha256 == &sha('9'))
    );
    assert_eq!(
        classify_create(&plan, true, None, &plan.before_repo).unwrap(),
        CreateDecision::RecoveryRequired
    );
    let desired = plan.desired_refs().unwrap();
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::ExitZero,
            &desired
        )
        .unwrap(),
        PushDecision::Landed
    );
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &RepositoryObservation::Present(RepositoryIdentity::Direct {
                remote_identity_sha256: sha('8')
            }),
            PushCallback::ExitZero,
            &desired
        )
        .unwrap(),
        PushDecision::RecoveryRequired
    );
    let mut empty_bare = plan.clone();
    empty_bare.before_refs = RefSnapshot {
        main: RawRef::Absent,
        version_tag: TagRef::Absent,
    };
    empty_bare.staged.commit = StagedCommit::New {
        oid: oid_hex('d'),
        parent_oid: None,
    };
    empty_bare.validate().unwrap();
    let ready = match classify_create(&empty_bare, false, None, &empty_bare.before_repo).unwrap() {
        CreateDecision::AlreadyPresent(ready) => ready,
        _ => panic!("operator-provisioned empty bare repo must skip create"),
    };
    assert_eq!(
        classify_push(
            &empty_bare,
            &ready,
            &empty_bare.before_repo,
            PushCallback::NotDispatched,
            &empty_bare.before_refs,
        )
        .unwrap(),
        PushDecision::NeedPush
    );
}

#[test]
fn atomic_push_requires_callback_and_exact_ref_poststate() {
    let plan = existing();
    let ready = match classify_create(&plan, false, None, &plan.before_repo).unwrap() {
        CreateDecision::AlreadyPresent(value) => value,
        _ => panic!(),
    };
    let desired = plan.desired_refs().unwrap();
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::NotDispatched,
            &plan.before_refs
        )
        .unwrap(),
        PushDecision::NeedPush
    );
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::ExitZero,
            &desired
        )
        .unwrap(),
        PushDecision::Landed
    );
    for observed in [&plan.before_refs, &desired] {
        assert_eq!(
            classify_push(
                &plan,
                &ready,
                &plan.before_repo,
                PushCallback::LostOrFailed,
                observed
            )
            .unwrap(),
            PushDecision::Uncertain
        );
    }
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::ExitZero,
            &plan.before_refs
        )
        .unwrap(),
        PushDecision::RecoveryRequired
    );
    let mut split = desired.clone();
    split.version_tag = plan.before_refs.version_tag.clone();
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::LostOrFailed,
            &split
        )
        .unwrap(),
        PushDecision::RecoveryRequired
    );
    let mut third = desired.clone();
    third.main = RawRef::Present { oid: oid_hex('7') };
    third.version_tag = TagRef::Present {
        raw_oid: oid_hex('8'),
        peeled_commit_oid: oid_hex('7'),
    };
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::LostOrFailed,
            &third
        )
        .unwrap(),
        PushDecision::RecoveryRequired
    );
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &RepositoryObservation::Present(hosted_identity(73)),
            PushCallback::ExitZero,
            &desired
        )
        .unwrap(),
        PushDecision::RecoveryRequired
    );
}

#[test]
fn peeled_target_idempotence_ignores_new_annotated_tag_metadata() {
    let mut plan = existing();
    plan.staged.commit = StagedCommit::ReusedMain { oid: oid_hex('1') };
    plan.staged.tag_target_commit_oid = oid_hex('1');
    assert_ne!(
        tag_raw(&plan.before_refs.version_tag),
        plan.staged.annotated_tag_oid.as_deref()
    );
    plan.validate().unwrap();
    let ready = match classify_create(&plan, false, None, &plan.before_repo).unwrap() {
        CreateDecision::AlreadyPresent(value) => value,
        _ => panic!(),
    };
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::NotDispatched,
            &plan.before_refs
        )
        .unwrap(),
        PushDecision::PreEffectNoOp
    );
    assert_ne!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::LostOrFailed,
            &plan.before_refs
        )
        .unwrap(),
        PushDecision::Landed
    );
    plan.staged.annotated_tag_oid = None; // no object is manufactured for this no-op
    plan.validate().unwrap();
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::NotDispatched,
            &plan.before_refs
        )
        .unwrap(),
        PushDecision::PreEffectNoOp
    );
    assert_eq!(
        classify_push(
            &plan,
            &ready,
            &plan.before_repo,
            PushCallback::LostOrFailed,
            &plan.before_refs
        )
        .unwrap(),
        PushDecision::RecoveryRequired
    );
}

#[test]
fn private_plan_roundtrip_is_strict_at_root_and_nested_rows() {
    let plan = direct();
    let bytes = serde_json::to_vec(&plan).unwrap();
    let decoded: PackageGitPlan = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, plan);
    assert_eq!(decoded.digest().unwrap(), plan.digest().unwrap());
    let mut root: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    root["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PackageGitPlan>(root).is_err());
    let mut nested: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    nested["package"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PackageGitPlan>(nested).is_err());
}
