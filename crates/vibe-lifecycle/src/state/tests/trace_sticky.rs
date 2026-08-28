//! Sticky compile-trace activation and state-proven displacement — the
//! trace half of run-identity selection (PROP-054 `##OBS-TRACE`,
//! R3.4 §5.3). The adoption matrix itself lives in `adoption.rs` and
//! every one of its oracles stays green beside these; this file pins
//! ONLY the three new facts: the effective sticky bit, the durable
//! `compile_trace` state member, and the exact `SupersededTrace`
//! ownership claim a displaced traced park hands the command owner.

use std::fs;
use std::path::Path;

use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use super::{RUN_ID, lease, record_for};
use crate::{LifecycleStateStore, SupersededTrace, select_run_identity};

const CHAIN: [&str; 3] = ["validate", "install", "create"];
const STARTED: &str = "2026-08-26T00:00:00Z";
const FRESH: &str = "2026-08-26T09:00:00Z";
const KEY: &str = "org.demo/tools#produce";

fn chain(phases: &[&str]) -> Vec<String> {
    phases.iter().map(|phase| (*phase).to_string()).collect()
}

/// Write a state that parked `KEY` under `RUN_ID` for `vibe create`,
/// with the run header carrying the given sticky bit.
fn parked(root: &Path, compile_trace: bool) {
    let mut store = LifecycleStateStore::begin(
        lease(root),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
        compile_trace,
    )
    .unwrap();
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Delegated, "sha256:x"),
        )
        .unwrap();
}

fn select(
    root: &Path,
    requested: &str,
    phases: &[&str],
    mode: RunAgentMode,
    force: bool,
    current_request: bool,
) -> crate::RunIdentity {
    let lease = lease(root);
    select_run_identity(
        &lease,
        root,
        requested,
        &chain(phases),
        mode,
        force,
        current_request,
        FRESH.into(),
    )
    .unwrap()
}

/// Case 1 — a current request on a fresh root (no prior state at all)
/// is the whole effective value: traced run, fresh identity, nothing
/// claimed.
#[test]
fn a_current_request_on_a_fresh_root_selects_an_effective_traced_identity() {
    let dir = tempfile::tempdir().unwrap();
    let identity = select(
        dir.path(),
        "create",
        &CHAIN,
        RunAgentMode::Agent,
        false,
        true,
    );
    assert!(!identity.adopted);
    assert!(identity.compile_trace, "a fresh run traces when asked to");
    assert_eq!(identity.started, FRESH);
    assert!(
        identity.superseded_trace.is_none(),
        "no prior state owns anything to displace"
    );
}

/// Case 2 — an adopted traced run stays traced with NO current request;
/// the bit is why it is called sticky, and adoption claims nothing.
#[test]
fn an_adopted_traced_run_stays_traced_without_a_current_request() {
    let dir = tempfile::tempdir().unwrap();
    parked(dir.path(), true);
    let identity = select(
        dir.path(),
        "create",
        &CHAIN,
        RunAgentMode::Agent,
        false,
        false,
    );
    assert!(identity.adopted);
    assert_eq!(identity.run_id, RUN_ID);
    assert!(
        identity.compile_trace,
        "the parked run's own bit survives its resume"
    );
    assert!(
        identity.superseded_trace.is_none(),
        "an adopted run supersedes nothing"
    );
}

/// Case 3 — an adopted UNtraced run upgrades when the resume requests
/// tracing, and the state's `begin` write makes the upgrade durable in
/// exactly the local writer convention: `compile_trace = true` once
/// set, the member absent while false (byte-compatible with every
/// pre-R3.4 file).
#[test]
fn an_adopted_untraced_run_upgrades_and_state_begin_writes_it() {
    let dir = tempfile::tempdir().unwrap();
    parked(dir.path(), false);
    let identity = select(
        dir.path(),
        "create",
        &CHAIN,
        RunAgentMode::Agent,
        false,
        true,
    );
    assert!(identity.adopted);
    assert!(
        identity.compile_trace,
        "current request OR the adopted false bit upgrades the run"
    );

    let store = LifecycleStateStore::begin(
        lease(dir.path()),
        "create".into(),
        chain(&CHAIN),
        identity.started,
        identity.run_id,
        identity.compile_trace,
    )
    .unwrap();
    let text = fs::read_to_string(store.path()).unwrap();
    assert!(
        text.contains("compile_trace = true"),
        "the effective bit is durable: {text}"
    );
    let parsed = LifecycleStateStore::peek(dir.path())
        .unwrap()
        .expect("the state reads back");
    assert!(parsed.run.compile_trace);

    // The false convention omits the member — the fresh untraced write
    // stays byte-compatible with a pre-R3.4 file.
    let plain = tempfile::tempdir().unwrap();
    let store = LifecycleStateStore::begin(
        lease(plain.path()),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
        false,
    )
    .unwrap();
    let text = fs::read_to_string(store.path()).unwrap();
    assert!(
        !text.contains("compile_trace"),
        "a false bit writes no member: {text}"
    );
}

