//! The run's own arc: terminal words, silent scopes, the byte budget, and
//! the guarded reopen.

use vibe_wire::generated::compiler_trace_index::e1::index::{
    PassStatus, RunStatus, ScopeStatus, Timestamp,
};

use super::super::{RunOutcome, TraceLimits, TraceOpenError, TraceRun, TraceWarning};
use super::support::{
    RUN_A, World, at, compile, compile_ok, entries, node_scope, open, project, read_index, roomy,
    run_dir, seed_index_at, seeded_index,
};

/// A failed run keeps every snapshot it already certified. The failure came
/// later — that is exactly the `StaticWrite` rollback case — so the events
/// stay green and only the root says `failed`.
#[test]
fn a_failed_root_retains_the_snapshots_it_already_wrote() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();

    let summary = run.finish(
        &RunOutcome::Failed("the boot transaction rolled back".to_string()),
        at(2_000),
    );
    assert!(summary.finalised);

    let index = read_index(&directory);
    assert_eq!(index.status, RunStatus::Failed);
    assert_eq!(
        index.failure.as_deref(),
        Some("the boot transaction rolled back")
    );
    let named: Vec<&str> = index
        .events
        .iter()
        .filter_map(|event| event.snapshot.as_deref())
        .collect();
    assert!(!named.is_empty(), "the successful passes kept their files");
    for name in named {
        assert!(
            directory.join(name).is_file(),
            "{name} survives the failure"
        );
    }
    assert!(
        index
            .events
            .iter()
            .all(|event| event.status == PassStatus::Ok),
        "a late failure does not retroactively fail a pass",
    );
}

/// A pass that really fails gets a timing row and NO certified snapshot, the
/// scope is `failed`, and the root refuses to call that run `ok`.
#[test]
fn a_failed_pass_produces_a_timing_row_and_no_snapshot() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();

    assert!(
        compile(&scope, &World::dangling_use()).is_none(),
        "the dangling `#use` really fails the compile",
    );
    scope.fail("the `#use` target is missing").unwrap();
    let summary = run.finish(
        &RunOutcome::Failed("the artifact did not compile".to_string()),
        at(2_000),
    );
    assert!(summary.finalised);

    let index = read_index(&directory);
    assert_eq!(index.status, RunStatus::Failed);
    assert_eq!(index.scopes[0].status, ScopeStatus::Failed);
    assert!(index.scopes[0].fingerprint.is_none());
    let failed = index
        .events
        .iter()
        .find(|event| event.status == PassStatus::PassFailed)
        .expect("a real pass failure is recorded");
    assert!(failed.snapshot.is_none(), "a failure certifies nothing");
    assert!(failed.pass_micros.is_some(), "the body it ran is timed");
    assert!(failed.diagnostic.is_some());
}

/// A caller that claims `ok` while a scope it declared is still pending has
/// asked for an index the epoch refuses. Nothing is written: the on-disk
/// index stays `running`, which is the honest description of the state.
#[test]
fn an_unfinished_scope_refuses_a_green_terminal_word() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let _scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();

    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    assert!(!summary.finalised);
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::NotFinalised { .. })),
        "{:?}",
        summary.warnings,
    );
    let index = read_index(&directory);
    assert_eq!(index.status, RunStatus::Running);
    assert!(index.finished.is_none());
}

/// A skipped scope is silent by law. It is legal before any event, and
/// refused once the scope has recorded one — the writer never has to hope the
/// caller got the order right.
#[test]
fn a_skipped_scope_carries_zero_events() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());

    let fresh = run
        .declare_scope(&node_scope("node:fresh", "fresh"))
        .unwrap();
    fresh.skip("already-fresh").unwrap();

    let busy = run.declare_scope(&node_scope("node:busy", "busy")).unwrap();
    compile_ok(&busy, &World::two_documents());
    let error = busy.skip("too late").unwrap_err();
    assert!(
        matches!(error, super::super::TraceError::SkipAfterEvents { .. }),
        "{error}"
    );
    busy.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    let index = read_index(&directory);
    let skipped = index
        .scopes
        .iter()
        .find(|scope| scope.id == "node:fresh")
        .unwrap();
    assert_eq!(skipped.status, ScopeStatus::Skipped);
    assert_eq!(skipped.fingerprint.as_deref(), Some("already-fresh"));
    assert!(
        !index.events.iter().any(|event| event.scope == "node:fresh"),
        "a skipped scope names no event",
    );
    assert_eq!(index.status, RunStatus::Ok, "and the run is still green");
}

