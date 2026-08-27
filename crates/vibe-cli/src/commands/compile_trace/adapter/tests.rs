//! Reds for the post-finalize adapter.
//!
//! Every notice test drives the PRODUCTION executor with a capturing sink, so
//! deleting either the `absorb_into` or the `emit` call inside it is red. A
//! test that called `NoticePlan` by hand would stay green through both.

use vibe_wire::generated::shared::TraceReportStatus;

use super::*;
use crate::commands::install::InstallDraft;

#[derive(Debug, thiserror::Error)]
#[error("the slot refused")]
struct Sentinel;

fn draft() -> RegisteredReportDraft {
    RegisteredReportDraft::Install(Box::new(InstallDraft::failed(
        std::path::Path::new("/p"),
        vibe_install::InstallProgress::default(),
        Vec::new(),
    )))
}

fn member() -> CompileTraceReport {
    CompileTraceReport {
        budget_exhausted: false,
        events: "3".into(),
        finalised: true,
        run_id: "0".repeat(32),
        snapshot_bytes: "0".into(),
        snapshots: "0".into(),
        status: TraceReportStatus::Failed,
        timings: Vec::new(),
        warnings: Vec::new(),
        run_path: Some("/p/.vibe/trace/run".into()),
    }
}

fn quiet_ctx() -> output::Context {
    output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Cli)
}

fn failed(
    trace: Option<CompileTraceReport>,
    notices: Vec<String>,
) -> FinalizedCommand<RegisteredReportDraft> {
    FinalizedCommand {
        report: draft(),
        plan: PlanDisposition::Flush,
        trace_requested: trace.is_some(),
        trace,
        original_error: Some(anyhow::Error::new(Sentinel).context("installing")),
        emit_report: true,
        notices,
    }
}

/// The failure path's contract: the SAME error object comes back, and the
/// quiet suffix rides it rather than a second line.
#[test]
fn a_failed_quiet_command_returns_its_original_error_with_the_suffix() {
    let error =
        render_finalized(&quiet_ctx(), failed(Some(member()), Vec::new())).expect_err("failed");
    let (original, suffix) = quiet::detach(error);
    assert_eq!(
        suffix.as_deref(),
        Some(", compile trace failed (3 event(s), 0 snapshot(s))")
    );
    assert!(original.downcast_ref::<Sentinel>().is_some());
    assert_eq!(format!("{original:#}"), "installing: the slot refused");
}

/// Quiet's one-line law survives notices that have no member to ride: they
/// become a COUNT in the suffix rather than a diagnostic line.
#[test]
fn quiet_folds_memberless_notices_into_the_one_suffix() {
    let error = render_finalized(
        &quiet_ctx(),
        failed(None, vec!["the displaced run is still running".into()]),
    )
    .expect_err("failed");
    let (original, suffix) = quiet::detach(error);
    assert_eq!(suffix.as_deref(), Some(", 1 trace notice(s)"));
    assert!(original.downcast_ref::<Sentinel>().is_some());
}

/// Trace off, nothing to say: no suffix, no wrapper, byte-identical line.
#[test]
fn a_disabled_failure_returns_an_unwrapped_error() {
    let mut finalized = failed(None, Vec::new());
    finalized.emit_report = false;
    let error = render_finalized(&quiet_ctx(), finalized).expect_err("failed");
    let (original, suffix) = quiet::detach(error);
    assert!(suffix.is_none());
    assert!(original.downcast_ref::<Sentinel>().is_some());
}

fn json_ctx() -> output::Context {
    output::Context::from_flags(false, true, None, true, crate::cli::AgentModeArg::Cli)
}

/// A sink that remembers what was really written — both channels.
#[derive(Default)]
struct Captured {
    diagnostics: Vec<String>,
    /// The `notices` member of the install root the executor actually
    /// rendered.
    root_notices: Option<Vec<String>>,
    /// The serialized root, for families whose capability is the point.
    root: Option<String>,
}

impl DiagnosticSink for Captured {
    fn diagnostic(&mut self, text: &str) {
        self.diagnostics.push(text.to_string());
    }

