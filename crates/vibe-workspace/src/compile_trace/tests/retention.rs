//! Retention: what gets collected, and — the load-bearing half — everything
//! that does not.

use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

use super::super::{TraceLimits, TraceWarning};
use super::support::{
    World, at, compile_ok, entries, node_scope, open, project, read_index, roomy, run_dir,
    run_directories, run_id, seed_index_at, seed_run, seeded_index,
};

/// Twelve completed owned runs plus one new run leave ten: the newest nine
/// seeds, ordered by `index.started`, and the live tenth.
#[test]
fn twelve_completed_runs_plus_one_new_leave_ten() {
    let root = project();
    for n in 1..=12u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }
    let live = run_id(99);
    let run = open(root.path(), &live, roomy());
    let summary = run.summary();

    let remaining = run_directories(root.path());
    assert_eq!(remaining.len(), 10, "{remaining:?}");
    assert!(remaining.contains(&live), "the live run is the tenth");
    for n in 4..=12u128 {
        assert!(
            remaining.contains(&run_id(n)),
            "run {n} is one of the newest"
        );
    }
    for n in 1..=3u128 {
        assert!(!remaining.contains(&run_id(n)), "run {n} was the oldest");
        assert!(!run_dir(root.path(), &run_id(n)).exists());
    }
    assert!(
        summary
            .warnings
            .iter()
            .all(|warning| !matches!(warning, TraceWarning::Residue { .. })),
        "clean seeds leave no residue: {:?}",
        summary.warnings,
    );
}

/// Every candidate retention is not entitled to delete survives AND is
/// reported. This is one test because it is one law: uncertainty means keep.
#[test]
fn every_uncertain_candidate_survives_and_is_reported() {
    let root = project();
    let trace = root.path().join(".vibe").join("trace");
    std::fs::create_dir_all(&trace).unwrap();

    // Enough clean seeds that retention genuinely wants to delete something.
    for n in 1..=12u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }

    // A sibling whose name is not a run id at all.
    std::fs::create_dir_all(trace.join("not-a-run-id")).unwrap();
    std::fs::write(trace.join("README.txt"), b"hands off").unwrap();

    // A trace that is still running.
    let running = run_id(101);
    seed_index_at(
        root.path(),
        &running,
        &seeded_index(root.path(), &running, 900, RunStatus::Running),
    );

    // A malformed index.
    let malformed = run_id(102);
    let directory = run_dir(root.path(), &malformed);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("index.json"), b"{ not json").unwrap();

    // An owned, terminal, validator-green run carrying one extra file.
    let extra = run_id(103);
    let directory = seed_index_at(
        root.path(),
        &extra,
        &seeded_index(root.path(), &extra, 800, RunStatus::Ok),
    );
    std::fs::write(directory.join("stray.json"), b"{}").unwrap();

    // An index that names a different run than the directory holding it.
    let mislabelled = run_id(104);
    let mut index = seeded_index(root.path(), &mislabelled, 700, RunStatus::Ok);
    index.run_id = run_id(105);
    seed_index_at(root.path(), &mislabelled, &index);

    // A link-like candidate, where the host lets one be made.
    let linked = run_id(106);
    let linkable = link_dir(&run_dir(root.path(), &run_id(12)), &trace.join(&linked));

    let live = run_id(200);
    let run = open(root.path(), &live, roomy());
    let warnings = run.summary().warnings;
    let reported = |name: &str| {
        warnings.iter().any(|warning| match warning {
            TraceWarning::Residue { path, .. } => path.ends_with(name),
            _ => false,
        })
    };

    for survivor in [&running, &malformed, &extra, &mislabelled] {
        assert!(
            run_dir(root.path(), survivor).exists(),
            "{survivor} must survive",
        );
        assert!(
            reported(survivor),
            "{survivor} must be reported: {warnings:?}"
        );
    }
    assert!(trace.join("not-a-run-id").exists());
    assert!(trace.join("README.txt").exists());
    assert!(reported("not-a-run-id"), "{warnings:?}");
    assert!(reported("README.txt"), "{warnings:?}");
    assert!(
        std::fs::read(run_dir(root.path(), &extra).join("stray.json")).unwrap() == b"{}",
        "the extra file is untouched",
    );
    if linkable {
        assert!(trace.join(&linked).exists(), "the link survives");
        assert!(reported(&linked), "{warnings:?}");
    }
    assert!(run_dir(root.path(), &live).exists());
}

