//! The input-measurement carriage REDs (R7.5 P2/A4b).
//!
//! PROP-054 `##EVIDENCE-MEASUREMENT-CARRIAGE` closes the matrix: ordinary
//! `ok|skip` success, a current fresh skip and a hosted satisfied resume
//! checkpoint the CURRENT invocation's measurement and run id in the same
//! record transaction; dispatch failure, preparation failure and hosted park
//! carry none; a state-blind clean run writes no state at all. Fresh never
//! copies the prior measurement, and a refused observation drops the old
//! claim rather than preserving it.
//!
//! Every fixture keeps the declared input scope (`data/**`) DISJOINT from
//! the handler's outputs (`docs/**`): a create that writes into its own
//! measured scope is the §4.3 stale oracle, not a carriage case.

use std::fs;

use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use super::{
    KEY, RUN_ID, lease, metadata, project_fixture, runtime, scratch, state, world_fixture,
};
use crate::ExecutionReuse;
use crate::agent::tests::support::{
    PROMPT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT, row_with_inputs,
};
use crate::execution::HandlerExecution;
use crate::{LifecycleRun, RunMetadata};

const OTHER_RUN: &str = "ffeeddccbbaa99887766554433221100";

/// A CLI-mode metadata over the standard create chain.
fn cli_metadata(run_id: &str) -> RunMetadata {
    let mut metadata = metadata(run_id, false);
    metadata.agent_mode = RunAgentMode::Cli;
    metadata
}

/// The standard fixture tree: one stable declared input, no outputs yet.
fn seed_input(root: &std::path::Path) {
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/a.txt"), "one").unwrap();
}

fn inputs_row(patterns: &[&str]) -> crate::ExtensionRegistryRow {
    row_with_inputs(
        TWO_OUTPUTS,
        PROMPT,
        Some(
            patterns
                .iter()
                .map(|pattern| (*pattern).to_string())
                .collect(),
        ),
    )
}

/// Drive one CLI-mode execution of the inputs-declaring agent row.
fn cli_row(root: &std::path::Path, backend: &RecordingBackend, run_id: &str) {
    let row = inputs_row(&["data/**"]);
    let mut run = LifecycleRun::begin(
        lease(root),
        project_fixture(root),
        world_fixture(root),
        cli_metadata(run_id),
        vec!["create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row);
    let rt = runtime(backend);
    run.execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap();
}

/// Ordinary `ok` success checkpoints the pre-dispatch measurement of THIS
/// invocation under THIS run id — declaration fingerprint, patterns, witness
/// and all. The measurement is PRE-dispatch: the scope is measured before
/// the handler wrote its outputs.
#[test]
fn an_ordinary_success_carries_the_current_runs_measurement() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &backend, RUN_ID);

    let state = state(fixture.root.as_path());
    let record = &state.execution[KEY];
    assert_eq!(record.status, ExecutionRecordStatus::Ok);
    let measurement = record.input_measurement.as_ref().unwrap();
    assert_eq!(measurement.execution, KEY);
    assert_eq!(measurement.phase, "create");
    assert_eq!(measurement.measured_run_id, RUN_ID);
    assert_eq!(measurement.patterns, vec!["data/**".to_string()]);
    assert!(
        measurement.declaration_fingerprint.starts_with("sha256:"),
        "the declaration sibling travels beside the manifest witness",
    );
    assert_eq!(measurement.witness.files, Some(1));
}

/// A fresh skip RE-MEASURES: the second invocation checkpoints its own
/// measurement under its own — different — run id, never a copy of the
/// prior row's claim.
#[test]
fn a_fresh_skip_remeasures_under_the_current_run_id() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &backend, RUN_ID);

    let second = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &second, OTHER_RUN);
    assert_eq!(second.calls(), 0, "the second invocation fresh-skips");

    let state = state(fixture.root.as_path());
    let record = &state.execution[KEY];
    assert_eq!(record.status, ExecutionRecordStatus::Fresh);
    let measurement = record.input_measurement.as_ref().unwrap();
    assert_eq!(
        measurement.measured_run_id, OTHER_RUN,
        "fresh attributes the re-measurement to the CURRENT run, never a copied id",
    );
}

