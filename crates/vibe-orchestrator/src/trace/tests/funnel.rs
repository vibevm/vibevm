//! The generic join: error identity, emission policy, the shared member's own
//! laws, and who really owns the cooperative lock at the end.

use vibe_wire::generated::shared::{TimingRow, TraceReportStatus};
use vibe_workspace::compile_trace::TraceRun;

use super::super::{CommandExit, PlanDisposition, finalize, prepare, without_workspace};
use super::support::{
    RUN_A, STARTED_A, Ticks, compile, identity, node_scope, project, run_dir, started,
};

/// A sentinel error type, so "the same object" is a downcast rather than a
/// string comparison.
#[derive(Debug, thiserror::Error)]
#[error("slot `{slot}` refused")]
struct SlotRefused {
    slot: &'static str,
}

/// The caller's error survives the funnel intact: same type, same fields, same
/// chain — never re-wrapped and never rebuilt from its own `Display`.
#[test]
fn the_original_error_object_survives_a_failed_close() {
    let root = project();
    let original = anyhow::Error::new(SlotRefused { slot: "create" })
        .context("running the create phase")
        .context("vibe lifecycle");
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
        &Ticks::new(4_500).clock(),
    );

    let returned = finalized
        .original_error
        .expect("a failure returns its error");
    let downcast = returned
        .downcast_ref::<SlotRefused>()
        .expect("the ORIGINAL object, not a reconstruction");
    assert_eq!(downcast.slot, "create");
    assert_eq!(returned.chain().count(), 3, "the context chain is intact");
    assert_eq!(returned.to_string(), "vibe lifecycle");
}

/// The deferred-plan disposition is read from the EXIT ARM, and only a park
/// discards.
///
/// The whole point of the typed answer is that it does NOT come from the
/// finished report: reading "does the draft carry a delegation member?" back
/// off the document would be an inference, and a surface that disagreed with
/// the funnel about whether a run parked would either drop a preview a
/// completed run owes or print one beside a handoff that is supposed to stand
/// alone. So the mapping is pinned here, on all three arms, with no filesystem
/// in the way: the surface's own `--json` framing golden (a parked run emits
/// ONE total document) is the other end of the same law.
#[test]
fn only_a_park_discards_the_deferred_plan_preview() {
    let disabled = || without_workspace(&identity(RUN_A, false, false));
    let clock = Ticks::new(10);

    assert_eq!(
        finalize(disabled(), CommandExit::Success(()), &clock.clock()).plan,
        PlanDisposition::Flush,
        "a completed run's preview records what it was doing",
    );
    assert_eq!(
        finalize(
            disabled(),
            CommandExit::Failed {
                report: (),
                original_error: anyhow::anyhow!("refused"),
                emit_when_trace_disabled: false,
            },
            &clock.clock(),
        )
        .plan,
        PlanDisposition::Flush,
        "a failure is an outcome too, and its preview still records it",
    );
    assert_eq!(
        finalize(disabled(), CommandExit::Parked(()), &clock.clock()).plan,
        PlanDisposition::Discard,
        "a park emits ONE document in total",
    );
}

