//! The artifact-witness carriage REDs (R7.5 P2/A4c1).
//!
//! PROP-054 `##EVIDENCE-ARTIFACT-WITNESS` splits this matrix in two, and the
//! split is the whole point. The durable `StateArtifact.witness` is a
//! **baseline**: ordinary `ok|skip` success and a hosted satisfied resume are
//! production/acceptance boundaries and record what they saw under their own
//! run id; hosted park and dispatch failure record nothing. A **fresh skip is
//! not a producer** — it preserves the prior pair byte-for-byte, legacy
//! absence included, and puts its current re-observation in the invocation's
//! transient map, which verify compares against that baseline.
//!
//! The decisive one is
//! [`a_fresh_skip_preserves_the_baseline_and_observes_the_mutation`]. Folding
//! the current reading into the baseline passes every other test here and
//! makes verify compare a mutated output against itself — `matched` forever.

use std::fs;

use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, StateArtifact};

use super::{
    KEY, RUN_ID, lease, metadata, project_fixture, runtime, scratch, state, world_fixture,
    write_state,
};
use crate::ExecutionReuse;
use crate::agent::tests::support::{
    PROMPT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT, row_with_inputs,
};
use crate::execution::HandlerExecution;
use crate::{LifecycleRun, RunMetadata};

const SECOND_RUN: &str = "ffeeddccbbaa99887766554433221100";
const THIRD_RUN: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";
const GUIDE: &str = "docs/guide.md";

fn cli_metadata(run_id: &str) -> RunMetadata {
    let mut metadata = metadata(run_id, false);
    metadata.agent_mode = RunAgentMode::Cli;
    metadata
}

fn seed_input(root: &std::path::Path) {
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/a.txt"), "one").unwrap();
}

fn row() -> crate::ExtensionRegistryRow {
    row_with_inputs(TWO_OUTPUTS, PROMPT, Some(vec!["data/**".to_string()]))
}

/// Drive one CLI-mode execution and hand back the run itself, so a caller can
/// read this invocation's transient observations beside the durable record.
/// The run holds the workspace lease, so it must be dropped before the next
/// invocation begins.
fn cli_run(
    root: &std::path::Path,
    backend: &RecordingBackend,
    run_id: &str,
) -> (LifecycleRun, crate::ExecutionTransition) {
    let mut run = LifecycleRun::begin(
        lease(root),
        project_fixture(root),
        world_fixture(root),
        cli_metadata(run_id),
        vec!["create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row());
    let rt = runtime(backend);
    let transition = run
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap();
    (run, transition)
}

/// The same drive when only the status matters.
fn cli_row(
    root: &std::path::Path,
    backend: &RecordingBackend,
    run_id: &str,
) -> ExecutionRecordStatus {
    cli_run(root, backend, run_id).1.status
}

/// The measured digest this invocation observed at `id`, or `None` when the
/// observation was refused. Panics when the id was never observed at all —
/// that is a carriage defect, not an outcome.
fn observed(run: &LifecycleRun, id: &str) -> Option<String> {
    match run
        .artifact_observation(id)
        .expect("every accumulated artifact is observed by this invocation")
    {
        crate::artifacts::observe::WitnessOutcome::Measured(witness) => {
            Some(witness.digest.clone())
        }
        crate::artifacts::observe::WitnessOutcome::Refused(_) => None,
    }
}

fn rows(root: &std::path::Path) -> Vec<StateArtifact> {
    state(root).execution[KEY].artifacts.clone()
}

fn row_for<'a>(rows: &'a [StateArtifact], id: &str) -> &'a StateArtifact {
    rows.iter()
        .find(|artifact| artifact.id == id)
        .expect("the declared row is recorded")
}

/// Ordinary success witnesses every reply row in the same record, and the
/// witness/run-id pair travels together — an id beside no witness would
/// attribute a measurement that does not exist.
#[test]
fn an_ordinary_success_witnesses_every_declared_artifact() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    assert_eq!(
        cli_row(fixture.root.as_path(), &backend, RUN_ID),
        ExecutionRecordStatus::Ok
    );

    let rows = rows(fixture.root.as_path());
    assert_eq!(rows.len(), 2);
    for artifact in &rows {
        let witness = artifact
            .witness
            .as_ref()
            .expect("a produced file is witnessed");
        assert_eq!(witness.algorithm, "sha256:file-v1");
        assert!(witness.digest.starts_with("sha256:"));
        assert_eq!(witness.files, None, "artifact forms carry no count pair");
        assert_eq!(witness.bytes, None);
        assert_eq!(artifact.measured_run_id.as_deref(), Some(RUN_ID));
    }
}

