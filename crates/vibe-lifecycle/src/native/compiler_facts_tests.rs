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
            .finish_ready()
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
            .finish_ready()
            .unwrap_err()
            .to_string()
            .contains("conflicts")
    );

    let taken = PendingFactRecorder::new();
    taken.finish_ready().expect("FACTS fixture");
    assert!(
        taken
            .finish_ready()
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
            .finish_ready()
            .unwrap_err()
            .to_string()
            .contains("poisoned")
    );
}

#[test]
fn prebuilt_success_and_hard_failure_record_no_pending_fact() {
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
    invoker.finish_ready().expect("FACTS fixture");

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
    missing_invoker.finish_ready().expect("FACTS fixture");
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
    invoker.finish_ready().expect("FACTS fixture");
}

#[test]
fn lifecycle_provider_drives_real_pending_fact_join_through_bound_analyzer() {
    let root = tempdir().expect("bound analyzer fixture");
    write_source_tree(root.path());
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.demo'\nname='compiler-host'\nversion='0.1.0'\n\n\
         [requires.packages]\n'org.fixture/content'={version='=1.0.0',link='static'}\n\n\
         [[extension]]\nid='source'\npoint='compile:source'\n\
         handler={kind='native',crate_dir='native'}\n",
    )
    .expect("workspace manifest");
    let group = vibe_core::Group::parse("org.fixture").expect("group");
    let version = "1.0.0".parse().expect("version");
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "content", &version);
    fs::create_dir_all(slot.join("boot")).expect("package boot directory");
    fs::write(
        slot.join("vibe.toml"),
        "[package]\ngroup='org.fixture'\nname='content'\nkind='tool'\nversion='1.0.0'\n\n\
         [boot_snippet]\nsource='boot/content.md'\nlink='static'\n",
    )
    .expect("package manifest");
    fs::write(slot.join("boot/content.md"), "# Input\n\nbody\n").expect("package boot input");
    let workspace = vibe_workspace::Workspace::load(root.path()).expect("workspace");
    let resolution = [vibe_workspace::install::ResolvedDep {
        kind: vibe_core::PackageKind::Tool,
        group,
        name: "content".to_owned(),
        version,
        content_dir: slot.clone(),
        source_hash: Some(vibe_core::ContentHash::parse("sha256:aa").expect("content hash")),
        manifest: vibe_core::manifest::Manifest::read(slot.join("vibe.toml"))
            .expect("package manifest"),
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    }];
    let lowered = vibe_workspace::extension_world::lower_owner_runtimes(
        &workspace,
        &vibe_workspace::extension_world::ExtensionWorldEpoch::from_resolution(
            root.path(),
            &resolution,
        )
        .expect("extension world"),
        vibe_workspace::extension_world::OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .expect("owner runtimes");
    let owner = vibe_workspace::extension_world::OwnerRuntimeId::Node {
        rel: ".".to_owned(),
    };
    let epoch = lowered.bind_run(vibe_workspace::extension_world::OwnerRuntimeRunFacts {
        run_id: RUN_ID.to_owned(),
        state_root: root.path().join(".vibe"),
        platform: current_platform().key().to_owned(),
        offline: true,
        created_at: "2026-09-01T00:00:00Z".to_owned(),
    });
    let mut provider = ArtifactCompilerNativeProvider::new(
        current_platform(),
        BTreeMap::from([(owner, vibe_spec::CompilerNativePolicy::collect())]),
    );
    let generated = root
        .path()
        .join(vibe_core::layout::current_boot_dir())
        .join(vibe_workspace::boot_artifacts::STATIC_FILE);
    assert!(!generated.exists());
    let analyzed = vibe_workspace::install::analyze_node_lane_bound_native(
        &workspace,
        ".",
        &resolution,
        &epoch,
        Some(&mut provider),
        None,
    )
    .expect("bound analyzer")
    .expect("static artifact");
    let text = std::str::from_utf8(analyzed.artifact.bytes()).expect("static utf8");
    assert!(text.contains("vibe:transforms-pending"));
    assert!(matches!(
        analyzed.native,
        Some(vibe_workspace::boot_artifacts::OwnerNativeCompileContinuation::Pending { .. })
    ));
    assert!(
        !generated.exists(),
        "the real analyzer publishes no boot bytes"
    );
    assert!(
        !root.path().join("target").exists(),
        "the analyzer performs no Cargo build"
    );
}

#[test]
fn lifecycle_replay_factory_terminally_refuses_leftover_policies() {
    use vibe_workspace::extension_world::CompilerNativeReplayFactory;

    let mut factory =
        crate::native::compiler::ArtifactCompilerNativeReplayFactory::new(current_platform());
    let empty = factory
        .create(BTreeMap::new())
        .expect("empty replay provider");
    factory.finish(empty).expect("empty provider finishes");

    let owner = vibe_workspace::extension_world::OwnerRuntimeId::Node {
        rel: ".".to_owned(),
    };
    let leftover = factory
        .create(BTreeMap::from([(
            owner,
            vibe_spec::CompilerNativePolicy::fail(),
        )]))
        .expect("leftover provider");
    let error = factory.finish(leftover).expect_err("leftover must refuse");
    assert!(error.to_string().contains("not consumed"));
}
