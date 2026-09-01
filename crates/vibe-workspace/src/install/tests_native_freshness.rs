use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tempfile::TempDir;
use vibe_core::PackageName;
use vibe_core::manifest::SpecFormat;
use vibe_extension_registry::DependencyProviderId;
use vibe_spec::CompilerNativePolicy;
use vibe_wire::generated::compiler_trace_index::e1::index::{CompilerTraceIndex, ScopeStatus};
use vibe_wire::generated::shared::Timestamp;

use super::*;
use crate::boot_artifacts::native_managed_tests::{FakeProvider, Reply};
use crate::compile_trace::{TraceLimits, TraceRun};

macro_rules! test_ok {
    ($value:expr, $message:literal) => {
        match $value {
            Ok(value) => value,
            Err(error) => panic!("{}: {error:?}", $message),
        }
    };
}

macro_rules! test_some {
    ($value:expr, $message:literal) => {
        match $value {
            Some(value) => value,
            None => panic!("{}", $message),
        }
    };
}
use crate::extension_world::{
    ExtensionWorldEpoch, OwnerRuntimeEpoch, OwnerRuntimeId, OwnerRuntimeLowering,
    OwnerRuntimeRunFacts, lower_owner_runtimes,
};

struct NativeGraph {
    _root: TempDir,
    workspace: Workspace,
    resolution: Vec<ResolvedDep>,
    epoch: OwnerRuntimeEpoch,
    middle: OwnerRuntimeId,
}

const COMPILE_NATIVE: &str = "[[extension]]\nid='native'\npoint='compile:emitted'\n\
handler={kind='native',crate_dir='native'}\n";

fn native_graph() -> NativeGraph {
    native_graph_with(COMPILE_NATIVE, "")
}

fn native_graph_with(middle_extension: &str, leaf_extension: &str) -> NativeGraph {
    let root = test_ok!(TempDir::new(), "native freshness workspace");
    write(
        &root.path().join("vibe.toml"),
        "[project]\ngroup='org.demo'\nname='host'\nversion='0.1.0'\n\n\
         [requires.packages]\n'org.lock/top'={version='=1.0.0',link='static'}\n",
    );
    slot(
        root.path(),
        "top",
        "[package]\ngroup='org.lock'\nname='top'\nkind='tool'\nversion='1.0.0'\n\n\
         [requires.packages]\n'org.lock/middle'={version='=1.0.0',link='static'}\n\n\
         [boot_snippet]\nsource='boot/top.md'\nlink='static'\n",
    );
    slot(
        root.path(),
        "middle",
        &format!(
            "[package]\ngroup='org.lock'\nname='middle'\nkind='tool'\nversion='1.0.0'\n\n\
         [requires.packages]\n'org.lock/leaf'={{version='=1.0.0',link='static'}}\n\n\
         [boot_snippet]\nsource='boot/middle.md'\nlink='static'\n\n{middle_extension}"
        ),
    );
    slot(
        root.path(),
        "leaf",
        &format!(
            "[package]\ngroup='org.lock'\nname='leaf'\nkind='tool'\nversion='1.0.0'\n\n\
         [boot_snippet]\nsource='boot/leaf.md'\nlink='static'\n\n{leaf_extension}"
        ),
    );
    slot(
        root.path(),
        "dynamic",
        "[package]\ngroup='org.lock'\nname='dynamic'\nkind='tool'\nversion='1.0.0'\n\n\
         [requires.packages]\n'org.lock/middle'={version='=1.0.0',link='dynamic'}\n\
         'org.lock/support'={version='=1.0.0',link='static'}\n\n\
         [boot_snippet]\nsource='boot/dynamic.md'\nlink='static'\n",
    );
    slot(
        root.path(),
        "support",
        "[package]\ngroup='org.lock'\nname='support'\nkind='tool'\nversion='1.0.0'\n\n\
         [boot_snippet]\nsource='boot/support.md'\nlink='static'\n",
    );
    for (name, body) in [
        ("top", "# top\n"),
        ("middle", "# middle\n"),
        ("leaf", "# leaf\n"),
        ("dynamic", "# dynamic\n"),
        ("support", "# support\n"),
    ] {
        let path = crate::vibedeps::slot_abs_path(root.path(), &group(), name, &version())
            .join(format!("boot/{name}.md"));
        write(&path, body);
    }
    let resolution = vec![
        resolved(root.path(), "top", "sha256:11", &["org.lock/middle@=1.0.0"]),
        resolved(
            root.path(),
            "middle",
            "sha256:22",
            &["org.lock/leaf@=1.0.0"],
        ),
        resolved(root.path(), "leaf", "sha256:33", &[]),
        resolved(
            root.path(),
            "dynamic",
            "sha256:44",
            &["org.lock/middle@=1.0.0", "org.lock/support@=1.0.0"],
        ),
        resolved(root.path(), "support", "sha256:55", &[]),
    ];
    write_lock(
        root.path(),
        vec![
            locked("top", "sha256:11", &["org.lock/middle@=1.0.0"]),
            locked("middle", "sha256:22", &["org.lock/leaf@=1.0.0"]),
            locked("leaf", "sha256:33", &[]),
            locked(
                "dynamic",
                "sha256:44",
                &["org.lock/middle@=1.0.0", "org.lock/support@=1.0.0"],
            ),
            locked("support", "sha256:55", &[]),
        ],
    );
    let workspace = test_ok!(Workspace::load(root.path()), "workspace");
    let world = test_ok!(
        ExtensionWorldEpoch::from_resolution(root.path(), &resolution),
        "world"
    );
    let epoch = test_ok!(
        lower_owner_runtimes(
            &workspace,
            &world,
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        ),
        "runtimes"
    )
    .bind_run(OwnerRuntimeRunFacts {
        run_id: "0123456789abcdef0123456789abcdef".to_owned(),
        state_root: root.path().join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-01T00:00:00Z".to_owned(),
    });
    let middle = OwnerRuntimeId::Unit {
        provider: DependencyProviderId::new(
            group(),
            test_ok!(PackageName::parse("middle"), "package name"),
        ),
    };
    NativeGraph {
        _root: root,
        workspace,
        resolution,
        epoch,
        middle,
    }
}

