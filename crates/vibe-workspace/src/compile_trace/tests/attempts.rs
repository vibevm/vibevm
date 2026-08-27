//! Reds for the attempt grammar's acquisition law (R3.4): pending reacquire,
//! terminal minting, conflicts refused, and the observer-shaped adapters.

use vibe_wire::generated::compiler_trace_index::e1::index::{
    CompilerTraceIndex, RunStatus, ScopeStatus,
};

use super::super::{RunOutcome, TraceError};
use super::support::{
    RUN_A, World, at, compile_ok, node_scope, open, project, read_index, roomy, run_dir, unit_scope,
};

/// The latest attempt number a base has already spent in `index`, for the
/// assertions that read the run back off disk.
fn latest_attempt(index: &CompilerTraceIndex, base: &str) -> u32 {
    index
        .scopes
        .iter()
        .filter_map(|scope| {
            scope
                .id
                .strip_prefix(&format!("{base}::attempt:"))
                .and_then(|n| n.parse().ok())
        })
        .max()
        .unwrap_or(0)
}

/// After a crash (or any interruption) the latest still-`pending` attempt is
/// reacquired EXACTLY: same id, same identity, one scope in the index.
#[test]
fn a_pending_occurrence_is_reacquired_exactly() {
    let root = project();
    let run = open(root.path(), RUN_A, roomy());
    let base = node_scope("node:.", ".");
    let first = run
        .acquire_scope(&base)
        .expect("the first attempt allocates");
    assert_eq!(first.id(), "node:.::attempt:1");
    let second = run
        .acquire_scope(&base)
        .expect("the pending attempt reacquires");
    assert_eq!(
        second.id(),
        first.id(),
        "a pending occurrence is continued, never forked"
    );
    let index = read_index(&run_dir(root.path(), RUN_A));
    assert_eq!(index.scopes.len(), 1, "no sibling was minted");
    assert_eq!(index.scopes[0].status, ScopeStatus::Pending);
}

/// A pending attempt wearing a different identity refuses — a conflict is
/// never silently redefined — and the lossy adapter turns that refusal into
/// `None` plus a retained warning, so the caller compiles untraced.
#[test]
fn a_pending_descriptor_conflict_refuses_and_warns_through_the_adapter() {
    let root = project();
    let run = open(root.path(), RUN_A, roomy());
    let base = node_scope("node:.", ".");
    // Plant the base's first attempt id under a DIFFERENT label, exactly as a
    // torn writer from another grammar would have left it.
    let mut foreign = base.clone();
    foreign.id = "node:.::attempt:1".to_string();
    foreign.label = "someone-else".to_string();
    run.declare_scope(&foreign)
        .expect("the exact API plants it");

    let error = run.acquire_scope(&base).expect_err("a conflict refuses");
    assert!(matches!(error, TraceError::ScopeConflict { .. }), "{error}");
    assert!(run.acquire_scope_lossy(&base).is_none(), "lossy is None");
    let summary = run.summary();
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| format!("{warning}").contains("could not be declared")),
        "the refusal is retained as a warning: {:?}",
        summary.warnings
    );
    // And the planted scope is untouched — the allocator neither redefined it
    // nor minted a sibling around it.
    let index = read_index(&run_dir(root.path(), RUN_A));
    assert_eq!(index.scopes.len(), 1);
    assert_eq!(index.scopes[0].label, "someone-else");
}

/// After a terminal attempt the NEXT positive attempt is minted
/// deterministically — once, twice, and through a skip alike.
#[test]
fn a_terminal_attempt_mints_the_next_positive_attempt() {
    let root = project();
    let run = open(root.path(), RUN_A, roomy());
    let base = node_scope("node:.", ".");

    let first = run.acquire_scope(&base).unwrap();
    let artifact = compile_ok(&first, &World::two_documents());
    first.complete_lossy(&artifact.output_fingerprint());
    let second = run.acquire_scope(&base).unwrap();
    assert_eq!(second.id(), "node:.::attempt:2");
    second.skip_lossy(&artifact.output_fingerprint()); // fresh: same output, zero events
    let third = run.acquire_scope(&base).unwrap();
    assert_eq!(third.id(), "node:.::attempt:3");

    let index = read_index(&run_dir(root.path(), RUN_A));
    assert_eq!(latest_attempt(&index, "node:."), 3);
    assert_eq!(index.scopes[0].status, ScopeStatus::Compiled);
    assert_eq!(index.scopes[1].status, ScopeStatus::Skipped);
    assert_eq!(index.scopes[2].status, ScopeStatus::Pending);
    // The skipped occurrence is silent; the compiled one carries the events.
    assert!(index.events.iter().all(|event| event.scope == *first.id()));
}

