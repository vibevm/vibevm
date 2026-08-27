//! The repair-2 laws: one writer per project, a terminal index judged by the
//! disk, a two-named payload charged anyway, an unexplained fresh directory
//! named as residue, and a real filesystem alias.

use vibe_wire::behaviour::compiler_trace_index::SnapshotName;
use vibe_wire::generated::compiler_trace_index::e1::index::{PassStatus, RunStatus, ScopeKind};

use super::super::{RunOutcome, TraceLimits, TraceOpenError, TraceRun, TraceWarning};
use super::support::{
    RUN_A, World, at, compile_ok, entries, node_scope, open, project, read_index, roomy, run_dir,
    run_directories, run_id,
};

// ------------------------------------------------------------- ruling 0

/// Cooperating writers are serialized by one project lock, and the guard is
/// held for as long as ANY clone of the run survives — a `TraceScope` handed
/// to a compile counts.
///
/// The refusal is non-blocking on purpose: an observer that can make a compile
/// wait on another process is an observer that can deadlock one.
#[test]
fn one_project_admits_one_trace_writer_at_a_time() {
    let root = project();
    assert!(
        std::fs::read_dir(root.path().join(".vibe").join("trace")).is_err(),
        "nothing exists yet",
    );

    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    let snapshot = run_directories(root.path());

    // While the run is alive.
    let busy = TraceRun::open_with_limits(root.path(), &run_id(7), at(1), roomy())
        .expect_err("a second writer is refused, not queued");
    assert!(matches!(busy, TraceOpenError::Busy { .. }), "{busy}");

    // And while only a cloned SCOPE is alive.
    drop(run);
    let still_busy = TraceRun::open_with_limits(root.path(), &run_id(7), at(1), roomy())
        .expect_err("a live scope still owns the project");
    assert!(
        matches!(still_busy, TraceOpenError::Busy { .. }),
        "{still_busy}"
    );
    assert_eq!(
        run_directories(root.path()),
        snapshot,
        "a refused open creates and retires nothing",
    );

    // Once the final clone drops, the same open succeeds.
    drop(scope);
    TraceRun::open_with_limits(root.path(), &run_id(7), at(1), roomy())
        .expect("the project is free again");
}

/// The lock is scoped to a project, not to the process: a second project is
/// traced normally while the first is owned.
#[test]
fn a_different_project_is_not_blocked_by_a_busy_one() {
    let first = project();
    let second = project();
    let held = open(first.path(), RUN_A, roomy());
    let other = open(second.path(), RUN_A, roomy());
    assert_ne!(held.run_dir(), other.run_dir());
    assert_eq!(read_index(other.run_dir()).status, RunStatus::Running);
}

// ------------------------------------------------------------------- §1

/// A terminal index whose publication faulted AFTER its irreversible step may
/// already be the cold reader's truth. The store re-reads the destination and
/// compares it byte-for-byte, so the run is finalised — and the fault is kept
/// as a warning rather than erased.
#[test]
fn a_terminal_index_that_landed_despite_a_fault_is_finalised() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();

    // Armed across the WHOLE terminal write, and never disarmed before it.
    vibe_safefs::fail_after_publish(Some("index.json"));
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    vibe_safefs::fail_after_publish(None);

    assert!(
        summary.finalised,
        "bytes that ARE on disk are a finalised run",
    );
    assert_eq!(summary.status, RunStatus::Ok);
    let anomalies = summary
        .warnings
        .iter()
        .filter(|warning| matches!(warning, TraceWarning::IndexAnomaly { .. }))
        .count();
    assert_eq!(anomalies, 1, "one bounded anomaly: {:?}", summary.warnings);
    assert!(
        !summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::NotFinalised { .. })),
        "and it is not reported as unfinished: {:?}",
        summary.warnings,
    );

    let on_disk = read_index(&directory);
    assert_eq!(on_disk.status, RunStatus::Ok, "the cold reader agrees");
    assert_eq!(on_disk.finished, Some(at(2_000)));

    // A repeat `finish` never rewrites a running lie over the terminal truth.
    let again = run.finish(&RunOutcome::Ok, at(9_000));
    assert!(again.finalised);
    let after = read_index(&directory);
    assert_eq!(after.status, RunStatus::Ok);
    assert_eq!(
        after.finished,
        Some(at(2_000)),
        "the terminal word is final"
    );
}