fn regenerate(
    graph: &NativeGraph,
    provider: &mut FakeProvider,
) -> bootgen::native_managed::BoundBootRegeneration {
    test_ok!(
        bootgen::native_managed::regenerate_boot_from_bound_native(
            &graph.workspace,
            &graph.resolution,
            SpecFormat::Mixed,
            None,
            &graph.epoch,
            Some(provider),
        ),
        "bound regeneration"
    )
}

fn regenerate_traced(
    graph: &NativeGraph,
    provider: &mut FakeProvider,
    trace: &TraceRun,
) -> Result<bootgen::native_managed::BoundBootRegeneration, WorkspaceError> {
    bootgen::native_managed::regenerate_boot_from_bound_native(
        &graph.workspace,
        &graph.resolution,
        SpecFormat::Mixed,
        Some(trace),
        &graph.epoch,
        Some(provider),
    )
}

fn trace_index(graph: &NativeGraph, run: &str) -> CompilerTraceIndex {
    let path = graph
        .workspace
        .root
        .join(".vibe/trace")
        .join(run)
        .join("index.json");
    let bytes = test_ok!(fs::read(path), "trace index");
    test_ok!(serde_json::from_slice(&bytes), "trace wire")
}

fn unit_file(graph: &NativeGraph, name: &str, file: &str) -> PathBuf {
    crate::vibedeps::slot_abs_path(&graph.workspace.root, &group(), name, &version())
        .join(vibe_core::layout::current_boot_dir())
        .join(file)
}

fn old_mtime(path: &Path) -> SystemTime {
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(946_684_800);
    let file = test_ok!(
        fs::OpenOptions::new().write(true).open(path),
        "generated artifact"
    );
    test_ok!(
        file.set_times(fs::FileTimes::new().set_modified(old)),
        "old timestamp"
    );
    let metadata = test_ok!(fs::metadata(path), "metadata");
    test_ok!(metadata.modified(), "mtime")
}

fn pending_set(
    mut result: bootgen::native_managed::BoundBootRegeneration,
    owner: &OwnerRuntimeId,
) -> vibe_spec::CompilerPendingSet {
    match test_some!(result.native.remove(owner), "native continuation") {
        crate::boot_artifacts::OwnerNativeCompileContinuation::Pending { pending, .. } => pending,
        crate::boot_artifacts::OwnerNativeCompileContinuation::Ready { .. } => {
            panic!("expected Pending")
        }
    }
}

