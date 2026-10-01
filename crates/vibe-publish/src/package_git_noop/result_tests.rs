use std::fs;
use std::path::Path;
use std::process::Command;

use super::*;
use crate::package_git_parent::tests::{World, binary, limits, world};
use crate::package_git_parent::{self, ParentInspection};

fn prepared(binary: &Path) -> (World, ReadyLimits) {
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
    let selected = ReadyLimits {
        census: parent_limits.census,
        parent: Some(parent_limits),
    };
    assert!(matches!(
        seal_ready(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            binary,
            selected
        ),
        NoEffectInspection::ReadyForObservation { .. }
    ));
    (world, selected)
}

fn move_ref(world: &World, binary: &Path, selected: &str) {
    let reference = if selected == "main" {
        "refs/heads/main"
    } else {
        "refs/tags/v1.1.0"
    };
    let output = Command::new(binary)
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
        .args(["update-ref", reference, &world.origin.previous])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn exact_noeffect_result_is_historical_after_eof_and_remote_drift() {
    let Some(binary) = binary() else { return };
    let (world, selected) = prepared(&binary);
    assert!(matches!(
        inspect_result_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &binary,
            selected
        ),
        NoEffectResultInspection::ReadyToObserve { .. }
    ));
    let status = observe_and_seal_result(
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
        matches!(status, NoEffectResultInspection::NoEffectCompleted { .. }),
        "{status:?}"
    );
    for suffix in [
        "ready",
        "objects",
        "approval-link-pending",
        "approval-link",
        "repo-create-pending",
        "repo-create",
        "atomic-push-pending",
        "atomic-push",
        "result",
    ] {
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, suffix)
                .unwrap()
                .is_none(),
            "{suffix}"
        );
    }
    move_ref(&world, &binary, "tag");
    assert!(matches!(
        package_git_parent_source::observe_local_bare(
            &world.origin.project,
            &binary,
            &world.plan,
            limits().source
        ),
        ParentSourceObservation::Drift(_) | ParentSourceObservation::Unavailable(_)
    ));
    drop(world.lease);
    let reopened = LifecycleLease::acquire(world.root.path()).unwrap();
    assert!(matches!(
        inspect_result_cold(
            &reopened,
            &world.token,
            &world.scope,
            "parent",
            &binary,
            selected
        ),
        NoEffectResultInspection::NoEffectCompleted { .. }
    ));
}

#[test]
fn changed_raw_main_or_tag_between_complete_epochs_never_writes_result() {
    let Some(binary) = binary() else { return };
    for selected_ref in ["main", "tag"] {
        let (world, selected) = prepared(&binary);
        let status = observe_inner(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            selected,
            || move_ref(&world, &binary, selected_ref),
            false,
        );
        assert!(
            matches!(status, NoEffectResultInspection::RecoveryRequired { .. }),
            "{status:?}"
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, RESULT)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn lost_result_reply_cold_recovers_without_reobserving_remote() {
    let Some(binary) = binary() else { return };
    let (world, selected) = prepared(&binary);
    assert!(matches!(
        observe_inner(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            selected,
            || {},
            true
        ),
        NoEffectResultInspection::RecoveryRequired { .. }
    ));
    let original = journal::read_bytes(&world.lease, &world.run_id, RESULT)
        .unwrap()
        .unwrap();
    assert!(matches!(
        inspect_result_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &binary,
            selected
        ),
        NoEffectResultInspection::NoEffectCompleted { .. }
    ));
    assert!(matches!(
        observe_inner(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            selected,
            || panic!("completed no-effect run must not reobserve remote"),
            false
        ),
        NoEffectResultInspection::NoEffectCompleted { .. }
    ));
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, RESULT)
            .unwrap()
            .unwrap(),
        original
    );
}

#[test]
fn damaged_noeffect_result_or_seal_refuses_cold_completion() {
    let Some(binary) = binary() else { return };
    for suffix in [RESULT, SEAL] {
        let (world, selected) = prepared(&binary);
        assert!(matches!(
            observe_and_seal_result(
                &world.lease,
                &world.token,
                &world.scope,
                "parent",
                &world.package,
                &world.origin.project,
                &binary,
                selected
            ),
            NoEffectResultInspection::NoEffectCompleted { .. }
        ));
        fs::write(
            world.root.path().join(format!(
                ".vibe/package-git-operations/{}.package-git-{suffix}.json",
                world.run_id
            )),
            b"not-json",
        )
        .unwrap();
        assert!(matches!(
            inspect_result_cold(
                &world.lease,
                &world.token,
                &world.scope,
                "parent",
                &binary,
                selected
            ),
            NoEffectResultInspection::RecoveryRequired { .. }
        ));
    }
}

#[test]
fn foreign_effect_record_refuses_noeffect_completion() {
    let Some(binary) = binary() else { return };
    for suffix in ["approval-link-pending", "atomic-push"] {
        let (world, selected) = prepared(&binary);
        assert!(matches!(
            observe_and_seal_result(
                &world.lease,
                &world.token,
                &world.scope,
                "parent",
                &world.package,
                &world.origin.project,
                &binary,
                selected
            ),
            NoEffectResultInspection::NoEffectCompleted { .. }
        ));
        fs::write(
            world.root.path().join(format!(
                ".vibe/package-git-operations/{}.package-git-{suffix}.json",
                world.run_id
            )),
            b"foreign-effect",
        )
        .unwrap();
        assert!(matches!(
            inspect_result_cold(
                &world.lease,
                &world.token,
                &world.scope,
                "parent",
                &binary,
                selected
            ),
            NoEffectResultInspection::RecoveryRequired { .. }
        ));
    }
}
