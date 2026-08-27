//! Reds for the update draft: the presentation policy, and the member.

use super::*;
use vibe_wire::behaviour::compile_trace_report::validate;
use vibe_wire::generated::shared::TraceReportStatus;

fn identity(scope: UpdateReportScope) -> UpdateIdentity {
    let packages = match scope {
        UpdateReportScope::All => Vec::new(),
        UpdateReportScope::Scoped => vec!["org.demo/tools".into()],
    };
    UpdateIdentity {
        project_root: PathBuf::from("/p"),
        scope,
        packages,
    }
}

fn presentation<'a>(scoped: bool, quiet: bool, quiet_suffix: &'a str) -> Presentation<'a> {
    Presentation {
        scoped,
        quiet,
        count: 2,
        bumps: 1,
        hooks: "",
        quiet_suffix,
    }
}

const TRACE_SUFFIX: &str = ", compile trace ok (7 event(s), 3 snapshot(s))";

/// Trace OFF, whole graph: silence in BOTH human and quiet, exactly as
/// `vibe update --all` has always behaved.
///
/// The failure this refuses is a real one and entirely silent: applying the
/// scoped summary to every success adds a completion line to a command that
/// never printed one, so a script grepping this command's stdout starts seeing
/// output it never saw.
#[test]
fn a_whole_update_with_tracing_off_prints_nothing_in_either_terminal_mode() {
    assert_eq!(success_line(presentation(false, false, "")), None);
    assert_eq!(success_line(presentation(false, true, "")), None);
}

/// Trace OFF, scoped: both existing lines, byte for byte.
#[test]
fn a_scoped_update_with_tracing_off_keeps_its_exact_lines() {
    assert_eq!(
        success_line(presentation(true, false, "")).as_deref(),
        Some("\nUpdated 2 packages (1 version bump)."),
    );
    assert_eq!(
        success_line(presentation(true, true, "")).as_deref(),
        Some("vibe update: 2 packages re-resolved, 1 bump"),
    );
}

/// Trace ON, whole graph, quiet: EXACTLY one line, and only because the
/// suffix needs somewhere to live.
#[test]
fn a_traced_whole_update_gets_one_quiet_line_and_still_no_human_summary() {
    assert_eq!(
        success_line(presentation(false, true, TRACE_SUFFIX)).as_deref(),
        Some("vibe update: 2 packages re-resolved, compile trace ok (7 event(s), 3 snapshot(s))"),
    );
    assert_eq!(
        success_line(presentation(false, false, TRACE_SUFFIX)),
        None,
        "human mode reads the adapter's table, not a second account of it",
    );
}

/// The suffix rides the scoped quiet line too, after the hook suffix.
#[test]
fn a_traced_scoped_update_appends_the_suffix_to_its_one_line() {
    let mut inputs = presentation(true, true, TRACE_SUFFIX);
    inputs.hooks = ", 1 lifecycle hook flagged";
    assert_eq!(
        success_line(inputs).as_deref(),
        Some(
            "vibe update: 2 packages re-resolved, 1 bump, 1 lifecycle hook flagged, \
             compile trace ok (7 event(s), 3 snapshot(s))"
        ),
    );
}

/// Singulars, so a one-package run does not read as a bug.
#[test]
fn the_counts_agree_with_their_nouns() {
    let mut inputs = presentation(true, false, "");
    inputs.count = 1;
    inputs.bumps = 1;
    assert_eq!(
        success_line(inputs).as_deref(),
        Some("\nUpdated 1 package (1 version bump)."),
    );
}

fn refused_member() -> CompileTraceReport {
    CompileTraceReport {
        budget_exhausted: false,
        events: "4".into(),
        finalised: false,
        run_id: "c".repeat(32),
        snapshot_bytes: "128".into(),
        snapshots: "1".into(),
        status: TraceReportStatus::Running,
        timings: Vec::new(),
        warnings: vec!["the terminal index could not be published".into()],
        run_path: Some(format!("/p/.vibe/trace/{}", "c".repeat(32))),
    }
}

/// A SUCCESSFUL update still reports `running` / `finalised: false` when that
/// is what the index says. Rewriting either from the command's own success is
/// the one lie a cold reader cannot detect.
#[test]
fn a_successful_update_root_carries_a_refused_member_unchanged() {
    let member = refused_member();
    let report = UpdateDraft::completed(
        &identity(UpdateReportScope::Scoped),
        2,
        vec!["org.demo/tools 0.1.0 -> 0.2.0".into()],
        InstallProgress::fresh(vec![".".into()]),
        Vec::new(),
        None,
    )
    .into_report(Some(member.clone()));

    assert!(report.ok, "the COMMAND succeeded");
    let attached = report.trace.expect("and the member was not dropped");
    assert_eq!(attached, member, "carried through byte for byte");
    assert_eq!(attached.status, TraceReportStatus::Running);
    assert!(!attached.finalised);
    validate(&attached).expect("and it is still a valid member");
}

/// Disabled omits the member entirely — the byte-for-byte law old corpora
/// depend on.
#[test]
fn a_disabled_update_root_omits_the_member() {
    let report = UpdateDraft::completed(
        &identity(UpdateReportScope::All),
        0,
        Vec::new(),
        InstallProgress::fresh(vec![".".into()]),
        Vec::new(),
        None,
    )
    .into_report(None);
    assert!(report.trace.is_none());
    let json = serde_json::to_string(&report).unwrap();
    assert!(
        !json.contains("trace"),
        "the key is absent from the wire, not merely null: {json}",
    );
    assert!(
        !json.contains("notices"),
        "and this root has never had a notices member: {json}",
    );
}

/// A parked update is `complete: false` with `ok: true`: the materialisation
/// finished and the command did not.
#[test]
fn a_park_is_an_incomplete_success() {
    let delegation = vibe_lifecycle::Delegation {
        resume: "vibe update".into(),
        run_id: "d".repeat(32),
        tasks: vec![".vibe/outbox/x/a.md".into()],
    };
    let report = UpdateDraft::completed(
        &identity(UpdateReportScope::Scoped),
        1,
        Vec::new(),
        InstallProgress {
            complete: true,
            fresh: false,
            materialised: vec!["vibedeps/org.demo.tools/0.2.0".into()],
            skipped: Vec::new(),
            pruned: Vec::new(),
            nodes_regenerated: Vec::new(),
        },
        Vec::new(),
        Some(&delegation),
    )
    .into_report(None);
    assert!(report.ok);
    assert!(!report.complete);
    assert_eq!(
        report.delegation.expect("the handoff member").resume,
        "vibe update",
        "and it resumes with THIS command, never install",
    );
}

/// A failed draft is never complete, whatever the accumulator measured.
#[test]
fn a_failure_is_never_complete_even_with_a_finished_materialisation() {
    let report = UpdateDraft::failed(
        &identity(UpdateReportScope::Scoped),
        1,
        vec!["org.demo/tools 0.1.0 -> 0.2.0".into()],
        InstallProgress {
            complete: true,
            fresh: false,
            materialised: vec!["vibedeps/org.demo.tools/0.2.0".into()],
            skipped: Vec::new(),
            pruned: Vec::new(),
            nodes_regenerated: vec![".".into()],
        },
        Vec::new(),
    )
    .into_report(None);
    assert!(!report.ok);
    assert!(!report.complete);
    assert_eq!(
        report.materialised,
        ["vibedeps/org.demo.tools/0.2.0"],
        "while the durable facts are still reported",
    );
}