#[test]
fn pending_forces_recompile_propagates_then_resolve_restores_exact_base() {
    let graph = native_graph();
    let middle_index = unit_file(&graph, "middle", "INDEX.md");
    let middle_static = unit_file(&graph, "middle", "STATIC.md");
    let top_index = unit_file(&graph, "top", "INDEX.md");
    let top_static = unit_file(&graph, "top", "STATIC.md");

    let mut ready_provider = FakeProvider::new(Reply::Skip);
    let ready = regenerate(&graph, &mut ready_provider);
    assert!(matches!(
        ready.native.get(&graph.middle),
        Some(crate::boot_artifacts::OwnerNativeCompileContinuation::Ready { .. })
    ));
    let base = [
        fs::read(&middle_index).expect("middle INDEX"),
        fs::read(&middle_static).expect("middle STATIC"),
        fs::read(&top_index).expect("top INDEX"),
        fs::read(&top_static).expect("top STATIC"),
    ];

    let mut pending_provider = FakeProvider::new(Reply::Missing);
    let pending = regenerate(&graph, &mut pending_provider);
    assert_eq!(*pending_provider.invocations.lock().expect("calls"), 1);
    let middle_record = crate::boot_artifacts::publication::read_unit_index_freshness(
        &fs::read_to_string(&middle_index).expect("middle INDEX"),
    )
    .expect("middle freshness");
    let top_record = crate::boot_artifacts::publication::read_unit_index_freshness(
        &fs::read_to_string(&top_index).expect("top INDEX"),
    )
    .expect("top freshness");
    assert!(middle_record.pending.is_some());
    assert!(top_record.pending.is_none());
    assert_ne!(fs::read(&middle_index).expect("middle INDEX"), base[0]);
    assert_ne!(fs::read(&top_index).expect("top INDEX"), base[2]);
    assert!(
        verify_boot_graph(&graph.workspace)
            .expect("verify Pending")
            .is_empty()
    );

    let pending_bytes = [
        fs::read(&middle_index).expect("middle INDEX"),
        fs::read(&middle_static).expect("middle STATIC"),
        fs::read(&top_index).expect("top INDEX"),
        fs::read(&top_static).expect("top STATIC"),
    ];
    let pending_text = String::from_utf8(pending_bytes[0].clone()).expect("pending INDEX utf8");
    let prior_marker = pending_text
        .lines()
        .find(|line| line.starts_with(crate::boot_artifacts::publication::NATIVE_PENDING_MARKER))
        .expect("prior marker");
    let poisoned = pending_text.replace(
        prior_marker,
        &format!(
            "{}{}",
            crate::boot_artifacts::publication::NATIVE_PENDING_MARKER,
            "e".repeat(64)
        ),
    );
    fs::write(&middle_index, poisoned).expect("poison prior marker");
    let mut current_provider = FakeProvider::new(Reply::Missing);
    let _ = regenerate(&graph, &mut current_provider);
    assert_eq!(*current_provider.invocations.lock().expect("calls"), 1);
    assert_eq!(
        fs::read(&middle_index).expect("current INDEX"),
        pending_bytes[0]
    );
    let aged = [
        old_mtime(&middle_index),
        old_mtime(&middle_static),
        old_mtime(&top_index),
        old_mtime(&top_static),
    ];
    let mut repeated_provider = FakeProvider::new(Reply::Missing);
    let repeated = regenerate(&graph, &mut repeated_provider);
    assert_eq!(*repeated_provider.invocations.lock().expect("calls"), 1);
    for (position, path) in [&middle_index, &middle_static, &top_index, &top_static]
        .into_iter()
        .enumerate()
    {
        assert_eq!(fs::read(path).expect("artifact"), pending_bytes[position]);
        assert_eq!(
            fs::metadata(path)
                .expect("metadata")
                .modified()
                .expect("mtime"),
            aged[position]
        );
    }

    let expected = pending_set(repeated, &graph.middle);
    let mut resolve_provider = FakeProvider::new(Reply::Skip).with_policy(
        graph.middle.clone(),
        CompilerNativePolicy::resolve(expected),
    );
    let resolved = regenerate(&graph, &mut resolve_provider);
    assert!(matches!(
        resolved.native.get(&graph.middle),
        Some(crate::boot_artifacts::OwnerNativeCompileContinuation::Ready { .. })
    ));
    for (position, path) in [&middle_index, &middle_static, &top_index, &top_static]
        .into_iter()
        .enumerate()
    {
        assert_eq!(fs::read(path).expect("resolved artifact"), base[position]);
    }
    assert!(
        verify_boot_graph(&graph.workspace)
            .expect("verify Ready")
            .is_empty()
    );
    drop(pending);
}

