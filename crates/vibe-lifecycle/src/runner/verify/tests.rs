//! The verify-reconciliation REDs (R7.5 P2/A5).
//!
//! Every case drives the REAL engine — an execution through
//! `LifecycleRun::execute_one`, then `reconcile_verification` on the same run
//! — so what is asserted is what a `vibe verify` would produce, not a
//! hand-assembled member.
//!
//! The decisive ones are [`a_mutated_artifact_is_stale_and_writes_no_state`]
//! and [`a_mutated_declared_input_is_stale`]: both fail the moment the
//! reconciler compares a durable claim against itself instead of against what
//! the verify instant can see.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use vibe_wire::generated::lifecycle::e1::context::{Project, RunAgentMode, World};
use vibe_wire::generated::lifecycle_state::LifecycleState;
use vibe_wire::generated::shared::{
    ArtifactWitness, EvidenceStatus, InputMeasurement, Timestamp, VerificationEvidence,
};

use crate::agent::tests::support::{
    PROMPT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT, row_with_inputs,
};
use crate::execution::HandlerExecution;
use crate::handlers::{HandlerRuntime, NoPackageBindingBackend};
use crate::process::{StreamMode, SystemProcessRunner};
use crate::{
    ExecutableContribution, ExecutionReuse, ExtensionRegistryRow, LifecycleLease, LifecycleRun,
    RunMetadata,
};
use vibe_workspace::hooks::SystemProbe;

const RUN_ID: &str = "00112233445566778899aabbccddeeff";
const SECOND_RUN: &str = "ffeeddccbbaa99887766554433221100";
const KEY: &str = "org.demo/tools#produce";
const GUIDE: &str = "docs/guide.md";
const OBSERVED: &str = "2026-08-28T12:00:05Z";
const LATER: &str = "2026-08-28T18:30:00Z";

struct Scratch {
    _dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

fn scratch() -> Scratch {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::write(dir.path().join("data/a.txt"), "one").unwrap();
    Scratch {
        root: dir.path().to_path_buf(),
        _dir: dir,
    }
}

fn lease(root: &Path) -> Arc<LifecycleLease> {
    Arc::new(LifecycleLease::acquire(root).expect("a temp root is leasable"))
}

/// A CLI-mode run over the full default chain THROUGH verify — the header a
/// reconciliation publishes, and the one the wire's run-identity law reads.
fn metadata(run_id: &str) -> RunMetadata {
    RunMetadata {
        requested: "verify".into(),
        chain: [
            "validate", "install", "generate", "build", "test", "create", "verify",
        ]
        .iter()
        .map(|phase| (*phase).to_string())
        .collect(),
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: run_id.into(),
        started: "2026-08-28T11:59:40Z".into(),
        selected: ".".into(),
    }
}

fn project_fixture(root: &Path) -> Project {
    let text = root.to_string_lossy().replace('\\', "/");
    Project {
        kind: "project".into(),
        manifest: format!("{text}/vibe.toml"),
        name: "demo".into(),
        root: text,
        spec_roots: Vec::new(),
        version: "0.1.0".into(),
    }
}

fn world_fixture(root: &Path) -> World {
    let text = root.to_string_lossy().replace('\\', "/");
    World {
        deps_root: format!("{text}/vibedeps"),
        lockfile: format!("{text}/vibe.lock"),
        packages: Vec::new(),
    }
}

fn runtime(backend: &RecordingBackend) -> HandlerRuntime<'_> {
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    HandlerRuntime {
        process: &PROCESS,
        binary: &crate::handlers::NoBinaryBackend,
        package_binding: &NoPackageBindingBackend,
        agent: backend,
        probe: &SystemProbe,
        streams: StreamMode::Capture,
    }
}

fn row(patterns: &[&str]) -> ExtensionRegistryRow {
    row_with_inputs(
        TWO_OUTPUTS,
        PROMPT,
        Some(patterns.iter().map(|p| (*p).to_string()).collect()),
    )
}

fn contribution(row: &ExtensionRegistryRow) -> ExecutableContribution {
    ExecutableContribution {
        phase: "create".into(),
        row: row.clone(),
    }
}

fn begin(root: &Path, run_id: &str) -> LifecycleRun {
    LifecycleRun::begin(
        lease(root),
        project_fixture(root),
        world_fixture(root),
        metadata(run_id),
        vec!["create".into(), "verify".into()],
    )
    .unwrap()
}

/// One executed create row, and the run that executed it — still live, so the
/// caller reconciles the SAME invocation that produced the record.
fn executed(root: &Path, backend: &RecordingBackend) -> LifecycleRun {
    let mut run = begin(root, RUN_ID);
    let handler = HandlerExecution::from_row(&row(&["data/**"]));
    let rt = runtime(backend);
    run.execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .expect("the create row executes");
    run
}

