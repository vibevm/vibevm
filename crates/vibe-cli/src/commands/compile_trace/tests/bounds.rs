//! What the member is allowed to carry: bounded whole messages, and never the
//! command's own error text.

use std::path::PathBuf;

use vibe_wire::behaviour::compile_trace_report::validate;
use vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;
use vibe_wire::generated::shared::TraceReportStatus;
use vibe_workspace::compile_trace::{TraceSummary, TraceWarning};

use super::super::{BoundedDiagnostic, CommandExit, finalize, prepare, report, supersede_notices};
use super::support::{RUN_A, Ticks, all_trace_bytes, identity, project, read_index};

/// A `TraceWarning` whose FIELDS are each already at the cap. Its `Display`
/// then adds a prefix and a second field, so the finished message is far over
/// the ceiling the epoch's validator enforces — which is precisely the case a
/// per-field clamp gets wrong.
fn over_cap_warning() -> TraceWarning {
    TraceWarning::Residue {
        path: "p".repeat(DIAGNOSTIC_CAP_BYTES),
        reason: "r".repeat(DIAGNOSTIC_CAP_BYTES),
    }
}

/// The whole message — warning Display and startup notice alike — is clamped
/// by the writer's own formatter, and the member validates.
#[test]
fn an_over_cap_warning_and_notice_are_whole_message_bounded() {
    let raw = over_cap_warning().to_string();
    assert!(
        raw.len() > DIAGNOSTIC_CAP_BYTES,
        "the premise: {} bytes of Display",
        raw.len(),
    );
    let summary = TraceSummary {
        run_dir: PathBuf::from(format!("/tmp/demo/.vibe/trace/{RUN_A}")),
        status: RunStatus::Ok,
        events: 3,
        snapshots: 2,
        snapshot_bytes: 4_096,
        budget_exhausted: false,
        finalised: true,
        aggregates: Vec::new(),
        warnings: vec![over_cap_warning()],
    };
    // A startup notice can only exist by going through the ONE constructor, so
    // this is also the clamp under test: strip it and the over-cap text reaches
    // the member.
    let notices = vec![BoundedDiagnostic::new(format_args!(
        "{}",
        "n".repeat(DIAGNOSTIC_CAP_BYTES * 2)
    ))];

    let (member, _) = report::from_summary(RUN_A, &summary, notices);

    let member = member.expect("a bounded member is a valid member");
    validate(&member).expect("every relational law, warning cap included");
    assert_eq!(member.warnings.len(), 2, "the warning and the notice");
    for warning in &member.warnings {
        assert!(
            warning.len() <= DIAGNOSTIC_CAP_BYTES,
            "{} > {DIAGNOSTIC_CAP_BYTES}",
            warning.len(),
        );
    }
    assert_eq!(member.status, TraceReportStatus::Ok);
    assert_eq!(member.events, "3");
    assert_eq!(member.snapshot_bytes, "4096");
}

/// The same clamp on the `unavailable` arm, whose reasons are the only thing
/// making the member legal at all.
#[test]
fn an_over_cap_unavailable_reason_is_bounded_and_still_nonblank() {
    let reasons = vec![BoundedDiagnostic::new(format_args!(
        "why{}",
        "!".repeat(DIAGNOSTIC_CAP_BYTES * 2)
    ))];

    let (member, notices) = report::unavailable(RUN_A, &reasons, Vec::new());

    let member = member.expect("a bounded unavailable member is valid");
    validate(&member).expect("including the nonblank-reason law");
    assert!(notices.is_empty());
    assert_eq!(member.warnings.len(), 1);
    assert!(member.warnings[0].len() <= DIAGNOSTIC_CAP_BYTES);
    assert!(member.warnings[0].starts_with("why"));
    assert!(member.run_path.is_none());
}

/// A terminal publication that never reached the disk is reported `running`
/// with `finalised = false` — never a lying `ok`.
#[test]
fn an_unpublished_terminal_status_is_reported_as_still_running() {
    let summary = TraceSummary {
        run_dir: PathBuf::from(format!("/tmp/demo/.vibe/trace/{RUN_A}")),
        // Exactly what the writer leaves behind when the terminal index is
        // rolled back: the in-memory root restored to running.
        status: RunStatus::Running,
        events: 1,
        snapshots: 0,
        snapshot_bytes: 0,
        budget_exhausted: false,
        finalised: false,
        aggregates: Vec::new(),
        warnings: vec![TraceWarning::NotFinalised {
            reason: "the terminal bytes never landed".to_string(),
        }],
    };

    let (member, _) = report::from_summary(RUN_A, &summary, Vec::new());

    let member = member.expect("a member");
    validate(&member).expect("running-is-not-finalised");
    assert_eq!(member.status, TraceReportStatus::Running);
    assert!(!member.finalised);

    // The dangerous shape, stated directly: a terminal word the writer did
    // NOT prove durable. Reading the status alone would report `ok` about an
    // index that still says `running` on the disk a cold reader will open.
    // Durability is the gate; the status is only what it gates.
    for claimed in [RunStatus::Ok, RunStatus::Failed] {
        let undurable = TraceSummary {
            status: claimed.clone(),
            finalised: false,
            warnings: Vec::new(),
            ..summary_at(RUN_A)
        };
        let (member, _) = report::from_summary(RUN_A, &undurable, Vec::new());
        let member = member.expect("a member");
        validate(&member).expect("no member ever lies about being finalised");
        assert_eq!(
            member.status,
            TraceReportStatus::Running,
            "`{claimed:?}` was never published, so the report says `running`",
        );
        assert!(!member.finalised);
    }
}

