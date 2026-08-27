//! The reds the dual-freeze repair added: durable finalisation, exact scope
//! identity, the closed sink, one canonical root, bounded hostile text, and a
//! budget counted in bytes that are really on disk.

use vibe_wire::behaviour::compiler_trace_index::{
    DIAGNOSTIC_CAP_BYTES, SCALAR_PREVIEW_BYTES, SnapshotName,
};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    PassStatus, RunStatus, ScopeKind, ScopeStatus,
};

use super::super::{
    RunOutcome, ScopeDescriptor, TraceError, TraceLimits, TraceOpenError, TraceRun, TraceWarning,
};
use super::support::{
    RUN_A, World, at, compile_ok, entries, node_scope, open, project, read_index, roomy, run_dir,
};

/// A hostile scalar: multi-megabyte and non-ASCII, so a byte cut that ignored
/// character boundaries would produce invalid UTF-8 rather than a short
/// string.
fn hostile() -> String {
    // Eight bytes per repeat: a three-byte snowman, a two-byte accented
    // letter and a three-byte bidi override.
    "☃é\u{202e}".repeat(600_000)
}

// ---------------------------------------------------------------- finding 2

/// `finalised` means the terminal bytes are DURABLE, not that a terminal word
/// was asked for.
///
/// The index destination is blocked for the whole `finish` call — a second
/// hard link makes it a file the publication contract will not replace — and
/// the fault is never disarmed before `finish` runs. The run must report
/// `finalised == false`, restore its in-memory root to `running`, and leave a
/// disk index that still reads as a valid `running` trace.
#[test]
fn a_blocked_terminal_write_is_not_a_finalised_run() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();

    let second_name = directory.join("second-name-of-the-index");
    if std::fs::hard_link(directory.join("index.json"), &second_name).is_err() {
        // Without hard links there is no portable way to block a write for a
        // whole call; an injected post-publication fault would land the very
        // bytes it claims to refuse, which is the confusion this red exists
        // to rule out.
        return;
    }

    // Armed for the ENTIRE finish call, and never disarmed before it.
    let summary = run.finish(&RunOutcome::Ok, at(2_000));

    assert!(
        !summary.finalised,
        "bytes that never landed are not a finalised run",
    );
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| matches!(warning, TraceWarning::NotFinalised { .. })),
        "{:?}",
        summary.warnings,
    );
    assert_eq!(
        summary.status,
        RunStatus::Running,
        "the in-memory root was restored, so a retry is still possible",
    );

    let on_disk = read_index(&directory);
    assert_eq!(
        on_disk.status,
        RunStatus::Running,
        "a cold reader sees exactly what durably happened",
    );
    assert!(on_disk.finished.is_none());
    assert!(
        !on_disk.events.is_empty(),
        "and the last whole index that DID land is still there",
    );

    // Unblocked, the very same run finalises — so the refusal was the write,
    // not a permanently broken state.
    std::fs::remove_file(&second_name).unwrap();
    let retry = run.finish(&RunOutcome::Ok, at(3_000));
    assert!(retry.finalised);
    assert_eq!(read_index(&directory).status, RunStatus::Ok);
}

// --------------------------------------------------------------- finding 3a

/// A fingerprint is an IDENTITY, not a diagnostic: it is stored byte-for-byte
/// however long it is. Silently shortening one would mint a different, still
/// well-formed fingerprint that names nothing the compiler produced.
#[test]
fn a_long_safe_fingerprint_is_stored_exactly() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();

    // Far past the diagnostic cap, and non-ASCII, so any clamp at all would
    // show up as a different string.
    let exact = format!("sha256:{}", "☃9".repeat(DIAGNOSTIC_CAP_BYTES));
    scope.complete(&exact).expect("a safe identity is accepted");
    run.finish(&RunOutcome::Ok, at(2_000));

    let index = read_index(&directory);
    assert_eq!(
        index.scopes[0].fingerprint.as_deref(),
        Some(exact.as_str()),
        "the identity survived byte-for-byte",
    );
    assert!(exact.len() > DIAGNOSTIC_CAP_BYTES);
}

/// The scalar the epoch refuses is refused here too — through the validator
/// itself, on the whole index, with the scope rolled back to `pending` rather
/// than silently repaired into something valid.
#[test]
fn an_unsafe_fingerprint_is_refused_rather_than_repaired() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();

    for unsafe_scalar in ["", "   ", "line\nbreak", "carriage\rreturn", "nul\0byte"] {
        let error = scope
            .complete(unsafe_scalar)
            .expect_err("the epoch's scalar gate refuses this identity");
        assert!(
            matches!(error, TraceError::IndexRefused { .. }),
            "{unsafe_scalar:?}: {error}"
        );
    }
    let index = read_index(&directory);
    assert_eq!(
        index.scopes[0].status,
        ScopeStatus::Pending,
        "a refused transition leaves the scope exactly where it was",
    );
    assert!(index.scopes[0].fingerprint.is_none());

    // And a good one still works afterwards.
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));
    assert_eq!(
        read_index(&directory).scopes[0].status,
        ScopeStatus::Compiled
    );
}

