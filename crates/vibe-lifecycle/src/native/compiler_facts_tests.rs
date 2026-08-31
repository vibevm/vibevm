use std::panic::{AssertUnwindSafe, catch_unwind};

use vibe_core::manifest::ExtensionKey;
use vibe_spec::{CompilerPendingSet, TransformPlan};
use vibe_workspace::extension_world::CompilerNativeFactBinding;

use super::*;
use crate::native::compiler_facts::{PendingFactRecorder, pending_source_capture};

fn source_stage_native(id: &str, config: Option<ExtensionConfig>) -> ExtensionDecl {
    let mut declaration = declaration(
        id,
        ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("native")),
            prebuilt: None,
        },
        "compile:source",
        config,
    );
    declaration.compiler_internals = None;
    declaration
}

fn write_source_tree(root: &Path) {
    if let Err(error) = fs::create_dir_all(root.join("native/src")) {
        panic!("FACTS fixture directory: {error}");
    }
    if let Err(error) = fs::write(
        root.join("native/Cargo.toml"),
        "[package]\nname='source'\nversion='0.1.0'\nedition='2024'\n[lib]\ncrate-type=['cdylib']\n",
    ) {
        panic!("FACTS fixture manifest: {error}");
    }
    if let Err(error) = fs::write(root.join("native/src/lib.rs"), "pub fn marker() {}\n") {
        panic!("FACTS fixture source: {error}");
    }
}

fn pending_set(
    row: &ExtensionRegistryRow,
    entries: Vec<(u32, ExtensionKey)>,
) -> CompilerPendingSet {
    let Ok(plan) = TransformPlan::from_effective_rows(&[row]) else {
        panic!("FACTS fixture plan");
    };
    let Ok(pending) = CompilerPendingSet::from_plan_entries_for_test(&plan, entries) else {
        panic!("FACTS fixture pending set");
    };
    pending
}

#[test]
fn exact_missing_source_records_one_coalesced_fact_and_is_one_shot() {
    let root = tempdir().expect("FACTS fixture");
    write_source_tree(root.path());
    let (registry, mechanisms) = registries(root.path(), vec![source_stage_native("source", None)]);
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project_value = project(root.path());
    let world_value = world();
    let invoker = make_invoker(
        &rows,
        &rows,
        root.path(),
        &mechanisms,
        &routes,
        &project_value,
        &world_value,
        RUN_ID,
    );
    let binding: &dyn CompilerNativeFactBinding = &invoker;
    let config = effective_config(rows[0]).expect("FACTS fixture");
    invoker
        .request_for_test(call(rows[0], 0, &config, CompilePoint::Source, rows[0]))
        .expect("prewarm the shared scratch identity before scoped concurrency");
    let outcomes = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            binding
                .invoker()
                .invoke(call(rows[0], 0, &config, CompilePoint::Source, rows[0]))
        });
        let second = scope.spawn(|| {
            binding
                .invoker()
                .invoke(call(rows[0], 0, &config, CompilePoint::Source, rows[0]))
        });
        [
            first.join().expect("FACTS scoped call"),
            second.join().expect("FACTS scoped call"),
        ]
    });
    for outcome in outcomes {
        let error = outcome.unwrap_err();
        assert_eq!(
            error.kind(),
            CompilerNativeInvokerErrorKind::BuildableSourceUnavailable
        );
    }
    let expected = pending_set(rows[0], vec![(0, rows[0].key().clone())]);
    let facts = binding
        .take_pending_build_facts(&expected)
        .expect("FACTS fixture");
    assert_eq!(facts.len(), 1, "repeated manager calls coalesce by order");
    let debug = format!("{:?}", &facts[0]);
    assert!(debug.contains("build:cargo"));
    assert!(debug.contains(current_platform().key()));
    assert!(
        binding.take_pending_build_facts(&expected).is_err(),
        "the recorder is terminal after its one drain"
    );
    let after_take = binding
        .invoker()
        .invoke(call(rows[0], 0, &config, CompilePoint::Source, rows[0]))
        .unwrap_err();
    assert_eq!(
        after_take.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed,
        "a terminal recorder failure can never masquerade as collectable unavailability"
    );
    assert!(!root.path().join("target").exists());
}

fn capture_fixture<'a>(
    execution: &'a NativeBuildExecution<'a>,
    row: &ExtensionRegistryRow,
) -> crate::native::compiler_facts::PendingFactCapture {
    let provider = crate::native::provider::facts(row);
    let Ok(rows) = crate::native::source_group_rows(
        execution.candidates,
        &provider.identity,
        "native",
        execution.platform,
    ) else {
        panic!("FACTS fixture source group");
    };
    let Ok(source) = crate::native::witness::source_witness_digest(&provider) else {
        panic!("FACTS fixture source witness");
    };
    let config = crate::native::witness::config_witness_digest(&rows);
    let Ok(selected) = crate::native::select_build_provider(execution) else {
        panic!("FACTS fixture build provider");
    };
    let Ok(capture) = pending_source_capture(row, 0, execution.platform, source, config, &selected)
    else {
        panic!("FACTS fixture capture");
    };
    capture
}

