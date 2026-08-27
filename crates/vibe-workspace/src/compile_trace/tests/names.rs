//! Filenames under pressure: the canonical short form, the writer's clamp,
//! and the depth at which the run refuses to open at all.

use vibe_wire::behaviour::compiler_trace_index::{SNAPSHOT_NAME_CAP, SnapshotName};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    ArtifactTarget, PassStatus, ScopeKind,
};

use super::super::{RunOutcome, ScopeDescriptor, TraceOpenError, TraceRun};
use super::support::{RUN_A, World, at, compile_ok, open, project, read_index, roomy, run_dir};

/// A label that is long AND non-ASCII has no full canonical spelling, so
/// every snapshot of that scope takes the digest form — still ASCII, still
/// under the cap, and still exactly what the validator reconstructs.
#[test]
fn a_long_unicode_label_takes_the_canonical_short_name() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let label = "узел/☃/очень-длинная-метка-которая-никуда-не-влезает";
    let scope = run
        .declare_scope(&ScopeDescriptor {
            id: "node:long".to_string(),
            kind: ScopeKind::Node,
            label: label.to_string(),
            artifact: "static-md".to_string(),
            target: ArtifactTarget::StaticMd,
        })
        .unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    let index = read_index(&directory);
    let mut seen = 0;
    for event in &index.events {
        let Some(name) = &event.snapshot else {
            continue;
        };
        assert_eq!(event.status, PassStatus::Ok);
        assert!(name.len() <= SNAPSHOT_NAME_CAP, "{name}");
        assert!(name.is_ascii(), "the codec emits ASCII only: {name}");
        let expected = SnapshotName {
            sequence: event.sequence,
            invocation: event.invocation,
            kind: &ScopeKind::Node,
            pass: &event.pass,
            label,
            artifact: "static-md",
        };
        assert_eq!(expected.full(), None, "no full form could fit");
        assert_eq!(*name, expected.short(), "so the digest form is the name");
        assert!(directory.join(name).is_file());
        seen += 1;
    }
    assert!(seen >= 2);
}

/// The pass name is percent-encoded too, so `emit:static-md` cannot smuggle a
/// colon — a byte Windows refuses in a filename — into a path component.
#[test]
fn a_colon_bearing_pass_name_never_reaches_the_filesystem() {
    let root = project();
    let directory = run_dir(root.path(), RUN_A);
    let run = open(root.path(), RUN_A, roomy());
    let scope = run
        .declare_scope(&super::support::node_scope("node:.", "."))
        .unwrap();
    compile_ok(&scope, &World::two_documents());
    scope.complete("fp").unwrap();
    run.finish(&RunOutcome::Ok, at(2_000));

    let index = read_index(&directory);
    let emit = index
        .events
        .iter()
        .find(|event| event.pass.starts_with("emit:"))
        .expect("the schedule really does run an emit pass");
    let name = emit.snapshot.as_deref().expect("it certified a carrier");
    assert!(name.contains("emit%3A"), "the colon is escaped: {name}");
    assert!(
        !name.contains(':') && name.is_ascii(),
        "and never lands raw: {name}",
    );
    assert!(directory.join(name).is_file());
}

/// A run directory deep enough that no canonical name could fit beside it
/// refuses to OPEN, rather than opening and failing every event. The floor is
/// named in the refusal, so the operator can see how much room is missing.
#[test]
fn a_run_directory_too_deep_for_any_name_refuses_to_open() {
    let root = project();
    // `.vibe/trace/<32 hex>` already costs 45 units; this pushes the whole
    // absolute run directory past the point where 32 units remain.
    let mut deep = root.path().to_path_buf();
    while deep.as_os_str().len() < 210 {
        deep.push("a-fairly-long-directory-component");
    }
    std::fs::create_dir_all(&deep).expect("a deep but legal directory");

    let error = TraceRun::open_with_limits(&deep, RUN_A, at(1_000), roomy())
        .expect_err("a directory with no room for a filename refuses");
    let TraceOpenError::RunDirectoryTooDeep {
        remaining, floor, ..
    } = &error
    else {
        panic!("expected the depth refusal, got {error}");
    };
    assert!(remaining < floor, "{error}");
    assert_eq!(*floor, 32);
    assert!(
        !run_dir(&deep, RUN_A).exists(),
        "and nothing was created before refusing",
    );
}

/// The two identity gates on the way in: a relative root and a run id that is
/// not the epoch's exact spelling.
#[test]
fn the_root_and_the_run_id_are_gated_before_anything_is_created() {
    let root = project();
    assert!(matches!(
        TraceRun::open_with_limits(std::path::Path::new("relative/root"), RUN_A, at(1), roomy()),
        Err(TraceOpenError::RelativeRoot { .. })
    ));
    for bad in [
        "",
        "0123456789ABCDEF0123456789ABCDEF",
        "0123456789abcdef0123456789abcde",
        "0123456789abcdef0123456789abcdeff",
        "0123456789abcdef0123456789abcdeg",
        "../../etc",
    ] {
        let outcome = TraceRun::open_with_limits(root.path(), bad, at(1), roomy());
        assert!(
            matches!(outcome, Err(TraceOpenError::RunId { .. })),
            "{bad:?} is not a run id",
        );
    }
    assert!(
        !root.path().join(".vibe").exists(),
        "a refused identity creates nothing",
    );
}
