//! Mode A one-use marker at the C.1 Pending → Ready seam.

use super::capability_tests::TinyPayloadWorld;
use super::prepare::{prepare_local_full_plan, trusted};
use super::*;
use vibe_lifecycle::ReservationDecision;
use vibe_lifecycle::process::propose_run_id;
use vibe_safefs::Project;

fn marker_path(tiny: &TinyPayloadWorld) -> std::path::PathBuf {
    let redeemed = tiny.settings.path().join("publication-approvals/redeemed");
    let files = std::fs::read_dir(redeemed)
        .unwrap()
        .map(|row| row.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    files[0].clone()
}

#[test]
fn lost_redemption_reply_stays_pending_and_duplicate_does_not_redeem() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = tests::scope();
    tiny.seed_approval(&lease, &selected);
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let output = Project::open(tiny.output.path()).unwrap();
    crate::publication_approval::lose_next_redemption_reply();
    let first = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "lost-marker-reply",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    let Inspection::RecoveryRequired { run_id } = first else {
        panic!("lost marker response must recover")
    };
    let marker = marker_path(&tiny);
    let before = std::fs::read(&marker).unwrap();
    let row = SubmissionReservations::inspect_existing_key(
        &lease,
        &namespace,
        &selected,
        "lost-marker-reply",
    )
    .unwrap()
    .unwrap();
    assert_eq!(row.journal_admission, JournalAdmission::Pending);
    assert!(journal::read_input(&lease, &run_id).unwrap().is_none());
    assert!(
        journal::read::<Intent>(&lease, &run_id, "intent")
            .unwrap()
            .is_none()
    );
    let second = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "lost-marker-reply",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    assert_eq!(second, Inspection::RecoveryRequired { run_id });
    assert_eq!(std::fs::read(marker).unwrap(), before);
}

#[test]
fn ready_marker_survives_current_approval_expiry_but_loss_downgrades() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = tests::scope();
    tiny.seed_approval(&lease, &selected);
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let output = Project::open(tiny.output.path()).unwrap();
    let first = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "ready-marker",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    let Inspection::JournalReady { run_id } = first else {
        panic!("marker plus vault must reach Ready")
    };
    let capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
    let marker_bytes = std::fs::read(marker_path(&tiny)).unwrap();
    assert_eq!(capsule.redemption_marker_sha256, Some(sha(&marker_bytes)));
    assert_eq!(
        journal::read::<Intent>(&lease, &run_id, "intent")
            .unwrap()
            .unwrap()
            .redemption_marker_sha256,
        capsule.redemption_marker_sha256
    );

    let approval_dir = tiny.settings.path().join("publication-approvals");
    let record = std::fs::read_dir(&approval_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_file())
        .unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    value["receipt"]["expires_at_unix"] = serde_json::json!(1);
    std::fs::write(record, serde_json::to_vec(&value).unwrap()).unwrap();
    let duplicate = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "ready-marker",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    assert_eq!(
        duplicate,
        Inspection::JournalReady {
            run_id: run_id.clone()
        }
    );
    std::fs::remove_file(marker_path(&tiny)).unwrap();
    assert_eq!(
        cold::inspect_full_plan_cold_with_home(
            &lease,
            namespace.id(),
            &selected,
            "ready-marker",
            &tiny.approval_home(),
        )
        .unwrap(),
        Inspection::RecoveryRequired { run_id },
    );
}

#[test]
fn one_approval_id_cannot_make_a_second_key_ready() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = tests::scope();
    tiny.seed_approval(&lease, &selected);
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let output = Project::open(tiny.output.path()).unwrap();
    assert!(matches!(
        prepare_local_full_plan(
            &lease,
            &namespace,
            &selected,
            "first",
            &tiny.subject,
            &tiny.plan,
            &tests::expected(),
            trusted(
                tiny.settings.path(),
                &lease,
                &selected,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals()
            ),
        )
        .unwrap(),
        Inspection::JournalReady { .. }
    ));
    let second = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "second",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    let Inspection::RecoveryRequired { run_id } = second else {
        panic!("reused approval must not reach Ready")
    };
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &selected, "second", &run_id)
            .unwrap(),
        JournalAdmission::Pending,
    );
    assert!(journal::read_input(&lease, &run_id).unwrap().is_none());
    assert!(
        root.path()
            .join(format!(
                ".vibe/publication-operations/{run_id}.payloads/complete.json"
            ))
            .is_file()
    );
}

