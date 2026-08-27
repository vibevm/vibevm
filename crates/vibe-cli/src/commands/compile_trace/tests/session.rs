//! The owner's state machine: which identity opens what, what each state
//! costs, and what none of them are allowed to create.

use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;
use vibe_wire::generated::shared::TraceReportStatus;
use vibe_workspace::compile_trace::TraceRun;

use super::super::{CommandExit, finalize, prepare, without_workspace};
use super::support::{
    RUN_A, RUN_B, STARTED_A, STARTED_B, Ticks, after, all_trace_bytes, at, displacing, identity,
    project, read_index, run_dir, run_directories, started, trace_root,
};

/// Off means off: no directory, no recorder, no clock, no member — and the
/// command still reports.
#[test]
fn a_disabled_run_opens_nothing_and_asks_no_clock() {
    let root = project();
    let supersede = Ticks::new(10);
    let finish = Ticks::new(20);

    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, false),
        &supersede.clock(),
    );
    assert!(preparation.recorder().is_none());
    assert!(!preparation.trace_requested());

    let finalized = finalize(preparation, CommandExit::Success("draft"), &finish.clock());

    assert!(finalized.trace.is_none(), "no member at all");
    assert!(finalized.emit_report, "success always reports");
    assert!(!finalized.trace_requested);
    assert!(finalized.notices.is_empty());
    assert_eq!((supersede.calls(), finish.calls()), (0, 0));
    assert!(!trace_root(root.path()).exists());
    assert!(all_trace_bytes(root.path()).is_empty());
}

/// A fresh request creates exactly one run, and success finalises it `ok` with
/// exactly one clock call.
#[test]
fn a_fresh_request_creates_one_run_and_finalises_it_ok() {
    let root = project();
    let finish = Ticks::new(4_000);
    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );
    assert!(preparation.recorder().is_some());
    assert!(preparation.trace_requested());

    let finalized = finalize(preparation, CommandExit::Success(()), &finish.clock());

    let trace = finalized.trace.expect("an opened run carries a member");
    assert_eq!(trace.status, TraceReportStatus::Ok);
    assert!(trace.finalised);
    assert_eq!(trace.run_id, RUN_A);
    assert_eq!(finish.calls(), 1, "exactly one finish instant");
    assert_eq!(run_directories(root.path()), vec![RUN_A.to_string()]);
    let index = read_index(root.path(), RUN_A);
    assert_eq!(index.status, RunStatus::Ok);
    assert_eq!(index.finished, Some(after(4_000)));
}

/// A failed command finalises the same run `failed`, and the index records the
/// FIXED word — never the command's own error text.
#[test]
fn a_failed_command_finalises_the_run_with_the_fixed_word() {
    let root = project();
    let finish = Ticks::new(4_100);
    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );

    let finalized = finalize(
        preparation,
        CommandExit::Failed {
            report: (),
            original_error: anyhow::anyhow!("the command's own words"),
            emit_when_trace_disabled: false,
        },
        &finish.clock(),
    );

    let trace = finalized
        .trace
        .expect("a failed run still carries a member");
    assert_eq!(trace.status, TraceReportStatus::Failed);
    assert!(trace.finalised);
    assert_eq!(finish.calls(), 1);
    let index = read_index(root.path(), RUN_A);
    assert_eq!(index.status, RunStatus::Failed);
    assert_eq!(index.failure.as_deref(), Some("command failed"));
}

