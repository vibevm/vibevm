use std::sync::{Arc, Mutex};

use crate::compiler::builtin::{
    compile_artifact_native_managed, compile_artifact_native_managed_observed,
    compile_artifact_native_managed_traced,
};
use crate::compiler::observer::{CompileObserver, EmissionEvent, StageDeltaEvent};
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};
use crate::compiler::transform::header::{
    transforms_header_payload, transforms_header_payload_excluding,
};

use super::lowering_worlds::Declared;
use super::native_manager_test_support::{FakeInvoker, ReplyMode, fixture_world, plan};
use super::native_policy::session::{NativePolicyResult, NativePolicySession};
use super::native_policy::{
    CompilerNativeOutcome, CompilerNativePolicy, CompilerPendingArtifact, CompilerPendingSet,
};
use crate::{SectionSource, SpecAddress};

struct WithoutReachedUse<'source, S>(&'source S);

impl<S: SectionSource> SectionSource for WithoutReachedUse<'_, S> {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        self.0.section_text(address).map(|text| {
            text.lines()
                .filter(|line| !line.contains("#use"))
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        self.0.expand_pattern(address)
    }
}

fn all_stages() -> crate::compiler::ir::ArtifactPlan {
    plan(vec![
        Declared::native("source", "compile:source"),
        Declared::native("document", "compile:document"),
        Declared::native("lane", "compile:lane"),
        Declared::native("emitted", "compile:emitted"),
    ])
}

fn ready(outcome: CompilerNativeOutcome) -> crate::compiler::ir::EmittedArtifact {
    match outcome {
        CompilerNativeOutcome::Ready(ready) => ready.into_artifact(),
        CompilerNativeOutcome::Pending(_) => panic!("expected publishable outcome"),
    }
}

fn pending(outcome: CompilerNativeOutcome) -> CompilerPendingArtifact {
    match outcome {
        CompilerNativeOutcome::Ready(_) => panic!("expected pending outcome"),
        CompilerNativeOutcome::Pending(pending) => pending,
    }
}

#[derive(Default)]
struct CountingSink {
    trace: Mutex<Vec<String>>,
    deltas: Mutex<Vec<String>>,
}

impl CompileTraceSink for CountingSink {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.trace.lock().unwrap().push(event.pass().to_string());
    }
}

impl CompileObserver for CountingSink {
    fn emission(&self, _event: &EmissionEvent) {}

    fn stage_delta(&self, event: &StageDeltaEvent) {
        self.deltas.lock().unwrap().push(event.pass().to_string());
    }
}

#[test]
fn collect_all_stages_keeps_original_carriers_and_coalesces_5_5_1_1() {
    let world = fixture_world();
    let plan = all_stages();
    let unavailable = FakeInvoker::new(ReplyMode::BuildableSourceUnavailable);
    let pending_outcome = pending(
        compile_artifact_native_managed(
            plan.clone(),
            &world.source,
            &unavailable,
            CompilerNativePolicy::collect(),
        )
        .unwrap(),
    );
    assert_eq!(unavailable.records().len(), 12);
    assert_eq!(
        pending_outcome
            .pending()
            .iter()
            .map(|entry| entry.order())
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );

    let skipped = ready(
        compile_artifact_native_managed(
            plan,
            &world.source,
            &FakeInvoker::new(ReplyMode::SkipAll),
            CompilerNativePolicy::collect(),
        )
        .unwrap(),
    );
    assert_eq!(pending_outcome.artifact_for_test(), &skipped);
}

#[test]
fn collect_selector_miss_skip_empty_and_builtin_only_are_ready() {
    let world = fixture_world();
    let missed = plan(vec![
        Declared::native("miss", "compile:source").scoped(&["nowhere/**"]),
    ]);
    let invoker = FakeInvoker::new(ReplyMode::BuildableSourceUnavailable);
    assert!(
        compile_artifact_native_managed(
            missed,
            &world.source,
            &invoker,
            CompilerNativePolicy::collect(),
        )
        .unwrap()
        .as_ready()
        .is_some()
    );
    assert!(invoker.records().is_empty());

    let skip = FakeInvoker::new(ReplyMode::SkipAll);
    assert!(
        compile_artifact_native_managed(
            all_stages(),
            &world.source,
            &skip,
            CompilerNativePolicy::collect(),
        )
        .unwrap()
        .as_ready()
        .is_some()
    );
    assert_eq!(skip.records().len(), 12, "handler skip is still execution");

    let empty = fixture_world().plan;
    let idle = FakeInvoker::new(ReplyMode::BuildableSourceUnavailable);
    assert!(
        compile_artifact_native_managed(
            empty,
            &world.source,
            &idle,
            CompilerNativePolicy::collect(),
        )
        .unwrap()
        .as_ready()
        .is_some()
    );
    assert!(idle.records().is_empty());

    let builtin = plan(vec![Declared::builtin(
        "builtin",
        "compile:emitted",
        "xml-minify",
    )]);
    let builtin_invoker = FakeInvoker::new(ReplyMode::BuildableSourceUnavailable);
    let _ = compile_artifact_native_managed(
        builtin,
        &world.source,
        &builtin_invoker,
        CompilerNativePolicy::collect(),
    );
    assert!(builtin_invoker.records().is_empty());
}