/// **E5, the decisive one.** A produced artifact is mutated in place without
/// moving its path. The declared INPUT scope is untouched, so the row still
/// fresh-skips with zero provider calls.
///
/// The fresh skip must then produce exactly the pair verify needs: the durable
/// **baseline** W1 with its producing run id, untouched, beside a **current**
/// observation W2 ≠ W1 in this invocation's transient map. Folding W2 into the
/// baseline — which is what a naive re-probe does — makes verify compare W2
/// against W2 and report `matched` forever after an external mutation. That
/// version of this test passes while proving the defect.
#[test]
fn a_fresh_skip_preserves_the_baseline_and_observes_the_mutation() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &backend, RUN_ID);
    let produced = rows(fixture.root.as_path());
    let baseline = row_for(&produced, GUIDE).witness.clone().unwrap();

    fs::write(fixture.root.join(GUIDE), "# Guide (tampered)\n").unwrap();

    let second = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let (run, transition) = cli_run(fixture.root.as_path(), &second, SECOND_RUN);
    assert_eq!(transition.status, ExecutionRecordStatus::Fresh);
    assert_eq!(
        second.calls(),
        0,
        "no provider call — the inputs did not move"
    );

    let rows = rows(fixture.root.as_path());
    let preserved = row_for(&rows, GUIDE);
    assert_eq!(
        preserved.witness.as_ref().unwrap().digest,
        baseline.digest,
        "the durable witness is the PRODUCED baseline; a fresh skip may not redefine it",
    );
    assert_eq!(
        preserved.measured_run_id.as_deref(),
        Some(RUN_ID),
        "and it keeps the id of the run that produced it, not the skipping one",
    );
    assert_eq!(
        transition.artifacts, rows,
        "the transition carries the same preserved rows the record does",
    );

    let current = observed(&run, GUIDE).expect("the mutated file is still witnessable");
    assert_ne!(
        current, baseline.digest,
        "and THIS invocation observed the mutation — the current half verify compares",
    );
    assert_eq!(
        observed(&run, "docs/reference.md"),
        row_for(&rows, "docs/reference.md")
            .witness
            .as_ref()
            .map(|witness| witness.digest.clone()),
        "the untouched sibling observes equal to its own baseline",
    );
}

/// An unchanged output: baseline preserved, and the current observation equals
/// it. This is the `matched` case A5 will report, and it must arrive as two
/// separately-derived equal values rather than one value compared to itself.
#[test]
fn an_unchanged_output_observes_equal_to_its_preserved_baseline() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &backend, RUN_ID);
    let baseline = row_for(&rows(fixture.root.as_path()), GUIDE)
        .witness
        .clone()
        .unwrap();

    let second = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let (run, _) = cli_run(fixture.root.as_path(), &second, SECOND_RUN);

    let preserved = row_for(&rows(fixture.root.as_path()), GUIDE).clone();
    assert_eq!(preserved.witness.unwrap().digest, baseline.digest);
    assert_eq!(preserved.measured_run_id.as_deref(), Some(RUN_ID));
    assert_eq!(observed(&run, GUIDE), Some(baseline.digest));
}