/// The failure emission truth table, in one place.
///
/// Success and park always report. A failure reports when its own typed bit
/// says so OR when tracing was requested — and "requested" is the pre-close
/// fact, so a member the validator then omits does not silence a command that
/// asked to be observed.
#[test]
fn the_failure_emission_truth_table_holds() {
    let root = project();
    let failed = |emit_when_trace_disabled| CommandExit::Failed {
        report: (),
        original_error: anyhow::anyhow!("refused"),
        emit_when_trace_disabled,
    };

    // Disabled × the bit: the bit alone decides.
    for (bit, expected) in [(false, false), (true, true)] {
        let preparation = prepare(
            root.path(),
            &identity(RUN_A, false, false),
            &Ticks::new(10).clock(),
        );
        let finalized = finalize(preparation, failed(bit), &Ticks::new(20).clock());
        assert_eq!(finalized.emit_report, expected, "disabled with bit={bit}");
        assert!(!finalized.trace_requested);
        assert!(finalized.trace.is_none());
    }

    // Requested and OPEN, with the bit off: requested wins.
    let opened = project();
    let preparation = prepare(
        opened.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );
    let finalized = finalize(preparation, failed(false), &Ticks::new(4_600).clock());
    assert!(finalized.emit_report, "a requested trace is observable");
    assert!(finalized.trace_requested);
    assert!(finalized.trace.is_some());

    // Requested, and the member itself is INVALID — an unavailable state whose
    // run id is not a lifecycle run id at all. The member is omitted, and
    // neither `trace_requested` nor `emit_report` moves.
    let broken = project();
    let mut junk = identity("not-a-run-id", false, true);
    junk.run_id = "not-a-run-id".to_string();
    let preparation = prepare(broken.path(), &junk, &Ticks::new(10).clock());
    assert!(preparation.recorder().is_none());
    let finalized = finalize(preparation, failed(false), &Ticks::new(4_700).clock());
    assert!(
        finalized.trace.is_none(),
        "an invalid member is omitted rather than emitted",
    );
    assert!(finalized.trace_requested, "but it was still requested");
    assert!(finalized.emit_report, "so the command still reports");
    assert!(
        finalized
            .notices
            .iter()
            .any(|notice| notice.contains("run-id")),
        "and the refusal is a secondary notice: {:?}",
        finalized.notices,
    );
}

/// Every produced member passes the shared validator, spells its counts as
/// canonical decimals, carries an absolute forward-slashed run path, and owns
/// the EXACT shared generated timing-row type — not a same-shaped copy.
#[test]
fn a_produced_member_obeys_every_shared_law() {
    let root = project();
    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );
    let scope = preparation
        .recorder()
        .expect("the run opened")
        .declare_scope(&node_scope("node:."))
        .expect("a scope is declared");
    compile(&scope);
    drop(scope);

    let finalized = finalize(
        preparation,
        CommandExit::Success(()),
        &Ticks::new(4_800).clock(),
    );
    let trace = finalized.trace.expect("a member");

    vibe_wire::behaviour::compile_trace_report::validate(&trace)
        .expect("the member obeys every relational law");
    // The exact shared type, pinned by an annotation a same-shaped copy would
    // fail to satisfy.
    let rows: &Vec<TimingRow> = &trace.timings;
    assert!(!rows.is_empty(), "a real compile produced aggregate rows");
    assert!(rows.iter().any(|row| row.pass == "parse"));
    assert_eq!(trace.status, TraceReportStatus::Ok);
    for count in [&trace.events, &trace.snapshots, &trace.snapshot_bytes] {
        assert!(!count.is_empty() && count.bytes().all(|b| b.is_ascii_digit()));
        assert!(count.len() == 1 || !count.starts_with('0'), "{count}");
    }
    assert_ne!(trace.events, "0", "the compile really was observed");
    let path = trace.run_path.expect("an opened run names its directory");
    assert!(!path.contains('\\'), "forward-slashed: {path}");
    assert!(path.ends_with(&format!(".vibe/trace/{RUN_A}")), "{path}");
}

/// Closing drops the command's handle — but a scope somebody deliberately
/// retained still owns the shared state, and therefore still owns the
/// cooperative lock. There is no hidden force-release, because forcing one
/// would be a lie about who is writing.
#[test]
fn a_retained_scope_keeps_the_lock_until_it_drops() {
    let root = project();
    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );
    let retained = preparation
        .recorder()
        .expect("the run opened")
        .declare_scope(&node_scope("node:."))
        .expect("a scope is declared");

    let finalized = finalize(
        preparation,
        CommandExit::Success(()),
        &Ticks::new(4_900).clock(),
    );
    assert!(finalized.trace.is_some());

    let while_held = TraceRun::open_existing(root.path(), RUN_A, started(STARTED_A));
    assert!(
        matches!(
            while_held,
            Err(vibe_workspace::compile_trace::TraceOpenError::Busy { .. })
        ),
        "the retained scope still owns the project lock",
    );

    drop(retained);
    let after = TraceRun::open_existing(root.path(), RUN_A, started(STARTED_A));
    assert!(
        !matches!(
            after,
            Err(vibe_workspace::compile_trace::TraceOpenError::Busy { .. })
        ),
        "and it really was the scope holding it",
    );
    assert!(run_dir(root.path(), RUN_A).exists());
}