/// `running + finalised` is impossible, so the conversion must NOT quietly
/// repair it. Carried through, the shared validator refuses it and the member
/// is omitted with a `status-matrix` notice — a defect that is visible instead
/// of one that reads as a normal parked run forever.
#[test]
fn an_impossible_running_and_finalised_pair_is_refused_not_masked() {
    let broken = TraceSummary {
        status: RunStatus::Running,
        finalised: true,
        ..summary_at(RUN_A)
    };

    let (member, notices) = report::from_summary(RUN_A, &broken, Vec::new());

    assert!(
        member.is_none(),
        "an impossible member is omitted, never normalised into a plausible one",
    );
    assert_eq!(notices.len(), 1);
    assert!(
        notices[0].as_str().contains("status-matrix"),
        "and the notice names the law it broke: {notices:?}",
    );
}

/// Closing a displaced predecessor keeps EVERY word its own writer said.
///
/// Both shapes matter and they fail in opposite directions. A run that WAS
/// finalised can still carry an `IndexAnomaly` — the terminal bytes landed
/// despite a post-publication fault — and a `finalised`-only check would throw
/// that away entirely. A run that was not finalised carries the exact
/// `NotFinalised` reason, and replacing it with the generic sentence would
/// lose the only text that says why.
#[test]
fn superseding_keeps_every_writer_warning_and_only_falls_back_once() {
    // Finalised, but the publication reported a fault after the point of no
    // return. There is nothing generic to add — the run IS closed.
    let anomalous = TraceSummary {
        finalised: true,
        status: RunStatus::Failed,
        warnings: vec![TraceWarning::IndexAnomaly {
            reason: "the replace reported a fault after its irreversible step".to_string(),
        }],
        ..summary_at(RUN_A)
    };
    let notices = supersede_notices(RUN_A, &anomalous);
    assert_eq!(notices.len(), 1, "kept, and nothing invented: {notices:?}");
    assert!(notices[0].as_str().contains("irreversible step"));
    assert!(
        notices[0].as_str().contains(RUN_A),
        "and it names which run"
    );

    // Not finalised, and the writer said exactly why. The precise reason is
    // the notice; the generic fallback would only duplicate it worse.
    let refused = TraceSummary {
        finalised: false,
        warnings: vec![TraceWarning::NotFinalised {
            reason: "the disk is full".to_string(),
        }],
        ..summary_at(RUN_A)
    };
    let notices = supersede_notices(RUN_A, &refused);
    assert_eq!(notices.len(), 1, "no duplicate generic line: {notices:?}");
    assert!(notices[0].as_str().contains("the disk is full"));

    // Not finalised and NOTHING explains it: now the generic line is the only
    // thing an operator would otherwise get.
    let silent = TraceSummary {
        finalised: false,
        warnings: Vec::new(),
        ..summary_at(RUN_A)
    };
    let notices = supersede_notices(RUN_A, &silent);
    assert_eq!(notices.len(), 1);
    assert!(notices[0].as_str().contains("still reads `running`"));

    // A finalised, silent close says nothing at all.
    let clean = TraceSummary {
        finalised: true,
        status: RunStatus::Failed,
        warnings: Vec::new(),
        ..summary_at(RUN_A)
    };
    assert!(supersede_notices(RUN_A, &clean).is_empty());

    // And the prefix plus a hostile field is still whole-message bounded.
    let hostile = TraceSummary {
        finalised: true,
        status: RunStatus::Failed,
        warnings: vec![over_cap_warning()],
        ..summary_at(RUN_A)
    };
    for notice in supersede_notices(RUN_A, &hostile) {
        let text = notice.as_str();
        assert!(text.len() <= DIAGNOSTIC_CAP_BYTES, "{}", text.len());
    }
}

/// A summary skeleton for the shapes a live writer would have to be broken to
/// produce — the ones the conversion must nevertheless survive honestly.
fn summary_at(run_id: &str) -> TraceSummary {
    TraceSummary {
        run_dir: PathBuf::from(format!("/tmp/demo/.vibe/trace/{run_id}")),
        status: RunStatus::Running,
        events: 0,
        snapshots: 0,
        snapshot_bytes: 0,
        budget_exhausted: false,
        finalised: false,
        aggregates: Vec::new(),
        warnings: Vec::new(),
    }
}

/// The security law: a command's error may carry captured stderr, a provider
/// body or a secret. It is BORROWED for the close and returned unchanged —
/// and nothing it says reaches the index, the member or the notices.
#[test]
fn a_failed_close_never_persists_the_command_error() {
    const SENTINEL: &str = "sk-live-2f9c-DO-NOT-PERSIST-9e41";
    let root = project();
    let original = anyhow::anyhow!("provider rejected the token {SENTINEL}")
        .context(format!("captured stderr: {}", SENTINEL.repeat(400)));
    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );

    let finalized = finalize(
        preparation,
        CommandExit::Failed {
            report: (),
            original_error: original,
            emit_when_trace_disabled: false,
        },
        &Ticks::new(5_000).clock(),
    );

    let index = read_index(root.path(), RUN_A);
    assert_eq!(index.status, RunStatus::Failed);
    assert_eq!(
        index.failure.as_deref(),
        Some("command failed"),
        "the fixed safe word, and only it",
    );
    assert!(
        !all_trace_bytes(root.path()).contains(SENTINEL),
        "no byte of the trace directory quotes the command's error",
    );
    let trace = finalized.trace.expect("a member");
    assert!(trace.warnings.iter().all(|w| !w.contains(SENTINEL)));
    assert!(finalized.notices.iter().all(|n| !n.contains(SENTINEL)));
    // And the caller keeps the rich object it handed over.
    let returned = finalized.original_error.expect("the error comes back");
    assert!(format!("{returned:#}").contains(SENTINEL));
}
