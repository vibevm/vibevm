use std::fs;
use std::path::Path;
use std::process::Command;

use super::*;
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_parent::tests::{binary, limits, payload_limits, world};
use crate::package_git_parent::{self, ParentInspection};

fn staged(binary: &Path) -> (crate::package_git_parent::tests::World, ReadyLimits) {
    let world = world(binary, true);
    let parent_limits = limits();
    assert!(matches!(
        package_git_parent::stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            binary,
            parent_limits,
        ),
        ParentInspection::ClosureCompleteButNotReady { .. }
    ));
    (
        world,
        ReadyLimits {
            census: parent_limits.census,
            parent: Some(parent_limits),
        },
    )
}

#[test]
fn reused_main_has_distinct_ready_seal_and_source_less_cold_proof() {
    let Some(binary) = binary() else { return };
    let (world, selected) = staged(&binary);
    let status = seal_ready(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        &world.origin.project,
        &binary,
        selected,
    );
    assert!(
        matches!(status, NoEffectInspection::ReadyForObservation { .. }),
        "{status:?}"
    );
    assert!(matches!(
        inspect_ready_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected
        ),
        NoEffectInspection::ReadyForObservation { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "ready")
            .unwrap()
            .is_none()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .is_none()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, SEAL)
            .unwrap()
            .is_some()
    );
    let namespace = SubmissionNamespace::from_host_token(world.token.clone()).unwrap();
    assert!(
        matches!(prepare_pending(&world.lease, &namespace, &world.scope, "parent",
        &world.package, &world.plan, payload_limits()).unwrap(),
        PendingInspection::ReadyRetainedNeedsColdProof { run_id } if run_id == world.run_id)
    );
    assert!(matches!(
        crate::package_git_ready::inspect_ready_seal_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected
        ),
        crate::package_git_ready::ReadyInspection::RecoveryRequired { .. }
    ));
}

#[test]
fn seal_before_c1_mark_resumes_same_run_without_republishing() {
    let Some(binary) = binary() else { return };
    let (world, selected) = staged(&binary);
    let first = seal_inner(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        &world.origin.project,
        &binary,
        selected,
        true,
    );
    assert!(
        matches!(first, NoEffectInspection::SealRetainedC1Pending { .. }),
        "{first:?}"
    );
    let before = journal::read_bytes(&world.lease, &world.run_id, SEAL)
        .unwrap()
        .unwrap();
    let namespace = SubmissionNamespace::from_host_token(world.token.clone()).unwrap();
    assert_eq!(
        SubmissionReservations::journal_admission(
            &world.lease,
            &namespace,
            &world.scope,
            "parent",
            &world.run_id
        )
        .unwrap(),
        JournalAdmission::Pending
    );
    let second = seal_ready(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        &world.origin.project,
        &binary,
        selected,
    );
    assert!(
        matches!(second, NoEffectInspection::ReadyForObservation { .. }),
        "{second:?}"
    );
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, SEAL)
            .unwrap()
            .unwrap(),
        before
    );
}

#[test]
fn changed_parent_tree_or_peeled_source_ref_refuses_noeffect_proof() {
    let Some(binary) = binary() else { return };
    for defect in ["parent", "tree", "peeled"] {
        let (world, selected) = staged(&binary);
        assert!(matches!(
            seal_ready(
                &world.lease,
                &world.token,
                &world.scope,
                "parent",
                &world.package,
                &world.origin.project,
                &binary,
                selected
            ),
            NoEffectInspection::ReadyForObservation { .. }
        ));
        if defect == "peeled" {
            let output = Command::new(&binary)
                .env_clear()
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_TERMINAL_PROMPT", "0")
                .env("HOME", world.origin.project.root_path())
                .env("USERPROFILE", world.origin.project.root_path())
                .env("PATH", binary.parent().unwrap())
                .arg(format!(
                    "--git-dir={}",
                    world.origin.project.root_path().display()
                ))
                .args(["update-ref", "refs/tags/v1.1.0", &world.origin.previous])
                .output()
                .unwrap();
            assert!(output.status.success());
            assert!(matches!(
                inspect_ready_cold(
                    &world.lease,
                    &world.token,
                    &world.scope,
                    "parent",
                    None,
                    None,
                    &binary,
                    selected
                ),
                NoEffectInspection::ReadyForObservation { .. }
            ));
            assert!(matches!(
                inspect_ready_cold(
                    &world.lease,
                    &world.token,
                    &world.scope,
                    "parent",
                    Some(&world.package),
                    Some(&world.origin.project),
                    &binary,
                    selected
                ),
                NoEffectInspection::RecoveryRequired { .. }
            ));
        } else {
            fs::write(
                world.root.path().join(format!(
                    ".vibe/package-git-operations/{}.package-git-{defect}.json",
                    world.run_id
                )),
                b"not-json",
            )
            .unwrap();
            assert!(matches!(
                inspect_ready_cold(
                    &world.lease,
                    &world.token,
                    &world.scope,
                    "parent",
                    None,
                    None,
                    &binary,
                    selected
                ),
                NoEffectInspection::RecoveryRequired { .. }
            ));
        }
    }
}

#[test]
fn duplicate_ready_refuses_foreign_key_or_plan_without_second_stage() {
    let Some(binary) = binary() else { return };
    let (world, selected) = staged(&binary);
    assert!(matches!(
        seal_ready(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            selected
        ),
        NoEffectInspection::ReadyForObservation { .. }
    ));
    let intent = journal::read_bytes(&world.lease, &world.run_id, "intent")
        .unwrap()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&intent).unwrap();
    let request = value["request_json"].as_str().unwrap().as_bytes();
    let plan_bytes = serde_json::to_vec(&world.plan).unwrap();
    let lease_root = world
        .lease
        .package_git_journal_project()
        .root_correlation_token()
        .unwrap();
    let source_root = world.package.root_correlation_token().unwrap();
    assert!(!ready_duplicate_matches(
        &world.lease,
        &world.run_id,
        &world.token,
        &world.scope,
        "foreign-key",
        request,
        &world.plan,
        &plan_bytes,
        &lease_root,
        &source_root
    ));
    let mut foreign_plan = world.plan.clone();
    foreign_plan.flattened_submodules_sha256 = sha(b"foreign provenance");
    let foreign_bytes = serde_json::to_vec(&foreign_plan).unwrap();
    assert!(!ready_duplicate_matches(
        &world.lease,
        &world.run_id,
        &world.token,
        &world.scope,
        "parent",
        request,
        &foreign_plan,
        &foreign_bytes,
        &lease_root,
        &source_root
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .is_none()
    );
}