fn observed_at(text: &str) -> Timestamp {
    text.parse().expect("a fixture timestamp parses")
}

fn reconcile(
    run: &mut LifecycleRun,
    backend: &RecordingBackend,
    prefix: &[ExecutableContribution],
) -> VerificationEvidence {
    run.reconcile_verification(prefix, backend, observed_at(OBSERVED))
        .expect("the reconciliation assembles a publishable member")
}

fn input_row<'a>(member: &'a VerificationEvidence, execution: &str) -> &'a InputMeasurement {
    member
        .inputs
        .iter()
        .find(|row| row.execution == execution)
        .expect("the declared-input row is present")
}

fn artifact_row<'a>(member: &'a VerificationEvidence, id: &str) -> &'a ArtifactWitness {
    member
        .artifacts
        .iter()
        .find(|row| row.id == id)
        .expect("the accumulated artifact row is present")
}

fn state_bytes(root: &Path) -> Vec<u8> {
    fs::read(root.join(".vibe/lifecycle.toml")).unwrap()
}

fn state(root: &Path) -> LifecycleState {
    toml::from_str(&fs::read_to_string(root.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}

fn write_state(root: &Path, state: &LifecycleState) {
    fs::write(
        root.join(".vibe/lifecycle.toml"),
        toml::to_string(state).unwrap(),
    )
    .unwrap();
}

/// An untouched tree reconciles to a valid, wholly matched member: the current
/// declaration equals the measured one, the re-walked scope equals the
/// measured manifest, and every produced artifact still is what it was.
#[test]
fn an_unchanged_prefix_reconciles_to_matched() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);

    let member = reconcile(&mut run, &backend, &[contribution(&row(&["data/**"]))]);
    assert_eq!(member.status, EvidenceStatus::Matched);
    assert_eq!(member.run.run_id, RUN_ID);
    assert_eq!(member.run.requested, "verify");
    assert_eq!(
        member.run.chain.len(),
        7,
        "the CURRENT full requested chain"
    );
    let inputs = input_row(&member, KEY);
    assert_eq!(inputs.status, EvidenceStatus::Matched);
    assert_eq!(inputs.patterns, vec!["data/**".to_string()]);
    assert!(inputs.reason_code.is_none(), "matched owes no reason");
    assert_eq!(inputs.measured_run_id.as_deref(), Some(RUN_ID));
    assert_eq!(inputs.measured, inputs.observed);
    assert_eq!(member.artifacts.len(), 2);
    assert_eq!(artifact_row(&member, GUIDE).status, EvidenceStatus::Matched);
    assert_eq!(artifact_row(&member, GUIDE).path, GUIDE);
}

/// A declared input changed after it was measured: the re-walk sees other
/// bytes under the SAME declaration, so the row is stale on its digests alone.
#[test]
fn a_mutated_declared_input_is_stale() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);
    fs::write(fixture.root.join("data/a.txt"), "two").unwrap();

    let member = reconcile(&mut run, &backend, &[contribution(&row(&["data/**"]))]);
    let inputs = input_row(&member, KEY);
    assert_eq!(inputs.status, EvidenceStatus::Stale);
    assert_eq!(member.status, EvidenceStatus::Stale);
    assert_ne!(
        inputs.measured, inputs.observed,
        "both halves are present and they differ — that IS the reason",
    );
    assert!(inputs.reason_code.is_none());
}

/// A produced artifact mutated between its execution and verify is stale, the
/// durable baseline is untouched, and the reconciliation writes no state at
/// all — the three facts that together make E5 impossible to fake.
#[test]
fn a_mutated_artifact_is_stale_and_writes_no_state() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);
    let baseline = state(fixture.root.as_path()).execution[KEY]
        .artifacts
        .iter()
        .find(|artifact| artifact.id == GUIDE)
        .and_then(|artifact| artifact.witness.clone())
        .expect("production witnessed the artifact");
    let before = state_bytes(fixture.root.as_path());
    fs::write(fixture.root.join(GUIDE), "externally rewritten\n").unwrap();

    let member = reconcile(&mut run, &backend, &[contribution(&row(&["data/**"]))]);
    let guide = artifact_row(&member, GUIDE);
    assert_eq!(guide.status, EvidenceStatus::Stale);
    assert_eq!(member.status, EvidenceStatus::Stale);
    assert_eq!(
        guide.measured.as_ref().map(|w| w.digest.clone()),
        Some(baseline.digest.clone()),
        "the durable production witness stays the baseline",
    );
    assert_ne!(guide.measured, guide.observed);
    assert_eq!(
        state_bytes(fixture.root.as_path()),
        before,
        "reconciliation observes; it never rewrites the freshness cache",
    );
}

