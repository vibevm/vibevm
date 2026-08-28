//! The prerequisite install's post-durability stage runs EXACTLY once, and its
//! captured tree is what the later phases plan against.
//!
//! `calls` was incremented and never read, and the single reader was
//! `collector.workspace.unwrap_or(&prelude_workspace)` — which planned the
//! remaining phases against the PRE-install world whenever the stage was
//! skipped: a `--git` delta and every freshly materialised slot would simply be
//! missing from the world the later phases collect, and nothing would say so.
//!
//! The mutation these kill is reinstating that fallback. With it, the zero-call
//! case below returns the prelude tree happily; with the law, it refuses. The
//! field being private to the parent module is the other half of the proof: the
//! call site cannot express `unwrap_or` at all any more.

use super::PrerequisiteInstall;

fn workspace() -> vibe_workspace::Workspace {
    let dir = tempfile::tempdir().expect("a temp project");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("the fixture is written");
    let root = crate::install::resolve_project_root(dir.path()).expect("a canonical root");
    let workspace = vibe_workspace::Workspace::discover(&root).expect("the project loads");
    // The TempDir is deliberately leaked: the Workspace value is what this
    // test uses, and it holds no handle keeping the directory alive.
    std::mem::forget(dir);
    workspace
}

/// An Install chain whose stage never ran is an internal error, not a
/// fallback to the pre-install world.
#[test]
fn zero_calls_on_an_install_chain_is_an_internal_error() {
    let prelude = workspace();
    let collector = PrerequisiteInstall::default();
    let error = collector
        .planning_workspace(&prelude, true)
        .expect_err("a skipped stage can never be papered over");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("ran 0 time(s)"), "{rendered}");
    assert!(rendered.contains("internal"), "{rendered}");
}

/// Two calls are equally wrong: the tree the later phases plan against
/// would be whichever call happened to land last.
#[test]
fn two_calls_on_an_install_chain_is_an_internal_error() {
    let prelude = workspace();
    let mut collector = PrerequisiteInstall {
        calls: 2,
        workspace: Some(workspace()),
        ..PrerequisiteInstall::default()
    };
    let error = collector
        .planning_workspace(&prelude, true)
        .expect_err("a doubled stage can never be accepted");
    assert!(format!("{error:#}").contains("ran 2 time(s)"), "{error:#}");
    collector.calls = 1;
    assert!(
        collector.planning_workspace(&prelude, true).is_ok(),
        "and exactly one call with a captured tree is the accepted shape",
    );
}

/// One call WITHOUT a captured tree is refused too — the count alone is
/// not the invariant, the tree is.
#[test]
fn one_call_without_a_captured_workspace_is_an_internal_error() {
    let prelude = workspace();
    let collector = PrerequisiteInstall {
        calls: 1,
        ..PrerequisiteInstall::default()
    };
    let error = collector
        .planning_workspace(&prelude, true)
        .expect_err("a stage that reported no workspace is a defect");
    assert!(
        format!("{error:#}").contains("reported no workspace"),
        "{error:#}",
    );
}

/// A chain with NO install phase legitimately never runs the stage, and
/// only then may the prelude load be the world. A call on such a chain is
/// itself the error.
#[test]
fn a_chain_without_install_uses_the_prelude_and_must_not_have_called() {
    let prelude = workspace();
    let quiet = PrerequisiteInstall::default();
    let used = quiet
        .planning_workspace(&prelude, false)
        .expect("no install phase, no stage, prelude world");
    assert_eq!(used.root, prelude.root, "the prelude tree, unchanged");

    let noisy = PrerequisiteInstall {
        calls: 1,
        workspace: Some(workspace()),
        ..PrerequisiteInstall::default()
    };
    assert!(
        noisy.planning_workspace(&prelude, false).is_err(),
        "a stage that ran on a chain with no install phase is a defect",
    );
}
