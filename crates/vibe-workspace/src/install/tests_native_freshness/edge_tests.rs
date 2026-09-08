use super::*;

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
