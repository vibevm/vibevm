//! The non-creating reopen: what it answers, and — the load-bearing half —
//! what it refuses to bring into existence in order to answer.
//!
//! Every red here is a MUTATION red. The interesting failures of this seam are
//! all of the form "it worked, and it also wrote something": a caller asking
//! whether a trace exists must not be the caller that makes it exist, must not
//! be the caller that collects nine other runs, and must not be the caller
//! that turns a present-but-unsafe object into a comfortable `None`.

use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

use super::super::{TraceLimits, TraceOpenError, TraceRun};
use super::support::{
    RUN_A, World, at, compile_ok, entries, node_scope, open, project, read_index, roomy, run_dir,
    run_directories, run_id, seed_index_at, seed_run, seeded_index,
};

/// The `.vibe/trace` directory itself, whose absence is the whole point of
/// several reds below.
fn trace_root(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".vibe").join("trace")
}

/// A project with no trace directory at all answers absence — and STILL has
/// no trace directory afterwards.
///
/// `.vibe/` is expected to appear: the cooperative lock lives there, and
/// taking it is the serialization this seam is required to perform. The
/// forbidden mutation is the trace tree, not the lock.
#[test]
fn a_missing_trace_directory_answers_absence_and_creates_nothing() {
    let root = project();
    let answer = TraceRun::open_existing(root.path(), RUN_A, at(1_000));

    assert!(
        matches!(answer, Ok(None)),
        "absence is `Ok(None)`, never a refusal: {:?}",
        answer.map(|run| run.is_some()),
    );
    assert!(
        !trace_root(root.path()).exists(),
        "no trace directory was created to answer the question",
    );
    assert!(
        root.path()
            .join(".vibe")
            .join("compile-trace.lock")
            .exists(),
        "the cooperative lock IS taken — serialization is the point",
    );
}

/// A trace directory that exists but does not hold THIS run answers absence,
/// creates no run child, and leaves every sibling exactly where it was.
#[test]
fn a_missing_run_child_answers_absence_and_touches_no_sibling() {
    let root = project();
    let sibling = run_id(7);
    seed_run(root.path(), &sibling, 900);

    let answer = TraceRun::open_existing(root.path(), RUN_A, at(1_000));

    assert!(matches!(answer, Ok(None)));
    assert!(!run_dir(root.path(), RUN_A).exists(), "no run was minted");
    assert_eq!(run_directories(root.path()), vec![sibling]);
}

/// An existing RUNNING run reopens exactly: the same directory, the restored
/// event count and spent bytes, and a dense sequence that continues rather
/// than restarting.
#[test]
fn an_existing_running_run_reopens_with_its_counters_restored() {
    let root = project();
    let world = World::two_documents();

    let first = open(root.path(), RUN_A, roomy());
    let scope = first.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &world);
    let before = first.summary();
    assert!(before.events > 0 && before.snapshots > 0, "{before:?}");
    drop(scope);
    drop(first);

    let reopened = TraceRun::open_existing(root.path(), RUN_A, at(1_000))
        .expect("a running run reopens")
        .expect("and it is present");
    let after = reopened.summary();

    assert_eq!(after.run_dir, before.run_dir);
    assert_eq!(after.events, before.events, "the event count is restored");
    assert_eq!(
        after.snapshot_bytes, before.snapshot_bytes,
        "the spent budget is recovered from the snapshots on disk",
    );
    assert_eq!(after.status, RunStatus::Running);
    assert!(!after.finalised);

    // The dense global sequence continues from the restored index rather than
    // minting a second zero — the property a fresh `open` would also have,
    // proved here for the path that never creates.
    let second = reopened
        .declare_scope(&node_scope("node:second", "second"))
        .unwrap();
    compile_ok(&second, &world);
    let index = read_index(&run_dir(root.path(), RUN_A));
    let sequences: Vec<u32> = index.events.iter().map(|event| event.sequence).collect();
    let dense: Vec<u32> = (0..u32::try_from(index.events.len()).unwrap()).collect();
    assert_eq!(sequences, dense, "one dense run sequence, not two");
    assert_eq!(run_directories(root.path()), vec![RUN_A.to_string()]);
}

