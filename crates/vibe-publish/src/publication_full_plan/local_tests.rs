//! Local publication preparation and cold-attach cases.

use super::prepare::*;
use super::*;
use crate::publication_full_plan::capability_tests::TinyPayloadWorld;
use crate::publication_full_plan::tests::{expected, scope};
use vibe_lifecycle::ReservationDecision;
use vibe_lifecycle::process::propose_run_id;
use vibe_safefs::Project;

#[test]
fn local_prepare_retains_ready_capsule_without_any_remote_effect_and_cold_attaches() {
    let root = tempfile::tempdir().unwrap();
    let tiny = TinyPayloadWorld::new();
    let (subject, plan) = (&tiny.subject, &tiny.plan);
    let output = Project::open(tiny.output.path()).unwrap();
    let scope = scope();
    let (token, run_id) = {
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        tiny.seed_approval(&lease, &scope);
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let outcome = prepare_local_full_plan(
            &lease,
            &namespace,
            &scope,
            "local-key",
            &subject,
            &plan,
            &expected(),
            trusted(
                tiny.settings.path(),
                &lease,
                &scope,
                subject,
                plan,
                &output,
                tiny.finals(),
            ),
        )
        .unwrap();
        let Inspection::JournalReady { run_id } = outcome else {
            panic!("local journal not Ready")
        };
        assert_eq!(
            SubmissionReservations::journal_admission(
                &lease,
                &namespace,
                &scope,
                "local-key",
                &run_id,
            )
            .unwrap(),
            JournalAdmission::Ready,
        );
        let capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
        assert_eq!(capsule.authority_mode.as_deref(), Some("A"));
        assert_eq!(capsule.authority_ref.as_deref(), Some("fixture-approval"));
        let catalogue =
            crate::publication_payload::expected_upload_catalogue(&subject, &expected().subject)
                .unwrap();
        let root_token = lease
            .publication_journal_project()
            .root_correlation_token()
            .unwrap();
        assert_eq!(capsule.upload_catalogue_sha256, catalogue.digest);
        assert_eq!(capsule.root_correlation_token, root_token);
        let manifest = capsule.vault_manifest_sha256.clone().unwrap();
        assert!(manifest.starts_with("sha256:"));
        let request: serde_json::Value = serde_json::from_str(&capsule.request_json).unwrap();
        assert_eq!(request["upload_catalogue_sha256"], catalogue.digest);
        assert_eq!(request["root_correlation_token"], root_token);
        let intent = journal::read::<Intent>(&lease, &run_id, "intent")
            .unwrap()
            .unwrap();
        assert_eq!(intent.upload_catalogue_sha256, catalogue.digest);
        assert_eq!(intent.root_correlation_token, root_token);
        assert_eq!(
            intent.vault_manifest_sha256.as_deref(),
            Some(manifest.as_str())
        );
        assert!(
            journal::read::<super::Step>(&lease, &run_id, "000-pending")
                .unwrap()
                .is_none()
        );
        assert!(
            journal::read::<super::FinalResult>(&lease, &run_id, "result")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            prepare_local_full_plan(
                &lease,
                &namespace,
                &scope,
                "local-key",
                &subject,
                &plan,
                &expected(),
                trusted(
                    tiny.settings.path(),
                    &lease,
                    &scope,
                    subject,
                    plan,
                    &output,
                    tiny.finals()
                ),
            )
            .unwrap(),
            Inspection::JournalReady {
                run_id: run_id.clone()
            },
        );
        (namespace.id().to_owned(), run_id)
    };
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        super::cold::inspect_full_plan_cold_with_home(
            &lease,
            &token,
            &scope,
            "local-key",
            &tiny.approval_home()
        )
        .unwrap(),
        Inspection::JournalReady {
            run_id: run_id.clone()
        },
    );
    let capsule_path = root.path().join(format!(
        ".vibe/publication-operations/{run_id}.full-plan-input.json"
    ));
    let original_capsule = std::fs::read(&capsule_path).unwrap();
    let mut capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
    capsule.upload_catalogue_sha256 = sha(b"foreign-catalogue");
    std::fs::write(&capsule_path, serde_json::to_vec(&capsule).unwrap()).unwrap();
    assert_eq!(
        super::cold::inspect_full_plan_cold_with_home(
            &lease,
            &token,
            &scope,
            "local-key",
            &tiny.approval_home()
        )
        .unwrap(),
        Inspection::RecoveryRequired {
            run_id: run_id.clone()
        },
    );
    std::fs::write(&capsule_path, &original_capsule).unwrap();
    let mut capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
    capsule.root_correlation_token = sha(b"foreign-root");
    std::fs::write(&capsule_path, serde_json::to_vec(&capsule).unwrap()).unwrap();
    assert_eq!(
        super::cold::inspect_full_plan_cold_with_home(
            &lease,
            &token,
            &scope,
            "local-key",
            &tiny.approval_home()
        )
        .unwrap(),
        Inspection::RecoveryRequired {
            run_id: run_id.clone()
        },
    );
    std::fs::write(&capsule_path, &original_capsule).unwrap();
    let mut capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
    capsule.authority_ref = Some("changed".into());
    std::fs::write(&capsule_path, serde_json::to_vec(&capsule).unwrap()).unwrap();
    assert_eq!(
        super::cold::inspect_full_plan_cold_with_home(
            &lease,
            &token,
            &scope,
            "local-key",
            &tiny.approval_home()
        )
        .unwrap(),
        Inspection::RecoveryRequired {
            run_id: run_id.clone()
        },
    );
    std::fs::remove_file(capsule_path).unwrap();
    assert_eq!(
        super::cold::inspect_full_plan_cold_with_home(
            &lease,
            &token,
            &scope,
            "local-key",
            &tiny.approval_home()
        )
        .unwrap(),
        Inspection::RecoveryRequired { run_id },
    );
}

