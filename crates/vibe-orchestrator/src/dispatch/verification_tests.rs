//! The verify-boundary reds, driven through the REAL dispatcher and the REAL
//! A5a reconciler (R7.5 P2/A5b).
//!
//! Nothing here stubs a member. Every assertion below is about a comparison
//! the engine actually made over a real tree, because the mutations these
//! exist to kill all live in the seam between the two: a boundary that fires
//! at the wrong index, an accumulator that carries rows but not the member, a
//! stop that travels silently, a projection that rebuilds what it was handed.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use vibe_lifecycle::{LifecycleLease, Phase, RunMetadata};
use vibe_wire::behaviour::verification_evidence::validate;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::shared::{EvidenceStatus, Timestamp};

use super::*;
use crate::failure::{Measurement, take};

const OBSERVED: &str = "2026-08-28T12:00:05Z";

fn observed_at() -> Timestamp {
    OBSERVED.parse().expect("a fixture instant")
}

/// A project whose build row DECLARES an input, so the reconciliation has a
/// real measured/observed pair to compare rather than an empty universe.
fn project(rows: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n{rows}"),
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::write(dir.path().join("data/a.txt"), "one").unwrap();
    dir
}

const DECLARED_BUILD_ROW: &str = "\n[[extension]]\nid = 'build-row'\npoint = 'phase:build'\n\
     handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"ROW-ONE\" }\n\
     inputs = [\"data/**\"]\n";
const UNDECLARED_BUILD_ROW: &str = "\n[[extension]]\nid = 'build-row'\npoint = 'phase:build'\n\
     handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"ROW-ONE\" }\n";
const VERIFY_ROW: &str = "\n[[extension]]\nid = 'verify-row'\npoint = 'phase:verify'\n\
     handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"SENTINEL\" }\n";

fn metadata(root: &Path, chain: &[Phase]) -> RunMetadata {
    let chain: Vec<String> = chain
        .iter()
        .map(|phase| phase.as_str().to_string())
        .collect();
    RunMetadata {
        requested: chain.last().cloned().unwrap_or_default(),
        chain,
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root).unwrap(),
        started: vibe_core::timestamp::now_utc(),
        selected: ".".into(),
    }
}

fn lease_for(root: &Path) -> Arc<LifecycleLease> {
    Arc::new(LifecycleLease::acquire(root).expect("the fixture root is leasable"))
}

/// An observer that can mutate the tree the instant a named row reports —
/// the only way to produce a REAL stale comparison from inside one dispatch,
/// because the change has to land after the measurement and before verify.
#[derive(Default)]
struct MutatingObserver {
    rows: std::sync::Mutex<Vec<LifecycleContributionReport>>,
    mutate_after: Option<(std::path::PathBuf, &'static str)>,
    machine_failure: bool,
}

impl RunObserver for MutatingObserver {
    fn stream_mode(&self) -> vibe_lifecycle::process::StreamMode {
        vibe_lifecycle::process::StreamMode::Null
    }

    fn binary_quiet(&self) -> bool {
        true
    }

    fn emit_machine_failure(&self) -> bool {
        self.machine_failure
    }