/// A current observation that REFUSES must not erase the baseline either. The
/// artifact is deleted outright; the declared inputs are untouched, so the
/// row would fresh-skip — but an agent contract row also physically probes its
/// outputs, so this drives the carriage through a reply-shaped batch instead
/// (see the blast-radius test for why that split exists).
#[test]
fn a_refused_current_observation_never_erases_the_baseline() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(fixture.root.as_path(), &backend, RUN_ID);
    let baseline = row_for(&rows(fixture.root.as_path()), GUIDE)
        .witness
        .clone()
        .unwrap();

    fs::remove_file(fixture.root.join(GUIDE)).unwrap();
    let text = vibe_core::machine_json_path(fixture.root.as_path());
    let observer = crate::artifacts::observe::ArtifactObserver::new(&text);
    let outcome = observer.observe(GUIDE, &format!("{text}/{GUIDE}"));
    assert!(
        matches!(
            outcome,
            crate::artifacts::observe::WitnessOutcome::Refused(
                crate::artifacts::observe::WitnessRefusal::Absent
            )
        ),
        "the current observation is a typed refusal, not a witness",
    );

    // The preserved row is what a fresh skip checkpoints: byte-for-byte the
    // prior pair, with the refusal living only in the invocation map.
    let rows = rows(fixture.root.as_path());
    let preserved = row_for(&rows, GUIDE);
    assert_eq!(preserved.witness.as_ref().unwrap().digest, baseline.digest);
    assert_eq!(preserved.measured_run_id.as_deref(), Some(RUN_ID));
}

/// A legacy row with no witness is not upgraded by looking at it. A fresh skip
/// that minted a baseline here would claim a measurement no execution ever
/// produced; A5 owes `unavailable` with an optional current observation
/// instead, which is exactly the shape this leaves behind.
#[test]
fn a_legacy_unwitnessed_row_is_not_upgraded_by_a_fresh_skip() {
    let fixture = scratch();
    let root = fixture.root.as_path();
    seed_input(root);
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(root, &backend, RUN_ID);

    // Rewrite the durable record to the pre-R7.5 shape: identity only.
    let mut durable = state(root);
    for artifact in &mut durable.execution.get_mut(KEY).unwrap().artifacts {
        artifact.witness = None;
        artifact.measured_run_id = None;
    }
    write_state(root, &durable);

    let second = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let (run, transition) = cli_run(root, &second, SECOND_RUN);
    assert_eq!(transition.status, ExecutionRecordStatus::Fresh);

    let rows = rows(root);
    let preserved = row_for(&rows, GUIDE);
    assert!(
        preserved.witness.is_none() && preserved.measured_run_id.is_none(),
        "legacy absence survives the skip — absence is a baseline too",
    );
    assert!(
        observed(&run, GUIDE).is_some(),
        "while the invocation still observed the current object for A5",
    );
}

/// The directory equivalent, and the reason empty descendants are entries:
/// deleting ONLY an empty subdirectory of a declared directory artifact must
/// move its witness.
#[test]
fn deleting_an_empty_descendant_of_a_directory_artifact_moves_its_witness() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let root = fixture.root.as_path();
    fs::create_dir_all(root.join("docs/hooks")).unwrap();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    cli_row(root, &backend, RUN_ID);

    // Re-witness `docs/` as a directory artifact through the same observer the
    // runner uses, before and after the empty child goes.
    let text = vibe_core::machine_json_path(root);
    let observer = crate::artifacts::observe::ArtifactObserver::new(&text);
    let before = observer.observe("docs", &format!("{text}/docs"));
    fs::remove_dir(root.join("docs/hooks")).unwrap();
    let after = observer.observe("docs", &format!("{text}/docs"));
    assert_ne!(
        before, after,
        "an empty descendant is an entry, so removing it is a change",
    );
}

/// A refused artifact keeps its identity row and writes NEITHER half of the
/// pair, while a clean sibling in the same batch stays witnessed.
///
/// Driven through the carriage helper rather than an agent execution on
/// purpose: for an AGENT contract row the freshness probe and the witness read
/// the same law — `probe_regular_nonempty` already refuses the hard link that
/// makes an output unwitnessable — so that row can never reach a checkpoint
/// carrying a refusal. Ordinary reply artifacts can (nothing contract-probes
/// them), and this is the shape they take.
#[test]
fn one_refused_artifact_leaves_only_the_clean_row_witnessed() {
    let fixture = scratch();
    let root = fixture.root.as_path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(GUIDE), "# Guide\n").unwrap();
    fs::write(root.join("docs/reference.md"), "# Reference\n").unwrap();
    fs::write(root.join("twin.md"), "# Guide\n").unwrap();
    fs::remove_file(root.join(GUIDE)).unwrap();
    if fs::hard_link(root.join("twin.md"), root.join(GUIDE)).is_err() {
        return;
    }

    let text = vibe_core::machine_json_path(root);
    let observer = crate::artifacts::observe::ArtifactObserver::new(&text);
    let batch = [GUIDE, "docs/reference.md"]
        .map(|relative| {
            let outcome = observer.observe(relative, &format!("{text}/{relative}"));
            crate::artifacts::observe::state_row(
                RUN_ID,
                relative.to_string(),
                "file".to_string(),
                format!("{text}/{relative}"),
                &outcome,
            )
        })
        .to_vec();

    let refused = row_for(&batch, GUIDE);
    assert!(refused.witness.is_none(), "a hard-linked output is refused");
    assert!(
        refused.measured_run_id.is_none(),
        "and the pair is dropped WHOLE — never a run id beside no witness",
    );
    assert_eq!(
        refused.path,
        format!("{text}/{GUIDE}"),
        "the identity row survives the refusal, so the reopen path is not lost",
    );

    let clean = row_for(&batch, "docs/reference.md");
    assert!(clean.witness.is_some(), "the sibling is untouched");
    assert_eq!(clean.measured_run_id.as_deref(), Some(RUN_ID));
}