/// Retention deletes the run's exact file set and nothing else — proved by
/// planting a real completed run with real snapshots and watching the whole
/// directory go, while a neighbour of the same age stays.
#[test]
fn a_collected_run_loses_exactly_its_own_files() {
    let root = project();
    // One real run, compiled and finished, so its snapshots are genuine.
    let doomed = run_id(1);
    let first = open(root.path(), &doomed, roomy());
    let scope = first.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    first.finish(&super::super::RunOutcome::Ok, at(1_100));
    // A finished run is released before the next one opens — the pinned
    // capability is what keeps its directory alive on Windows, and the real
    // lifecycle holds exactly one run at a time.
    drop(scope);
    drop(first);
    let directory = run_dir(root.path(), &doomed);
    assert!(entries(&directory).len() > 1, "it really wrote snapshots");

    let keeper = run_id(2);
    seed_run(root.path(), &keeper, 1_200);

    // Keep exactly one older run: the doomed one is the older of the two.
    let live = run_id(3);
    open(root.path(), &live, TraceLimits::for_test(u64::MAX, 1));

    assert!(!directory.exists(), "the whole run directory is gone");
    assert!(
        run_dir(root.path(), &keeper).exists(),
        "the newer one stays"
    );
    assert_eq!(run_directories(root.path()), vec![keeper, live]);
}

/// Retention never reaches outside `.vibe/trace`, whatever else is in the
/// project.
#[test]
fn retention_never_touches_anything_outside_the_trace_directory() {
    let root = project();
    std::fs::create_dir_all(root.path().join(".vibe").join("cache")).unwrap();
    std::fs::write(root.path().join(".vibe").join("cache").join("a"), b"keep").unwrap();
    std::fs::write(root.path().join("vibe.toml"), b"keep").unwrap();
    for n in 1..=12u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }

    let live = run_id(50);
    let run = open(root.path(), &live, roomy());
    assert_eq!(
        read_index(&run_dir(root.path(), &live)).status,
        RunStatus::Running
    );
    drop(run);

    assert_eq!(
        std::fs::read(root.path().join(".vibe").join("cache").join("a")).unwrap(),
        b"keep",
    );
    assert_eq!(
        std::fs::read(root.path().join("vibe.toml")).unwrap(),
        b"keep"
    );
}

/// Reopening an EXISTING run does not sweep: retention is what happens before
/// a fresh directory is created, and a resumed run is not that.
#[test]
fn reopening_a_run_does_not_retire_anything() {
    let root = project();
    for n in 1..=12u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }
    let live = run_id(60);
    let first = open(root.path(), &live, roomy());
    drop(first);
    let before = run_directories(root.path());

    let resumed = super::super::TraceRun::open_with_limits(root.path(), &live, at(1_000), roomy())
        .expect("the running trace reopens");
    assert_eq!(run_directories(root.path()), before, "no second sweep");
    assert!(resumed.summary().warnings.is_empty());
}