#[test]
fn mixed_repeated_availability_fails_with_entry_attribution() {
    let world = fixture_world();
    let plan = plan(vec![Declared::native("one", "compile:source")]);
    for mode in [
        ReplyMode::BuildableFirstCall(0),
        ReplyMode::BuildableAfterFirstCall(0),
    ] {
        let error = compile_artifact_native_managed(
            plan.clone(),
            &world.source,
            &FakeInvoker::new(mode),
            CompilerNativePolicy::collect(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("entry 0"), "{error}");
        assert!(error.to_string().contains("mixed"), "{error}");
    }
}

fn collect_expected(
    plan: crate::compiler::ir::ArtifactPlan,
    mode: ReplyMode,
) -> (crate::compiler::ir::ArtifactPlan, CompilerPendingSet) {
    let world = fixture_world();
    let pending = pending(
        compile_artifact_native_managed(
            plan.clone(),
            &world.source,
            &FakeInvoker::new(mode),
            CompilerNativePolicy::collect(),
        )
        .unwrap(),
    );
    let expected = pending.into_pending_set();
    (plan, expected)
}

#[test]
fn resolve_counts_expected_calls_allows_nonexpected_and_skip() {
    let world = fixture_world();
    let two = plan(vec![
        Declared::native("expected", "compile:source"),
        Declared::native("ordinary", "compile:source"),
    ]);
    let (two, expected) = collect_expected(two, ReplyMode::BuildableOrder(0));
    let resolved = compile_artifact_native_managed(
        two.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::Ok),
        CompilerNativePolicy::resolve(expected),
    )
    .unwrap();
    let receipts = resolved.as_ready().unwrap().receipts();
    assert_eq!(
        receipts.iter().map(|(_, count)| count).collect::<Vec<_>>(),
        [5]
    );

    let (_, expected) = collect_expected(two.clone(), ReplyMode::BuildableOrder(0));
    let skipped = compile_artifact_native_managed(
        two,
        &world.source,
        &FakeInvoker::new(ReplyMode::SkipAll),
        CompilerNativePolicy::resolve(expected),
    )
    .unwrap();
    assert_eq!(
        skipped
            .as_ready()
            .unwrap()
            .receipts()
            .iter()
            .map(|(_, count)| count)
            .collect::<Vec<_>>(),
        [5]
    );
}

#[test]
fn resolve_refuses_residual_unexpected_and_missing_receipts() {
    let world = fixture_world();
    let two = plan(vec![
        Declared::native("expected", "compile:source"),
        Declared::native("ordinary", "compile:source"),
    ]);
    let (_, residual) = collect_expected(two.clone(), ReplyMode::BuildableOrder(0));
    assert!(
        compile_artifact_native_managed(
            two.clone(),
            &world.source,
            &FakeInvoker::new(ReplyMode::BuildableOrder(0)),
            CompilerNativePolicy::resolve(residual),
        )
        .unwrap_err()
        .to_string()
        .contains("remained unavailable")
    );
    let (_, unexpected) = collect_expected(two.clone(), ReplyMode::BuildableOrder(0));
    assert!(
        compile_artifact_native_managed(
            two,
            &world.source,
            &FakeInvoker::new(ReplyMode::BuildableOrder(1)),
            CompilerNativePolicy::resolve(unexpected),
        )
        .unwrap_err()
        .to_string()
        .contains("nonexpected")
    );
}

#[test]
fn resolve_refuses_an_expected_selector_row_not_invoked_in_the_replay_world() {
    let world = fixture_world();
    let scoped = plan(vec![
        Declared::native("reached-only", "compile:source").scoped(&["boot/base"]),
    ]);
    let (_, expected) = collect_expected(scoped.clone(), ReplyMode::BuildableSourceUnavailable);
    let error = compile_artifact_native_managed(
        scoped,
        &WithoutReachedUse(&world.source),
        &FakeInvoker::new(ReplyMode::Ok),
        CompilerNativePolicy::resolve(expected),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("without invocation receipts"),
        "{error}"
    );
}

