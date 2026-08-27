//! What one real traced compile leaves on disk, and what it refuses to
//! leave.

use vibe_wire::generated::compiler_trace_index::e1::index::{PassStatus, RunStatus, ScopeStatus};

use super::super::{RunOutcome, TraceWarning};
use super::support::{
    RUN_A, World, at, compile_ok, entries, node_scope, open, project, read_index, roomy, run_dir,
    unit_scope,
};

/// The index is readable the moment the run exists, after every single event,
/// and after the terminal word — and every one of those states passes the
/// epoch's own validator.
#[test]
fn the_running_index_is_readable_at_every_moment_of_the_run() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());

    // Immediately: an empty running index, not an absence.
    let fresh = read_index(&directory);
    assert_eq!(fresh.status, RunStatus::Running);
    assert_eq!(fresh.run_id, RUN_A);
    assert_eq!(fresh.project.display, ".");
    assert!(fresh.scopes.is_empty() && fresh.events.is_empty());
    assert!(fresh.finished.is_none());

    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    let declared = read_index(&directory);
    assert_eq!(declared.scopes.len(), 1);
    assert_eq!(declared.scopes[0].status, ScopeStatus::Pending);
    assert!(declared.scopes[0].fingerprint.is_none());

    compile_ok(&scope, &World::two_documents());
    let mid = read_index(&directory);
    assert!(!mid.events.is_empty());
    assert_eq!(mid.status, RunStatus::Running, "the terminal word is last");

    scope.complete("fp-node").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    assert!(summary.finalised);
    assert!(summary.warnings.is_empty(), "{:?}", summary.warnings);

    let done = read_index(&directory);
    assert_eq!(done.status, RunStatus::Ok);
    assert_eq!(done.finished, Some(at(2_000)));
    assert_eq!(done.scopes[0].status, ScopeStatus::Compiled);
    assert_eq!(done.scopes[0].fingerprint.as_deref(), Some("fp-node"));
    assert_eq!(done.events.len(), mid.events.len(), "finish adds no event");
}

/// Two addressed documents mean two `parse` invocations, which mean two
/// distinct files and dense ordinals `0` and `1` — never one overwritten
/// `NN-parse.json`.
#[test]
fn two_documents_produce_two_parse_files_with_dense_ordinals() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    let index = read_index(&directory);
    let parses: Vec<_> = index
        .events
        .iter()
        .filter(|event| event.pass == "parse")
        .collect();
    assert_eq!(parses.len(), 2, "one parse per addressed document");
    assert_eq!(
        parses
            .iter()
            .map(|event| event.invocation)
            .collect::<Vec<_>>(),
        vec![0, 1],
    );
    let files: Vec<&str> = parses
        .iter()
        .filter_map(|event| event.snapshot.as_deref())
        .collect();
    assert_eq!(files.len(), 2);
    assert_ne!(files[0], files[1], "two invocations, two names");
    for name in &files {
        assert!(
            directory.join(name).is_file(),
            "the index names a file that landed: {name}"
        );
    }
}

/// Two artifact scopes share ONE global sequence: the numbering is dense
/// across the whole run, and no two events collide on a filename.
#[test]
fn two_scopes_share_one_dense_global_sequence() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());

    let node = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    let unit = run
        .declare_scope(&unit_scope("unit:org.demo.tool", "org.demo.tool"))
        .unwrap();
    compile_ok(&node, &World::two_documents());
    compile_ok(&unit, &World::two_documents());
    node.complete("fp-node").unwrap();
    unit.complete("fp-unit").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    let index = read_index(&directory);
    assert_eq!(
        index
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        (0..u32::try_from(index.events.len()).unwrap()).collect::<Vec<_>>(),
        "one dense sequence, not two",
    );
    // Each scope restarts its own `(scope, pass)` ordinals at zero.
    for id in ["node:.", "unit:org.demo.tool"] {
        let ordinals: Vec<u32> = index
            .events
            .iter()
            .filter(|event| event.scope == id && event.pass == "parse")
            .map(|event| event.invocation)
            .collect();
        assert_eq!(ordinals, vec![0, 1], "{id}");
    }
    let mut names: Vec<&str> = index
        .events
        .iter()
        .filter_map(|event| event.snapshot.as_deref())
        .collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total, "no two events claim one filename");
    assert_eq!(entries(&directory).len(), total + 1, "plus the index");
}