#[test]
fn verify_refuses_hostile_pending_markers_and_propagates_static_staleness() {
    let graph = native_graph();
    let middle_index = unit_file(&graph, "middle", "INDEX.md");
    let top_index = unit_file(&graph, "top", "INDEX.md");
    let mut provider = FakeProvider::new(Reply::Missing);
    let _ = regenerate(&graph, &mut provider);
    let canonical = fs::read_to_string(&middle_index).expect("pending INDEX");
    let marker = canonical
        .lines()
        .find(|line| line.starts_with(crate::boot_artifacts::publication::NATIVE_PENDING_MARKER))
        .expect("pending marker");
    let marker_value = marker
        .strip_prefix(crate::boot_artifacts::publication::NATIVE_PENDING_MARKER)
        .expect("marker value");
    let fingerprint = canonical
        .lines()
        .find(|line| line.starts_with(crate::boot_artifacts::FP_MARKER))
        .expect("fingerprint marker");
    for hostile in [
        canonical.replace(&format!("{marker}\n"), ""),
        canonical.replace(marker_value, &"c".repeat(64)),
        canonical.replace(marker_value, &marker_value.to_uppercase()),
        canonical.replace(marker, &format!("{marker}\n{marker}")),
        canonical.replace(marker, "# vibe:native-pending short"),
        canonical.replace(&format!("{fingerprint}\n"), ""),
        canonical.replace(fingerprint, &format!("{fingerprint}\n{fingerprint}")),
    ] {
        fs::write(&middle_index, hostile).expect("hostile INDEX");
        let stale = verify_boot_graph(&graph.workspace).expect("verify hostile marker");
        assert!(stale.iter().any(|id| id.1 == "middle"), "{stale:?}");
        assert!(stale.iter().any(|id| id.1 == "top"), "{stale:?}");
        assert!(stale.iter().all(|id| id.1 != "dynamic"), "{stale:?}");
    }
    fs::write(&middle_index, canonical).expect("restore Pending INDEX");

    let top = fs::read_to_string(&top_index).expect("top INDEX");
    let top_fingerprint = top
        .lines()
        .find(|line| line.starts_with(crate::boot_artifacts::FP_MARKER))
        .expect("top fingerprint");
    let top_id = (group(), "top".to_owned());
    let forged_fingerprint = test_some!(
        bootgen::native_managed::fingerprint_with_pending_for_test(
            &graph.workspace,
            &graph.resolution,
            &graph.epoch,
            &top_id,
            [0xdd; 32],
        ),
        "forged top composite"
    );
    let forged = top.replace(
        top_fingerprint,
        &format!(
            "{}{}\n{}{}",
            crate::boot_artifacts::FP_MARKER,
            forged_fingerprint,
            crate::boot_artifacts::publication::NATIVE_PENDING_MARKER,
            "d".repeat(64)
        ),
    );
    let recorded = crate::boot_artifacts::publication::read_unit_index_freshness(&forged)
        .expect("self-consistent forged builtin marker");
    assert_eq!(recorded.fingerprint, forged_fingerprint);
    fs::write(&top_index, forged).expect("forged builtin marker");
    let stale = verify_boot_graph(&graph.workspace).expect("verify builtin forgery");
    assert!(stale.iter().any(|id| id.1 == "top"), "{stale:?}");
    assert!(stale.iter().all(|id| id.1 != "dynamic"), "{stale:?}");
}

#[test]
fn no_static_native_and_phase_native_only_keep_the_exact_legacy_skip() {
    const PHASE_NATIVE: &str = "[[extension]]\nid='phase-native'\npoint='phase:build'\n\
handler={kind='native',crate_dir='native'}\n";
    for graph in [
        native_graph_with("", COMPILE_NATIVE),
        native_graph_with(PHASE_NATIVE, ""),
    ] {
        let middle_index = unit_file(&graph, "middle", "INDEX.md");
        let middle_static = unit_file(&graph, "middle", "STATIC.md");
        let mut first = FakeProvider::new(Reply::Missing);
        let result = regenerate(&graph, &mut first);
        assert!(result.native.is_empty());
        assert!(first.owners.is_empty());
        assert_eq!(*first.invocations.lock().expect("calls"), 0);
        let index_bytes = fs::read(&middle_index).expect("legacy INDEX");
        let static_bytes = fs::read(&middle_static).expect("legacy STATIC");
        let aged = [old_mtime(&middle_index), old_mtime(&middle_static)];

        let mut second = FakeProvider::new(Reply::Missing);
        let _ = regenerate(&graph, &mut second);
        assert!(second.owners.is_empty());
        assert_eq!(*second.invocations.lock().expect("calls"), 0);
        assert_eq!(fs::read(&middle_index).expect("legacy INDEX"), index_bytes);
        assert_eq!(
            fs::read(&middle_static).expect("legacy STATIC"),
            static_bytes
        );
        assert_eq!(
            fs::metadata(&middle_index)
                .expect("metadata")
                .modified()
                .expect("mtime"),
            aged[0]
        );
        assert_eq!(
            fs::metadata(&middle_static)
                .expect("metadata")
                .modified()
                .expect("mtime"),
            aged[1]
        );
    }
}