/// The recovered spend is a real budget, not a display number: a run reopened
/// under a ceiling its existing snapshots already passed is exhausted at once.
#[test]
fn the_recovered_spend_re_arms_the_budget() {
    let root = project();
    let world = World::two_documents();
    let first = open(root.path(), RUN_A, roomy());
    let scope = first.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &world);
    let spent = first.summary().snapshot_bytes;
    assert!(spent > 0);
    drop(scope);
    drop(first);

    let reopened = TraceRun::open_existing_with_limits(
        root.path(),
        RUN_A,
        at(1_000),
        TraceLimits::for_test(1, 9),
    )
    .expect("a running run reopens")
    .expect("and it is present");

    assert!(
        reopened.summary().budget_exhausted,
        "a run whose disk already holds {spent} bytes cannot reopen with an unspent budget",
    );
}

/// A run that already reached its terminal word is residue to this seam, not
/// absence: a finished trace is never reopened, and never silently replaced.
#[test]
fn a_terminal_run_refuses_rather_than_reporting_absence() {
    let root = project();
    let done = run_id(3);
    seed_run(root.path(), &done, 1_000);

    let answer = TraceRun::open_existing(root.path(), &done, at(1_000));

    assert!(
        matches!(answer, Err(TraceOpenError::Residue { .. })),
        "a terminal run is refused, not reported absent",
    );
    assert_eq!(
        read_index(&run_dir(root.path(), &done)).status,
        RunStatus::Ok
    );
    assert_eq!(entries(&run_dir(root.path(), &done)), vec!["index.json"]);
}

/// A running index that belongs to another project, or records another start
/// for this run id, is refused — the identity law is the ordinary reopen's,
/// unchanged.
#[test]
fn a_foreign_project_or_a_different_start_refuses() {
    let root = project();

    let foreign = run_id(11);
    let mut index = seeded_index(root.path(), &foreign, 1_000, RunStatus::Running);
    index.project = super::support::foreign_identity();
    seed_index_at(root.path(), &foreign, &index);
    assert!(matches!(
        TraceRun::open_existing(root.path(), &foreign, at(1_000)),
        Err(TraceOpenError::Residue { .. }),
    ));

    let drifted = run_id(12);
    seed_index_at(
        root.path(),
        &drifted,
        &seeded_index(root.path(), &drifted, 1_000, RunStatus::Running),
    );
    assert!(
        matches!(
            TraceRun::open_existing(root.path(), &drifted, at(2_000)),
            Err(TraceOpenError::Residue { .. }),
        ),
        "the same run id with a different start is a different run",
    );
}

/// Contention is decided BEFORE existence is. A busy project says `Busy` even
/// when the run it was asked about does not exist — because the writer that
/// owns the lock may be creating it at that very moment, and answering
/// "there is no such run" would be a claim this process cannot make.
#[test]
fn a_busy_project_refuses_before_it_looks_for_the_run() {
    let root = project();
    let holder = open(root.path(), &run_id(1), roomy());

    let answer = TraceRun::open_existing(root.path(), RUN_A, at(1_000));

    assert!(
        matches!(answer, Err(TraceOpenError::Busy { .. })),
        "busy wins over missing by design",
    );
    drop(holder);
    assert!(matches!(
        TraceRun::open_existing(root.path(), RUN_A, at(1_000)),
        Ok(None),
    ));
}

/// A present run child that is not a link-free directory is residue. This is
/// the exact failure the naive `Path::exists` version gets wrong in the
/// dangerous direction: it would see "not a directory I can open" and report
/// absence about an object that is very much there.
#[test]
fn a_present_but_unsafe_run_child_refuses_as_residue() {
    let root = project();
    std::fs::create_dir_all(trace_root(root.path())).unwrap();
    std::fs::write(trace_root(root.path()).join(RUN_A), b"not a directory").unwrap();

    let answer = TraceRun::open_existing(root.path(), RUN_A, at(1_000));

    assert!(
        matches!(answer, Err(TraceOpenError::Residue { .. })),
        "a present non-directory is residue, never absence",
    );
    assert_eq!(
        std::fs::read(trace_root(root.path()).join(RUN_A)).unwrap(),
        b"not a directory",
        "and it was left exactly as it was",
    );
}