    fn render_root(
        &mut self,
        _ctx: &output::Context,
        report: RegisteredReportDraft,
        trace: Option<CompileTraceReport>,
        _quiet_suffix: &str,
    ) -> Result<()> {
        match report {
            RegisteredReportDraft::Install(draft) => {
                let built = draft.into_report(trace);
                self.root = Some(serde_json::to_string(&built).unwrap());
                self.root_notices = Some(built.notices);
            }
            RegisteredReportDraft::Update(draft) => {
                self.root = Some(serde_json::to_string(&draft.into_report(trace)).unwrap());
            }
            RegisteredReportDraft::Reinstall(draft) => {
                self.root = Some(serde_json::to_string(&draft.into_report(trace)).unwrap());
            }
            RegisteredReportDraft::Lifecycle(draft) => {
                self.root = Some(serde_json::to_string(&draft.into_report(trace)).unwrap());
            }
        }
        Ok(())
    }
}

const NOTICE: &str = "the displaced run could not be superseded";

/// A JSON command whose failure is historically SILENT emits no root — so
/// there is no `notices` member to fold into, and the notice must not
/// simply cease to exist.
///
/// Driven through the SAME plan/execute seam production uses, with a
/// captured sink: deleting the emission loop makes this fail, which a
/// routing-tuple assertion could not.
#[test]
fn json_notices_survive_a_failure_that_emits_no_root() {
    let plan = plan_notices(&None, false, true, false, vec![NOTICE.to_string()]);
    let mut report = draft();
    assert_eq!(
        plan.absorb_into(&mut report).folded,
        0,
        "quiet folds nothing here"
    );

    let mut sink = Captured::default();
    plan.emit(&mut sink);
    assert_eq!(
        sink.diagnostics,
        vec![NOTICE.to_string()],
        "with no root to carry it, the notice is really WRITTEN",
    );

    // And the root that was never emitted did not silently swallow it.
    let RegisteredReportDraft::Install(built) = report else {
        panic!("family");
    };
    assert!(built.notices.is_empty());
}

/// The same notice with a root to ride goes into the document, once, and
/// is not also printed.
///
/// Deleting the `absorb_notices` call makes the first assertion fail.
#[test]
fn json_notices_ride_the_root_when_one_is_emitted() {
    let plan = plan_notices(&None, false, true, true, vec![NOTICE.to_string()]);
    let mut report = draft();
    let residue = plan.absorb_into(&mut report);
    assert_eq!(residue.folded, 0);
    assert!(
        residue.unabsorbed.is_empty(),
        "this root declares a `notices` member",
    );

    let RegisteredReportDraft::Install(built) = report else {
        panic!("family");
    };
    assert_eq!(
        built.notices,
        vec![NOTICE.to_string()],
        "the emitted root really CARRIES it",
    );

    let mut sink = Captured::default();
    plan.emit(&mut sink);
    assert!(
        sink.diagnostics.is_empty(),
        "and it is not printed a second time",
    );
}

/// A member's own warnings are the only copy: nothing is folded, nothing
/// is printed, and quiet counts nothing.
#[test]
fn a_member_deduplicates_every_other_channel() {
    let plan = plan_notices(&Some(member()), false, true, true, vec![NOTICE.to_string()]);
    let mut report = draft();
    assert_eq!(plan.absorb_into(&mut report).folded, 0);
    let mut sink = Captured::default();
    plan.emit(&mut sink);
    assert!(sink.diagnostics.is_empty(), "the member already carries it");
    let RegisteredReportDraft::Install(built) = report else {
        panic!("family");
    };
    assert!(built.notices.is_empty());
}

/// Quiet gets a COUNT and no line of its own — its one-line law.
#[test]
fn quiet_counts_notices_instead_of_printing_them() {
    let plan = plan_notices(
        &None,
        true,
        false,
        true,
        vec![NOTICE.to_string(), "and another".to_string()],
    );
    let mut report = draft();
    assert_eq!(
        plan.absorb_into(&mut report).folded,
        2,
        "quiet folds a count"
    );
    let mut sink = Captured::default();
    plan.emit(&mut sink);
    assert!(sink.diagnostics.is_empty(), "and never a second line");
}