/// A snapshot is the ONE strict compiler-IR wire, not a trace-shaped copy of
/// it: every published file parses through the generated strict reader, and
/// the bytes on disk are exactly the bytes the sink was handed.
#[test]
fn every_snapshot_is_the_strict_compiler_ir_wire() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    let index = read_index(&directory);
    let mut checked = 0;
    for event in &index.events {
        let Some(name) = &event.snapshot else {
            continue;
        };
        assert_eq!(event.status, PassStatus::Ok, "only `ok` certifies a file");
        let bytes = std::fs::read(directory.join(name)).expect("the named file is readable");
        serde_json::from_slice::<vibe_wire::generated::compiler_ir::e1::ir::Ir>(&bytes)
            .unwrap_or_else(|error| panic!("`{name}` is not strict compiler IR: {error}"));
        checked += 1;
    }
    assert!(checked >= 2, "the fixture certifies several carriers");
}

/// An accepted output whose destination is already taken is NEVER overwritten,
/// however the collision was planted: the event honestly becomes
/// `snapshot-failed`, the occupant survives byte-for-byte, and the compile is
/// unchanged.
#[test]
fn an_occupied_snapshot_name_is_never_overwritten() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();

    // The name the very first accepted output will choose, occupied first.
    let taken = "0000-parse-node_._static%2Dmd-000.json";
    std::fs::write(directory.join(taken), b"planted").unwrap();

    let emitted = compile_ok(&scope, &World::two_documents());
    assert!(!emitted.bytes().is_empty(), "the compile is untouched");
    scope.complete("fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));

    assert_eq!(
        std::fs::read(directory.join(taken)).unwrap(),
        b"planted",
        "the occupant is exactly as it was",
    );
    let index = read_index(&directory);
    assert_eq!(index.status, RunStatus::Ok, "root `ok` admits it");
    let first = &index.events[0];
    assert_eq!(first.status, PassStatus::SnapshotFailed);
    assert!(first.snapshot.is_none(), "no filename is claimed");
    assert!(first.diagnostic.is_some(), "and the reason is recorded");
    assert!(
        first.encode_micros.is_some(),
        "the encode the compiler already spent is still reported",
    );
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::Snapshot { sequence: 0, .. })),
        "{:?}",
        summary.warnings,
    );
}

/// The same law when the occupant is a DIRECTORY rather than a file — the
/// create-new publication refuses every occupant, not just the replaceable
/// ones.
#[test]
fn a_directory_in_a_snapshot_name_is_never_replaced() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    let taken = "0000-parse-node_._static%2Dmd-000.json";
    std::fs::create_dir(directory.join(taken)).unwrap();

    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    assert!(directory.join(taken).is_dir());
    let index = read_index(&directory);
    assert_eq!(index.events[0].status, PassStatus::SnapshotFailed);
    assert_eq!(index.status, RunStatus::Ok);
}

/// A hard link is another name for one file. The publication refuses it for
/// the same reason it refuses a plain file, and both names survive.
#[test]
fn a_hard_linked_snapshot_name_is_never_written_through() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    let taken = "0000-parse-node_._static%2Dmd-000.json";
    std::fs::write(directory.join("original"), b"shared").unwrap();
    if std::fs::hard_link(directory.join("original"), directory.join(taken)).is_err() {
        return;
    }

    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    assert_eq!(
        std::fs::read(directory.join("original")).unwrap(),
        b"shared"
    );
    assert_eq!(std::fs::read(directory.join(taken)).unwrap(), b"shared");
    let index = read_index(&directory);
    assert_eq!(index.events[0].status, PassStatus::SnapshotFailed);
}