/// A dispatch failure records no artifacts at all, so there is nothing to
/// witness — the pair is absent because the row is.
#[test]
fn a_dispatch_failure_witnesses_nothing() {
    let fixture = scratch();
    seed_input(fixture.root.as_path());
    let backend = RecordingBackend::refusing_provider("the provider refused");
    let mut run = LifecycleRun::begin(
        lease(fixture.root.as_path()),
        project_fixture(fixture.root.as_path()),
        world_fixture(fixture.root.as_path()),
        cli_metadata(RUN_ID),
        vec!["create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row());
    let rt = runtime(&backend);
    run.execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap_err();

    let state = state(fixture.root.as_path());
    assert_eq!(state.execution[KEY].status, ExecutionRecordStatus::Fail);
    assert!(state.execution[KEY].artifacts.is_empty());
}

/// **B3, the three-invocation convergence.** A park writes planned rows with
/// no witness; the satisfied resume adopts the host's outputs and witnesses
/// them; and a THIRD invocation must fresh-skip that completed row — no task,
/// no provider, no re-park — and re-probe under its own run id.
///
/// Before the repair the third invocation compared whole rows against the
/// witness-free planned contract, rejected its own recorded output and parked
/// again, forever.
#[test]
fn a_witnessed_hosted_row_converges_on_the_third_invocation() {
    let fixture = scratch();
    let root = fixture.root.as_path();
    seed_input(root);
    let chain = || vec!["validate".to_string(), "create".to_string()];

    // 1. Park.
    let parking = RecordingBackend::answering(r#"{"outputs":[]}"#);
    {
        let mut run = LifecycleRun::begin(
            lease(root),
            project_fixture(root),
            world_fixture(root),
            metadata(RUN_ID, false),
            chain(),
        )
        .unwrap();
        let parked = run
            .execute_one(
                &HandlerExecution::from_row(&row()),
                "create",
                ExecutionReuse::FreshnessAware,
                &runtime(&parking),
            )
            .unwrap();
        assert_eq!(parked.status, ExecutionRecordStatus::Delegated);
    }
    for artifact in rows(root) {
        assert!(artifact.witness.is_none(), "a planned row is a contract");
        assert!(artifact.measured_run_id.is_none());
    }

    // 2. The host writes the declared outputs; the resume adopts and witnesses.
    //    It runs under the PARKING run id on purpose — adoption is identity,
    //    and a fresh run id would drop the delegated row instead of resuming.
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(GUIDE), "hosted body\n").unwrap();
    fs::write(root.join("docs/reference.md"), "hosted reference\n").unwrap();
    let resuming = RecordingBackend::answering(r#"{"outputs":[]}"#);
    {
        let mut run = LifecycleRun::begin(
            lease(root),
            project_fixture(root),
            world_fixture(root),
            metadata(RUN_ID, false),
            chain(),
        )
        .unwrap();
        let resumed = run
            .execute_one(
                &HandlerExecution::from_row(&row()),
                "create",
                ExecutionReuse::FreshnessAware,
                &runtime(&resuming),
            )
            .unwrap();
        assert_eq!(resumed.status, ExecutionRecordStatus::Ok);
    }
    assert_eq!(resuming.calls(), 0, "a satisfied resume spends nothing");
    let adopted = rows(root);
    let adopted_digest = row_for(&adopted, GUIDE).witness.clone().unwrap().digest;
    assert_eq!(
        row_for(&adopted, GUIDE).measured_run_id.as_deref(),
        Some(RUN_ID),
        "the adopting run owns the witness it took",
    );

    // 3. The convergence step: a witnessed row must not re-park.
    let third = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let mut run = LifecycleRun::begin(
        lease(root),
        project_fixture(root),
        world_fixture(root),
        metadata(THIRD_RUN, false),
        chain(),
    )
    .unwrap();
    let transition = run
        .execute_one(
            &HandlerExecution::from_row(&row()),
            "create",
            ExecutionReuse::FreshnessAware,
            &runtime(&third),
        )
        .unwrap();
    assert_eq!(
        transition.status,
        ExecutionRecordStatus::Fresh,
        "a completed hosted row fresh-skips; it does not re-park",
    );
    assert!(transition.delegation.is_none(), "and publishes no new task");
    assert!(run.parked_delegation().is_none());
    assert_eq!(third.calls(), 0, "and spends nothing");

    let final_rows = rows(root);
    let converged = row_for(&final_rows, GUIDE);
    assert_eq!(
        converged.witness.clone().unwrap().digest,
        adopted_digest,
        "the baseline stays the one the hosted resume ACCEPTED",
    );
    assert_eq!(
        converged.measured_run_id.as_deref(),
        Some(RUN_ID),
        "and keeps the accepting run's id — the third invocation produced nothing, \
         so it may not mint a third-run baseline",
    );
    assert_eq!(
        observed(&run, GUIDE),
        Some(adopted_digest),
        "while the third invocation still observed the current object for A5",
    );
}

/// The repair narrowed the probe to `(id, kind, path)` and no further: a
/// tampered identity must still refuse, or a rewritten `path` would survive
/// into the hydrated envelope as a real artifact this run produced. An
/// ADDITIVE witness on an otherwise exact row must still be accepted — that
/// is the whole point of the repair.
#[test]
fn the_probe_accepts_an_added_witness_but_never_a_tampered_identity() {
    let fixture = scratch();
    let root = fixture.root.as_path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(GUIDE), "# Guide\n").unwrap();
    fs::write(root.join("docs/reference.md"), "# Reference\n").unwrap();

    let row = row();
    let context = crate::agent::tests::support::context(root, &row);
    let contract = crate::agent::OutputContract::parse(&context).unwrap();
    let text = vibe_core::machine_json_path(root);
    let planned = contract.planned_state_rows(&text);
    assert!(
        crate::agent::probe_outputs(root, &contract, &planned),
        "the exact planned identity is accepted",
    );

    let observer = crate::artifacts::observe::ArtifactObserver::new(&text);
    let witnessed = planned
        .iter()
        .map(|artifact| {
            let outcome = observer.observe(&artifact.id, &artifact.path);
            crate::artifacts::observe::state_row(
                RUN_ID,
                artifact.id.clone(),
                artifact.kind.clone(),
                artifact.path.clone(),
                &outcome,
            )
        })
        .collect::<Vec<_>>();
    assert!(
        witnessed.iter().all(|artifact| artifact.witness.is_some()),
        "the fixture rows really are witnessed, so the next assert means something",
    );
    assert!(
        crate::agent::probe_outputs(root, &contract, &witnessed),
        "evidence describes the output; it does not redefine which was promised",
    );

    for mutate in [
        (|artifact: &mut StateArtifact| artifact.id.push_str("-tampered"))
            as fn(&mut StateArtifact),
        |artifact: &mut StateArtifact| artifact.kind = "directory".to_string(),
        |artifact: &mut StateArtifact| artifact.path = format!("{}-elsewhere", artifact.path),
    ] {
        let mut tampered = witnessed.clone();
        mutate(&mut tampered[0]);
        assert!(
            !crate::agent::probe_outputs(root, &contract, &tampered),
            "a tampered identity is not the contract's row",
        );
    }
    assert!(
        !crate::agent::probe_outputs(root, &contract, &witnessed[..1]),
        "and a short row set is not the contract's set",
    );
}