#[test]
fn selector_miss_forces_boot_compile_but_invokes_no_native_or_pending_work() {
    const SELECTOR_MISS: &str = "[[extension]]\nid='native'\npoint='compile:source'\n\
handler={kind='native',crate_dir='native'}\napplies_to=[['static-xml']]\n";
    let graph = native_graph_with(SELECTOR_MISS, "");
    let mut provider = FakeProvider::new(Reply::Missing);
    let result = regenerate(&graph, &mut provider);
    assert_eq!(
        provider.owners.as_slice(),
        std::slice::from_ref(&graph.middle)
    );
    assert_eq!(*provider.invocations.lock().expect("calls"), 0);
    assert_eq!(*provider.fact_drains.lock().expect("fact drains"), 0);
    assert_eq!(*provider.ready_finishes.lock().expect("finish"), 1);
    assert!(matches!(
        result.native.get(&graph.middle),
        Some(crate::boot_artifacts::OwnerNativeCompileContinuation::Ready { .. })
    ));
    let index =
        fs::read_to_string(unit_file(&graph, "middle", "INDEX.md")).expect("selector-miss INDEX");
    assert!(!index.contains(crate::boot_artifacts::publication::NATIVE_PENDING_MARKER));
    assert!(
        verify_boot_graph(&graph.workspace)
            .expect("verify selector miss")
            .is_empty()
    );
    assert!(!graph.workspace.root.join("target").exists());
}

#[test]
fn base_fresh_native_unit_is_compiled_twice_in_one_trace_and_never_skipped() {
    const RUN: &str = "6123456789abcdef0123456789abcdef";
    let graph = native_graph();
    let trace = TraceRun::open_with_limits(
        &graph.workspace.root,
        RUN,
        Timestamp::from_timestamp(1_100, 0).expect("timestamp"),
        TraceLimits::for_test(u64::MAX, 9),
    )
    .expect("trace run");
    for _ in 0..2 {
        let mut provider = FakeProvider::new(Reply::Skip);
        regenerate_traced(&graph, &mut provider, &trace).expect("traced native generation");
        assert_eq!(*provider.invocations.lock().expect("calls"), 1);
    }
    let index = trace_index(&graph, RUN);
    let scopes = index
        .scopes
        .iter()
        .filter(|scope| scope.id.starts_with("unit:org.lock/middle#static-md"))
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 2);
    assert!(
        scopes
            .iter()
            .all(|scope| scope.status == ScopeStatus::Compiled)
    );
    assert!(
        scopes
            .iter()
            .all(|scope| scope.status != ScopeStatus::Skipped)
    );
}

#[test]
fn native_unit_failure_preserves_prior_artifacts_and_leaves_no_pending_scope() {
    const RUN: &str = "7123456789abcdef0123456789abcdef";
    let graph = native_graph();
    let paths = [
        unit_file(&graph, "middle", "INDEX.md"),
        unit_file(&graph, "middle", "STATIC.md"),
        unit_file(&graph, "top", "INDEX.md"),
        unit_file(&graph, "top", "STATIC.md"),
    ];
    let mut ready = FakeProvider::new(Reply::Skip);
    let _ = regenerate(&graph, &mut ready);
    let before = paths
        .iter()
        .map(|path| fs::read(path).expect("prior artifact"))
        .collect::<Vec<_>>();
    let trace = TraceRun::open_with_limits(
        &graph.workspace.root,
        RUN,
        Timestamp::from_timestamp(1_200, 0).expect("timestamp"),
        TraceLimits::for_test(u64::MAX, 9),
    )
    .expect("trace run");
    let mut hard = FakeProvider::new(Reply::Hard);
    assert!(regenerate_traced(&graph, &mut hard, &trace).is_err());
    for (path, before) in paths.iter().zip(before) {
        assert_eq!(fs::read(path).expect("preserved artifact"), before);
    }
    let index = trace_index(&graph, RUN);
    let scope = index
        .scopes
        .iter()
        .find(|scope| scope.id.starts_with("unit:org.lock/middle#static-md"))
        .expect("failed unit scope");
    assert_eq!(scope.status, ScopeStatus::Failed);
    assert!(
        index
            .scopes
            .iter()
            .all(|scope| scope.status != ScopeStatus::Pending)
    );
}
