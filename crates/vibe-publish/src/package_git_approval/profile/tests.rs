use std::fs;

use super::*;
use crate::package_git_objects::{self, ObjectInspection};
use crate::package_git_provenance_populated::tests::{World, binary, limits, world};
use crate::package_git_provenance_populated::{self, AggregateInspection, ApprovedGitdir};
use crate::package_git_ready::populated::{self, PopulatedInspection};
use crate::package_git_ready::profile::inspect_profile_cold;

fn prepared() -> Option<(World, tempfile::TempDir, ApprovalHome, SubmissionNamespace)> {
    let binary = binary()?;
    let world = world(&binary);
    world.stage_links(2);
    let approved = world.held();
    assert!(matches!(
        package_git_provenance_populated::retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits(),
        ),
        AggregateInspection::AggregateCompleteButNotReady { .. }
    ));
    assert!(matches!(
        package_git_objects::stage_parentless_populated_objects_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits(),
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
    assert!(matches!(
        populated::seal_parentless_ready(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits()
        ),
        PopulatedInspection::ReadyForLocalProof { .. }
    ));
    let namespace = SubmissionNamespace::from_host_token(world.namespace.clone()).unwrap();
    let settings = tempfile::tempdir().unwrap();
    fs::create_dir_all(settings.path().join(HOME).join(APPROVAL_DIR)).unwrap();
    let home = ApprovalHome::open_selected(settings.path()).unwrap();
    let ready_bytes = journal::read_bytes(&world.lease, &world.run_id, "ready")
        .unwrap()
        .unwrap();
    let seal: journal::ReadySeal = serde_json::from_slice(&ready_bytes).unwrap();
    let (_, proof) = package_git_operation::ready_proof_cold(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        Some(&world.source),
    )
    .unwrap();
    let plan = proof.unwrap().plan;
    let now = host_now().unwrap();
    let record = ApprovalRecord {
        format: FORMAT.into(),
        approval_id: "populated-human-approval".into(),
        approver: "Oleg Chirukhin".into(),
        issued_at_unix: now.saturating_sub(1),
        expires_at_unix: now + 60,
        semantic_plan_sha256: seal.semantic_plan_sha256.clone(),
        c1_plan_sha256: seal.c1_plan_sha256.clone(),
        request_sha256: seal.request_sha256.clone(),
        ready_seal_sha256: sha(&ready_bytes),
        namespace_id: namespace.id().into(),
        run_id: world.run_id.clone(),
        key_sha256: seal.key_sha256,
        context: world.scope.context.clone(),
        target: world.scope.target.clone(),
        principal: world.scope.principal.clone(),
        workspace_root_token: seal.lease_root_token,
        settings_root_token: home.settings.root_correlation_token().unwrap(),
        repository: plan.target.clone(),
        effects: effects(&plan).unwrap(),
    };
    let path = settings
        .path()
        .join(HOME)
        .join(APPROVAL_DIR)
        .join(approval_name(&record.semantic_plan_sha256).unwrap());
    fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
    Some((world, settings, home, namespace))
}

#[test]
fn exact_populated_mode_a_record_loads_with_complete_current_gitdirs() {
    let Some((world, settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    let loaded = home
        .load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
        )
        .unwrap();
    assert_eq!(loaded.record.run_id, world.run_id);
    let proof = inspect_profile_cold(
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        None,
        &world.binary,
        ReadyProfile::ParentlessPopulated {
            limits: limits(),
            approved_gitdirs: None,
        },
    )
    .unwrap();
    assert_eq!(proof.seal_sha256, loaded.record.ready_seal_sha256);
    assert_eq!(
        proof.plan.digest().unwrap(),
        loaded.record.semantic_plan_sha256
    );
    assert!(
        settings
            .path()
            .join(HOME)
            .join(REDEEMED)
            .read_dir()
            .is_err(),
        "loading approval must not redeem its ID"
    );
}

#[test]
fn partial_foreign_or_mismatched_current_gitdirs_refuse_before_record_use() {
    let Some((world, _settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    for choice in [Some(&approved[..1]), None] {
        assert!(
            home.load_mode_a_populated(
                &world.lease,
                &namespace,
                &world.scope,
                "aggregate",
                Some(&world.source),
                choice,
                &world.binary,
                limits()
            )
            .is_err()
        );
    }
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
    assert!(
        home.load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&wrong),
            &world.binary,
            limits()
        )
        .is_err()
    );
    assert!(
        home.load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            Some(&approved),
            &world.binary,
            limits()
        )
        .is_err()
    );
}

#[test]
fn no_gitlinks_or_stale_seal_claim_cannot_approve_populated_profile() {
    let Some((world, settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    assert!(matches!(
        inspect_profile_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            &world.binary,
            ReadyProfile::NoGitlinks {
                limits: ReadyLimits {
                    census: limits().census,
                    parent: None
                },
                local_bare_source: None,
            }
        ),
        Err(ReadyInspection::RecoveryRequired { .. }) | Err(ReadyInspection::Unavailable { .. })
    ));
    let path = settings.path().join(HOME).join(APPROVAL_DIR).join(
        approval_name(
            &journal::read::<journal::ReadySeal>(&world.lease, &world.run_id, "ready")
                .unwrap()
                .unwrap()
                .semantic_plan_sha256,
        )
        .unwrap(),
    );
    let mut record: ApprovalRecord = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    record.ready_seal_sha256 = sha(b"old NoGitlinks seal");
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    assert!(
        home.load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits()
        )
        .is_err()
    );
}

#[test]
fn populated_link_redeems_once_and_historical_cold_survives_expiry() {
    let Some((world, settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    let loaded = home
        .load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
        )
        .unwrap();
    let status = home.link_mode_a_populated(
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        Some(&world.source),
        Some(&approved),
        &world.binary,
        limits(),
        &loaded,
    );
    assert!(
        matches!(&status, LinkInspection::Landed { run_id, .. } if run_id == &world.run_id),
        "{status:?}"
    );
    assert!(
        settings
            .path()
            .join(HOME)
            .join(REDEEMED)
            .join(marker_name(loaded.authority_id()))
            .is_file()
    );
    let expired_now = loaded.record.expires_at_unix;
    assert!(
        home.load_at_profile(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            &world.binary,
            ReadyProfile::ParentlessPopulated {
                limits: limits(),
                approved_gitdirs: Some(&approved),
            },
            expired_now
        )
        .is_err()
    );
    assert_eq!(
        home.inspect_link_populated_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &world.binary,
            limits(),
            loaded.authority_id()
        ),
        status
    );
    assert!(matches!(
        home.link_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
            &loaded
        ),
        LinkInspection::RecoveryRequired { .. }
    ));
}

