//! Fixed Cargo owner intent; no toolchain process runs in this slice.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactKind, ArtifactOutput, ExtensionsControl, MechanismRoutes,
};
use vibe_extension_registry::collect_mechanisms;

use crate::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(MANIFEST), MANIFEST_BYTES).unwrap();
    std::fs::create_dir_all(root.path().join("src")).unwrap();
    std::fs::write(root.path().join(MAIN), MAIN_BYTES).unwrap();
    root
}

fn target() -> ArtifactBuildTarget {
    let select =
        toml::from_str::<toml::Table>("package = 'vibe-r8-fixture'\nbin = 'r8probe'\n").unwrap();
    let config = toml::from_str::<toml::Table>("profile = 'release'\noffline = true\n").unwrap();
    ArtifactBuildTarget {
        id: TARGET.into(),
        mechanism: "build:cargo".parse().unwrap(),
        provider: None,
        workdir: ".".into(),
        inputs: None,
        outputs: vec![ArtifactOutput {
            id: OUTPUT.into(),
            kind: ArtifactKind::Executable,
            select: Some(vibe_core::manifest::ExtensionConfig::from_table(select)),
        }],
        config: Some(vibe_core::manifest::ExtensionConfig::from_table(config)),
    }
}

fn registry() -> vibe_extension_registry::MechanismRegistry {
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: ".".into(),
                version: "1.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    collect_mechanisms(&world).unwrap()
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
    targets: &'a [ArtifactBuildTarget],
    registry: &'a vibe_extension_registry::MechanismRegistry,
    routes: &'a MechanismRoutes,
) -> BuildExecution<'a> {
    BuildExecution {
        project_root: root,
        targets,
        registry,
        routes,
        build_root: BuildExecution::default_build_root(),
        offline: true,
        created_at: "2026-10-01T00:00:00Z",
    }
}

#[test]
fn retired_unreserved_cargo_key_remains_retired_without_intent_or_effect() {
    let root = fixture();
    let targets = [target()];
    let registry = registry();
    let routes = MechanismRoutes::default();
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    SubmissionReservations::retire_namespace(&lease, &namespace, &scope).unwrap();
    assert_eq!(
        reserve_fixed_cargo_intent(&lease, &namespace, &scope, "build", &selected).unwrap(),
        BuildJournalInspection::Retired { run_id: None }
    );
    assert!(!root.path().join(".vibe/build-operations").exists());
    assert!(!root.path().join("target").exists());
}

#[test]
fn fixed_cargo_intent_is_ready_before_target_lock_or_record_exists() {
    let root = fixture();
    let targets = [target()];
    let registry = registry();
    let routes = MechanismRoutes::default();
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let first = reserve_fixed_cargo_intent(&lease, &namespace, &scope, "build", &selected).unwrap();
    let BuildJournalInspection::IntentReady { run_id } = first else {
        panic!("new Cargo intent")
    };
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope, "build", &run_id)
            .unwrap(),
        JournalAdmission::Ready
    );
    assert!(
        root.path()
            .join(".vibe/build-operations")
            .join(intent_name(&run_id))
            .is_file()
    );
    for path in ABSENT {
        assert!(!root.path().join(path).exists(), "{path}");
    }
    SubmissionReservations::retire_namespace(&lease, &namespace, &scope).unwrap();
    let second =
        reserve_fixed_cargo_intent(&lease, &namespace, &scope, "build", &selected).unwrap();
    let BuildJournalInspection::Uncertain {
        run_id: seen,
        partial,
    } = second
    else {
        panic!("ready intent is uncertain")
    };
    assert_eq!(seen, run_id);
    assert_eq!(partial.target_root, ObservedPresence::Absent);
}

#[test]
fn changed_source_or_foreign_scope_refuses_without_cargo_effect() {
    let root = fixture();
    let targets = [target()];
    let registry = registry();
    let routes = MechanismRoutes::default();
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let foreign = SubmissionScope {
        principal: "other".into(),
        ..scope.clone()
    };
    assert!(matches!(
        reserve_fixed_cargo_intent(&lease, &namespace, &foreign, "build", &selected),
        Err(BuildJournalError::Submission(
            crate::SubmissionError::ForeignScope
        ))
    ));
    assert!(matches!(
        reserve_fixed_cargo_intent(&lease, &namespace, &scope, "build", &selected).unwrap(),
        BuildJournalInspection::IntentReady { .. }
    ));
    std::fs::write(root.path().join(MAIN), b"fn main() {}\n").unwrap();
    assert!(reserve_fixed_cargo_intent(&lease, &namespace, &scope, "build", &selected).is_err());
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("Cargo.lock").exists());
}

#[test]
fn prior_target_or_record_refuses_before_intent() {
    for occupied in ["target", ".vibe/state/artifacts/fixture-tool.bin.json"] {
        let root = fixture();
        let targets = [target()];
        let registry = registry();
        let routes = MechanismRoutes::default();
        let selected = execution(root.path(), &targets, &registry, &routes);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let path = root.path().join(occupied);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"occupant").unwrap();
        assert!(
            reserve_fixed_cargo_intent(&lease, &namespace, &scope, "build", &selected).is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), b"occupant");
        assert!(!root.path().join(".vibe/build-operations").exists());
    }
}

