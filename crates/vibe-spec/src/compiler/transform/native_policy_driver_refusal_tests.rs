//! Compatibility and hard-refusal panel for the managed driver.

use std::sync::{Arc, Mutex};

use crate::compiler::builtin::{
    ArtifactCompileError, compile_artifact_native, compile_artifact_native_managed,
    compile_artifact_native_managed_observed, compile_artifact_native_managed_traced,
    compile_artifact_native_observed, compile_artifact_native_traced,
};
use crate::compiler::observer::{CompileObserver, EmissionEvent, StageDeltaEvent};
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};

use super::lowering_worlds::Declared;
use super::native_manager_test_support::{FakeInvoker, ReplyMode, fixture_world, plan};
use super::native_policy::{CompilerNativeOutcome, CompilerNativePolicy};

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
        CompilerNativeOutcome::Pending(_) => panic!("Fail cannot return Pending"),
    }
}

fn error_chain(error: &ArtifactCompileError) -> Vec<String> {
    let mut chain = vec![error.to_string()];
    let mut source = std::error::Error::source(error);
    while let Some(error) = source {
        chain.push(error.to_string());
        source = error.source();
    }
    chain
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
fn managed_fail_is_whole_value_error_trace_and_observer_compatible() {
    let world = fixture_world();
    let all = all_stages();
    let old = compile_artifact_native(all.clone(), &world.source, &FakeInvoker::new(ReplyMode::Ok))
        .unwrap();
    let managed = ready(
        compile_artifact_native_managed(
            all.clone(),
            &world.source,
            &FakeInvoker::new(ReplyMode::Ok),
            CompilerNativePolicy::fail(),
        )
        .unwrap(),
    );
    assert_eq!(managed, old);

    for mode in [
        ReplyMode::BuildableSourceUnavailable,
        ReplyMode::InvocationFailed,
        ReplyMode::MalformedJson,
        ReplyMode::InvalidUtf8,
        ReplyMode::UnknownRoot,
        ReplyMode::WrongEnvelope,
        ReplyMode::FailOrder(0),
    ] {
        let old =
            compile_artifact_native(all.clone(), &world.source, &FakeInvoker::new(mode.clone()))
                .unwrap_err();
        let managed = compile_artifact_native_managed(
            all.clone(),
            &world.source,
            &FakeInvoker::new(mode),
            CompilerNativePolicy::fail(),
        )
        .unwrap_err();
        assert_eq!(error_chain(&managed), error_chain(&old));
    }

    let source_plan = plan(vec![Declared::native("source", "compile:source")]);
    let recorder = FakeInvoker::new(ReplyMode::Ok);
    compile_artifact_native(source_plan, &world.source, &recorder).unwrap();
    let wrong = plan(vec![Declared::native("document", "compile:document")]);
    let payload = recorder.records()[0].payload.clone();
    let collect_payload = payload.clone();
    let old = compile_artifact_native(
        wrong.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::ReturnPayload(payload.clone())),
    )
    .unwrap_err();
    let managed = compile_artifact_native_managed(
        wrong,
        &world.source,
        &FakeInvoker::new(ReplyMode::ReturnPayload(payload)),
        CompilerNativePolicy::fail(),
    )
    .unwrap_err();
    assert_eq!(error_chain(&managed), error_chain(&old));
    let collect = compile_artifact_native_managed(
        plan(vec![Declared::native("document", "compile:document")]),
        &world.source,
        &FakeInvoker::new(ReplyMode::ReturnPayload(collect_payload)),
        CompilerNativePolicy::collect(),
    )
    .unwrap_err();
    assert!(collect.to_string().contains("entry 0"), "{collect}");

    let old_trace = CountingSink::default();
    let managed_trace = CountingSink::default();
    compile_artifact_native_traced(
        all.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::Ok),
        &old_trace,
    )
    .unwrap();
    compile_artifact_native_managed_traced(
        all.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::Ok),
        CompilerNativePolicy::fail(),
        &managed_trace,
    )
    .unwrap();
    assert_eq!(
        *managed_trace.trace.lock().unwrap(),
        *old_trace.trace.lock().unwrap()
    );

    let old_observer = Arc::new(CountingSink::default());
    let managed_observer = Arc::new(CountingSink::default());
    compile_artifact_native_observed(
        all.clone(),
        &world.source,
        &FakeInvoker::new(ReplyMode::Ok),
        old_observer.clone(),
    )
    .unwrap();
    compile_artifact_native_managed_observed(
        all,
        &world.source,
        &FakeInvoker::new(ReplyMode::Ok),
        CompilerNativePolicy::fail(),
        managed_observer.clone(),
    )
    .unwrap();
    assert_eq!(
        *managed_observer.deltas.lock().unwrap(),
        *old_observer.deltas.lock().unwrap()
    );
}

#[test]
fn collect_keeps_every_nonbuildable_failure_hard_and_attributed() {
    let world = fixture_world();
    let one = plan(vec![Declared::native("one", "compile:source")]);
    for mode in [
        ReplyMode::InvocationFailed,
        ReplyMode::MalformedJson,
        ReplyMode::InvalidUtf8,
        ReplyMode::UnknownRoot,
        ReplyMode::WrongSchema,
        ReplyMode::FailOrder(0),
    ] {
        let error = compile_artifact_native_managed(
            one.clone(),
            &world.source,
            &FakeInvoker::new(mode),
            CompilerNativePolicy::collect(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("entry 0"), "{error}");
    }
}

#[test]
fn managed_driver_has_no_external_execution_dependency_or_third_compile_api() {
    let source = format!(
        "{}\n{}\n{}",
        include_str!("../builtin/driver.rs"),
        include_str!("native_manager.rs"),
        include_str!("native_policy/outcome.rs")
    );
    for forbidden in [
        "std::fs",
        "std::process",
        "vibe_workspace",
        "vibe_lifecycle",
        "artifact_resolver",
        "NativeLoader",
        "native::loader",
        "Cargo",
        "journal",
        "third_compile",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden driver dependency {forbidden}"
        );
    }
}