    fn observe_plan(
        &self,
        _plan: &RitualPlan,
        _metadata: &RunMetadata,
        _emit_empty: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn observe_contribution(&self, report: &LifecycleContributionReport) {
        self.rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(report.clone());
        if report.phase == Phase::Build.as_str()
            && let Some((path, bytes)) = self.mutate_after.as_ref()
        {
            fs::write(path, bytes).expect("the fixture input is writable");
        }
    }

    fn observe_untracked_failure(
        &self,
        _metadata: &RunMetadata,
        _phase: &str,
        _contributions: &[LifecycleContributionReport],
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl MutatingObserver {
    fn keys(&self) -> Vec<String> {
        self.rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|row| row.key.clone())
            .collect()
    }
}

fn agent() -> Arc<dyn AgentBackend> {
    Arc::new(vibe_lifecycle::NoAgentBackend)
}

fn dispatch(
    observer: &dyn RunObserver,
    root: &Path,
    phases: &[Phase],
    permission: Option<Timestamp>,
) -> Result<DispatchOutcome> {
    let plan = world::plan_default(root, phases).expect("the plan loads");
    let meta = metadata(root, phases);
    let state_chain = phases
        .iter()
        .map(|phase| phase.as_str().to_string())
        .collect();
    dispatch_plan(
        observer,
        &plan,
        lease_for(root),
        &agent(),
        meta,
        state_chain,
        permission,
    )
}

/// RED 1 — a complete `[build, verify]` chain whose declared inputs did not
/// move reconciles to a VALID matched member, even though the project has no
/// verify contribution at all: the boundary fires at the end of the prefix.
#[test]
fn an_unchanged_prefix_with_no_verify_row_still_publishes_a_matched_member() {
    let fixture = project(DECLARED_BUILD_ROW);
    let observer = MutatingObserver::default();

    let outcome = dispatch(
        &observer,
        fixture.path(),
        &[Phase::Build, Phase::Verify],
        Some(observed_at()),
    )
    .expect("a matched comparison continues past the boundary");

    let member = outcome
        .verification
        .expect("an empty verify phase still reconciles");
    validate(&member).expect("the published member obeys its own wire law");
    assert_eq!(member.status, EvidenceStatus::Matched);
    assert_eq!(member.observed_at, observed_at(), "the INJECTED instant");
    assert_eq!(member.inputs.len(), 1, "the declared build row: {member:?}");
    assert_eq!(member.inputs[0].status, EvidenceStatus::Matched);
    assert_eq!(member.run.requested, "verify");
}

/// RED 2 — nothing declared anything: the member is `unavailable`, it is
/// still published, and the command continues rather than inventing a policy
/// the project never declared.
#[test]
fn an_undeclared_project_publishes_an_unavailable_member_and_continues() {
    let fixture = project(&format!("{UNDECLARED_BUILD_ROW}{VERIFY_ROW}"));
    let observer = MutatingObserver::default();

    let outcome = dispatch(
        &observer,
        fixture.path(),
        &[Phase::Build, Phase::Verify],
        Some(observed_at()),
    )
    .expect("an unavailable comparison never stops verify");

    let member = outcome.verification.expect("the member is still published");
    validate(&member).expect("an empty comparison is still a valid member");
    assert_eq!(member.status, EvidenceStatus::Unavailable);
    assert!(member.inputs.is_empty() && member.artifacts.is_empty());
    assert_eq!(
        observer.keys().len(),
        2,
        "the verify row ran: {:?}",
        observer.keys(),
    );
}

/// RED 3 — a declared input really changes between its measurement and the
/// boundary: verify STOPS, the sentinel verify row is never dispatched, the
/// exact member travels on the carrier, and the stop uses the observer's
/// machine-document policy rather than the generic stage's silence.
#[test]
fn a_mutated_declared_input_stops_verify_before_its_contribution() {
    let fixture = project(&format!("{DECLARED_BUILD_ROW}{VERIFY_ROW}"));
    let observer = MutatingObserver {
        mutate_after: Some((fixture.path().join("data/a.txt"), "two")),
        machine_failure: true,
        ..MutatingObserver::default()
    };

    let error = dispatch(
        &observer,
        fixture.path(),
        &[Phase::Build, Phase::Verify],
        Some(observed_at()),
    )
    .expect_err("a stale comparison stops the chain");

    let carried = take(error).unwrap_or_else(|error| {
        panic!("a stale stop must arrive MEASURED, not bare: {error:#}");
    });
    assert!(
        carried.emit_machine_failure,
        "`vibe verify --json` must still return the member it is told to read",
    );
    assert!(
        format!("{:#}", carried.original).contains("verification evidence is `stale`"),
        "{:#}",
        carried.original,
    );
    let Measurement::Lifecycle {
        rows, verification, ..
    } = carried.evidence
    else {
        panic!("a verify stop measures lifecycle rows");
    };
    let member = verification.expect("the stop carries the comparison it made");
    validate(&member).expect("a stopping member is still a valid member");
    assert_eq!(member.status, EvidenceStatus::Stale);
    assert_eq!(rows.len(), 1, "the build row only: {rows:?}");
    assert_eq!(rows[0].phase, "build");
    let keys = observer.keys();
    assert_eq!(keys.len(), 1, "one row ran: {keys:?}");
    assert!(
        keys[0].ends_with("#build-row"),
        "the sentinel verify row was never dispatched: {keys:?}",
    );
}

/// A verify row that reports a semantic failure through the ordinary reply
/// protocol — the only handler kind that can fail on demand.
fn failing_verify_script(root: &Path) {
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(
        root.join("scripts/verify.sh"),
        "printf '%s' '{\"artifacts\":[],\"envelope\":1,\"message\":\"the guide is wrong\",\
         \"status\":\"fail\",\"tasks\":[]}' > \"$VIBE_REPLY\"\n",
    )
    .unwrap();
    fs::write(
        root.join("scripts/verify.ps1"),
        "'{\"artifacts\":[],\"envelope\":1,\"message\":\"the guide is wrong\",\
         \"status\":\"fail\",\"tasks\":[]}' | Set-Content -NoNewline $env:VIBE_REPLY\n",
    )
    .unwrap();
}

const FAILING_VERIFY_ROW: &str = "\n[[extension]]\nid = 'verify-row'\npoint = 'phase:verify'\n\
     handler = { kind = \"script\", base = \"scripts/verify\" }\n";

/// RED 4 — a verify HANDLER fails after a matched comparison. The command's
/// `ok` goes false and the identity member stays exactly `matched`: two axes,
/// and neither rewrites the other.
#[test]
fn a_failing_verify_handler_leaves_a_matched_member_untouched() {
    let fixture = project(&format!("{DECLARED_BUILD_ROW}{FAILING_VERIFY_ROW}"));
    failing_verify_script(fixture.path());
    let observer = MutatingObserver {
        machine_failure: true,
        ..MutatingObserver::default()
    };

    let error = dispatch(
        &observer,
        fixture.path(),
        &[Phase::Build, Phase::Verify],
        Some(observed_at()),
    )
    .expect_err("a failed verify contribution fails the command");

    let carried =
        take(error).unwrap_or_else(|error| panic!("a handler failure arrives measured: {error:#}"));
    let Measurement::Lifecycle {
        rows, verification, ..
    } = carried.evidence
    else {
        panic!("a handler failure measures lifecycle rows");
    };
    let member = verification.expect("the comparison made BEFORE the handler survives it");
    assert_eq!(
        member.status,
        EvidenceStatus::Matched,
        "a handler outcome never rewrites an identity: {member:?}",
    );
    assert!(
        rows.iter().any(|row| row.status == "fail"),
        "and the command really failed: {rows:?}",
    );
    // The two axes, side by side on one document.
    let report = crate::values::LifecycleValues::failed_with_verification(
        "verify",
        vec!["build".into(), "verify".into()],
        "verify",
        rows,
        Some((*member).clone()),
    )
    .into_report(None);
    assert!(!report.ok, "lifecycle ok is false");
    assert_eq!(
        report.verification.as_ref().map(|row| &row.status),
        Some(&EvidenceStatus::Matched),
        "while the identity stayed matched",
    );
    assert_eq!(
        report.verification.as_ref(),
        Some(&*member),
        "and the member arrived whole, not rebuilt",
    );
}

/// RED 5 — a GENERIC post-row failure after the boundary keeps the member.
///
/// The injection fires after the verify row, which is after the boundary, so
/// this is exactly the shape no fixture can provoke: a state write or
/// checkpoint fault whose carrier is built by `carry_once` from the
/// accumulator. An accumulator that only carried rows loses the comparison
/// here and nowhere else.
#[test]
fn a_generic_post_row_failure_after_the_boundary_keeps_the_member() {
    let fixture = project(&format!("{DECLARED_BUILD_ROW}{VERIFY_ROW}"));
    let observer = MutatingObserver::default();

    let guard = inject::fail_after(2);
    let result = dispatch(
        &observer,
        fixture.path(),
        &[Phase::Build, Phase::Verify],
        Some(observed_at()),
    );
    drop(guard);

    let carried = take(result.expect_err("the injected fault fails the dispatch"))
        .unwrap_or_else(|error| panic!("a post-row failure arrives measured: {error:#}"));
    assert!(
        !carried.emit_machine_failure,
        "a generic stage failure keeps its historical silence",
    );
    let Measurement::Lifecycle {
        rows, verification, ..
    } = carried.evidence
    else {
        panic!("a dispatch failure measures lifecycle rows");
    };
    assert_eq!(rows.len(), 2, "both rows ran before the fault");
    let member = verification.expect("the comparison survives an unrelated fault");
    assert_eq!(
        member.status,
        EvidenceStatus::Matched,
        "and its status is untouched by a failure on another axis",
    );
}

/// RED 7 — the two-epoch guard. A PARTIAL dispatch carrying a full chain that
/// names verify must publish NO member: the permission, not the chain, is what
/// says a plan is the whole chain.
///
/// Delete the `Option<Timestamp>` seam and decide from `metadata.chain`
/// instead, and this run reconciles inside the install callback — before build
/// and create have run — and publishes a member about a prefix that does not
/// exist yet.
#[test]
fn a_partial_epoch_publishes_no_member_however_full_its_chain_is() {
    let fixture = project(DECLARED_BUILD_ROW);
    let observer = MutatingObserver::default();
    let plan = world::plan_default(fixture.path(), &[Phase::Validate, Phase::Build])
        .expect("the partial plan loads");
    // The OUTER command's chain, exactly as the post-durability stage carries
    // it: every phase through verify, while the plan above is a prefix.
    let meta = metadata(
        fixture.path(),
        &[Phase::Validate, Phase::Build, Phase::Verify],
    );
    assert!(
        meta.chain.iter().any(|phase| phase == "verify"),
        "the trap this test names: the chain really does say verify",
    );

    let outcome = dispatch_plan(
        &observer,
        &plan,
        lease_for(fixture.path()),
        &agent(),
        meta,
        vec!["validate".into(), "build".into()],
        None,
    )
    .expect("the partial epoch completes");

    assert!(
        outcome.verification.is_none(),
        "a partial epoch reconciles nothing: {:?}",
        outcome.verification,
    );
}