#[test]
fn pre_ready_missing_capsule_and_unapproved_local_selection_never_complete() {
    let root = tempfile::tempdir().unwrap();
    let tiny = TinyPayloadWorld::new();
    let (subject, plan) = (&tiny.subject, &tiny.plan);
    let output = Project::open(tiny.output.path()).unwrap();
    let scope = scope();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let unapproved = trusted(
        tiny.settings.path(),
        &lease,
        &scope,
        subject,
        plan,
        &output,
        tiny.finals(),
    );
    assert!(matches!(
        prepare_local_full_plan(
            &lease,
            &namespace,
            &scope,
            "local-key",
            &subject,
            &plan,
            &expected(),
            unapproved
        ),
        Err(Error::Refused(_))
    ));
    let selected = select(
        &lease,
        &scope,
        &subject,
        &plan,
        &expected(),
        Some(("A", "fixture-approval")),
    )
    .unwrap();
    let run_id = propose_run_id(lease.root()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &scope,
            "local-key",
            &selected.request,
            &selected.plan_bytes,
            &run_id,
        )
        .unwrap(),
        ReservationDecision::Created { .. }
    ));
    assert_eq!(
        super::inspect_full_plan_cold(&lease, namespace.id(), &scope, "local-key",).unwrap(),
        Inspection::RecoveryRequired { run_id },
    );
}

#[test]
fn copied_tree_at_same_path_cannot_cold_attach_as_ready() {
    use std::fs;
    use std::path::Path;

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let destination = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &destination);
            } else {
                fs::copy(entry.path(), destination).unwrap();
            }
        }
    }

    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let tiny = TinyPayloadWorld::new();
    let (subject, plan) = (&tiny.subject, &tiny.plan);
    let output = Project::open(tiny.output.path()).unwrap();
    let scope = scope();
    let (token, run_id, original_root_token) = {
        let lease = LifecycleLease::acquire(&root).unwrap();
        tiny.seed_approval(&lease, &scope);
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let inspection = prepare_local_full_plan(
            &lease,
            &namespace,
            &scope,
            "copied-key",
            &subject,
            &plan,
            &expected(),
            trusted(
                tiny.settings.path(),
                &lease,
                &scope,
                subject,
                plan,
                &output,
                tiny.finals(),
            ),
        )
        .unwrap();
        let Inspection::JournalReady { run_id } = inspection else {
            panic!("not Ready")
        };
        let root_token = lease
            .publication_journal_project()
            .root_correlation_token()
            .unwrap();
        (namespace.id().to_owned(), run_id, root_token)
    };
    let old = parent.path().join("original");
    fs::rename(&root, &old).unwrap();
    copy_tree(&old, &root);
    let lease = LifecycleLease::acquire(&root).unwrap();
    assert_ne!(
        lease
            .publication_journal_project()
            .root_correlation_token()
            .unwrap(),
        original_root_token,
    );
    let copied = super::inspect_full_plan_cold(&lease, &token, &scope, "copied-key");
    assert!(
        matches!(&copied, Ok(Inspection::RecoveryRequired { run_id: same }) if same == &run_id),
        "copied same-path root must recover, got {copied:?}"
    );
}