#[test]
fn expired_mode_a_record_refuses_before_c1_or_vault() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = tests::scope();
    tiny.seed_approval(&lease, &selected);
    let record = std::fs::read_dir(tiny.settings.path().join("publication-approvals"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_file())
        .unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    value["receipt"]["expires_at_unix"] = serde_json::json!(1);
    std::fs::write(record, serde_json::to_vec(&value).unwrap()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let output = Project::open(tiny.output.path()).unwrap();
    assert!(
        prepare_local_full_plan(
            &lease,
            &namespace,
            &selected,
            "expired",
            &tiny.subject,
            &tiny.plan,
            &tests::expected(),
            trusted(
                tiny.settings.path(),
                &lease,
                &selected,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals()
            ),
        )
        .is_err()
    );
    assert!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &selected, "expired",)
            .unwrap()
            .is_none()
    );
    assert!(
        lease
            .publication_journal_project()
            .dir_if_present(&[".vibe", "publication-operations"])
            .unwrap()
            .is_none()
    );
}

#[test]
fn existing_key_retains_run_when_settings_home_is_unavailable() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = tests::scope();
    tiny.seed_approval(&lease, &selected);
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let output = Project::open(tiny.output.path()).unwrap();
    let first = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "settings-gone",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    let Inspection::JournalReady { run_id } = first else {
        panic!("initial run must be Ready")
    };
    let path = tiny.settings.path().to_path_buf();
    let archived = path.with_extension("cold-unavailable");
    std::fs::rename(&path, &archived).unwrap();
    let retry = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "settings-gone",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    assert_eq!(
        retry,
        Inspection::RecoveryRequired {
            run_id: run_id.clone()
        }
    );
    assert_eq!(
        SubmissionReservations::journal_admission(
            &lease,
            &namespace,
            &selected,
            "settings-gone",
            &run_id,
        )
        .unwrap(),
        JournalAdmission::Ready
    );
    std::fs::rename(archived, path).unwrap();
    assert_eq!(
        cold::inspect_full_plan_cold_with_home(
            &lease,
            namespace.id(),
            &selected,
            "settings-gone",
            &tiny.approval_home(),
        )
        .unwrap(),
        Inspection::JournalReady { run_id },
    );
}

#[test]
fn retired_existing_key_is_inspected_without_opening_settings() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected_scope = tests::scope();
    let namespace =
        SubmissionReservations::issue_namespace(&lease, &selected_scope, 60_000).unwrap();
    let selected = select(
        &lease,
        &selected_scope,
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        Some(("A", "fixture-approval")),
    )
    .unwrap();
    let run_id = propose_run_id(lease.root()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &selected_scope,
            "retired",
            &selected.request,
            &selected.plan_bytes,
            &run_id,
        )
        .unwrap(),
        ReservationDecision::Created { .. }
    ));
    SubmissionReservations::retire_namespace(&lease, &namespace, &selected_scope).unwrap();
    let path = tiny.settings.path().to_path_buf();
    let archived = path.with_extension("retired-home");
    std::fs::rename(&path, &archived).unwrap();
    let output = Project::open(tiny.output.path()).unwrap();
    let answer = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected_scope,
        "retired",
        &tiny.subject,
        &tiny.plan,
        &tests::expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected_scope,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    assert_eq!(
        answer,
        Inspection::Retired {
            run_id: Some(run_id)
        }
    );
    std::fs::rename(archived, path).unwrap();
}
