//! Fixed package intent, effect, partial inspection, and retained result.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_core::manifest::{
    ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget, MechanismRoutes,
};
use vibe_extension_registry::MechanismRegistry;

use crate::mechanism::package::support::{empty_world, key, registry, temp};

fn target() -> ArtifactPackageTarget {
    ArtifactPackageTarget {
        id: TARGET.into(),
        mechanism: key("package:static-file"),
        provider: None,
        when: None,
        inputs: Some(vec![ArtifactInput::Path {
            path: SOURCE.into(),
        }]),
        outputs: vec![ArtifactOutput {
            id: OUTPUT.into(),
            kind: ArtifactKind::File,
            select: None,
        }],
        config: None,
    }
}

fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "context".into(),
        target: "root".into(),
        principal: "owner".into(),
    }
}

fn execution<'a>(
    root: &'a Path,
    targets: &'a [ArtifactPackageTarget],
    registry: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
) -> PackageExecution<'a> {
    PackageExecution {
        project_root: root,
        targets,
        registry,
        routes,
        package_root: PackageExecution::default_package_root(),
        created_at: "2026-10-01T00:00:00Z",
    }
}

#[test]
fn fixed_intent_is_ready_before_any_output_or_artifact_record() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source bytes").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let first =
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).unwrap();
    let StaticFileJournalInspection::IntentReady { run_id } = first else {
        panic!("new intent must reach Ready")
    };
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope, "copy", &run_id)
            .unwrap(),
        JournalAdmission::Ready
    );
    assert!(
        root.path()
            .join(".vibe/package-operations")
            .join(intent_name(&run_id))
            .is_file()
    );
    assert!(!root.path().join(OUTPUT_DIR).exists());
    assert!(!root.path().join(RECORD).exists());
    let second =
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).unwrap();
    let StaticFileJournalInspection::Uncertain {
        run_id: seen,
        partial,
    } = second
    else {
        panic!("ready intent without result is uncertain")
    };
    assert_eq!(seen, run_id);
    assert_eq!(partial.output_directory, ObservedPresence::Absent);
    assert_eq!(partial.output_file, ObservedFile::Absent);
    assert_eq!(partial.artifact_record, ObservedFile::Absent);
    assert!(!root.path().join(OUTPUT_DIR).exists());
}

#[test]
fn changed_source_or_plan_conflicts_without_copy() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"first").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let identity = fresh_preflight(&lease, &selected, subject(&lease, &selected).unwrap()).unwrap();
    let first =
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).unwrap();
    let StaticFileJournalInspection::IntentReady { run_id } = first else {
        panic!("new intent")
    };
    let mut changed_plan = identity.plan.clone();
    changed_plan.push(b'!');
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &scope,
            "copy",
            &identity.request,
            &changed_plan,
            &run_id
        ),
        Err(crate::SubmissionError::KeyConflict)
    ));
    std::fs::write(root.path().join(SOURCE), b"changed").unwrap();
    let error = reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected)
        .unwrap_err();
    assert!(matches!(
        error,
        StaticFileJournalError::Submission(crate::SubmissionError::KeyConflict)
    ));
    assert!(!root.path().join(OUTPUT_DIR).exists());
}

#[test]
fn foreign_scope_and_prior_occupants_refuse_before_intent() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..scope.clone()
    };
    assert!(matches!(
        reserve_fixed_static_file_intent(&lease, &namespace, &foreign, "copy", &selected),
        Err(StaticFileJournalError::Submission(
            crate::SubmissionError::ForeignScope
        ))
    ));
    std::fs::create_dir_all(root.path().join(OUTPUT_DIR)).unwrap();
    std::fs::write(root.path().join(OUTPUT_DIR).join("sentinel"), b"keep").unwrap();
    assert!(
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).is_err()
    );
    assert_eq!(
        std::fs::read(root.path().join(OUTPUT_DIR).join("sentinel")).unwrap(),
        b"keep"
    );
    assert!(!root.path().join(".vibe/package-operations").exists());
    std::fs::remove_dir_all(root.path().join(OUTPUT_DIR)).unwrap();
    std::fs::create_dir_all(root.path().join(".vibe/state/artifacts")).unwrap();
    std::fs::write(root.path().join(RECORD), b"orphan record").unwrap();
    assert!(
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).is_err()
    );
    assert_eq!(
        std::fs::read(root.path().join(RECORD)).unwrap(),
        b"orphan record"
    );
    assert!(!root.path().join(".vibe/package-operations").exists());
}

#[test]
fn reservation_without_owner_intent_requires_recovery() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let identity = fresh_preflight(&lease, &selected, subject(&lease, &selected).unwrap()).unwrap();
    let run_id = propose_run_id(root.path()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &scope,
            "copy",
            &identity.request,
            &identity.plan,
            &run_id
        )
        .unwrap(),
        ReservationDecision::Created { .. }
    ));
    assert_eq!(
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).unwrap(),
        StaticFileJournalInspection::RecoveryRequired {
            run_id: run_id.clone()
        }
    );
    SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope, "copy", &run_id)
        .unwrap();
    assert_eq!(
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).unwrap(),
        StaticFileJournalInspection::RecoveryRequired {
            run_id: run_id.clone()
        }
    );
    assert!(!root.path().join(".vibe/package-operations").exists());
    assert!(!root.path().join(OUTPUT_DIR).exists());
}