// --------------------------------------------------------------- finding 3b

/// A sink outlives the scope it was taken from. Once that scope reached its
/// terminal word, a REAL traced compile through the same sink must change
/// nothing at all — no file, no event, no counter — whichever terminal word
/// it was.
#[test]
fn a_closed_scope_records_nothing_from_a_real_compile() {
    for (label, close) in [("compiled", 0usize), ("failed", 1), ("skipped", 2)] {
        let root = project();
        let directory = run_dir(root.path(), RUN_A);
        let run = open(root.path(), RUN_A, roomy());
        let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
        match close {
            0 => scope.complete("fp").unwrap(),
            1 => scope.fail("it did not compile").unwrap(),
            _ => scope.skip("already fresh").unwrap(),
        }
        let before_entries = entries(&directory);
        let before_index = read_index(&directory);

        // The real built-in schedule, through the very same sink.
        compile_ok(&scope, &World::two_documents());

        let after = read_index(&directory);
        assert_eq!(after.events.len(), 0, "{label}: no event was recorded");
        assert_eq!(after.events, before_index.events, "{label}");
        assert_eq!(after.aggregates, before_index.aggregates, "{label}");
        assert_eq!(
            entries(&directory),
            before_entries,
            "{label}: no snapshot was published",
        );

        let summary = run.summary();
        assert_eq!(summary.snapshot_bytes, 0, "{label}: nothing was charged");
        assert!(
            summary
                .warnings
                .iter()
                .any(|warning| matches!(warning, TraceWarning::Dropped { .. })),
            "{label}: the drop is reported: {:?}",
            summary.warnings,
        );

        // The run still terminates honestly on the word the scope already has.
        let outcome = if close == 1 {
            RunOutcome::Failed("the artifact did not compile".to_string())
        } else {
            RunOutcome::Ok
        };
        assert!(run.finish(&outcome, at(2_000)).finalised, "{label}");
    }
}

// ---------------------------------------------------------------- finding 4

/// One root spelled several ways is ONE project. The digest, the run
/// directory and the reopen verdict must not depend on which spelling the
/// caller happened to hold.
#[test]
fn aliases_of_one_root_are_one_project() {
    let root = project();
    std::fs::create_dir_all(root.path().join("members").join("tool")).unwrap();
    let detour = root
        .path()
        .join("members")
        .join("tool")
        .join("..")
        .join("..");

    let first = open(root.path(), RUN_A, roomy());
    let canonical_dir = first.run_dir().to_path_buf();
    let scope = first.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    let digest = read_index(&canonical_dir).project.root_digest.clone();
    drop(scope);
    drop(first);

    // The same project through a `..` detour: same digest, and the run
    // REOPENS rather than being refused as somebody else's.
    let resumed = TraceRun::open_with_limits(&detour, RUN_A, at(1_000), roomy())
        .expect("a detoured spelling is the same project");
    assert_eq!(
        resumed.run_dir(),
        canonical_dir,
        "one canonical run directory, whichever spelling opened it",
    );
    assert_eq!(read_index(&canonical_dir).project.root_digest, digest);

    let scope = resumed
        .declare_scope(&node_scope("node:.", "."))
        .expect("the pending scope is reacquired, not conflicted");
    scope.complete("fp").unwrap();
    resumed.finish(&RunOutcome::Ok, at(2_000));
    assert_eq!(read_index(&canonical_dir).status, RunStatus::Ok);
    assert_eq!(
        super::support::run_directories(root.path()).len(),
        1,
        "an alias never mints a second run directory",
    );
}

/// A root that cannot be resolved at all is an open failure — the caller
/// compiles untraced — and it is reported before anything is created.
#[test]
fn an_unresolvable_root_refuses_before_creating_anything() {
    let root = project();
    let missing = root.path().join("no-such-tree");
    let error = TraceRun::open_with_limits(&missing, RUN_A, at(1_000), roomy())
        .expect_err("a root that is not there cannot be canonicalised");
    assert!(matches!(error, TraceOpenError::Directory { .. }), "{error}");
    assert!(!missing.exists());
}

// ---------------------------------------------------------------- finding 5

