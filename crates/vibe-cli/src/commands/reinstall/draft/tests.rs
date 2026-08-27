//! Reds for the reinstall draft: the member, and what `forced`/`complete` mean.

use super::*;
use vibe_wire::behaviour::compile_trace_report::validate;
use vibe_wire::generated::shared::TraceReportStatus;

/// The SELECTED node — a member, deliberately, so a draft that reported the
/// workspace root instead would be visible in every assertion below.
fn identity(forced: bool) -> ReinstallIdentity {
    ReinstallIdentity {
        selected_project_root: PathBuf::from("/p/member"),
        forced,
    }
}

/// The progress a plain regeneration reports has always been exactly the
/// regenerated nodes and the pruned slots — never the materialise pass's own
/// slot lists.
#[test]
fn the_regenerated_shape_reports_nodes_and_prunes_and_nothing_else() {
    let progress = regenerated(
        vec![".".into(), "member".into()],
        vec!["vibedeps/x/1.0.0".into()],
    );
    assert!(progress.complete);
    assert!(!progress.fresh);
    assert!(progress.materialised.is_empty());
    assert!(progress.skipped.is_empty());
    assert_eq!(progress.nodes_regenerated, [".", "member"]);
    assert_eq!(progress.pruned, ["vibedeps/x/1.0.0"]);
}

/// A SUCCESSFUL reinstall still reports `running` / `finalised: false` when the
/// index says so. Both other command roots prove the same law; a divergence
/// would mean one of the four told a different story about the same run.
#[test]
fn a_successful_reinstall_root_carries_a_refused_member_unchanged() {
    let member = CompileTraceReport {
        budget_exhausted: false,
        events: "2".into(),
        finalised: false,
        run_id: "e".repeat(32),
        snapshot_bytes: "0".into(),
        snapshots: "0".into(),
        status: TraceReportStatus::Running,
        timings: Vec::new(),
        warnings: vec!["the terminal index could not be published".into()],
        run_path: Some(format!("/p/.vibe/trace/{}", "e".repeat(32))),
    };
    let report = ReinstallDraft::completed(
        &identity(false),
        regenerated(vec![".".into()], Vec::new()),
        Vec::new(),
        None,
    )
    .into_report(Some(member.clone()));

    assert!(report.ok, "the COMMAND succeeded");
    assert_eq!(
        report.project,
        vibe_core::machine_json_path(&PathBuf::from("/p/member")),
        "the report names the SELECTED node — a reinstall invoked inside a          member regenerates the whole tree, but the document is about the          invocation, not the tree",
    );
    let attached = report.trace.expect("and the member was not dropped");
    assert_eq!(attached, member, "carried through byte for byte");
    assert_eq!(attached.status, TraceReportStatus::Running);
    assert!(!attached.finalised);
    validate(&attached).expect("and it is still a valid member");
}

#[test]
fn a_disabled_reinstall_root_omits_the_member_and_has_no_notices() {
    let report = ReinstallDraft::completed(
        &identity(false),
        regenerated(vec![".".into()], Vec::new()),
        Vec::new(),
        None,
    )
    .into_report(None);
    assert!(report.trace.is_none());
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("trace"), "absent from the wire: {json}");
    assert!(
        !json.contains("notices"),
        "and this root has never had a notices member: {json}",
    );
}

/// `forced` is the MATERIALISATION force. A parked forced reinstall is
/// `ok: true, complete: false`, and its handoff names the base verb that can
/// actually service the continuation.
#[test]
fn a_parked_forced_reinstall_is_an_incomplete_success() {
    let delegation = vibe_lifecycle::Delegation {
        resume: "vibe reinstall".into(),
        run_id: "f".repeat(32),
        tasks: vec![".vibe/outbox/x/a.md".into()],
    };
    let report = ReinstallDraft::completed(
        &identity(true),
        InstallProgress {
            complete: true,
            fresh: false,
            materialised: vec!["vibedeps/org.demo.tools/0.1.0".into()],
            skipped: Vec::new(),
            pruned: Vec::new(),
            nodes_regenerated: Vec::new(),
        },
        Vec::new(),
        Some(&delegation),
    )
    .into_report(None);
    assert!(report.ok);
    assert!(report.forced);
    assert!(!report.complete);
    assert_eq!(
        report.project,
        vibe_core::machine_json_path(&PathBuf::from("/p/member")),
        "a parked forced reinstall names its selected node too",
    );
    assert_eq!(
        report.delegation.expect("the handoff").resume,
        "vibe reinstall"
    );
}

/// A failed forced reinstall reports what it really made durable, and is never
/// complete.
#[test]
fn a_failed_reinstall_keeps_its_measured_rows_and_is_never_complete() {
    let rows = vec![
        row("slot:pre-install", "ok"),
        row("slot:post-install", "fail"),
    ];
    let report = ReinstallDraft::failed(
        &identity(true),
        InstallProgress {
            complete: true,
            fresh: false,
            materialised: vec!["vibedeps/org.demo.tools/0.1.0".into()],
            skipped: Vec::new(),
            pruned: Vec::new(),
            nodes_regenerated: vec![".".into()],
        },
        rows,
    )
    .into_report(None);

    assert!(!report.ok);
    assert!(!report.complete);
    assert_eq!(report.materialised, ["vibedeps/org.demo.tools/0.1.0"]);
    assert_eq!(report.nodes_regenerated, ["."]);
    let statuses: Vec<&str> = report
        .contributions
        .iter()
        .map(|row| row.status.as_str())
        .collect();
    assert_eq!(
        statuses,
        ["ok", "fail"],
        "the earlier successful row precedes the later failed one",
    );
    assert!(report.delegation.is_none(), "a failure never parked");
}

fn row(point: &str, status: &str) -> SlotLifecycleReport {
    SlotLifecycleReport {
        key: format!("org.demo/tools#{point}"),
        point: point.into(),
        handler: "builtin".into(),
        provider: "org.demo/tools".into(),
        tier: "dependency".into(),
        status: status.into(),
        message: None,
        version: None,
        reference: "spec://org.demo/tools".into(),
        flagged: false,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
        slot_target: None,
    }
}