#[test]
fn pending_lane_and_emitted_calls_emit_no_analyzer_delta_but_trace_attempts_remain() {
    let world = fixture_world();
    let plan = all_stages();
    let pending_observer = Arc::new(CountingSink::default());
    let outcome = compile_artifact_native_managed_observed(
        plan.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::BuildableSourceUnavailable),
        CompilerNativePolicy::collect(),
        pending_observer.clone(),
    )
    .unwrap();
    assert!(outcome.as_pending().is_some());
    assert!(pending_observer.deltas.lock().unwrap().is_empty());

    let executed_observer = Arc::new(CountingSink::default());
    compile_artifact_native_managed_observed(
        plan.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::Ok),
        CompilerNativePolicy::collect(),
        executed_observer.clone(),
    )
    .unwrap();
    assert_eq!(executed_observer.deltas.lock().unwrap().len(), 2);

    let trace = CountingSink::default();
    compile_artifact_native_managed_traced(
        plan,
        &world.source,
        &FakeInvoker::new(ReplyMode::BuildableSourceUnavailable),
        CompilerNativePolicy::collect(),
        &trace,
    )
    .unwrap();
    assert_eq!(
        trace
            .trace
            .lock()
            .unwrap()
            .iter()
            .filter(|name| name.starts_with("transform:"))
            .count(),
        12
    );
}

#[test]
fn managed_plain_traced_and_observed_collect_outcomes_agree() {
    let world = fixture_world();
    let plan = all_stages();
    let plain = compile_artifact_native_managed(
        plan.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::BuildableSourceUnavailable),
        CompilerNativePolicy::collect(),
    )
    .unwrap();
    let traced = compile_artifact_native_managed_traced(
        plan.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::BuildableSourceUnavailable),
        CompilerNativePolicy::collect(),
        &CountingSink::default(),
    )
    .unwrap();
    let observed = compile_artifact_native_managed_observed(
        plan,
        &world.source,
        &FakeInvoker::new(ReplyMode::BuildableSourceUnavailable),
        CompilerNativePolicy::collect(),
        Arc::new(CountingSink::default()),
    )
    .unwrap();
    let parts = |outcome: CompilerNativeOutcome| {
        let (artifact, set) = pending(outcome).into_parts();
        let refs = set
            .iter()
            .map(|entry| (entry.order(), entry.key().to_string()))
            .collect::<Vec<_>>();
        (artifact, refs)
    };
    assert_eq!(parts(plain), parts(traced));
    assert_eq!(
        parts(observed),
        parts(
            compile_artifact_native_managed(
                all_stages(),
                &world.source,
                &FakeInvoker::new(ReplyMode::BuildableSourceUnavailable),
                CompilerNativePolicy::collect(),
            )
            .unwrap()
        )
    );
}

#[test]
fn pending_outcome_api_and_header_projection_are_nonpublishing_and_exact() {
    let world = fixture_world();
    let artifact_plan = plan(vec![
        Declared::native("first", "compile:source"),
        Declared::native("second", "compile:source"),
    ]);
    let digest_before = artifact_plan.transforms().digest_hex();
    let pending = pending(
        compile_artifact_native_managed(
            artifact_plan.clone(),
            &world.source,
            &FakeInvoker::new(ReplyMode::BuildableOrder(0)),
            CompilerNativePolicy::collect(),
        )
        .unwrap(),
    );
    assert_eq!(pending.status(), crate::CompilerNativeStatus::Pending);
    assert!(!format!("{pending:?}").contains("bytes"));
    assert_eq!(
        transforms_header_payload_excluding(artifact_plan.transforms(), pending.pending()).unwrap(),
        Some("vibe:transforms __host__/demo#second".to_string())
    );
    assert_eq!(artifact_plan.transforms().digest_hex(), digest_before);

    let set = pending.into_pending_set();
    let other = plan(vec![Declared::native("other", "compile:source")]);
    assert!(transforms_header_payload_excluding(other.transforms(), &set).is_err());

    let source = include_str!("native_policy/outcome.rs");
    let pending_impl = source
        .split("impl CompilerPendingArtifact")
        .nth(1)
        .unwrap()
        .split("impl fmt::Debug")
        .next()
        .unwrap();
    assert!(!pending_impl.contains("pub fn artifact"));
    assert!(!pending_impl.contains("pub fn into_artifact"));
    assert!(pending_impl.contains("pub fn into_pending_set(self) -> CompilerPendingSet"));
    assert!(!pending_impl.contains("pub fn into_pending_set(self) -> ("));
    assert!(!source.contains("impl Clone for CompilerPendingArtifact"));
}

#[test]
fn empty_pending_projection_is_byte_identical_to_the_existing_header() {
    let plan = all_stages();
    let session =
        NativePolicySession::new(plan.transforms(), CompilerNativePolicy::collect()).unwrap();
    let empty = match session.finish().unwrap() {
        NativePolicyResult::Collected(empty) => empty,
        _ => panic!("collect result"),
    };
    assert_eq!(
        transforms_header_payload_excluding(plan.transforms(), &empty).unwrap(),
        transforms_header_payload(plan.transforms())
    );
}