/// After a TERMINAL attempt the next number is minted under the CURRENT
/// descriptor: a unit base deliberately survives a version (label) change
/// inside one adopted run, so an update is the next attempt of the same
/// artifact — not a conflict, not a warning, and not a new base.
#[test]
fn a_terminal_attempt_evolves_to_the_current_descriptor() {
    let root = project();
    let run = open(root.path(), RUN_A, roomy());
    let base = "unit:org.x/y#static-md";

    let one = unit_scope(base, "org.x/y@1.0");
    let first = run
        .acquire_scope(&one)
        .expect("the first attempt allocates");
    assert_eq!(first.id(), "unit:org.x/y#static-md::attempt:1");
    let artifact = compile_ok(&first, &World::two_documents());
    first
        .complete(&artifact.output_fingerprint())
        .expect("the first attempt completes");

    // The SAME base at a new version — a different display label, and the
    // exact-identity rule deliberately does not apply to a terminal attempt.
    let two = unit_scope(base, "org.x/y@2.0");
    let second = run
        .acquire_scope(&two)
        .expect("a version change after a terminal attempt is not a conflict");
    assert_eq!(second.id(), "unit:org.x/y#static-md::attempt:2");

    let index = read_index(&run_dir(root.path(), RUN_A));
    assert_eq!(index.scopes.len(), 2);
    assert_eq!(
        index.scopes[0].label, "org.x/y@1.0",
        "history keeps the label it compiled under"
    );
    assert_eq!(
        index.scopes[1].label, "org.x/y@2.0",
        "the new attempt wears the descriptor that is compiling NOW"
    );
    assert!(
        run.summary().warnings.is_empty(),
        "an evolved descriptor is not a fault: {:?}",
        run.summary().warnings
    );
}

/// The packet's headline contrast: the SAME base compiling twice in one run
/// appends a new occurrence and continues the one dense global sequence —
/// where the exact single-id API would have refused with
/// `ScopeAlreadyResolved`.
#[test]
fn a_second_compilation_appends_an_occurrence_and_sequence() {
    let root = project();
    let run = open(root.path(), RUN_A, roomy());
    let base = node_scope("node:.", ".");

    let first = run.acquire_scope(&base).unwrap();
    let artifact = compile_ok(&first, &World::two_documents());
    first.complete_lossy(&artifact.output_fingerprint());
    let after_first = read_index(&run_dir(root.path(), RUN_A)).events.len();

    // The exact API on the base id itself is the old law: one id, one life.
    let exact = run.declare_scope(&base).expect("the bare base id is free");
    exact
        .complete(&artifact.output_fingerprint())
        .expect("the bare occurrence completes");
    assert!(
        matches!(
            run.declare_scope(&base),
            Err(TraceError::ScopeAlreadyResolved { .. })
        ),
        "the exact API still refuses a redeclaration — attempts exist for this"
    );

    let second = run.acquire_scope(&base).unwrap();
    assert_eq!(second.id(), "node:.::attempt:2");
    let again = compile_ok(&second, &World::two_documents());
    second.complete_lossy(&again.output_fingerprint());

    let index = read_index(&run_dir(root.path(), RUN_A));
    assert_eq!(index.scopes.len(), 3, "attempt:1, the bare id, attempt:2");
    assert_eq!(index.scopes[2].id, "node:.::attempt:2");
    // One dense global sequence across both compilations, snapshots included.
    for (position, event) in index.events.iter().enumerate() {
        assert_eq!(event.sequence, position as u32);
    }
    assert!(index.events.len() > after_first);
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    assert_eq!(summary.status, RunStatus::Ok);
    assert!(summary.finalised, "a fully terminal run finalises");
}