#[test]
fn fixed_copy_retains_verified_result_and_duplicate_never_reapplies() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"exact source").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let copies = AtomicUsize::new(0);
    let counted = || {
        copies.fetch_add(1, Ordering::SeqCst);
    };
    let first = execute_fixed_static_file_with_hook(
        &lease, &namespace, &scope, "copy", &selected, &counted,
    )
    .unwrap();
    let StaticFileJournalInspection::Completed {
        run_id,
        output_sha256,
        artifact_record,
    } = first
    else {
        panic!("fresh fixed copy must retain Completed")
    };
    assert_eq!(copies.load(Ordering::SeqCst), 1);
    assert_eq!(
        std::fs::read(root.path().join(OUTPUT_FILE)).unwrap(),
        b"exact source"
    );
    assert_eq!(artifact_record, RECORD);
    assert_eq!(
        output_sha256,
        format!("{:x}", Sha256::digest(b"exact source"))
    );
    assert!(
        root.path()
            .join(".vibe/package-operations")
            .join(result_name(&run_id))
            .is_file()
    );
    let second = execute_fixed_static_file_with_hook(
        &lease, &namespace, &scope, "copy", &selected, &counted,
    )
    .unwrap();
    assert_eq!(
        second,
        StaticFileJournalInspection::Completed {
            run_id,
            output_sha256,
            artifact_record
        }
    );
    assert_eq!(copies.load(Ordering::SeqCst), 1);
}

#[test]
fn interruption_after_copy_before_record_is_uncertain_and_never_retries() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"copied once").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let stop = || panic!("injected owner crash after provider.apply");
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = execute_fixed_static_file_with_hook(
            &lease, &namespace, &scope, "copy", &selected, &stop,
        );
    }));
    assert!(crashed.is_err());
    assert_eq!(
        std::fs::read(root.path().join(OUTPUT_FILE)).unwrap(),
        b"copied once"
    );
    assert!(!root.path().join(RECORD).exists());
    let inspected =
        execute_fixed_static_file(&lease, &namespace, &scope, "copy", &selected).unwrap();
    let StaticFileJournalInspection::Uncertain { partial, .. } = inspected else {
        panic!("copy without record is uncertain")
    };
    assert_eq!(partial.output_directory, ObservedPresence::Present);
    assert!(matches!(partial.output_file, ObservedFile::Present { .. }));
    assert_eq!(partial.artifact_record, ObservedFile::Absent);
    assert_eq!(
        std::fs::read(root.path().join(OUTPUT_FILE)).unwrap(),
        b"copied once"
    );
    assert!(!root.path().join(RECORD).exists());
}

#[test]
fn artifact_record_without_owner_result_never_completes_or_recopies() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"copied once").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let stop = || panic!("injected owner crash after A2 record");
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = execute_fixed_static_file_with_record_hook(
            &lease, &namespace, &scope, "copy", &selected, &stop,
        );
    }));
    assert!(crashed.is_err());
    assert!(root.path().join(RECORD).is_file());
    assert_eq!(
        std::fs::read(root.path().join(OUTPUT_FILE)).unwrap(),
        b"copied once"
    );
    let inspected =
        execute_fixed_static_file(&lease, &namespace, &scope, "copy", &selected).unwrap();
    let StaticFileJournalInspection::Uncertain { partial, .. } = inspected else {
        panic!("record without owner result is uncertain")
    };
    assert!(matches!(partial.output_file, ObservedFile::Present { .. }));
    assert!(matches!(
        partial.artifact_record,
        ObservedFile::Present { .. }
    ));
    assert_eq!(
        std::fs::read(root.path().join(OUTPUT_FILE)).unwrap(),
        b"copied once"
    );
}

#[test]
fn completed_record_or_output_drift_downgrades_inspection_without_copy() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    assert!(matches!(
        execute_fixed_static_file(&lease, &namespace, &scope, "copy", &selected).unwrap(),
        StaticFileJournalInspection::Completed { .. }
    ));
    std::fs::write(root.path().join(OUTPUT_FILE), b"changed").unwrap();
    assert!(matches!(
        execute_fixed_static_file(&lease, &namespace, &scope, "copy", &selected).unwrap(),
        StaticFileJournalInspection::Uncertain { .. }
    ));
    assert_eq!(
        std::fs::read(root.path().join(OUTPUT_FILE)).unwrap(),
        b"changed"
    );
}

#[test]
fn intent_plan_digest_mismatch_with_c1_refuses_effect() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"source").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let first =
        reserve_fixed_static_file_intent(&lease, &namespace, &scope, "copy", &selected).unwrap();
    let StaticFileJournalInspection::IntentReady { run_id } = first else {
        panic!("new intent")
    };
    let path = root
        .path()
        .join(".vibe/package-operations")
        .join(intent_name(&run_id));
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["plan_sha256"] = serde_json::Value::String(format!("sha256:{}", "0".repeat(64)));
    std::fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    let inspected =
        execute_fixed_static_file(&lease, &namespace, &scope, "copy", &selected).unwrap();
    assert_eq!(
        inspected,
        StaticFileJournalInspection::RecoveryRequired { run_id }
    );
    assert!(!root.path().join(OUTPUT_DIR).exists());
    assert!(!root.path().join(RECORD).exists());
}

#[test]
fn source_drift_after_ready_refuses_before_static_file_copy() {
    let root = temp();
    std::fs::write(root.path().join(SOURCE), b"initial").unwrap();
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let drift = || std::fs::write(root.path().join(SOURCE), b"changed").unwrap();
    let first = execute_fixed_static_file_with_pre_execute_hook(
        &lease, &namespace, &scope, "copy", &selected, &drift,
    )
    .unwrap();
    assert!(matches!(
        first,
        StaticFileJournalInspection::Uncertain { .. }
    ));
    assert!(!root.path().join(OUTPUT_DIR).exists());
    assert!(!root.path().join(RECORD).exists());
}