/// Park is suspension: a running member, no finish, no clock — and the lock
/// really is released, which is exactly what the resume needs.
#[test]
fn a_park_leaves_a_running_index_and_releases_the_lock() {
    let root = project();
    let finish = Ticks::new(4_200);
    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );

    let finalized = finalize(preparation, CommandExit::Parked(()), &finish.clock());

    let trace = finalized.trace.expect("a parked run carries a member");
    assert_eq!(trace.status, TraceReportStatus::Running);
    assert!(!trace.finalised);
    assert!(finalized.emit_report);
    assert_eq!(finish.calls(), 0, "park never finishes, so it never asks");
    let index = read_index(root.path(), RUN_A);
    assert_eq!(index.status, RunStatus::Running);
    assert!(
        index.finished.is_none(),
        "no terminal timestamp was written"
    );
    // The proof the lock went: an existing-only reopen now succeeds instead of
    // reporting the project busy.
    assert!(
        TraceRun::open_existing(root.path(), RUN_A, started(STARTED_A))
            .expect("the reopen is not refused")
            .is_some(),
    );
}

/// A resume adopts the SAME run: it reopens, it appends, and it never mints a
/// second directory.
#[test]
fn an_adopted_run_reopens_the_run_the_park_left() {
    let root = project();
    let parked = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );
    // The park's own join is not this test's subject — but it still has to be
    // consumed, which is exactly what `#[must_use]` is there to insist on.
    let _ = finalize(parked, CommandExit::Parked(()), &Ticks::new(20).clock());

    let finish = Ticks::new(4_300);
    let resumed = prepare(
        root.path(),
        &identity(RUN_A, true, true),
        &Ticks::new(30).clock(),
    );
    assert!(resumed.recorder().is_some(), "the parked run was reopened");
    let finalized = finalize(resumed, CommandExit::Success(()), &finish.clock());

    let trace = finalized.trace.expect("the resumed run carries a member");
    assert_eq!(trace.status, TraceReportStatus::Ok);
    assert_eq!(trace.run_id, RUN_A);
    assert_eq!(
        run_directories(root.path()),
        vec![RUN_A.to_string()],
        "one run, resumed — not two",
    );
    assert_eq!(read_index(root.path(), RUN_A).started, started(STARTED_A));
    assert_eq!(finish.calls(), 1);
}

/// The sticky bit proves a REQUEST. A resume whose original invocation never
/// opened a trace stays unavailable rather than starting a partial history
/// halfway through the lifecycle run — and creates nothing while saying so.
#[test]
fn a_previously_unavailable_run_never_starts_mid_run() {
    let root = project();

    // Invocation one: tracing was requested, but another writer owned the
    // project, so no recorder opened. It parks.
    let blocker = TraceRun::open(root.path(), RUN_B, at(900)).expect("the blocking run opens");
    let first = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );
    assert!(first.recorder().is_none(), "a busy project is not traced");
    assert!(first.trace_requested(), "but it WAS requested");
    let parked = finalize(first, CommandExit::Parked(()), &Ticks::new(20).clock());
    let member = parked.trace.expect("unavailable is still reported");
    assert_eq!(member.status, TraceReportStatus::Unavailable);
    assert!(member.run_path.is_none());
    assert!(!run_dir(root.path(), RUN_A).exists());
    drop(blocker);

    // Invocation two: the sticky bit survives the park, so it adopts — and
    // must STILL be unavailable.
    let finish = Ticks::new(4_400);
    let resumed = prepare(
        root.path(),
        &identity(RUN_A, true, true),
        &Ticks::new(30).clock(),
    );
    assert!(resumed.recorder().is_none());
    let finalized = finalize(resumed, CommandExit::Success(()), &finish.clock());

    let trace = finalized.trace.expect("a member says why");
    assert_eq!(trace.status, TraceReportStatus::Unavailable);
    assert_eq!(
        (&trace.events, &trace.snapshots),
        (&"0".to_string(), &"0".to_string())
    );
    assert!(trace.timings.is_empty());
    assert!(
        trace.warnings.iter().any(|w| !w.trim().is_empty()),
        "and it is not silent about it: {:?}",
        trace.warnings,
    );
    assert_eq!(finish.calls(), 0, "there was nothing to finish");
    assert!(
        !run_dir(root.path(), RUN_A).exists(),
        "no phantom mid-run trace was created",
    );
}

