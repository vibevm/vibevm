//! Freeze the Ready-resume row merge.
//!
//! A serviced continuation arrives with rows of its own, and the apply that
//! serviced it has rows of its own too. The merge is one line of ordering and
//! two carriers, which is exactly the shape that goes wrong quietly: reverse
//! it and the document tells the operator the older park happened first; skip
//! one carrier and the report and the callback describe different runs.
//!
//! So the two carriers here start with DIFFERENT resumed rows. That is what
//! makes "updated only one" and "copied the wrong vector into both" separately
//! visible — with identical rows on both sides, either mistake would still
//! produce a plausible-looking answer.
//!
//! This drives the same [`prefix_applied_rows`] the one production completion
//! site calls, so the two cannot drift apart.

use vibe_lifecycle::RunMetadata;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use super::*;

/// A slot row identified only by its key — everything else is fixed, so an
/// assertion failure prints the ordering rather than a wall of fields.
fn row(key: &str) -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: key.to_string(),
        reference: format!("org.demo/tools#{key}"),
        slot_target: None,
        point: "slot:post-install".to_string(),
        provider: "org.demo/tools".to_string(),
        handler: "builtin".to_string(),
        tier: "dependency".to_string(),
        version: Some("0.1.0".to_string()),
        status: "ok".to_string(),
        flagged: false,
        message: None,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

fn keys(rows: &[vibe_install::SlotLifecycleReport]) -> Vec<&str> {
    rows.iter().map(|row| row.key.as_str()).collect()
}

/// A serviced continuation carrying one row in each carrier, deliberately
/// different from each other.
fn resumed() -> resume::ResumedInstall {
    let mut run = InstallRun::new(
        std::path::PathBuf::from("/demo"),
        InstallDisposition::Applied,
    );
    run.slot_reports = vec![row("resumed-in-run")];
    resume::ResumedInstall {
        run,
        context: InstallRunContext {
            metadata: RunMetadata {
                requested: "install".to_string(),
                chain: vec!["validate".to_string(), "install".to_string()],
                offline: true,
                assume_yes: true,
                agent_mode: RunAgentMode::Cli,
                force: false,
                trace_compile: false,
                run_id: "fixed-run-id".to_string(),
                started: "2026-08-27T00:00:00Z".to_string(),
            },
            lifecycle_run: None,
            lifecycle_reports: vec![row("resumed-in-context")],
        },
    }
}

/// The applied rows go IN FRONT, in both carriers, and each carrier keeps its
/// own tail.
#[test]
fn the_applied_rows_precede_the_resumed_ones_in_both_carriers() {
    let mut resumed = resumed();
    prefix_applied_rows(&mut resumed, &[row("applied")]);

    assert_eq!(
        keys(&resumed.run.slot_reports),
        ["applied", "resumed-in-run"],
        "the document's carrier: this apply's work, then the park it finished",
    );
    assert_eq!(
        keys(&resumed.context.lifecycle_reports),
        ["applied", "resumed-in-context"],
        "and the callback's carrier, merged the same way from its OWN tail",
    );
}

/// Several applied rows keep their relative order, and nothing is duplicated.
///
/// The prefix is a whole vector in the order the apply produced it, not a
/// single row and not a set.
#[test]
fn a_multi_row_prefix_keeps_its_order_and_duplicates_nothing() {
    let mut resumed = resumed();
    prefix_applied_rows(&mut resumed, &[row("applied-first"), row("applied-second")]);

    assert_eq!(
        keys(&resumed.run.slot_reports),
        ["applied-first", "applied-second", "resumed-in-run"],
    );
    assert_eq!(
        keys(&resumed.context.lifecycle_reports),
        ["applied-first", "applied-second", "resumed-in-context"],
    );
}

/// An empty prefix leaves both carriers exactly as they were.
///
/// The early return is not an optimisation — a Ready apply whose own slot
/// lifecycle produced nothing must not gain a row, and must not have its
/// resumed rows reordered or cloned on the way past.
#[test]
fn an_empty_prefix_changes_neither_carrier() {
    let before = resumed();
    let mut resumed = resumed();
    prefix_applied_rows(&mut resumed, &[]);

    assert_eq!(resumed.run.slot_reports, before.run.slot_reports);
    assert_eq!(
        resumed.context.lifecycle_reports,
        before.context.lifecycle_reports,
    );
}

/// The two carriers are never conflated.
///
/// Merging one carrier and copying the result into the other would satisfy a
/// test written with identical rows on both sides. Here it cannot: each
/// carrier's tail is its own, and this says so directly.
#[test]
fn neither_carrier_receives_the_others_rows() {
    let mut resumed = resumed();
    prefix_applied_rows(&mut resumed, &[row("applied")]);

    assert!(
        !keys(&resumed.run.slot_reports).contains(&"resumed-in-context"),
        "the document's carrier never picks up the callback's tail: {:?}",
        keys(&resumed.run.slot_reports),
    );
    assert!(
        !keys(&resumed.context.lifecycle_reports).contains(&"resumed-in-run"),
        "nor the other way round: {:?}",
        keys(&resumed.context.lifecycle_reports),
    );
}