/// A fault injected BEFORE the publication leaves nothing at that name at
/// all, and the event says so honestly.
#[test]
fn an_injected_pre_publication_fault_becomes_snapshot_failed() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    let target = "0000-parse-node_._static%2Dmd-000.json";

    vibe_safefs::fail_before_publish(Some(target));
    let emitted = compile_ok(&scope, &World::two_documents());
    vibe_safefs::fail_before_publish(None);

    assert!(!emitted.bytes().is_empty(), "the artifact is unchanged");
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    assert!(!directory.join(target).exists(), "nothing was published");
    let index = read_index(&directory);
    assert_eq!(index.events[0].status, PassStatus::SnapshotFailed);
    assert!(index.events[0].snapshot.is_none());
    assert_eq!(index.status, RunStatus::Ok);
}

/// A fault injected AFTER the publication is the harder case: the file IS
/// there. The event still refuses to claim it, because a filename in the
/// index is a promise the writer verified — and the unclaimed file is named
/// as residue rather than silently kept.
#[test]
fn an_injected_post_publication_fault_never_claims_the_file() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    let target = "0000-parse-node_._static%2Dmd-000.json";

    vibe_safefs::fail_after_publish(Some(target));
    compile_ok(&scope, &World::two_documents());
    vibe_safefs::fail_after_publish(None);

    scope.complete("fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));

    assert!(directory.join(target).is_file(), "the file really landed");
    let index = read_index(&directory);
    assert_eq!(index.events[0].status, PassStatus::SnapshotFailed);
    assert!(index.events[0].snapshot.is_none());
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::Snapshot { sequence: 0, .. })),
        "{:?}",
        summary.warnings,
    );
}

/// An index update that cannot land is a WARNING, not a lost run: the
/// in-memory model keeps going and the next update writes everything,
/// including the events whose own write failed.
///
/// The refusal is a real one rather than an injected one. A second hard link
/// to `index.json` makes it a file the publication contract will not replace
/// — it is no longer exclusively owned — so every update genuinely fails
/// before publication while the stale index stays exactly as it was.
#[test]
fn a_failed_index_update_is_retried_by_the_next_one() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();

    let second_name = directory.join("second-name-of-the-index");
    if std::fs::hard_link(directory.join("index.json"), &second_name).is_err() {
        // Some filesystems refuse hard links; there is then no portable way
        // to make a write fail without injecting one, and an injected
        // post-publication fault would land the file it claims to refuse.
        return;
    }
    let before = read_index(&directory).events.len();
    compile_ok(&scope, &World::two_documents());

    let stale = read_index(&directory);
    assert_eq!(stale.events.len(), before, "no update landed while blocked");

    std::fs::remove_file(&second_name).unwrap();
    scope.complete("fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    assert!(summary.finalised);
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::IndexWrite { .. })),
        "{:?}",
        summary.warnings,
    );

    let recovered = read_index(&directory);
    assert_eq!(recovered.status, RunStatus::Ok);
    assert!(
        recovered.events.len() > before,
        "the retry carries every event the armed writes lost",
    );
    assert_eq!(
        recovered
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        (0..u32::try_from(recovered.events.len()).unwrap()).collect::<Vec<_>>(),
        "and the sequence never rewound",
    );
}

/// An event for a scope nobody declared is dropped with a reason rather than
/// invented into the index.
#[test]
fn an_undeclared_scope_records_nothing() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    scope.complete("fp").unwrap();

    // The scope is resolved, so a late event has no pending home; the
    // stronger case — a sink whose scope was never declared at all — cannot
    // be built through the public API, which is the point.
    let error = scope.complete("again").unwrap_err();
    assert!(
        matches!(error, super::super::TraceError::ScopeAlreadyResolved { .. }),
        "{error}"
    );
    run.finish(&RunOutcome::Ok, at(2_000));
    assert_eq!(read_index(&directory).scopes.len(), 1);
}