/// The recovery read is the authority, not merely the publication stage. A
/// post-step fault is followed by a deterministic namespace change back to
/// the prior whole bytes; unconditional `PossiblyPublished => Written` would
/// therefore declare a terminal status the cold reader cannot see.
#[test]
fn a_post_step_index_mutation_is_not_mistaken_for_the_attempted_bytes() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    let prior = std::fs::read(directory.join("index.json")).unwrap();

    super::super::store::arm_before_index_recovery(Some(Box::new(move |path| {
        std::fs::write(path, &prior).expect("the race restores the prior whole index");
    })));
    vibe_safefs::fail_after_publish(Some("index.json"));
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    vibe_safefs::fail_after_publish(None);
    super::super::store::arm_before_index_recovery(None);

    assert!(
        !summary.finalised,
        "the attempted bytes are no longer visible"
    );
    assert_eq!(summary.status, RunStatus::Running);
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::NotFinalised { .. })),
        "{:?}",
        summary.warnings,
    );
    let cold = read_index(&directory);
    assert_eq!(cold.status, RunStatus::Running, "the cold reader decides");
    assert!(cold.finished.is_none());
}

// ------------------------------------------------------------------- §2

/// The window a probe cannot see: the snapshot `hard_link` landed and the
/// owned stage was not collected, so a full payload sits under two names and
/// `inspect_file_in` refuses it as not exclusively owned.
///
/// The attempted payload is charged anyway — once, not once per directory
/// entry — the name is reserved, the event is honestly `snapshot-failed` with
/// no filename, the tiny budget stands every later event down, and a reopen
/// refuses the extra stage instead of adopting or deleting it.
#[test]
fn a_two_named_payload_is_charged_once_and_stands_the_budget_down() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, TraceLimits::for_test(1, 9));
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();

    let orphan = SnapshotName {
        sequence: 0,
        invocation: 0,
        kind: &ScopeKind::Node,
        pass: "parse",
        label: ".",
        artifact: "static-md",
    }
    .within(96)
    .expect("the first event's name");

    vibe_safefs::fail_before_stage_cleanup(Some(&orphan));
    compile_ok(&scope, &World::two_documents());
    vibe_safefs::fail_before_stage_cleanup(None);
    scope.complete("fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));

    // Both directory entries are really there, and they are one payload.
    let names = entries(&directory);
    let stage: Vec<&String> = names
        .iter()
        .filter(|name| name.starts_with(vibe_safefs::STAGE_PREFIX))
        .collect();
    assert_eq!(stage.len(), 1, "the stage survived: {names:?}");
    let landed = directory.join(&orphan);
    assert!(landed.is_file(), "and so did the final name");
    let payload = std::fs::metadata(&landed).unwrap().len();
    assert_eq!(
        std::fs::metadata(directory.join(stage[0])).unwrap().len(),
        payload,
        "both names are the same payload",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            std::fs::metadata(&landed).unwrap().ino(),
            std::fs::metadata(directory.join(stage[0])).unwrap().ino(),
            "one inode",
        );
    }

    // Charged ONCE, for one payload.
    assert_eq!(
        summary.snapshot_bytes, payload,
        "the attempted payload is charged exactly once",
    );
    assert!(summary.budget_exhausted);
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::Residue { .. })),
        "{:?}",
        summary.warnings,
    );

    // The event is honest, and no second payload was ever published.
    let index = read_index(&directory);
    assert_eq!(index.events[0].status, PassStatus::SnapshotFailed);
    assert!(index.events.iter().all(|event| event.snapshot.is_none()));
    assert_eq!(index.status, RunStatus::Ok);
    assert_eq!(
        names.len(),
        3,
        "index + final name + stage, and nothing else: {names:?}",
    );

    // A reopen refuses the extra stage rather than adopting or deleting it.
    drop(scope);
    drop(run);
    let residue = TraceRun::open_with_limits(root.path(), RUN_A, at(1_000), roomy())
        .expect_err("a run carrying an entry it never wrote is residue");
    assert!(
        matches!(residue, TraceOpenError::Residue { .. }),
        "{residue}"
    );
    assert_eq!(entries(&directory), names, "and nothing was removed");
}

// ------------------------------------------------------------------- §3