/// A produced artifact that is simply gone is `missing`, not `unstable`: the
/// object is owed and absent, which is a different fact from an observation
/// that could not be established.
#[test]
fn a_deleted_artifact_is_missing() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);
    fs::remove_file(fixture.root.join(GUIDE)).unwrap();

    let member = reconcile(&mut run, &backend, &[contribution(&row(&["data/**"]))]);
    let guide = artifact_row(&member, GUIDE);
    assert_eq!(guide.status, EvidenceStatus::Missing);
    assert_eq!(guide.reason_code.as_deref(), Some("artifact-absent"));
    assert!(guide.observed.is_none());
    assert_eq!(member.status, EvidenceStatus::Missing);
}

/// A legacy artifact — a row a pre-R7.5 writer left with no witness — is
/// visible as `unavailable`. It neither vanishes nor is upgraded into a
/// baseline by the mere fact that this run could read the path, and the input
/// half of the same record is judged independently.
#[test]
fn a_legacy_unwitnessed_artifact_is_unavailable() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    drop(executed(fixture.root.as_path(), &backend));
    let mut legacy = state(fixture.root.as_path());
    for artifact in &mut legacy.execution.get_mut(KEY).unwrap().artifacts {
        artifact.witness = None;
        artifact.measured_run_id = None;
    }
    write_state(fixture.root.as_path(), &legacy);

    // The prefix must have COMPLETED in this invocation, so the row is run
    // again: it fresh-skips, preserving the unwitnessed rows byte-for-byte.
    let second = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = begin(fixture.root.as_path(), SECOND_RUN);
    let handler = HandlerExecution::from_row(&row(&["data/**"]));
    let rt = runtime(&second);
    let fresh = run
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .expect("the recorded row fresh-skips");
    assert!(fresh.is_fresh());

    let member = reconcile(&mut run, &second, &[contribution(&row(&["data/**"]))]);
    let guide = artifact_row(&member, GUIDE);
    assert_eq!(guide.status, EvidenceStatus::Unavailable);
    assert_eq!(guide.reason_code.as_deref(), Some("artifact-unwitnessed"));
    assert!(guide.measured.is_none() && guide.measured_run_id.is_none());
    assert!(
        guide.observed.is_some(),
        "a safe current reading still rides an unavailable row",
    );
    assert_eq!(
        input_row(&member, KEY).status,
        EvidenceStatus::Matched,
        "the two halves are judged independently",
    );
    assert_eq!(member.status, EvidenceStatus::Unavailable);
}

/// A completed row that could not measure its declared scope attributes no
/// measurement at all, so verify reports `unavailable` rather than inventing a
/// baseline out of the reading it can take now.
#[test]
fn an_unmeasured_input_row_is_unavailable() {
    let fixture = scratch();
    // The measurement is refused at execution time (a hard link is not a
    // single-link file), while the legacy fingerprint keeps its bytes.
    fs::remove_file(fixture.root.join("data/a.txt")).unwrap();
    fs::write(fixture.root.join("twin.bin"), "one").unwrap();
    fs::hard_link(
        fixture.root.join("twin.bin"),
        fixture.root.join("data/a.txt"),
    )
    .unwrap();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);
    assert!(
        state(fixture.root.as_path()).execution[KEY]
            .input_measurement
            .is_none(),
        "the refused observation stored no measurement",
    );
    // The refusal was transient: the same bytes, now a plain file.
    fs::remove_file(fixture.root.join("data/a.txt")).unwrap();
    fs::write(fixture.root.join("data/a.txt"), "one").unwrap();

    let member = reconcile(&mut run, &backend, &[contribution(&row(&["data/**"]))]);
    let inputs = input_row(&member, KEY);
    assert_eq!(inputs.status, EvidenceStatus::Unavailable);
    assert_eq!(inputs.reason_code.as_deref(), Some("input-unwitnessed"));
    assert!(inputs.measured.is_none() && inputs.measured_run_id.is_none());
    assert!(
        inputs.observed.is_some(),
        "a safe current reading still rides an unavailable row",
    );
    assert_eq!(member.status, EvidenceStatus::Unavailable);
}

/// The artifact universe is the invocation's ACCUMULATION, not the phase
/// prefix. An install-stage slot execution accumulates through the same seam
/// and is named by no `RitualPlan` row — modelled here by reconciling with an
/// EMPTY prefix — and its output must still be compared, not silently dropped.
#[test]
fn an_accumulated_artifact_survives_an_empty_phase_prefix() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);

    let member = reconcile(&mut run, &backend, &[]);
    assert!(
        member.inputs.is_empty(),
        "the declared-input half stays the phase prefix's alone",
    );
    assert_eq!(
        member.artifacts.len(),
        2,
        "the accumulation is the universe"
    );
    assert_eq!(artifact_row(&member, GUIDE).status, EvidenceStatus::Matched);
    assert_eq!(member.status, EvidenceStatus::Matched);
}