#[test]
fn populated_lost_marker_reply_leaves_pending_without_reissue() {
    let Some((world, settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    let loaded = home
        .load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
        )
        .unwrap();
    let status = home.link_inner_profile(
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        Some(&world.source),
        &world.binary,
        ReadyProfile::ParentlessPopulated {
            limits: limits(),
            approved_gitdirs: Some(&approved),
        },
        &loaded,
        Some(host_now().unwrap()),
        true,
        false,
    );
    assert!(matches!(status, LinkInspection::RecoveryRequired { .. }));
    let marker = settings
        .path()
        .join(HOME)
        .join(REDEEMED)
        .join(marker_name(loaded.authority_id()));
    let before = fs::read(&marker).unwrap();
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link-pending")
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        home.inspect_link_populated_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &world.binary,
            limits(),
            loaded.authority_id()
        ),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert!(matches!(
        home.link_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
            &loaded
        ),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert_eq!(fs::read(marker).unwrap(), before);
}

#[test]
fn incomplete_or_foreign_populated_gitdirs_refuse_before_link_pending() {
    let Some((world, settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    let loaded = home
        .load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
        )
        .unwrap();
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
    for choice in [Some(&approved[..1]), Some(&wrong[..]), None] {
        assert!(matches!(
            home.link_mode_a_populated(
                &world.lease,
                &namespace,
                &world.scope,
                "aggregate",
                Some(&world.source),
                choice,
                &world.binary,
                limits(),
                &loaded
            ),
            LinkInspection::RecoveryRequired { .. }
        ));
    }
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link-pending")
            .unwrap()
            .is_none()
    );
    assert!(!settings.path().join(HOME).join(REDEEMED).exists());
}

#[test]
fn changed_populated_ready_seal_or_marker_refuses_link_cold() {
    let Some((world, settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    let loaded = home
        .load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
        )
        .unwrap();
    fs::write(
        world.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-ready.json",
            world.run_id
        )),
        b"not-json",
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
            limits(),
            &loaded
        ),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link-pending")
            .unwrap()
            .is_none()
    );
    assert!(!settings.path().join(HOME).join(REDEEMED).exists());

    let Some((world, settings, home, namespace)) = prepared() else {
        return;
    };
    let approved = world.held();
    let loaded = home
        .load_mode_a_populated(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &world.binary,
            limits(),
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
            limits(),
            &loaded
        ),
        LinkInspection::Landed { .. }
    ));
    fs::write(
        settings
            .path()
            .join(HOME)
            .join(REDEEMED)
            .join(marker_name(loaded.authority_id())),
        b"tampered",
    )
    .unwrap();
    assert!(matches!(
        home.inspect_link_populated_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &world.binary,
            limits(),
            loaded.authority_id()
        ),
        LinkInspection::RecoveryRequired { .. }
    ));
}