/// The byte budget: the run publishes until the ceiling is REACHED, then
/// stands down. Dense timings continue, no further file appears, and the root
/// still validates as `ok` — a diagnostic that ran out of room never turns a
/// green run red.
#[test]
fn a_spent_budget_stands_down_without_losing_a_single_timing() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    // One byte: the first accepted output crosses the ceiling atomically and
    // every later one stands down.
    let run = open(root.path(), RUN_A, TraceLimits::for_test(1, 9));
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    assert!(summary.budget_exhausted);
    assert_eq!(summary.snapshots, 1, "exactly the crossing event");

    let index = read_index(&directory);
    assert_eq!(index.status, RunStatus::Ok);
    assert_eq!(index.events[0].status, PassStatus::Ok);
    let skipped: Vec<_> = index
        .events
        .iter()
        .skip(1)
        .map(|event| event.status.clone())
        .collect();
    assert!(
        skipped
            .iter()
            .all(|status| *status == PassStatus::SnapshotSkippedBudget),
        "every later event stands down: {skipped:?}",
    );
    for event in index.events.iter().skip(1) {
        assert!(event.pass_micros.is_some(), "the pass is still timed");
        assert!(event.verify_micros.is_some(), "so is the verification");
        assert!(
            event.encode_micros.is_none(),
            "but no encode clock ever started",
        );
        assert!(event.snapshot.is_none());
    }
    assert_eq!(
        entries(&directory),
        vec![
            "0000-parse-node_._static%2Dmd-000.json".to_string(),
            "index.json".to_string()
        ],
        "no later file exists at all",
    );
    // The aggregate table still reconciles across the stand-down.
    assert_eq!(
        index
            .aggregates
            .iter()
            .map(|row| row.invocations)
            .sum::<u32>(),
        u32::try_from(index.events.len()).unwrap(),
    );
}

/// Reopen restores the counters EXACTLY: the sequence continues, the
/// per-`(scope, pass)` ordinals continue, and the spent budget is recovered
/// from the files that are really there.
#[test]
fn reopening_a_running_trace_restores_every_counter() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let first = open(root.path(), RUN_A, roomy());
    let scope = first.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    let before = read_index(&directory);
    let spent_before = first.summary().snapshot_bytes;
    // Both the run AND its scope hold the project lock: the writer is
    // serialized for as long as any clone survives, so a reopen is only
    // possible once the last one is gone.
    drop(scope);
    drop(first);

    let resumed = TraceRun::open_with_limits(root.path(), RUN_A, at(1_000), roomy())
        .expect("a running trace of this run reopens");
    assert_eq!(
        resumed.summary().snapshot_bytes,
        spent_before,
        "the spent budget is recovered from the files on disk",
    );
    // The identical pending scope is reacquired, never redeclared.
    let scope = resumed
        .declare_scope(&node_scope("node:.", "."))
        .expect("an identical pending descriptor is reacquirable");
    assert_eq!(read_index(&directory).scopes.len(), 1);

    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    resumed.finish(&RunOutcome::Ok, at(2_000));

    let after = read_index(&directory);
    assert!(after.events.len() > before.events.len());
    assert_eq!(
        after
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        (0..u32::try_from(after.events.len()).unwrap()).collect::<Vec<_>>(),
        "the sequence continued rather than restarting",
    );
    let ordinals: Vec<u32> = after
        .events
        .iter()
        .filter(|event| event.pass == "parse")
        .map(|event| event.invocation)
        .collect();
    assert_eq!(ordinals, vec![0, 1, 2, 3], "and so did the ordinals");
}

/// A pending scope may be reacquired only by the EXACT descriptor, and a
/// scope that already reached a terminal status is never silently reset.
#[test]
fn a_reopened_scope_is_reacquired_only_by_its_own_identity() {
    let root = project();
    let first = open(root.path(), RUN_A, roomy());
    let scope = first.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    drop(scope);
    drop(first);

    let resumed = TraceRun::open_with_limits(root.path(), RUN_A, at(1_000), roomy()).unwrap();
    let conflict = resumed
        .declare_scope(&node_scope("node:.", "a different label"))
        .expect_err("a different identity under a taken id refuses");
    assert!(
        matches!(conflict, super::super::TraceError::ScopeConflict { .. }),
        "{conflict}"
    );
    resumed
        .declare_scope(&node_scope("node:.", "."))
        .unwrap()
        .complete("fp")
        .unwrap();
    let resolved = resumed
        .declare_scope(&node_scope("node:.", "."))
        .expect_err("a compiled scope id is not reusable");
    assert!(
        matches!(
            resolved,
            super::super::TraceError::ScopeAlreadyResolved { .. }
        ),
        "{resolved}"
    );
}

