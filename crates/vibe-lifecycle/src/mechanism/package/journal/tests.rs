//! The first protected package owner stops at a durable intent, before copy.

use super::*;

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
    assert_eq!(second, StaticFileJournalInspection::Uncertain { run_id });
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