/// A refused current observation overwrites a reusable row with NO
/// measurement. The refusal leaves the selected SET and BYTES untouched —
/// `data/a.txt` becomes a hard link to an unselected twin of identical
/// content — so freshness holds while evidence honestly refuses.
#[test]
fn a_refused_current_observation_drops_the_old_claim() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &backend, RUN_ID);
    assert!(
        state(fixture.root.as_path()).execution[KEY]
            .input_measurement
            .is_some()
    );

    fs::remove_file(fixture.root.join("data/a.txt")).unwrap();
    fs::write(fixture.root.join("twin.bin"), "one").unwrap();
    fs::hard_link(
        fixture.root.join("twin.bin"),
        fixture.root.join("data/a.txt"),
    )
    .unwrap();
    let second = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &second, OTHER_RUN);
    assert_eq!(second.calls(), 0, "unchanged selected bytes stay fresh");

    let state = state(fixture.root.as_path());
    assert_eq!(state.execution[KEY].status, ExecutionRecordStatus::Fresh);
    assert!(
        state.execution[KEY].input_measurement.is_none(),
        "a refused observation stores no measurement and drops the old claim",
    );
}

/// A hosted park executes nothing and persists no measurement; the satisfied
/// resume then carries the ADOPTING invocation's pre-probe measurement. The
/// first run is dropped before the resume so the workspace lease releases.
#[test]
fn a_hosted_park_measures_nothing_and_its_satisfied_resume_measures() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let row = inputs_row(&["data/**"]);
    {
        let mut run = LifecycleRun::begin(
            lease(fixture.root.as_path()),
            project_fixture(fixture.root.as_path()),
            world_fixture(fixture.root.as_path()),
            metadata(RUN_ID, false),
            vec!["validate".into(), "create".into()],
        )
        .unwrap();
        let handler = HandlerExecution::from_row(&row);
        let rt = runtime(&backend);
        let parked = run
            .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
            .unwrap();
        assert_eq!(parked.status, ExecutionRecordStatus::Delegated);
    }
    assert!(
        state(fixture.root.as_path()).execution[KEY]
            .input_measurement
            .is_none(),
        "a parked row claims no measurement — nothing was executed",
    );

    fs::create_dir_all(fixture.root.join("docs")).unwrap();
    fs::write(fixture.root.join("docs/guide.md"), "hosted body\n").unwrap();
    fs::write(fixture.root.join("docs/reference.md"), "hosted reference\n").unwrap();
    // The same inputs-declaring row — the shared `execute_with` fixture
    // declares none, so its fingerprint could never adopt this park.
    let mut resume = LifecycleRun::begin(
        lease(fixture.root.as_path()),
        project_fixture(fixture.root.as_path()),
        world_fixture(fixture.root.as_path()),
        metadata(RUN_ID, false),
        vec!["validate".into(), "create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row);
    let rt = runtime(&backend);
    let resumed = resume
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap();
    assert_eq!(resumed.status, ExecutionRecordStatus::Ok);

    let state = state(fixture.root.as_path());
    let measurement = state.execution[KEY]
        .input_measurement
        .as_ref()
        .expect("the satisfied resume carries the pre-probe measurement");
    assert_eq!(measurement.measured_run_id, RUN_ID);
    assert_eq!(measurement.patterns, vec!["data/**".to_string()]);
    assert_eq!(measurement.witness.files, Some(1));
}

/// A dispatch failure persists the failure record with NO measurement — the
/// handler produced nothing attributable.
#[test]
fn a_dispatch_failure_carries_no_measurement() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::refusing_provider("the provider refused");
    let row = inputs_row(&["data/**"]);
    let mut run = LifecycleRun::begin(
        lease(fixture.root.as_path()),
        project_fixture(fixture.root.as_path()),
        world_fixture(fixture.root.as_path()),
        cli_metadata(RUN_ID),
        vec!["create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row);
    let rt = runtime(&backend);
    let error = run
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap_err();
    assert!(error.to_string().contains("the provider refused"));

    let state = state(fixture.root.as_path());
    assert_eq!(state.execution[KEY].status, ExecutionRecordStatus::Fail);
    assert!(
        state.execution[KEY].input_measurement.is_none(),
        "a dispatch failure owes no measurement",
    );
}

/// An authored-empty declaration persists a REAL measurement (a complete
/// empty scope), keeping the absent/refused distinction durable.
#[test]
fn an_authored_empty_scope_persists_a_real_measurement() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let row = row_with_inputs(TWO_OUTPUTS, PROMPT, Some(vec![]));
    let mut run = LifecycleRun::begin(
        lease(fixture.root.as_path()),
        project_fixture(fixture.root.as_path()),
        world_fixture(fixture.root.as_path()),
        cli_metadata(RUN_ID),
        vec!["create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row);
    let rt = runtime(&backend);
    run.execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap();

    let state = state(fixture.root.as_path());
    let measurement = state.execution[KEY].input_measurement.as_ref().unwrap();
    assert!(measurement.patterns.is_empty());
    assert_eq!(measurement.witness.files, Some(0));
    assert_eq!(measurement.witness.bytes, Some("0".to_string()));
}
