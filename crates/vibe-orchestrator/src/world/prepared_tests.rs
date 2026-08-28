//! Reds for the prepared world-collection seam.
//!
//! The mutation each of these kills is the same one: reintroducing a
//! `Workspace::discover` or a selected `Manifest::read` inside world planning.
//! So every test here corrupts or deletes the selected manifest AFTER the
//! workspace was prepared, and requires the prepared value to be the one used.

use std::fs;

use vibe_lifecycle::Phase;
use vibe_workspace::Workspace;

use super::{plan_default, plan_default_prepared};
use crate::install::resolve_project_root;

/// A project whose `phase:build` plans one builtin contribution.
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [[extension]]\nid = 'first'\npoint = 'phase:build'\n\
         handler = { kind = \"builtin\", name = \"log\" }\n\
         config = { message = \"ROW-ONE\" }\n",
    )
    .unwrap();
    dir
}

/// The prepared seam plans from the workspace it was HANDED, not from disk.
///
/// The manifest is corrupted between preparation and planning, so the
/// compatibility wrapper — which discovers — fails, while the prepared seam
/// produces the same plan and the same host identity it would have before.
#[test]
fn the_prepared_seam_plans_from_the_workspace_it_was_given() {
    let dir = project();
    let selected = resolve_project_root(dir.path()).unwrap();
    let workspace = Workspace::discover(&selected).expect("the project loads");

    let before = plan_default_prepared(&selected, &workspace, &[Phase::Build])
        .expect("a prepared plan before any mutation");

    fs::write(selected.join("vibe.toml"), "[project\nbroken\n").unwrap();
    assert!(
        plan_default(&selected, &[Phase::Build]).is_err(),
        "the compatibility wrapper really does re-read, and the file really is broken",
    );

    let after = plan_default_prepared(&selected, &workspace, &[Phase::Build])
        .expect("the prepared seam does not re-read");
    assert_eq!(
        after.count_for(Phase::Build),
        before.count_for(Phase::Build),
        "the same plan, from the same prepared value",
    );
    assert_eq!(
        after.project.root, before.project.root,
        "and the same host identity",
    );
    assert_eq!(after.workspace_root, workspace.root);
}

/// A DELETED selected manifest is the sharper version of the same claim: there
/// is nothing on disk to fall back to, so a plan can only come from the
/// prepared value.
#[test]
fn a_deleted_selected_manifest_does_not_stop_prepared_planning() {
    let dir = project();
    let selected = resolve_project_root(dir.path()).unwrap();
    let workspace = Workspace::discover(&selected).expect("the project loads");

    fs::remove_file(selected.join("vibe.toml")).unwrap();
    let plan = plan_default_prepared(&selected, &workspace, &[Phase::Build])
        .expect("the prepared workspace still describes the node");
    assert_eq!(plan.count_for(Phase::Build), 1);
}

/// An in-memory edit to the prepared workspace reaches the plan.
///
/// This is the `--git` case in miniature: the command mutated its own copy of
/// the selected node and never wrote that shape to disk in a form planning
/// could re-read. If planning rediscovered, the edit would vanish.
#[test]
fn an_in_memory_edit_to_the_prepared_workspace_reaches_the_plan() {
    let dir = project();
    let selected = resolve_project_root(dir.path()).unwrap();
    let mut workspace = Workspace::discover(&selected).expect("the project loads");

    // Drop the declaration in memory only; disk still has it.
    workspace.root_manifest.extensions.clear();
    let plan = plan_default_prepared(&selected, &workspace, &[Phase::Build])
        .expect("the prepared workspace plans");
    assert_eq!(
        plan.count_for(Phase::Build),
        0,
        "planning saw the in-memory value, not the one still on disk",
    );

    // And the disk copy is untouched, so the compatibility wrapper still sees
    // the original — which is what makes the line above a real difference.
    assert_eq!(
        plan_default(&selected, &[Phase::Build])
            .expect("disk is fine")
            .count_for(Phase::Build),
        1,
    );
}