/// Case 4 — the displacement matrix: force, a different command, a
/// different chain and CLI mode each mint a fresh identity AND claim
/// exactly the prior traced park — its run id and original start,
/// nothing else.
#[test]
fn force_command_chain_or_mode_displacement_claims_the_prior_traced_park() {
    for (label, requested, phases, mode, force) in [
        (
            "--force",
            "create",
            CHAIN.as_slice(),
            RunAgentMode::Agent,
            true,
        ),
        (
            "a different requested phase",
            "build",
            CHAIN.as_slice(),
            RunAgentMode::Agent,
            false,
        ),
        (
            "a shorter chain",
            "create",
            ["install", "create"].as_slice(),
            RunAgentMode::Agent,
            false,
        ),
        (
            "cli mode",
            "create",
            CHAIN.as_slice(),
            RunAgentMode::Cli,
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        parked(dir.path(), true);
        let identity = select(dir.path(), requested, phases, mode, force, false);
        assert!(!identity.adopted, "{label} must not inherit the park");
        assert_ne!(identity.run_id, RUN_ID, "{label}");
        assert_eq!(identity.started, FRESH, "{label} restarts the clock");
        assert!(
            !identity.compile_trace,
            "{label}: a fresh run carries the current request only"
        );
        assert_eq!(
            identity.superseded_trace,
            Some(SupersededTrace {
                run_id: RUN_ID.to_string(),
                started: STARTED.to_string(),
            }),
            "{label}: the displaced traced park is named exactly"
        );
    }
}

/// Case 5 — a traced prior WITHOUT a delegated row is complete, not
/// parked: nothing is displaced, so nothing is claimed — and it never
/// adopts.
#[test]
fn a_traced_prior_without_a_delegated_row_claims_no_superseded_trace() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = LifecycleStateStore::begin(
        lease(dir.path()),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
        true,
    )
    .unwrap();
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x"),
        )
        .unwrap();
    drop(store);
    let identity = select(
        dir.path(),
        "create",
        &CHAIN,
        RunAgentMode::Agent,
        false,
        false,
    );
    assert!(!identity.adopted, "a complete run is not resumable");
    assert!(identity.superseded_trace.is_none());
}

/// Case 6 — an UNtraced displaced park owns no running trace to
/// terminalise: displacement without the sticky bit claims nothing.
#[test]
fn an_untraced_displaced_park_claims_no_superseded_trace() {
    let dir = tempfile::tempdir().unwrap();
    parked(dir.path(), false);
    let identity = select(
        dir.path(),
        "create",
        &CHAIN,
        RunAgentMode::Agent,
        true,
        false,
    );
    assert!(!identity.adopted);
    assert!(
        identity.superseded_trace.is_none(),
        "no sticky bit, no trace ownership to hand over"
    );
}

/// Case 7 — a pre-R3.4 file (no `compile_trace` member anywhere) reads
/// the bit as false, selects untraced, and survives a fresh begin
/// whose bytes stay member-for-member the old shape. The pre-existing
/// adoption oracles live in `adoption.rs` and run beside this file.
#[test]
fn a_pre_r34_state_reads_the_absent_bit_as_false() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "schema = 1\n\
         [run]\nrequested = 'create'\nchain = ['validate', 'install', 'create']\n\
         started = '2026-08-20T09:00:00Z'\n\
         [execution.'org.demo/tools#produce']\n\
         phase = 'create'\nfingerprint = 'sha256:old'\nstatus = 'ok'\nduration_ms = 4\n\
         artifacts = []\n",
    )
    .unwrap();
    let state = LifecycleStateStore::peek(dir.path())
        .unwrap()
        .expect("an old file still reads");
    assert!(!state.run.compile_trace, "the absent member defaults false");
    let identity = select(
        dir.path(),
        "create",
        &CHAIN,
        RunAgentMode::Agent,
        false,
        false,
    );
    assert!(!identity.adopted, "an identity-less run cannot be resumed");
    assert!(!identity.compile_trace);
    assert!(identity.superseded_trace.is_none());
}