#[test]
fn reserved_run_without_cargo_intent_requires_recovery() {
    let root = fixture();
    let targets = [target()];
    let registry = registry();
    let routes = MechanismRoutes::default();
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let initial = fresh_preflight(
        &lease,
        &selected,
        subject(&lease, &selected).unwrap(),
        &namespace,
        "build",
    )
    .unwrap();
    let run_id = propose_run_id(root.path()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &scope,
            "build",
            &initial.request,
            &initial.plan,
            &run_id,
        )
        .unwrap(),
        ReservationDecision::Created { .. }
    ));
    SubmissionReservations::retire_namespace(&lease, &namespace, &scope).unwrap();
    assert_eq!(
        reserve_fixed_cargo_intent(&lease, &namespace, &scope, "build", &selected).unwrap(),
        BuildJournalInspection::RecoveryRequired { run_id }
    );
    assert!(!root.path().join(".vibe/build-operations").exists());
}

#[test]
fn one_real_offline_cargo_build_retains_verified_result_and_never_respawns() {
    let root = fixture();
    let targets = [target()];
    let registry = registry();
    let routes = MechanismRoutes::default();
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let calls = AtomicUsize::new(0);
    let counted = |_: &str, _: &[String]| {
        calls.fetch_add(1, Ordering::SeqCst);
    };
    let first = execute_fixed_cargo_with_hooks(
        &lease,
        &namespace,
        &scope,
        "build",
        &selected,
        Some(&counted),
        None,
    )
    .unwrap();
    let BuildJournalInspection::Completed {
        run_id,
        output_relative,
        output_sha256,
        artifact_record,
        warning,
    } = first
    else {
        panic!("Cargo build must have a verified result: {first:?}")
    };
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert!(root.path().join(&output_relative).is_file());
    assert_eq!(output_sha256.len(), 64);
    assert_eq!(
        artifact_record,
        ".vibe/state/artifacts/fixture-tool.bin.json"
    );
    assert!(warning.contains("rustup"));
    let saved_run_id = run_id.clone();
    let saved_output_relative = output_relative.clone();
    SubmissionReservations::retire_namespace(&lease, &namespace, &scope).unwrap();
    let second = execute_fixed_cargo_with_hooks(
        &lease,
        &namespace,
        &scope,
        "build",
        &selected,
        Some(&counted),
        None,
    )
    .unwrap();
    assert_eq!(
        second,
        BuildJournalInspection::Completed {
            run_id,
            output_relative,
            output_sha256,
            artifact_record,
            warning,
        }
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    let output_path = root.path().join(&saved_output_relative);
    let original = std::fs::read(&output_path).unwrap();
    std::fs::write(&output_path, b"changed behind A2 record").unwrap();
    assert!(matches!(
        execute_fixed_cargo_with_hooks(
            &lease,
            &namespace,
            &scope,
            "build",
            &selected,
            Some(&counted),
            None,
        )
        .unwrap(),
        BuildJournalInspection::Uncertain { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    std::fs::write(&output_path, original).unwrap();
    assert!(matches!(
        execute_fixed_cargo_with_hooks(
            &lease,
            &namespace,
            &scope,
            "build",
            &selected,
            Some(&counted),
            None,
        )
        .unwrap(),
        BuildJournalInspection::Completed { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    let result = root
        .path()
        .join(".vibe/build-operations")
        .join(format!("{saved_run_id}.cargo-result.json"));
    std::fs::remove_file(result).unwrap();
    assert!(matches!(
        execute_fixed_cargo_with_hooks(
            &lease,
            &namespace,
            &scope,
            "build",
            &selected,
            Some(&counted),
            None,
        )
        .unwrap(),
        BuildJournalInspection::Uncertain { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[test]
fn crash_after_cargo_apply_before_record_is_partial_and_never_respawns() {
    let root = fixture();
    let targets = [target()];
    let registry = registry();
    let routes = MechanismRoutes::default();
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let calls = AtomicUsize::new(0);
    let counted = |_: &str, _: &[String]| {
        calls.fetch_add(1, Ordering::SeqCst);
    };
    let stop = || panic!("injected crash after Cargo apply");
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = execute_fixed_cargo_with_hooks(
            &lease,
            &namespace,
            &scope,
            "build",
            &selected,
            Some(&counted),
            Some(&stop),
        );
    }));
    assert!(crashed.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert!(
        !root
            .path()
            .join(".vibe/state/artifacts/fixture-tool.bin.json")
            .exists()
    );
    let retry = execute_fixed_cargo_with_hooks(
        &lease,
        &namespace,
        &scope,
        "build",
        &selected,
        Some(&counted),
        None,
    )
    .unwrap();
    let BuildJournalInspection::Uncertain { partial, .. } = retry else {
        panic!("crashed build must remain uncertain")
    };
    assert_eq!(partial.target_root, ObservedPresence::Present);
    assert_eq!(partial.cargo_home, ObservedPresence::Present);
    assert_eq!(partial.artifact_record, ObservedFile::Absent);
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[test]
fn cold_build_attach_recovers_completed_without_execution_inputs_or_spawn() {
    let root = fixture();
    let trusted = scope();
    let calls = AtomicUsize::new(0);
    let (token, original) = {
        let targets = [target()];
        let registry = registry();
        let routes = MechanismRoutes::default();
        let selected = execution(root.path(), &targets, &registry, &routes);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let counted = |_: &str, _: &[String]| {
            calls.fetch_add(1, Ordering::SeqCst);
        };
        let result = execute_fixed_cargo_with_hooks(
            &lease,
            &namespace,
            &trusted,
            "cold-build",
            &selected,
            Some(&counted),
            None,
        )
        .unwrap();
        assert!(matches!(result, BuildJournalInspection::Completed { .. }));
        (namespace.id().to_owned(), result)
    };
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let cold = inspect_fixed_cargo_cold(&lease, &token, &trusted, "cold-build")
        .unwrap()
        .unwrap();
    assert_eq!(cold, original);
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..trusted.clone()
    };
    assert!(matches!(
        inspect_fixed_cargo_cold(&lease, &token, &foreign, "cold-build"),
        Err(BuildJournalError::Submission(
            crate::SubmissionError::ForeignScope
        ))
    ));
    let BuildJournalInspection::Completed { run_id, .. } = original else {
        unreachable!()
    };
    let capsule = root
        .path()
        .join(".vibe/build-operations")
        .join(format!("{run_id}.cargo-input.json"));
    let original_capsule = std::fs::read(&capsule).unwrap();
    std::fs::write(&capsule, b"{\"tampered\":true}").unwrap();
    let malformed = inspect_fixed_cargo_cold(&lease, &token, &trusted, "cold-build")
        .unwrap()
        .unwrap();
    assert!(
        matches!(malformed, BuildJournalInspection::Uncertain { run_id: same, partial }
        if same == run_id && matches!(partial.artifact_record, ObservedFile::Present { .. }))
    );
    let mut changed: serde_json::Value = serde_json::from_slice(&original_capsule).unwrap();
    changed["request"] = serde_json::json!("{}");
    std::fs::write(&capsule, serde_json::to_vec(&changed).unwrap()).unwrap();
    let tampered = inspect_fixed_cargo_cold(&lease, &token, &trusted, "cold-build")
        .unwrap()
        .unwrap();
    assert!(
        matches!(tampered, BuildJournalInspection::Uncertain { run_id: same, .. }
        if same == run_id)
    );
    std::fs::remove_file(&capsule).unwrap();
    let missing = inspect_fixed_cargo_cold(&lease, &token, &trusted, "cold-build")
        .unwrap()
        .unwrap();
    assert!(
        matches!(missing, BuildJournalInspection::Uncertain { run_id: same, .. }
        if same == run_id)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[test]
fn cold_build_attach_retains_ready_without_result_as_uncertain() {
    let root = fixture();
    let trusted = scope();
    let (token, run_id) = {
        let targets = [target()];
        let registry = registry();
        let routes = MechanismRoutes::default();
        let selected = execution(root.path(), &targets, &registry, &routes);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let first =
            reserve_fixed_cargo_intent(&lease, &namespace, &trusted, "cold-pending", &selected)
                .unwrap();
        let BuildJournalInspection::IntentReady { run_id } = first else {
            panic!("fixed intent must be ready")
        };
        (namespace.id().to_owned(), run_id)
    };
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let cold = inspect_fixed_cargo_cold(&lease, &token, &trusted, "cold-pending")
        .unwrap()
        .unwrap();
    let BuildJournalInspection::Uncertain {
        run_id: seen,
        partial,
    } = cold
    else {
        panic!("ready intent without result must remain uncertain")
    };
    assert_eq!(seen, run_id);
    assert_eq!(partial.target_root, ObservedPresence::Absent);
    assert_eq!(partial.cargo_lock, ObservedFile::Absent);
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("Cargo.lock").exists());
}

#[test]
fn changed_input_capsule_cannot_complete_or_restart_warm_build() {
    let root = fixture();
    let targets = [target()];
    let registry = registry();
    let routes = MechanismRoutes::default();
    let selected = execution(root.path(), &targets, &registry, &routes);
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let trusted = scope();
    let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
    let BuildJournalInspection::IntentReady { run_id } =
        reserve_fixed_cargo_intent(&lease, &namespace, &trusted, "capsule-drift", &selected)
            .unwrap()
    else {
        panic!("fresh fixed intent required")
    };
    let capsule = root
        .path()
        .join(".vibe/build-operations")
        .join(format!("{run_id}.cargo-input.json"));
    std::fs::write(capsule, b"{\"tampered\":true}").unwrap();
    let seen = reserve_fixed_cargo_intent(&lease, &namespace, &trusted, "capsule-drift", &selected)
        .unwrap();
    assert!(matches!(seen, BuildJournalInspection::Uncertain { .. }));
    assert!(!root.path().join("target").exists());
}
