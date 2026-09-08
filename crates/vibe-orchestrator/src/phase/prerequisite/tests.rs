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
use crate::ports::AfterDurableWorld;

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

#[test]
fn the_exact_native_epoch_crosses_the_prerequisite_callback_once() {
    let workspace = workspace();
    let root = workspace.root.clone();
    let (world, lowering, sidecar) =
        crate::world::prepare_owner_runtime_inputs(&root, &workspace, &[]).unwrap();
    let platform = vibe_lifecycle::native::NativePlatform::current().unwrap();
    let facts = vibe_workspace::extension_world::OwnerRuntimeRunFacts {
        run_id: "install-epoch".into(),
        state_root: root.join(".vibe"),
        platform: platform.key().into(),
        offline: true,
        created_at: "2026-09-08T00:00:00Z".into(),
    };
    let mut make_provider = |policies| {
        Ok(vibe_lifecycle::native::ArtifactCompilerNativeProvider::new(
            platform, policies,
        ))
    };
    let (_, carriage) = vibe_workspace::install::regenerate_boot_from_traced_native(
        &workspace,
        &[],
        world,
        vibe_core::manifest::SpecFormat::Mixed,
        None,
        lowering,
        facts,
        &mut make_provider,
    )
    .unwrap();
    let lease = std::sync::Arc::new(vibe_lifecycle::LifecycleLease::acquire(&root).unwrap());
    let context = crate::install::InstallRunContext {
        metadata: vibe_lifecycle::RunMetadata {
            requested: "build".into(),
            chain: vec!["validate".into(), "install".into(), "build".into()],
            offline: true,
            assume_yes: true,
            agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id: "install-epoch".into(),
            started: "2026-09-08T00:00:00Z".into(),
            selected: ".".into(),
        },
        lease,
        lifecycle_run: None,
        lifecycle_reports: Vec::new(),
        native: Some(crate::install::NativeInstallContext::new(carriage, sidecar)),
    };
    let mut collector = PrerequisiteInstall::default();
    collector.after(&root, context, &workspace).unwrap();
    let native = collector.native(true).unwrap().unwrap();
    let (epoch, sidecar) = native.parts();
    assert_eq!(epoch.run().run_id, "install-epoch");
    let plan = crate::world::plan_default_from_runtime(
        epoch,
        sidecar,
        &[
            vibe_lifecycle::Phase::Validate,
            vibe_lifecycle::Phase::Install,
            vibe_lifecycle::Phase::Build,
        ],
    )
    .unwrap();
    assert_eq!(plan.workspace_root(), root);
    assert!(collector.take_native().is_some());
    assert!(collector.take_native().is_none(), "the carrier moves once");
}