/// Every retained text is bounded at the wire epoch's own cap, marker
/// included — and the index the run writes still validates with a
/// multi-megabyte hostile scalar in play.
#[test]
fn hostile_text_never_escapes_the_wire_caps() {
    let root = project();
    let huge = hostile();
    assert!(huge.len() > 4 * 1024 * 1024);

    // An untrusted IDENTITY inside an open refusal takes the preview cap.
    let error = TraceRun::open_with_limits(root.path(), &huge, at(1), roomy())
        .expect_err("a 4 MiB run id is not a run id");
    let TraceOpenError::RunId { run_id } = &error else {
        panic!("expected the run-id refusal, got {error}");
    };
    assert!(run_id.len() <= SCALAR_PREVIEW_BYTES, "{}", run_id.len());

    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());

    // A hostile scope id inside a scope refusal takes the preview cap too.
    let scope = run
        .declare_scope(&ScopeDescriptor {
            id: huge.clone(),
            kind: ScopeKind::Node,
            label: huge.clone(),
            artifact: "static-md".to_string(),
            target: vibe_wire::generated::compiler_trace_index::e1::index::ArtifactTarget::StaticMd,
        })
        .expect("a huge but safe identity is legal");
    let refusal = scope
        .complete("bad\nfingerprint")
        .expect_err("an unsafe scalar refuses");
    let TraceError::IndexRefused { reason } = &refusal else {
        panic!("expected the index refusal, got {refusal}");
    };
    assert!(reason.len() <= DIAGNOSTIC_CAP_BYTES, "{}", reason.len());

    // A hostile scope id quoted in a dropped-event warning.
    scope.complete("fp").unwrap();
    compile_ok(&scope, &World::two_documents());

    // A hostile ROOT FAILURE is a diagnostic and is bounded into the index
    // itself — which must still validate.
    let summary = run.finish(&RunOutcome::Failed(huge.clone()), at(2_000));
    assert!(summary.finalised);
    for warning in &summary.warnings {
        for field in warning_fields(warning) {
            assert!(field.len() <= DIAGNOSTIC_CAP_BYTES, "{}", field.len());
        }
    }
    let index = read_index(&directory);
    let failure = index.failure.as_deref().expect("a failed run says why");
    assert!(failure.len() <= DIAGNOSTIC_CAP_BYTES, "{}", failure.len());
    assert!(
        failure.chars().next().is_some_and(|c| c == '☃'),
        "and it is still the real text, just shorter",
    );
    assert_eq!(
        index.scopes[0].label, huge,
        "a LABEL is identity and is never shortened",
    );
}

/// Both string fields of every warning shape, so a new arm cannot quietly
/// escape the cap.
fn warning_fields(warning: &TraceWarning) -> Vec<&str> {
    match warning {
        TraceWarning::Residue { path, reason } => vec![path.as_str(), reason.as_str()],
        TraceWarning::IndexWrite { reason }
        | TraceWarning::IndexAnomaly { reason }
        | TraceWarning::Dropped { reason }
        | TraceWarning::NotFinalised { reason } => vec![reason.as_str()],
        TraceWarning::Snapshot { reason, .. } => vec![reason.as_str()],
    }
}

// ---------------------------------------------------------------- finding 6

/// The budget counts BYTES THAT ARE ON DISK, not index references.
///
/// A publication that fails after the irreversible step still leaves a real
/// file. It is charged and its name reserved even though no event will ever
/// name it — so the stand-down happens, and repeated faults cannot put more
/// than the budget on disk.
#[test]
fn a_file_that_landed_without_being_named_is_still_charged() {
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

    vibe_safefs::fail_after_publish(Some(&orphan));
    compile_ok(&scope, &World::two_documents());
    vibe_safefs::fail_after_publish(None);
    scope.complete("fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));

    // The file really is on disk, and no event names it.
    let landed = directory.join(&orphan);
    assert!(landed.is_file(), "the failed publication still landed");
    let index = read_index(&directory);
    assert!(
        index.events.iter().all(|event| event.snapshot.is_none()),
        "no event claims a file",
    );
    assert_eq!(index.events[0].status, PassStatus::SnapshotFailed);
    assert_eq!(index.status, RunStatus::Ok, "and the run is still green");

    // It is charged, reported, and it exhausted the tiny budget — so nothing
    // else was ever published.
    let on_disk = std::fs::metadata(&landed).unwrap().len();
    assert_eq!(
        summary.snapshot_bytes, on_disk,
        "the landed bytes are charged"
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
    assert_eq!(
        entries(&directory),
        vec![orphan, "index.json".to_string()],
        "exactly one payload reached the disk",
    );
}

/// The tiny-budget stand-down, counted against the FILESYSTEM rather than
/// against what the index says.
#[test]
fn the_budget_is_measured_against_the_files_that_exist() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, TraceLimits::for_test(1, 9));
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));

    let payloads: Vec<String> = entries(&directory)
        .into_iter()
        .filter(|name| name != "index.json")
        .collect();
    assert_eq!(
        payloads.len(),
        1,
        "one file crossed the ceiling: {payloads:?}"
    );
    let bytes: u64 = payloads
        .iter()
        .map(|name| std::fs::metadata(directory.join(name)).unwrap().len())
        .sum();
    assert_eq!(
        summary.snapshot_bytes, bytes,
        "the counter is the sum of what is really there",
    );
    assert!(summary.budget_exhausted);
    assert_eq!(summary.snapshots, 1, "and exactly one event named a file");
}