#[cfg(unix)]
fn link_dir(target: &std::path::Path, link: &std::path::Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_dir(target: &std::path::Path, link: &std::path::Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
}

/// **The file swap.** A run is judged deletable, and its `index.json` is
/// rebound to a different ordinary file before the removal runs. Deletion is
/// identity-bound, so it refuses: the replacement survives byte-for-byte, the
/// run directory stays, and the sweep reports residue.
#[test]
fn a_snapshot_swapped_after_eligibility_is_never_deleted() {
    let root = project();
    let doomed = run_id(1);
    seed_run(root.path(), &doomed, 1_000);
    for n in 2..=11u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }
    let target = run_dir(root.path(), &doomed).join("index.json");

    let planted = target.clone();
    vibe_safefs::arm_before_proved_removal(Some(Box::new(move |_, _| {
        std::fs::remove_file(&planted).unwrap();
        std::fs::write(&planted, b"SOMEBODY ELSE'S FILE").unwrap();
    })));
    let live = run_id(99);
    let run = open(root.path(), &live, roomy());
    vibe_safefs::arm_before_proved_removal(None);

    assert_eq!(
        std::fs::read(&target).unwrap(),
        b"SOMEBODY ELSE'S FILE",
        "the swapped-in file survives exactly",
    );
    assert!(
        run_dir(root.path(), &doomed).exists(),
        "and its directory with it",
    );
    let warnings = run.summary().warnings;
    assert!(
        warnings.iter().any(|warning| match warning {
            TraceWarning::Residue { path, .. } => path.ends_with(&doomed),
            _ => false,
        }),
        "the sweep reports what it refused: {warnings:?}",
    );
}

/// **The directory swap.** The run directory itself is rebound after its files
/// are gone and before the directory removal. Identity — not emptiness —
/// refuses, and the replacement stays in place with its contents.
#[test]
fn a_run_directory_swapped_after_eligibility_is_never_deleted() {
    let root = project();
    let doomed = run_id(1);
    seed_run(root.path(), &doomed, 1_000);
    for n in 2..=11u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }
    let directory = run_dir(root.path(), &doomed);

    // The FIRST proved removal is the index file; the second is the directory.
    // Arming re-arms itself once so the swap lands in the directory window.
    let planted = directory.clone();
    let armed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&armed);
    vibe_safefs::arm_before_proved_removal(Some(Box::new(move |_, _| {
        if counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            let inner = planted.clone();
            let again = std::sync::Arc::clone(&counter);
            vibe_safefs::arm_before_proved_removal(Some(Box::new(move |_, _| {
                again.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                std::fs::remove_dir_all(&inner).unwrap();
                std::fs::create_dir(&inner).unwrap();
                std::fs::write(inner.join("someone-elses.txt"), b"keep").unwrap();
            })));
        }
    })));
    let live = run_id(99);
    let run = open(root.path(), &live, roomy());
    vibe_safefs::arm_before_proved_removal(None);

    assert!(directory.is_dir(), "the replacement directory is in place");
    assert_eq!(
        std::fs::read(directory.join("someone-elses.txt")).unwrap(),
        b"keep",
        "with its own contents untouched",
    );
    let warnings = run.summary().warnings;
    assert!(
        warnings.iter().any(|warning| match warning {
            TraceWarning::Residue { path, .. } => path.ends_with(&doomed),
            _ => false,
        }),
        "the sweep reports what it refused: {warnings:?}",
    );
}

/// A validator-green, terminal, self-naming run directory that belongs to a
/// DIFFERENT project. Every other obligation is discharged; ownership is the
/// only thing missing, and it is enough to keep it.
#[test]
fn another_projects_completed_run_survives_and_is_reported() {
    let root = project();
    for n in 1..=11u128 {
        seed_run(root.path(), &run_id(n), 1_000 + i64::try_from(n).unwrap());
    }
    let foreign = run_id(77);
    let mut index = seeded_index(root.path(), &foreign, 500, RunStatus::Ok);
    index.project = super::support::foreign_identity();
    let directory = seed_index_at(root.path(), &foreign, &index);

    let live = run_id(99);
    let run = open(root.path(), &live, roomy());

    assert!(
        directory.exists(),
        "another project's run is not ours to collect"
    );
    assert!(
        std::fs::read(directory.join("index.json")).is_ok(),
        "and its index is untouched",
    );
    let warnings = run.summary().warnings;
    assert!(
        warnings.iter().any(|warning| match warning {
            TraceWarning::Residue { path, reason } =>
                path.ends_with(&foreign) && reason.contains("different project"),
            _ => false,
        }),
        "the refusal names ownership: {warnings:?}",
    );
    // The oldest OWNED run was still collected, so the sweep really ran.
    assert!(!run_dir(root.path(), &run_id(1)).exists());
}