/// A displaced predecessor whose trace really is running becomes a terminal
/// `failed` — with the fixed superseded word — BEFORE the fresh run opens.
#[test]
fn a_real_superseded_running_trace_becomes_terminal_first() {
    let root = project();
    let old = TraceRun::open(root.path(), RUN_B, started(STARTED_B)).expect("the parked run opens");
    drop(old);
    assert_eq!(read_index(root.path(), RUN_B).status, RunStatus::Running);

    let supersede = Ticks::new(3_000);
    let preparation = prepare(root.path(), &displacing(RUN_A, RUN_B), &supersede.clock());

    assert_eq!(supersede.calls(), 1, "one terminal write, one instant");
    let displaced = read_index(root.path(), RUN_B);
    assert_eq!(displaced.status, RunStatus::Failed);
    assert_eq!(
        displaced.failure.as_deref(),
        Some("superseded by a later invocation of this workspace"),
    );
    assert_eq!(displaced.finished, Some(after(3_000)));
    assert!(
        preparation.recorder().is_some(),
        "and the fresh run opened afterwards — the old lock was released",
    );
    let finalized = finalize(
        preparation,
        CommandExit::Success(()),
        &Ticks::new(50).clock(),
    );
    assert!(finalized.notices.is_empty(), "{:?}", finalized.notices);
    let mut runs = run_directories(root.path());
    runs.sort();
    assert_eq!(runs, vec![RUN_A.to_string(), RUN_B.to_string()]);
}

/// The displaced predecessor is terminalised even when THIS run is not being
/// traced at all.
///
/// The two facts are independent: the sticky bit that made the predecessor a
/// traced park is its own, and an abandoned running trace stays abandoned —
/// and retention-ineligible — forever if a later untraced invocation walks
/// past it. Disabled still means disabled for the current run: no member, no
/// finish clock, no directory of its own.
#[test]
fn a_disabled_run_still_terminalises_its_displaced_predecessor() {
    let root = project();
    let old = TraceRun::open(root.path(), RUN_B, started(STARTED_B)).expect("the parked run opens");
    drop(old);

    let supersede = Ticks::new(3_100);
    let finish = Ticks::new(3_200);
    let mut identity = displacing(RUN_A, RUN_B);
    identity.compile_trace = false;

    let preparation = prepare(root.path(), &identity, &supersede.clock());
    assert!(!preparation.trace_requested(), "this run is NOT traced");
    assert!(preparation.recorder().is_none());
    let finalized = finalize(preparation, CommandExit::Success(()), &finish.clock());

    assert_eq!(
        supersede.calls(),
        1,
        "the predecessor still got its instant"
    );
    assert_eq!(finish.calls(), 0, "and this run had nothing to finish");
    let displaced = read_index(root.path(), RUN_B);
    assert_eq!(displaced.status, RunStatus::Failed);
    assert_eq!(displaced.finished, Some(after(3_100)));
    assert!(
        finalized.trace.is_none(),
        "a disabled run carries no member"
    );
    assert!(!run_dir(root.path(), RUN_A).exists());
    assert_eq!(run_directories(root.path()), vec![RUN_B.to_string()]);
}

/// Dropping the owner without `finalize` writes NOTHING. There is no I/O
/// `Drop` and no `Drop` that invents a timestamp — dropping releases the
/// cooperative lock and that is all it does, so the run stays exactly as
/// honest as a park.
#[test]
fn dropping_the_owner_without_finalising_writes_nothing() {
    let root = project();
    let preparation = prepare(
        root.path(),
        &identity(RUN_A, false, true),
        &Ticks::new(10).clock(),
    );
    assert!(preparation.recorder().is_some());

    drop(preparation);

    let index = read_index(root.path(), RUN_A);
    assert_eq!(index.status, RunStatus::Running);
    assert!(
        index.finished.is_none(),
        "no `Drop` invented a terminal instant",
    );
    assert!(
        TraceRun::open_existing(root.path(), RUN_A, started(STARTED_A))
            .expect("the reopen is not refused")
            .is_some(),
        "and the lock really was released by the drop",
    );
}

