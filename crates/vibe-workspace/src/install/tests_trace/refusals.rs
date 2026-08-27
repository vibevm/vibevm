//! Everything that must NOT produce a recorded compile — and the error
//! identities that must survive the observer unchanged.

use std::cell::RefCell;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageKind};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    ArtifactTarget, ScopeKind, ScopeStatus,
};

use super::super::test_helpers::*;
use super::super::*;
use super::support::*;
use crate::compile_trace::ScopeDescriptor;

/// RED 5 — a compiler failure marks the scope `failed`, preserves the exact
/// historical workspace error identity, and touches no boot file.
#[test]
fn a_compiler_failure_fails_the_scope_and_preserves_the_error() {
    fn fixture() -> (TempDir, Workspace, Vec<ResolvedDep>, TempDir) {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n\
             [requires.packages]\n\
             \"org.vibevm/broken\" = { version = \"^1.0\", link = \"static\" }\n",
        );
        write(ws_dir.path(), boot_rel("00-core.md"), "# core");
        // Pre-seed the root boot lane: a failed compile must not touch it.
        fs::write(ws_dir.path().join(boot_rel("INDEX.md")), b"OLD-INDEX").unwrap();
        fs::write(ws_dir.path().join(boot_rel("STATIC.md")), b"OLD-STATIC").unwrap();

        // A `normal`-format package whose boot contract dangles a `#use` at
        // an uninstalled package: the real compiler fails inside the closure
        // walk. Built by hand because the shared helper hardcodes the
        // `[package]` table and this one needs `format = "normal"` in it.
        let src = TempDir::new().unwrap();
        write(
            src.path(),
            "vibe.toml",
            "[package]\ngroup = \"org.vibevm\"\nname = \"broken\"\nkind = \"flow\"\n\
             version = \"1.0.0\"\nformat = \"normal\"\n\n\
             [boot_snippet]\nsource = \"vibevm/vibespecs/boot/broken.md\"\n",
        );
        write(
            src.path(),
            "vibevm/vibespecs/boot/broken.md",
            "# Broken {#root}\n#use spec://org.vibevm/ghost/boot/base#root\nBROKEN\n",
        );
        let broken = ResolvedDep {
            kind: PackageKind::Flow,
            group: Group::parse("org.vibevm").unwrap(),
            name: "broken".to_string(),
            version: ver("1.0.0"),
            content_dir: src.path().to_path_buf(),
            source_hash: Some(source_hash()),
            manifest: Manifest::read(src.path().join("vibe.toml")).unwrap(),
            requires: vec![],
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        };
        let ws = Workspace::load(ws_dir.path()).unwrap();
        (ws_dir, ws, vec![broken], src)
    }
    let (_plain_dir, plain_ws, plain_resolution, plain_src) = fixture();
    let plain_error = apply_traced(&plain_ws, &plain_resolution, None).unwrap_err();

    let (ws_dir, ws, resolution, src) = fixture();
    let run = traced_run(&ws.root);
    let traced_error = apply_traced(&ws, &resolution, Some(&run)).unwrap_err();
    drop((plain_src, src));

    let WorkspaceError::InlineCompile { .. } = &traced_error else {
        panic!("expected InlineCompile, got {traced_error}")
    };
    let WorkspaceError::InlineCompile { .. } = &plain_error else {
        panic!("expected InlineCompile untraced, got {plain_error}")
    };
    assert_eq!(
        format!("{traced_error:#}"),
        format!("{plain_error:#}"),
        "the error identity is exactly the untraced one"
    );
    assert_eq!(
        fs::read(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap(),
        b"OLD-INDEX"
    );
    assert_eq!(
        fs::read(ws_dir.path().join(boot_rel("STATIC.md"))).unwrap(),
        b"OLD-STATIC"
    );
    let index = run_index(&ws.root);
    let node = index
        .scopes
        .iter()
        .find(|scope| scope.id == "node:.#static-md::attempt:1")
        .expect("the root node declared");
    assert_eq!(node.status, ScopeStatus::Failed);
    assert!(
        node.failure.is_some(),
        "the failure diagnostic is bounded in"
    );
    assert!(
        index.events.iter().any(|event| event.scope == node.id),
        "the failed compile recorded its real pass events"
    );
}

