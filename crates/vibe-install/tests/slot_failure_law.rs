//! The failure law of an ordinary package's `slot:` contributions, proven by
//! dispatching real rows rather than by reading the classifier
//! (PROP-065 `#ordinary-failure`).
//!
//! Three facts, each one the law says something different about:
//!
//! 1. a **required** contribution's failure ends the point's sequence for
//!    that package and the message names the retry — `vibe reinstall
//!    <group>/<name>`, a rerun and not a resume;
//! 2. an **optional** one is recorded `failed` and the sequence continues,
//!    so a later row of the same point is still reached;
//! 3. a failure under a **requested stop** is recorded `interrupted` and
//!    stops regardless, because no contribution may be marked `done` by an
//!    interrupted exit and nothing after it runs.
//!
//! The failing row is an `agent` contribution against the named refusing
//! backend: it fails deterministically, on every platform, with no script
//! interpreter and no child process involved.

use std::sync::Arc;

use vibe_core::manifest::Manifest;
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_install::{InstallSlotLifecycle, SlotLifecycleReport, SlotLifecycleSeams};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_workspace::install::{ResolvedDep, SlotLifecycle, SlotLifecycleContext};

/// The row that fails: an `agent` handler the refusing backend never serves.
fn failing_row(optional: bool) -> String {
    format!(
        "\n[[extension]]\nid = \"produce\"\npoint = \"slot:post-install\"\n\
         title = \"Let an agent produce the document\"\n\
         handler = {{ kind = \"agent\", prompt = \
         \"spec://org.demo/tools/common/agent-prompt#root\" }}\n\
         config.outputs = [\n  \
         {{ path = \"docs/slot.md\", kind = \"file\", accept = \"non-empty file\" }},\n]\n\
         {}",
        if optional { "optional = true\n" } else { "" }
    )
}

/// The row AFTER it: a contribution whose condition is false under the
/// consenting judgment, so reaching it is observable as a `skip` row and
/// needs no interpreter of its own.
const LATER_ROW: &str = "\n[[extension]]\nid = \"later\"\npoint = \"slot:post-install\"\n\
     title = \"The row after the failure\"\n\
     handler = { kind = \"script\", base = \"hooks/later\" }\n\
     when = { command_present = \"no-such-command-anywhere\" }\n";

fn metadata(project_root: &std::path::Path) -> RunMetadata {
    RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(project_root).unwrap(),
        started: "2026-11-10T00:00:00Z".into(),
        selected: ".".into(),
    }
}

/// A project with one materialised dependency slot whose manifest declares
/// `rows`, plus the resolution that names it.
fn fixture(rows: &str) -> (tempfile::TempDir, Manifest, Vec<ResolvedDep>) {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.demo/tools\" = \"=1.0.0\"\n",
    )
    .unwrap();
    let host = Manifest::read(project.path().join("vibe.toml")).unwrap();

    let slot = project
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.demo.tools")
        .join("1.0.0");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(
        slot.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"tool\"\n\
             version = \"1.0.0\"\n{rows}"
        ),
    )
    .unwrap();
    let specs = slot.join("vibevm/vibespecs/common");
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("agent-prompt.md"),
        "# Prompt {#root}\n\nWrite the slot document.\n",
    )
    .unwrap();

    let manifest = Manifest::read(slot.join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Tool,
        group: Group::parse("org.demo").unwrap(),
        name: "tools".into(),
        version: "1.0.0".parse().unwrap(),
        content_dir: slot,
        source_hash: Some(ContentHash::parse("sha256:aa").unwrap()),
        embedded_sources: Vec::new(),
        manifest,
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
        configuration: None,
    };
    (project, host, vec![dep])
}

/// Dispatch `slot:post-install` for the one dependency and return what the
/// seam answered together with every row it recorded.
fn dispatch(
    rows: &str,
    cancel: bool,
) -> (
    std::result::Result<(), String>,
    Vec<SlotLifecycleReport>,
    tempfile::TempDir,
) {
    let (project, host, resolution) = fixture(rows);
    let mut lifecycle = InstallSlotLifecycle::from_resolution_observed(
        project.path(),
        &host,
        &resolution,
        metadata(project.path()),
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
        Arc::new(vibe_lifecycle::LifecycleLease::acquire(project.path()).unwrap()),
    )
    .expect("the slot lifecycle constructs");
    if cancel {
        let token = vibe_core::cancellation::CancellationToken::new();
        token.cancel();
        lifecycle = lifecycle.with_cancellation(token);
    }
    let dep = &resolution[0];
    let answer = lifecycle.post_install(SlotLifecycleContext {
        group: &dep.group,
        name: &dep.name,
        version: &dep.version,
        kind: &dep.kind,
        slot: &dep.content_dir,
        manifest: &dep.manifest,
    });
    let reports = lifecycle.take_reports().expect("the rows are readable");
    (answer, reports, project)
}

fn row<'a>(reports: &'a [SlotLifecycleReport], id: &str) -> &'a SlotLifecycleReport {
    reports
        .iter()
        .find(|report| report.key.contains(id))
        .unwrap_or_else(|| panic!("a row for `{id}` among {reports:#?}"))
}

/// A required contribution's failure stops the point and names the retry.
#[test]
fn a_required_failure_stops_the_point_and_names_the_rerun() {
    let (answer, reports, _project) = dispatch(&failing_row(false), false);

    let error = answer.expect_err("a required contribution's failure refuses");
    assert!(
        error.contains("`vibe reinstall org.demo/tools`"),
        "the failure names the exact retry command: {error}",
    );
    assert!(
        error.contains("rerun") && error.contains("--resume"),
        "and says it is a rerun, not a resume: {error}",
    );
    let failed = row(&reports, "produce");
    assert_eq!(failed.status, "fail");
    assert!(
        !failed.flagged,
        "a stopping failure leaves nothing installed to flag",
    );
    assert!(
        failed
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("`vibe reinstall org.demo/tools`"),
        "the reported row carries the same sentence: {failed:#?}",
    );
}

/// An optional contribution's failure is recorded and the sequence goes on.
#[test]
fn an_optional_failure_is_recorded_and_the_sequence_continues() {
    let rows = format!("{}{LATER_ROW}", failing_row(true));
    let (answer, reports, _project) = dispatch(&rows, false);

    assert!(
        answer.is_ok(),
        "an optional failure does not refuse the installation: {answer:?}",
    );
    let failed = row(&reports, "produce");
    assert_eq!(failed.status, "fail", "recorded, never silently skipped");
    assert!(
        failed.flagged,
        "a failure the run goes on past is flagged, never a silent success",
    );
    let later = row(&reports, "later");
    assert_eq!(
        later.status, "skip",
        "the row after the optional failure was reached: {later:#?}",
    );
}

/// A failure under a requested stop is an interrupt: recorded
/// `interrupted`, nothing after it runs, and `optional` does not excuse
/// carrying on.
#[test]
fn a_failure_under_a_requested_stop_is_recorded_interrupted_and_stops() {
    let rows = format!("{}{LATER_ROW}", failing_row(true));
    let (answer, reports, _project) = dispatch(&rows, true);

    assert!(
        answer.is_err(),
        "an interrupted pass stops, optional or not: {answer:?}",
    );
    let stopped = row(&reports, "produce");
    assert_eq!(stopped.status, "interrupted");
    assert!(!stopped.flagged, "an interrupted run flags nothing");
    assert!(
        !reports.iter().any(|report| report.key.contains("later")),
        "nothing after an interrupted contribution runs: {reports:#?}",
    );
}