/// A JSON root that has NO `notices` member still shows the notice — once,
/// on stderr — and never grows an invented field.
///
/// Driven through the production executor with a captured sink, so deleting
/// the leftover-routing loop is red. `cli-update-report` is emitted here (it
/// is a successful command), which is exactly the case a "fold it into the
/// root" implementation silently loses.
#[test]
fn the_executor_routes_a_notice_no_update_root_can_carry() {
    let finalized = FinalizedCommand {
        report: update_draft(),
        plan: PlanDisposition::Flush,
        trace: None,
        original_error: None,
        emit_report: true,
        trace_requested: false,
        notices: vec![NOTICE.to_string()],
    };
    let mut sink = Captured::default();
    render_finalized_with_sink(&json_ctx(), finalized, &mut sink).expect("the command succeeded");

    assert_eq!(
        sink.diagnostics,
        vec![NOTICE.to_string()],
        "the notice reached the one channel this root leaves for it",
    );
    let rendered = sink.root.expect("the update root was rendered");
    assert!(
        !rendered.contains("notices"),
        "and the registered format grew no member: {rendered}",
    );
    assert!(
        !rendered.contains("\"trace\""),
        "nor a trace field it never had: {rendered}",
    );
}

fn update_draft() -> RegisteredReportDraft {
    RegisteredReportDraft::Update(Box::new(crate::commands::update::UpdateDraft::completed(
        &crate::commands::update::UpdateIdentity {
            project_root: std::path::PathBuf::from("/p"),
            scope: vibe_wire::generated::update_report::UpdateReportScope::All,
            packages: Vec::new(),
        },
        0,
        Vec::new(),
        vibe_install::InstallProgress::fresh(vec![".".into()]),
        Vec::new(),
        None,
    )))
}

/// The PRODUCTION executor, driven with a capturing sink.
///
/// This is the test that kills the deletion: `render_finalized_with_sink`
/// is the same function `render_finalized` delegates to, so removing the
/// `notice_plan.emit(sink)` call from it makes the captured Vec empty.
#[test]
fn the_executor_really_emits_a_notice_no_root_can_carry() {
    let finalized = FinalizedCommand {
        report: draft(),
        plan: PlanDisposition::Flush,
        trace: None,
        original_error: Some(anyhow::Error::new(Sentinel)),
        emit_report: false,
        trace_requested: false,
        notices: vec![NOTICE.to_string()],
    };
    let mut sink = Captured::default();
    let error = render_finalized_with_sink(&json_ctx(), finalized, &mut sink)
        .expect_err("the command failed");

    assert_eq!(
        sink.diagnostics,
        vec![NOTICE.to_string()],
        "the silent-JSON path WROTE the notice through the sink",
    );
    assert!(
        error.downcast_ref::<Sentinel>().is_some(),
        "and the original error identity is untouched",
    );
}

/// The same executor with a root to carry it: the notice is folded into
/// the built root exactly once, and nothing is printed.
///
/// Removing the production `report.absorb_notices(...)` call empties the
/// root; removing the `emit` call cannot hide here either, because the
/// sink is asserted empty for the opposite reason.
#[test]
fn the_executor_folds_a_notice_into_the_root_it_emits() {
    let finalized = FinalizedCommand {
        report: draft(),
        plan: PlanDisposition::Flush,
        trace: None,
        original_error: None,
        emit_report: true,
        trace_requested: false,
        notices: vec![NOTICE.to_string()],
    };
    let mut sink = Captured::default();
    render_finalized_with_sink(&json_ctx(), finalized, &mut sink).expect("the command succeeded");

    assert_eq!(
        sink.root_notices.as_deref(),
        Some(&[NOTICE.to_string()][..]),
        "the root the EXECUTOR rendered really carries it, once",
    );
    assert!(
        sink.diagnostics.is_empty(),
        "and it is not also printed: {:?}",
        sink.diagnostics,
    );
}

/// The end-to-end shape still returns the original error.
#[test]
fn a_silent_json_failure_still_returns_its_original_error() {
    let finalized = FinalizedCommand {
        report: draft(),
        plan: PlanDisposition::Flush,
        trace: None,
        original_error: Some(anyhow::Error::new(Sentinel)),
        emit_report: false,
        trace_requested: false,
        notices: vec![NOTICE.to_string()],
    };
    let error = render_finalized(&json_ctx(), finalized).expect_err("failed");
    assert!(error.downcast_ref::<Sentinel>().is_some());
}