/// Every shape of residue a reopen can meet, and the refusal each one earns.
/// None of them is repaired, deleted, or guessed at.
#[test]
fn residue_refuses_a_reopen_instead_of_being_guessed_at() {
    // A terminal run is not something to continue.
    expect_residue(|root, id| {
        seed_index_at(root, id, &seeded_index(root, id, 1_000, RunStatus::Ok));
    });
    // A run that started at a different moment is a different run.
    expect_residue(|root, id| {
        seed_index_at(root, id, &seeded_index(root, id, 999, RunStatus::Running));
    });
    // An index that names a different run id than the directory holding it.
    expect_residue(|root, id| {
        let mut index = seeded_index(root, id, 1_000, RunStatus::Running);
        index.run_id = "fedcba9876543210fedcba9876543210".to_string();
        assert_ne!(index.run_id, id, "the plant must really differ");
        seed_index_at(root, id, &index);
    });
    // A directory with no index at all.
    expect_residue(|root, id| {
        std::fs::create_dir_all(run_dir(root, id)).unwrap();
    });
    // A malformed index.
    expect_residue(|root, id| {
        let directory = run_dir(root, id);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("index.json"), b"{ not json").unwrap();
    });
    // An extra entry nobody's index references.
    expect_residue(|root, id| {
        let directory = seed_index_at(root, id, &seeded_index(root, id, 1_000, RunStatus::Running));
        std::fs::write(directory.join("stray.json"), b"{}").unwrap();
    });
    // A referenced snapshot that is not there — the crash-between-writes case.
    expect_residue(|root, id| {
        let mut index = seeded_index(root, id, 1_000, RunStatus::Running);
        index.scopes.push(pending_scope());
        index.events.push(orphan_event());
        index.aggregates =
            vibe_wire::behaviour::compiler_trace_index::build_aggregates(&index.events).unwrap();
        seed_index_at(root, id, &index);
    });
}

/// The digest pins WHICH tree was compiled: an index written for another root
/// is residue here, however well formed it is.
#[test]
fn an_index_from_another_project_root_refuses() {
    let root = project();
    let other = project();
    let first = open(other.path(), RUN_A, roomy());
    first.declare_scope(&node_scope("node:.", ".")).unwrap();
    drop(first);

    let planted = run_dir(root.path(), RUN_A);
    std::fs::create_dir_all(&planted).unwrap();
    std::fs::copy(
        run_dir(other.path(), RUN_A).join("index.json"),
        planted.join("index.json"),
    )
    .unwrap();

    let error = TraceRun::open_with_limits(root.path(), RUN_A, at(1_000), roomy())
        .expect_err("another root's index is not this run");
    assert!(matches!(error, TraceOpenError::Residue { .. }), "{error}");
}

fn pending_scope() -> vibe_wire::generated::compiler_trace_index::e1::index::Scope {
    use vibe_wire::generated::compiler_trace_index::e1::index::{ArtifactTarget, Scope, ScopeKind};
    Scope {
        artifact: "static-md".to_string(),
        id: "node:.".to_string(),
        kind: ScopeKind::Node,
        label: ".".to_string(),
        status: ScopeStatus::Pending,
        target: ArtifactTarget::StaticMd,
        failure: None,
        fingerprint: None,
    }
}

/// An `ok` event naming a snapshot file that was never written — exactly what
/// a crash between the two independently atomic writes leaves behind.
fn orphan_event() -> vibe_wire::generated::compiler_trace_index::e1::index::PassEvent {
    use vibe_wire::generated::compiler_trace_index::e1::index::{
        Duration, IrCardinality, IrLevel, PassEvent, PassShape,
    };
    let micros = || {
        Some(Duration {
            micros: 1,
            saturated: false,
        })
    };
    PassEvent {
        input_shape: PassShape {
            cardinality: IrCardinality::Document,
            level: IrLevel::Source,
        },
        invocation: 0,
        output_shape: PassShape {
            cardinality: IrCardinality::Document,
            level: IrLevel::Document,
        },
        pass: "parse".to_string(),
        scope: "node:.".to_string(),
        sequence: 0,
        status: PassStatus::Ok,
        diagnostic: None,
        encode_micros: micros(),
        pass_micros: micros(),
        snapshot: Some("0000-parse-node_._static%2Dmd-000.json".to_string()),
        verify_micros: micros(),
    }
}

/// Plant one shape of residue under a fresh root and prove the reopen refuses
/// it, leaves it exactly as it was, and creates nothing beside it.
fn expect_residue(plant: impl Fn(&std::path::Path, &str)) {
    let root = project();
    plant(root.path(), RUN_A);
    let directory = run_dir(root.path(), RUN_A);
    let before = entries(&directory);

    let error = TraceRun::open_with_limits(root.path(), RUN_A, at(1_000), roomy())
        .expect_err("residue refuses a reopen");
    assert!(matches!(error, TraceOpenError::Residue { .. }), "{error}");
    assert_eq!(entries(&directory), before, "the residue is untouched");
}

/// The clock is injected, never read: the same fixture instants appear
/// verbatim in the index, and nothing here consults the host's wall clock.
#[test]
fn every_timestamp_in_the_index_was_injected() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = TraceRun::open_with_limits(root.path(), RUN_A, at(11), roomy()).unwrap();
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(22));

    let index = read_index(&directory);
    assert_eq!(index.started, at(11));
    assert_eq!(index.finished, Some(at(22)));
    assert_eq!(
        index.finished.unwrap() - index.started,
        Timestamp::from_timestamp(22, 0).unwrap() - Timestamp::from_timestamp(11, 0).unwrap(),
    );
}
