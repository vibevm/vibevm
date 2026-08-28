//! The sticky trace bit across the REAL engine seam (PROP-054
//! `##OBS-TRACE`, R3.4). `state/tests/trace_sticky.rs` pins what the
//! selector computes; this file pins that the computed bit SURVIVES the
//! path production carries it along — `RunMetadata` →
//! `LifecycleRun::begin` → `LifecycleStateStore::begin`, which replaces
//! the whole run header on every invocation.

use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use super::{RUN_ID, execute_with, metadata, scratch, state};
use crate::agent::tests::support::RecordingBackend;
use crate::{RunMetadata, select_run_identity};

/// Seeded from a real state-owned traced park, adopted by a resume that
/// requests nothing, and proven still true in the durable bytes
/// afterwards — the integration fact the four selector-derived CLI
/// constructors now depend on.
#[test]
fn an_adopted_traced_run_keeps_its_effective_bit_through_the_state_rewrite() {
    let scratch = scratch();
    let root = scratch.root.as_path();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);

    // A traced invocation parks: the delegated row and the sticky bit
    // are both made durable by the production store, not by a fixture.
    let parked = execute_with(
        root,
        &backend,
        RunMetadata {
            trace_compile: true,
            ..metadata(RUN_ID, false)
        },
    );
    assert_eq!(parked.status, ExecutionRecordStatus::Delegated);
    assert!(
        state(root).run.compile_trace,
        "the parked run's own bit is durable"
    );

    // The resume asks for NOTHING — no flag, no manifest opt-in. The
    // selector alone carries the park's bit forward.
    // The lease scopes to the selection: the resume below begins its own run.
    let identity = {
        let lease = super::lease(root);
        select_run_identity(
            &lease,
            root,
            "create",
            &["validate".into(), "install".into(), "create".into()],
            RunAgentMode::Agent,
            false,
            false,
            "2026-08-26T02:00:00Z".into(),
        )
        .unwrap()
    };
    assert!(identity.adopted, "the park is resumable");
    assert_eq!(identity.run_id, RUN_ID);
    assert!(identity.compile_trace, "sticky across the resume");
    assert!(
        identity.superseded_trace.is_none(),
        "adoption displaces nothing"
    );

    // …and the metadata is built the way every production constructor
    // now builds it: `trace_compile` FROM the selection.
    let resumed = execute_with(
        root,
        &backend,
        RunMetadata {
            trace_compile: identity.compile_trace,
            run_id: identity.run_id.clone(),
            started: identity.started.clone(),
            ..metadata(RUN_ID, false)
        },
    );
    assert_eq!(resumed.status, ExecutionRecordStatus::Delegated);
    let after = state(root);
    assert_eq!(after.run.run_id.as_deref(), Some(RUN_ID), "the same run");
    assert!(
        after.run.compile_trace,
        "begin replaces the header on every invocation; the adopted bit must survive it"
    );

    // The bug this pins, demonstrated on the same state: a constructor
    // that hard-codes false — or preserves a pre-selection false through
    // a struct update — hands `begin` an untraced header, and the resume
    // silently stops tracing. That is what makes the assertion above
    // decisive rather than incidental.
    let dropped = execute_with(root, &backend, metadata(RUN_ID, false));
    assert_eq!(dropped.status, ExecutionRecordStatus::Delegated);
    assert!(
        !state(root).run.compile_trace,
        "a hard-coded false overwrites the adopted bit"
    );
}