/// RED 6 — a workspace with no static artifact anywhere creates no scope.
#[test]
fn a_workspace_with_no_static_artifact_creates_no_scope() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n\
         [requires.packages]\n\"org.vibevm/child\" = \"^1.0\"\n",
    );
    let (child, src) = dep_with_boot(
        "child",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/child.md\"",
        "boot/child.md",
        "# child boot",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();
    let run = traced_run(&ws.root);
    apply_traced(&ws, &[child], Some(&run)).expect("a dynamic-only install succeeds");
    drop(src);

    let index = run_index(&ws.root);
    assert!(index.scopes.is_empty(), "no static artifact, no scope");
    assert!(index.events.is_empty(), "no compile, no events");
}

/// RED 7 — the old untraced wrappers preserve bytes and mtimes and leave no
/// trace-specific side effect: no run directory, no `.vibe` at all.
#[test]
fn the_old_wrappers_write_no_trace_and_preserve_freshness() {
    let (ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    apply_resolution_with_spec_format(
        &ws,
        &resolution,
        SlotIntegrity::TrustPresence,
        SpecFormat::Mixed,
        None,
        None,
    )
    .expect("the untraced install succeeds");
    assert!(
        !ws_dir.path().join(".vibe").exists(),
        "off mode allocates no recorder, no lock, no run directory"
    );
    let static_md = unit_static(ws_dir.path(), "parent");
    let first = fs::read_to_string(&static_md).unwrap();
    let before = fs::metadata(&static_md).unwrap().modified().unwrap();

    // A second untraced pass keeps the fresh unit untouched.
    apply_resolution_with_spec_format(
        &ws,
        &resolution,
        SlotIntegrity::TrustPresence,
        SpecFormat::Mixed,
        None,
        None,
    )
    .expect("the second untraced install succeeds");
    assert_eq!(fs::read_to_string(&static_md).unwrap(), first);
    assert_eq!(
        fs::metadata(&static_md).unwrap().modified().unwrap(),
        before,
        "the wrapper's fresh path churns no mtime"
    );
    // And the bytes are what the traced path also produces (same compiler).
    let (traced_dir, traced_ws, traced_resolution, _s) = unit_and_root_fixture();
    let run = traced_run(&traced_ws.root);
    apply_traced(&traced_ws, &traced_resolution, Some(&run)).unwrap();
    assert_eq!(
        fs::read_to_string(unit_static(traced_dir.path(), "parent")).unwrap(),
        first,
        "tracing changes no artifact bytes"
    );
}

/// RED 8a — a pre-boot failure (here: materialisation refusing a fetched-set
/// gap) still prevents boot compilation: zero scopes, original error.
#[test]
fn a_materialise_boundary_failure_prevents_any_boot_compilation() {
    let (_plain_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let mut resolution = resolution;
    resolution[0].source_hash = None; // the materialiser refuses before boot
    let plain = apply_traced(&ws, &resolution, None).unwrap_err();

    let (dir2, ws2, resolution2, _s2) = unit_and_root_fixture();
    let mut resolution2 = resolution2;
    resolution2[0].source_hash = None;
    let run = traced_run(&ws2.root);
    let traced = apply_traced(&ws2, &resolution2, Some(&run)).unwrap_err();

    // The error embeds the dep's absolute temp content dir; normalise that
    // one spelling away and the identities must be the same string.
    let strip = |error: &WorkspaceError, dir: &Path| {
        format!("{error:#}").replace(&dir.display().to_string(), "<src>")
    };
    assert_eq!(
        strip(&traced, &resolution2[0].content_dir),
        strip(&plain, &resolution[0].content_dir)
    );
    let index = run_index(&ws2.root);
    assert!(index.scopes.is_empty(), "boot never compiled, no scope");
    assert!(index.events.is_empty());
    assert!(
        !dir2.path().join(boot_rel("STATIC.md")).exists(),
        "no node artifact was written"
    );
}

/// A `SlotLifecycle` that refuses at the pre-install timing point — the real
/// park/failure seam an orchestrated install stops on, before any boot
/// artifact is compiled.
#[derive(Default)]
struct RefusingPreInstall {
    seen: RefCell<Vec<String>>,
}

impl SlotLifecycle for RefusingPreInstall {
    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        self.seen
            .borrow_mut()
            .push(format!("{}/{}", context.group, context.name));
        Err("fixture pre-install refusal".to_string())
    }

    fn post_install(&self, _context: SlotLifecycleContext<'_>) -> Result<(), String> {
        panic!("post-install is unreachable once pre-install refuses");
    }
}

/// RED 8b — the traced `SlotLifecycle` seam itself: a pre-install callback
/// that refuses stops the install BEFORE boot compilation, so the borrowed run
/// records no scope and no event at all, and the error identity is exactly the
/// untraced one.
#[test]
fn a_traced_pre_install_callback_refusal_prevents_any_boot_compilation() {
    let (_plain_dir, plain_ws, plain_resolution, _plain_srcs) = unit_and_root_fixture();
    let plain_callback = RefusingPreInstall::default();
    let plain = apply_lifecycle_traced(
        &plain_ws,
        &plain_resolution,
        SlotLifecycleMode::Callback(&plain_callback),
        None,
    )
    .unwrap_err();

    let (ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let callback = RefusingPreInstall::default();
    let run = traced_run(&ws.root);
    let traced = apply_lifecycle_traced(
        &ws,
        &resolution,
        SlotLifecycleMode::Callback(&callback),
        Some(&run),
    )
    .unwrap_err();

    assert!(
        !callback.seen.borrow().is_empty(),
        "the pre-install seam really ran"
    );
    assert_eq!(
        format!("{traced:#}"),
        format!("{plain:#}"),
        "observing an install cannot change the park/failure identity"
    );
    let index = run_index(&ws.root);
    assert!(
        index.scopes.is_empty() && index.events.is_empty(),
        "a pre-install refusal compiles no boot artifact, so the run records nothing"
    );
    assert!(
        !ws_dir.path().join(boot_rel("STATIC.md")).exists(),
        "no node artifact was written"
    );
}

/// RED 9 — a planted declaration conflict on the unit base leaves the compile
/// UNTRACED and successful, the planted scope untouched, and the warning
/// retained on the run.
#[test]
fn a_declaration_fault_compiles_untraced_and_keeps_the_warning() {
    let (ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let run = traced_run(&ws.root);
    // Plant the unit base's first attempt id under a foreign identity — the
    // transition/declaration fault this seam must survive. It is still
    // `pending`, which is the ONE state exact identity matching applies to.
    let mut foreign = ScopeDescriptor {
        id: "unit:org.vibevm/parent#static-md::attempt:1".to_string(),
        kind: ScopeKind::Unit,
        label: "someone-else".to_string(),
        artifact: "static-md".to_string(),
        target: ArtifactTarget::StaticMd,
    };
    run.declare_scope(&foreign)
        .expect("the exact API plants it");
    foreign.id = "irrelevant".to_string();

    apply_traced(&ws, &resolution, Some(&run)).expect("the install succeeds regardless");

    let parent_static = fs::read_to_string(unit_static(ws_dir.path(), "parent")).unwrap();
    assert!(
        parent_static.contains("# parent boot") && parent_static.contains("# child boot"),
        "the unit compiled untraced: {parent_static}"
    );
    let index = run_index(&ws.root);
    assert_eq!(
        index.scopes.len(),
        2,
        "the planted scope plus the traced node"
    );
    assert_eq!(
        index.scopes[0].id,
        "unit:org.vibevm/parent#static-md::attempt:1"
    );
    assert_eq!(
        index.scopes[0].label, "someone-else",
        "the plant is untouched"
    );
    assert_eq!(index.scopes[0].status, ScopeStatus::Pending);
    assert_eq!(index.scopes[1].id, "node:.#static-md::attempt:1");
    assert_eq!(index.scopes[1].status, ScopeStatus::Compiled);
    let summary = run.summary();
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| format!("{warning}").contains("could not be declared")),
        "the fault is retained: {:?}",
        summary.warnings
    );
}