/// A displaced predecessor that never opened a trace produces no phantom
/// terminal run and costs no instant at all.
#[test]
fn a_missing_superseded_trace_creates_no_phantom_and_asks_no_clock() {
    let root = project();
    let supersede = Ticks::new(3_000);

    let preparation = prepare(root.path(), &displacing(RUN_A, RUN_B), &supersede.clock());
    let finalized = finalize(
        preparation,
        CommandExit::Success(()),
        &Ticks::new(60).clock(),
    );

    assert_eq!(supersede.calls(), 0, "nothing needed a terminal write");
    assert!(
        !run_dir(root.path(), RUN_B).exists(),
        "a trace was not manufactured merely so it could be superseded",
    );
    assert_eq!(run_directories(root.path()), vec![RUN_A.to_string()]);
    assert!(finalized.notices.is_empty());
}

/// A start the lifecycle cannot spell as RFC 3339 is a reason to compile
/// untraced, not a reason to fail — and it costs no filesystem work.
#[test]
fn an_unparsable_start_is_unavailable_rather_than_an_error() {
    let root = project();
    let mut identity = identity(RUN_A, false, true);
    identity.started = "not a timestamp".to_string();

    let preparation = prepare(root.path(), &identity, &Ticks::new(10).clock());
    assert!(preparation.recorder().is_none());
    let finalized = finalize(
        preparation,
        CommandExit::Success(()),
        &Ticks::new(70).clock(),
    );

    let trace = finalized.trace.expect("a member says why");
    assert_eq!(trace.status, TraceReportStatus::Unavailable);
    assert!(!trace_root(root.path()).exists());
}

/// No canonical root means no supersession — and the operator is told so.
///
/// A displaced predecessor is a fact about the STATE, not about this
/// invocation's flags: its index still reads `running`, this command has taken
/// its identity, and closing it needs the very root that could not be found.
/// So the notice exists whether or not this run wanted a trace, and nothing is
/// created on disk to say it.
#[test]
fn a_workspaceless_owner_reports_the_predecessor_it_could_not_supersede() {
    for requested in [false, true] {
        let root = project();
        let mut identity = displacing(RUN_A, RUN_B);
        identity.compile_trace = requested;

        let preparation = without_workspace(&identity);
        assert_eq!(
            preparation.trace_requested(),
            requested,
            "the SESSION follows the request",
        );
        assert!(
            preparation.recorder().is_none(),
            "and never opens a recorder without a root",
        );

        let finalized = finalize(
            preparation,
            CommandExit::Success(()),
            &Ticks::new(9_000).clock(),
        );

        let notices = &finalized.notices;
        assert_eq!(
            notices.len(),
            1,
            "exactly one notice, whatever the request: {notices:?}",
        );
        assert!(
            notices[0].contains(RUN_B),
            "naming the displaced run: {}",
            notices[0],
        );
        assert!(
            notices[0].contains("could not be superseded"),
            "and saying what could not be done: {}",
            notices[0],
        );

        if requested {
            let member = finalized
                .trace
                .expect("a requested trace still reports its own fate");
            assert_eq!(
                member.status,
                TraceReportStatus::Unavailable,
                "requested and never opened is `unavailable`",
            );
            assert!(
                member.warnings.iter().any(|w| w.contains(RUN_B)),
                "and the member carries the notice too: {:?}",
                member.warnings,
            );
        } else {
            assert!(
                finalized.trace.is_none(),
                "an unrequested run has no member to carry",
            );
        }

        // Nothing was created to hold any of it.
        assert!(
            run_directories(root.path()).is_empty(),
            "no run tree without a canonical root",
        );
        assert!(
            !trace_root(root.path()).exists(),
            "not even the `.vibe/trace` parent",
        );
    }
}