/// Retention is a fresh run's job. Reopening an existing run well past the
/// newest-nine threshold collects nothing at all — a caller that only wanted
/// to look must not be able to delete.
#[test]
fn reopening_an_existing_run_retires_no_completed_sibling() {
    let root = project();
    for n in 1..=12u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }
    let live = run_id(50);
    seed_index_at(
        root.path(),
        &live,
        &seeded_index(root.path(), &live, 900, RunStatus::Running),
    );
    let before = run_directories(root.path());
    assert_eq!(before.len(), 13);

    let reopened = TraceRun::open_existing_with_limits(
        root.path(),
        &live,
        at(900),
        TraceLimits::for_test(u64::MAX, 1),
    )
    .expect("the running run reopens")
    .expect("and it is present");

    assert_eq!(
        run_directories(root.path()),
        before,
        "not one completed sibling was retired, even under a retention of one",
    );
    assert!(reopened.summary().warnings.is_empty());
}

/// The same law on the branch the present-run test cannot reach: asking about
/// a run that is NOT there, with far more than nine collectable siblings and a
/// retention of one.
///
/// A sweep placed before the presence check would be invisible to every test
/// that asks about a run it then keeps open — the answer would still be right.
/// Here the answer is `Ok(None)`, so the only observable is the directory, and
/// eleven runs would be gone.
#[test]
fn a_missing_target_retires_no_sibling_either() {
    let root = project();
    for n in 1..=12u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }
    let before = run_directories(root.path());
    let bytes: u64 = before
        .iter()
        .map(|name| {
            std::fs::metadata(run_dir(root.path(), name).join("index.json"))
                .expect("a seeded index")
                .len()
        })
        .sum();
    assert_eq!(before.len(), 12);

    let answer = TraceRun::open_existing_with_limits(
        root.path(),
        RUN_A,
        at(1_000),
        TraceLimits::for_test(u64::MAX, 1),
    );

    assert!(matches!(answer, Ok(None)));
    assert_eq!(
        run_directories(root.path()),
        before,
        "a question about an absent run collected nothing",
    );
    let after: u64 = before
        .iter()
        .map(|name| {
            std::fs::metadata(run_dir(root.path(), name).join("index.json"))
                .expect("a seeded index")
                .len()
        })
        .sum();
    assert_eq!(after, bytes, "and rewrote nothing either");
}

/// An unsafe ANCESTOR is a `Directory` refusal, not absence. `.vibe/trace`
/// occupied by a regular file is not "no trace directory" — it is a thing this
/// writer cannot walk through, and calling that absence would let a caller
/// conclude a run never existed because somebody planted a file.
#[test]
fn an_unsafe_trace_ancestor_refuses_as_a_directory_fault() {
    let root = project();
    std::fs::create_dir_all(root.path().join(".vibe")).unwrap();
    std::fs::write(trace_root(root.path()), b"not a directory").unwrap();

    let answer = TraceRun::open_existing(root.path(), RUN_A, at(1_000));

    assert!(
        matches!(answer, Err(TraceOpenError::Directory { .. })),
        "an unwalkable ancestor is a fault, never `Ok(None)`",
    );
    assert_eq!(
        std::fs::read(trace_root(root.path())).unwrap(),
        b"not a directory",
        "and it was left exactly as it was",
    );
}

/// The path-pressure gate runs BEFORE the project capability and the lock,
/// exactly as the ordinary open's order fixes it — even for a run that does
/// not exist.
///
/// The proof is what is NOT on disk afterwards: a `.vibe/` directory and a
/// lock file would both be there if the measurement had come later.
#[test]
fn a_too_deep_missing_target_refuses_before_any_capability_is_taken() {
    let root = project();
    // Each segment is 40 characters plus a separator; four of them put the
    // canonical root past the point where `.vibe/trace/<32 hex>/<name>` can
    // still afford the shortest canonical snapshot spelling.
    let segment = "d".repeat(40);
    let mut deep = root.path().to_path_buf();
    for _ in 0..4 {
        deep.push(&segment);
    }
    std::fs::create_dir_all(&deep).expect("a deep but legal project root");

    let answer = TraceRun::open_existing(&deep, RUN_A, at(1_000));

    assert!(
        matches!(answer, Err(TraceOpenError::RunDirectoryTooDeep { .. })),
        "a directory that cannot afford a snapshot name refuses to open: {answer:?}",
        answer = answer.map(|run| run.is_some()),
    );
    assert!(
        !deep.join(".vibe").exists(),
        "no capability was opened and no lock was taken, so nothing was created",
    );
}
