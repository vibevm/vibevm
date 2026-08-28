//! Producer-level reds for the dispatch boundary's row accumulator.
//!
//! These drive the REAL `dispatch_plan` over a REAL planned contribution. The
//! point is the mutation they kill: deleting the `measured.clone_from(...)`
//! refresh inside the loop, or the `carry_measured` wrapper around the result,
//! leaves a hand-built-vector test green while the command reports a run that
//! "did nothing" after it had already done something.
//!
//! The injection point is AFTER a row is pushed, which is where the generic
//! post-row failures it stands in for actually occur. There is deliberately no
//! before-any-row case: the injection cannot express one, and a test that
//! armed at zero would fire after the first row anyway and assert something
//! weaker than the real one below. The pre-row shape is covered where it is
//! real — `phase.rs` freezes an empty prefix before dispatch is even called.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use vibe_lifecycle::{LifecycleLease, Phase, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use super::*;
use crate::cli::AgentModeArg;
use crate::commands::compile_trace::uncarry;

/// A project whose `phase:build` runs one builtin contribution that succeeds.
fn project_with_one_row() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [[extension]]\nid = 'first'\npoint = 'phase:build'\n\
         handler = { kind = \"builtin\", name = \"log\" }\n\
         config = { message = \"ROW-ONE\" }\n",
    )
    .unwrap();
    dir
}

fn metadata(root: &std::path::Path) -> RunMetadata {
    RunMetadata {
        requested: "build".to_string(),
        chain: vec!["build".to_string()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root).unwrap(),
        started: crate::commands::init::current_timestamp_utc(),
        selected: ".".into(),
    }
}

fn quiet_ctx() -> output::Context {
    output::Context::from_flags(true, false, None, true, AgentModeArg::Cli)
}

/// A real lease over the fixture's own root: `dispatch_plan` derives its
/// state home from the lease, so the proof must name the same workspace the
/// plan was built over.
fn lease_for(root: &Path) -> Arc<LifecycleLease> {
    Arc::new(LifecycleLease::acquire(root).expect("the fixture root is leasable"))
}

/// One real row runs; a GENERIC failure follows it; the row comes back.
///
/// The failure is injected after the first report so it is exactly the shape
/// no fixture can produce on demand — a state/checkpoint fault, which the
/// `failed_transition` branch never sees. If the accumulator refresh or the
/// carry wrapper is deleted, the draft arrives empty and this fails.
#[test]
fn a_generic_failure_after_a_real_row_carries_that_row_outward() {
    let project = project_with_one_row();
    let plan = world::plan_default(project.path(), &[Phase::Build]).expect("the plan loads");
    assert_eq!(
        plan.count_for(Phase::Build),
        1,
        "the fixture really plans one contribution",
    );

    let ctx = quiet_ctx();
    let meta = metadata(project.path());
    let guard = inject::fail_after(1);
    let result = dispatch_plan(
        &ctx,
        &plan,
        lease_for(project.path()),
        meta,
        vec!["build".to_string()],
    );
    drop(guard);

    let error = result.expect_err("the injected fault fails the dispatch");
    let carried = uncarry(error).unwrap_or_else(|error| {
        panic!("a post-row failure must arrive CARRIED, not bare: {error:#}")
    });

    // The original object, untouched: same words, same context chain.
    assert_eq!(
        format!("{:#}", carried.original),
        "writing the execution checkpoint: injected state fault",
        "context is neither stripped nor re-added in transit",
    );
    assert!(
        !carried.emit_when_trace_disabled,
        "a generic stage failure was historically silent",
    );

    let RegisteredReportDraft::Lifecycle(draft) = carried.draft else {
        panic!("a dispatch failure reports this command's own family");
    };
    assert_eq!(
        draft.contributions.len(),
        1,
        "the row measured BEFORE the fault came back with it: {:?}",
        draft.contributions,
    );
    assert_eq!(draft.contributions[0].phase, "build");
    assert!(
        matches!(draft.contributions[0].status.as_str(), "ok" | "fresh"),
        "and it is the SUCCESSFUL row, not a fabricated failure: {:?}",
        draft.contributions[0],
    );
}

/// The guard disarms even when the body panics, so a failed assertion cannot
/// leak the injection into the next test that runs on this thread.
#[test]
fn the_injection_guard_disarms_on_unwind() {
    let armed = std::panic::catch_unwind(|| {
        let _guard = inject::fail_after(1);
        panic!("the body failed while armed");
    });
    assert!(armed.is_err(), "the body really panicked");

    // Same thread, fresh dispatch: the fault must be gone.
    let project = project_with_one_row();
    let plan = world::plan_default(project.path(), &[Phase::Build]).expect("the plan loads");
    let ctx = quiet_ctx();
    let meta = metadata(project.path());
    assert!(
        dispatch_plan(
            &ctx,
            &plan,
            lease_for(project.path()),
            meta,
            vec!["build".to_string()],
        )
        .is_ok(),
        "the guard disarmed while unwinding",
    );
}