#[test]
fn recorder_distinguishes_repeat_missing_extra_conflict_poison_and_taken() {
    let root = tempdir().expect("FACTS fixture");
    write_source_tree(root.path());
    let (registry, mechanisms) = registries(root.path(), vec![source_stage_native("source", None)]);
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let execution = execution(&rows, root.path(), &mechanisms, &routes);
    let capture = capture_fixture(&execution, rows[0]);
    let expected = pending_set(rows[0], vec![(0, rows[0].key().clone())]);
    let empty = pending_set(rows[0], Vec::new());

    let repeated = PendingFactRecorder::new();
    repeated.record(capture.clone()).expect("FACTS fixture");
    repeated.record(capture.clone()).expect("FACTS fixture");
    assert_eq!(repeated.take(&expected).expect("FACTS fixture").len(), 1);

    let missing = PendingFactRecorder::new();
    assert!(
        missing
            .take(&expected)
            .unwrap_err()
            .to_string()
            .contains("missing")
    );

    let extra = PendingFactRecorder::new();
    extra.record(capture.clone()).expect("FACTS fixture");
    assert!(
        extra
            .take(&empty)
            .unwrap_err()
            .to_string()
            .contains("extra")
    );

    let conflict = PendingFactRecorder::new();
    let mut changed = capture.clone();
    changed.source = vibe_workspace::extension_world::PendingSourceWitness::new([9; 32]);
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| conflict.record(changed));
        let second = scope.spawn(|| conflict.record(capture.clone()));
        [
            first.join().expect("FACTS scoped record"),
            second.join().expect("FACTS scoped record"),
        ]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert!(
        conflict
            .take(&expected)
            .unwrap_err()
            .to_string()
            .contains("conflicts")
    );

    let taken = PendingFactRecorder::new();
    assert!(taken.take(&empty).expect("FACTS fixture").is_empty());
    assert!(
        taken
            .take(&empty)
            .unwrap_err()
            .to_string()
            .contains("already taken")
    );

    let poisoned = PendingFactRecorder::new();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        poisoned.poison_for_test(|| panic!("poison fact recorder"));
    }));
    assert!(
        poisoned
            .take(&empty)
            .unwrap_err()
            .to_string()
            .contains("poisoned")
    );
}

#[test]
fn prebuilt_success_and_hard_failure_record_no_pending_fact() {
    let source_root = tempdir().expect("FACTS fixture");
    let (source_registry, _) = registries(
        source_root.path(),
        vec![source_stage_native("expected", None)],
    );
    let empty = pending_set(&source_registry.rows()[0], Vec::new());

    let root = tempdir().expect("FACTS fixture");
    let relative = fixture(root.path());
    let (registry, mechanisms) = registries(
        root.path(),
        vec![native("compiler-ok", Some(&relative), None)],
    );
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project_value = project(root.path());
    let world_value = world();
    let invoker = make_invoker(
        &rows,
        &rows,
        root.path(),
        &mechanisms,
        &routes,
        &project_value,
        &world_value,
        RUN_ID,
    );
    let config = effective_config(rows[0]).expect("FACTS fixture");
    assert!(
        invoker
            .invoke(call(rows[0], 0, &config, CompilePoint::Pass, rows[0]))
            .is_ok()
    );
    assert!(
        invoker
            .take_pending_build_facts(&empty)
            .expect("FACTS fixture")
            .is_empty()
    );

    let missing_root = tempdir().expect("FACTS fixture");
    let missing_path = PathBuf::from(format!("missing{}", current_platform().suffix()));
    let (missing_registry, missing_mechanisms) = registries(
        missing_root.path(),
        vec![native("compiler-ok", Some(&missing_path), None)],
    );
    let missing_rows = missing_registry.rows().iter().collect::<Vec<_>>();
    let missing_project = project(missing_root.path());
    let missing_world = world();
    let missing_invoker = make_invoker(
        &missing_rows,
        &missing_rows,
        missing_root.path(),
        &missing_mechanisms,
        &routes,
        &missing_project,
        &missing_world,
        RUN_ID,
    );
    let missing_config = effective_config(missing_rows[0]).expect("FACTS fixture");
    assert_eq!(
        missing_invoker
            .invoke(call(
                missing_rows[0],
                0,
                &missing_config,
                CompilePoint::Pass,
                missing_rows[0],
            ))
            .unwrap_err()
            .kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    assert!(
        missing_invoker
            .take_pending_build_facts(&empty)
            .expect("FACTS fixture")
            .is_empty()
    );
}

#[test]
fn valid_source_record_and_later_loader_failure_record_no_pending_fact() {
    let root = tempdir().expect("FACTS fixture");
    write_source_tree(root.path());
    let (registry, mechanisms) = registries(root.path(), vec![source_stage_native("source", None)]);
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    build_native_sources(&execution(&rows, root.path(), &mechanisms, &routes))
        .expect("FACTS fixture");

    let project_value = project(root.path());
    let world_value = world();
    let invoker = make_invoker(
        &rows,
        &rows,
        root.path(),
        &mechanisms,
        &routes,
        &project_value,
        &world_value,
        RUN_ID,
    );
    let config = effective_config(rows[0]).expect("FACTS fixture");
    let error = invoker
        .invoke(call(rows[0], 0, &config, CompilePoint::Source, rows[0]))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    let empty = pending_set(rows[0], Vec::new());
    assert!(
        invoker
            .take_pending_build_facts(&empty)
            .expect("FACTS fixture")
            .is_empty()
    );
}