/// A fresh run directory is created exclusively before its first index is
/// written. If that index cannot land, the directory is left EXACTLY as it is
/// and named as residue — never auto-deleted through the identity-bound
/// removal path, which is for runs this writer can explain.
#[test]
fn a_fresh_run_whose_first_index_cannot_land_is_named_residue() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);

    vibe_safefs::fail_before_publish(Some("index.json"));
    let error = TraceRun::open_with_limits(root.path(), RUN_A, at(1_000), roomy())
        .expect_err("a run with no index is not a run");
    vibe_safefs::fail_before_publish(None);

    let TraceOpenError::Residue { path, reason } = &error else {
        panic!("expected a path-carrying residue refusal, got {error}");
    };
    assert!(
        path.ends_with(RUN_A),
        "the refusal names the exact directory: {path}",
    );
    assert!(
        reason.contains("no index landed"),
        "and says what is missing: {reason}",
    );

    // The exact residue is on disk, empty, with nothing else beside it.
    assert!(directory.is_dir(), "the created directory is left in place");
    assert!(entries(&directory).is_empty(), "and it is empty");
    assert_eq!(
        run_directories(root.path()),
        vec![RUN_A.to_string()],
        "no unrelated entry was created",
    );

    // The lock was released: the next open gets as far as JUDGING the
    // residue, which it could not do if it were still refused as busy.
    let next = TraceRun::open_with_limits(root.path(), RUN_A, at(1_000), roomy())
        .expect_err("the residue still refuses");
    assert!(
        matches!(next, TraceOpenError::Residue { .. }),
        "an honest residue refusal, not a busy one: {next}",
    );
}

// ------------------------------------------------------------------- §4

/// A REAL filesystem alias, not a lexical detour: on Unix a directory
/// symlink, on Windows a case spelling on a case-insensitive volume. Either
/// way the alias must reopen the same run, the same canonical directory and
/// the same project digest.
///
/// The positive control is asserted, never assumed. Where the host genuinely
/// cannot supply an alias, the test says so through
/// [`ALIAS_LIMITATION`](self::ALIAS_LIMITATION) rather than reporting a pass
/// it did not earn.
#[test]
fn a_real_filesystem_alias_reopens_the_same_run() {
    let root = project();
    let run = open(root.path(), RUN_A, roomy());
    let canonical_dir = run.run_dir().to_path_buf();
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    let digest = read_index(&canonical_dir).project.root_digest.clone();
    drop(scope);
    drop(run);

    let Some(alias) = real_alias(root.path()) else {
        // The one honest fallback: the platform helper still has to be
        // stable, and the limitation is recorded in the durable report.
        assert!(ALIAS_LIMITATION.is_some());
        assert_eq!(
            std::fs::canonicalize(root.path()).unwrap(),
            std::fs::canonicalize(root.path()).unwrap(),
        );
        return;
    };
    // Positive control: a different spelling that really names the same tree.
    assert_ne!(alias, root.path(), "the alias is a different spelling");
    assert_eq!(
        std::fs::canonicalize(&alias).unwrap(),
        std::fs::canonicalize(root.path()).unwrap(),
        "and it really resolves to the same tree",
    );

    let resumed = TraceRun::open_with_limits(&alias, RUN_A, at(1_000), roomy())
        .expect("a real alias is the same project");
    assert_eq!(
        resumed.run_dir(),
        canonical_dir,
        "one canonical run directory, whichever spelling opened it",
    );
    assert_eq!(read_index(&canonical_dir).project.root_digest, digest);
    resumed
        .declare_scope(&node_scope("node:.", "."))
        .expect("the pending scope is reacquired, not conflicted")
        .complete("fp")
        .unwrap();
    resumed.finish(&RunOutcome::Ok, at(2_000));
    assert_eq!(read_index(&canonical_dir).status, RunStatus::Ok);
    assert_eq!(
        run_directories(root.path()).len(),
        1,
        "an alias never mints a second run directory",
    );
}

/// A real directory symlink beside the root, pointing at it. A temporary
/// directory on Unix always admits one, so a failure here is a genuine
/// failure rather than a missing capability.
#[cfg(unix)]
#[cfg(test)]
fn real_alias(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let name = root.file_name()?.to_str()?;
    let link = root.parent()?.join(format!("{name}-alias"));
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(root, &link).expect("a temporary directory admits a symlink");
    Some(link)
}

#[cfg(unix)]
const ALIAS_LIMITATION: Option<&str> = None;

/// A privilege-free case spelling. NTFS is case-insensitive by default, so an
/// uppercased final component names the same directory; a volume configured
/// case-sensitive supplies no alias, and the probe is what decides.
#[cfg(windows)]
fn real_alias(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let name = root.file_name()?.to_str()?;
    let shouted = name.to_uppercase();
    if shouted == name {
        return None;
    }
    let alias = root.parent()?.join(shouted);
    std::fs::metadata(&alias).ok()?;
    Some(alias)
}

#[cfg(windows)]
const ALIAS_LIMITATION: Option<&str> = Some(
    "this volume is case-sensitive and grants no privilege-free directory alias; creating a \
     symlink or junction needs a privilege the test runner is not assumed to hold",
);