/// A parked row made no durable production, so it remembers no baseline and
/// accumulates nothing — even though its state record carries the PLANNED
/// rows. A reconciler that read those rows out of state would publish a
/// comparison about work the hosting agent has not done yet.
#[test]
fn a_parked_row_remembers_no_baseline() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let mut hosted = metadata(RUN_ID);
    hosted.agent_mode = RunAgentMode::Agent;
    let mut run = LifecycleRun::begin(
        lease(fixture.root.as_path()),
        project_fixture(fixture.root.as_path()),
        world_fixture(fixture.root.as_path()),
        hosted,
        vec!["create".into(), "verify".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row(&["data/**"]));
    let rt = runtime(&backend);
    let parked = run
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .expect("the hosted row parks");
    assert!(parked.delegation.is_some());
    assert_eq!(
        state(fixture.root.as_path()).execution[KEY].artifacts.len(),
        2,
        "the park records the PLANNED rows in state",
    );

    let member = reconcile(&mut run, &backend, &[]);
    assert!(
        member.artifacts.is_empty(),
        "planned rows are not accumulated artifacts and own no baseline",
    );
    assert_eq!(member.status, EvidenceStatus::Unavailable);
}

/// A declaration that no longer matches the one that was measured is stale on
/// IDENTITY, and the published row carries the CURRENT spelling while the
/// measured half stays visibly prior.
#[test]
fn a_changed_declaration_is_stale_under_the_current_fingerprint() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);
    let measured = state(fixture.root.as_path()).execution[KEY]
        .input_measurement
        .clone()
        .expect("the execution measured its scope");

    let member = reconcile(&mut run, &backend, &[contribution(&row(&["data/*.txt"]))]);
    let inputs = input_row(&member, KEY);
    assert_eq!(inputs.status, EvidenceStatus::Stale);
    assert_eq!(
        inputs.reason_code.as_deref(),
        Some("input-declaration-changed")
    );
    assert_eq!(inputs.patterns, vec!["data/*.txt".to_string()]);
    assert_ne!(
        inputs.declaration_fingerprint, measured.declaration_fingerprint,
        "the row carries the CURRENT declaration, not the measured one",
    );
    assert_eq!(
        inputs.measured.as_ref().map(|w| w.digest.clone()),
        Some(measured.witness.digest.clone()),
        "the measured half stays the prior claim",
    );
    assert_eq!(inputs.measured_run_id.as_deref(), Some(RUN_ID));
}

/// A current observation the evidence law refuses, with a prior measurement in
/// hand, is `unstable` under a cause-specific reason — never `stale` (nothing
/// was compared) and never `matched`.
#[test]
fn a_refused_current_observation_is_unstable() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);
    fs::remove_file(fixture.root.join("data/a.txt")).unwrap();
    fs::write(fixture.root.join("twin.bin"), "one").unwrap();
    fs::hard_link(
        fixture.root.join("twin.bin"),
        fixture.root.join("data/a.txt"),
    )
    .unwrap();

    let member = reconcile(&mut run, &backend, &[contribution(&row(&["data/**"]))]);
    let inputs = input_row(&member, KEY);
    assert_eq!(inputs.status, EvidenceStatus::Unstable);
    assert!(inputs.observed.is_none(), "nothing comparable was obtained");
    assert!(inputs.measured.is_some(), "something WAS measured");
    let reason = inputs
        .reason_code
        .as_deref()
        .expect("a cause-specific code");
    assert!(
        reason.starts_with("input-") && reason != "input-unwitnessed",
        "unstable names WHY the observation refused: {reason}",
    );
    assert_eq!(member.status, EvidenceStatus::Unstable);
}

/// A project with nothing to compare still gets the member: an empty evidence
/// set is the `unavailable` root, said out loud.
#[test]
fn an_empty_prefix_is_an_unavailable_root() {
    let fixture = scratch();
    let mut run = begin(fixture.root.as_path(), RUN_ID);
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);

    let member = reconcile(&mut run, &backend, &[]);
    assert_eq!(member.status, EvidenceStatus::Unavailable);
    assert!(member.inputs.is_empty() && member.artifacts.is_empty());
    assert!(member.evidence_id.starts_with("sha256:"));
}

/// The `evidence_id` cases live beside this cell — same 600-line budget, and
/// identity is its own subject.
#[cfg(test)]
#[path = "tests/identity.rs"]
mod identity;
