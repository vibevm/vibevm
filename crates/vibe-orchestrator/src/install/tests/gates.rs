//! The public entry point's agreement gates, and its registry epoch.
//!
//! Split from `install/tests.rs` when that cell reached its line budget
//! (guide#surface-form). The fixtures — the counting factory, environment,
//! mutation and stage, the silent observers and the real per-fixture lease —
//! all live in the parent and are used through `super::*`.

use super::*;

/// A FOREIGN lease refuses before any mutation, at the public entry point.
///
/// `execute_prepared` takes lease, root, workspace and `metadata.selected` as
/// four independent values. Nothing but this gate proves they describe the same
/// tree, and everything past it mutates: the surface's manifest write, the
/// registry reads, the materialisation, the state store rooted at the lease.
///
/// The mutation this kills is deleting either `ensure_root` call: the run would
/// then proceed under a lease belonging to a different workspace and write state
/// beside another process's lock. The counting mutation proves the refusal is
/// EARLY — zero applications, so nothing was recorded before the refusal.
#[test]
fn a_foreign_lease_refuses_before_the_core_mutates_anything() {
    let project = empty_project();
    let root = resolve_project_root(project.path()).unwrap();
    let selection = SelectedManifest::read(&root).prepare();
    let environment = CountingEnvironment::default();
    let factory = CountingFactory::default();
    let mutation = CountingMutation::default();
    let mut stage = CountingStage::default();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);

    // A lease over a DIFFERENT tree entirely.
    let elsewhere = empty_project();
    let foreign = resolve_project_root(elsewhere.path()).unwrap();

    let outcome = execute_prepared(
        InstallExecution {
            args: InstallInputs::default(),
            environment: &environment,
            policy: InstallPolicy {
                offline: true,
                ..InstallPolicy::default()
            },
            lease: lease_for(&foreign),
            manifest_mutation: &mutation,
            selection,
            metadata: metadata(&root),
            sources: &factory,
            confirm_gate: &AlwaysYes,
            observer: &SilentInstall,
            agent,
            trace: None,
        },
        &mut stage,
    );

    let Err(error) = outcome else {
        panic!("a foreign lease can never reach a durable action");
    };
    // The lease's OWN typed refusal, not a hand-rolled spelling.
    assert!(
        error
            .downcast_ref::<vibe_lifecycle::LifecycleLeaseError>()
            .is_some(),
        "the refusal is the lease's typed error: {error:#}",
    );
    assert!(
        format!("{error:#}").contains("at install execution"),
        "and it names the boundary it fired at: {error:#}",
    );
    assert_eq!(mutation.applied(), 0, "no manifest mutation was applied");
    assert_eq!(factory.builds(), 0, "no package source was constructed");
    assert_eq!(stage.calls(), 0, "no post-durability stage ran");
}

/// A lease over the RIGHT root but a `metadata.selected` naming a different
/// node refuses too, on the selected gate.
///
/// The mutation this kills is deleting the `ensure_selected` call. The root
/// alone is not enough: two members of one workspace share a root, and a run
/// whose recorded selected node disagrees with the tree it loaded would write a
/// sibling's handoff under this node's identity.
#[test]
fn a_selected_node_mismatch_refuses_before_the_core_mutates_anything() {
    let project = empty_project();
    let root = resolve_project_root(project.path()).unwrap();
    let selection = SelectedManifest::read(&root).prepare();
    let environment = CountingEnvironment::default();
    let factory = CountingFactory::default();
    let mutation = CountingMutation::default();
    let mut stage = CountingStage::default();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);

    let mut wrong = metadata(&root);
    // The tree really maps this root to `"."`; the run claims a member.
    wrong.selected = "members/other".to_string();

    let outcome = execute_prepared(
        InstallExecution {
            args: InstallInputs::default(),
            environment: &environment,
            policy: InstallPolicy {
                offline: true,
                ..InstallPolicy::default()
            },
            lease: lease_for(&root),
            manifest_mutation: &mutation,
            selection,
            metadata: wrong,
            sources: &factory,
            confirm_gate: &AlwaysYes,
            observer: &SilentInstall,
            agent,
            trace: None,
        },
        &mut stage,
    );

    let Err(error) = outcome else {
        panic!("a selected-node mismatch can never reach a durable action");
    };
    assert!(
        error
            .downcast_ref::<vibe_lifecycle::LifecycleLeaseError>()
            .is_some(),
        "the refusal is the lease's typed error: {error:#}",
    );
    assert_eq!(mutation.applied(), 0, "no manifest mutation was applied");
    assert_eq!(factory.builds(), 0, "no package source was constructed");
    assert_eq!(stage.calls(), 0, "no post-durability stage ran");
}

/// The registry environment is prepared EXACTLY once, in seed→load order, and
/// before any package source is built.
///
/// The defect this pins is an ordering one, and ordering is invisible to a
/// success assertion: the core used to load `GlobalRegistryConfig` early and
/// ask the surface for its embedded root far below, past the empty-world fast
/// path. On a source install that surface lookup is also what SEEDS the
/// machine-global defaults the load reads, so on a fresh machine the first run
/// resolved against a file its own seed had not written yet.
///
/// The mutation this kills is splitting the port back into two calls, or
/// hoisting the source build above the empty-world return: the counting
/// environment would then report a different step order, a second preparation,
/// or a nonzero build.
#[test]
fn the_registry_environment_is_prepared_once_seed_before_load() {
    let project = empty_project();
    let root = resolve_project_root(project.path()).unwrap();
    let selection = SelectedManifest::read(&root).prepare();
    let environment = CountingEnvironment::default();
    let factory = CountingFactory::default();
    let mut stage = CountingStage::default();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);

    execute_prepared(
        InstallExecution {
            args: InstallInputs::default(),
            environment: &environment,
            policy: InstallPolicy {
                offline: true,
                ..InstallPolicy::default()
            },
            lease: lease_for(&root),
            manifest_mutation: &NoManifestMutation,
            selection,
            metadata: metadata(&root),
            sources: &factory,
            confirm_gate: &AlwaysYes,
            observer: &SilentInstall,
            agent,
            trace: None,
        },
        &mut stage,
    )
    .expect("an empty world regenerates");

    assert_eq!(
        environment.prepares(),
        1,
        "one environment epoch per run — even on the empty world, exactly as the \
         global load always ran there",
    );
    assert_eq!(
        environment.steps(),
        vec!["seed", "load"],
        "and the surface seeds BEFORE it loads; the reverse is the fresh-machine bug",
    );
    assert_eq!(
        factory.builds(),
        0,
        "while the expensive half — the package source — still never runs on an empty world",
    );
}

/// A run on a machine with NO global registry file succeeds on its FIRST
/// invocation.
///
/// The isolated first-run red. `VIBE_NO_DEFAULT_REGISTRY` is deliberately not
/// used: what is under test is that the core asks the surface to prepare its
/// environment ONCE, ahead of every read of it, so a surface whose seed writes
/// the file is observed by the load in the same invocation. A core that read
/// the global config before handing control to the surface would need a second
/// run.
#[test]
fn a_first_run_with_an_absent_global_registry_needs_no_retry() {
    /// A surface whose seed genuinely CREATES the global file, and whose load
    /// then refuses if it is not there — the fresh-machine shape, in miniature.
    struct SeedingEnvironment {
        home: PathBuf,
    }

    impl RegistryEnvironment for SeedingEnvironment {
        fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
            let marker = self.home.join("registry.toml");
            // SEED.
            std::fs::write(&marker, "# seeded\n")?;
            // LOAD — and refuse if the seed had not happened yet.
            anyhow::ensure!(
                marker.exists(),
                "the global registry was read before it was seeded",
            );
            Ok(RegistryEnvironmentSnapshot {
                embedded_root: None,
                global: vibe_core::GlobalRegistryConfig::load()?,
            })
        }
    }

    let project = empty_project();
    let root = resolve_project_root(project.path()).unwrap();
    let home = tempfile::tempdir().unwrap();
    let selection = SelectedManifest::read(&root).prepare();
    let environment = SeedingEnvironment {
        home: home.path().to_path_buf(),
    };
    let mut stage = CountingStage::default();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);

    assert!(
        !home.path().join("registry.toml").exists(),
        "the fixture machine starts with no global registry at all",
    );
    let run = execute_prepared(
        InstallExecution {
            args: InstallInputs::default(),
            environment: &environment,
            policy: InstallPolicy {
                offline: true,
                ..InstallPolicy::default()
            },
            lease: lease_for(&root),
            manifest_mutation: &NoManifestMutation,
            selection,
            metadata: metadata(&root),
            sources: &CountingFactory::default(),
            confirm_gate: &AlwaysYes,
            observer: &SilentInstall,
            agent,
            trace: None,
        },
        &mut stage,
    )
    .expect("the FIRST invocation succeeds — no `run it twice`");
    assert!(matches!(run.disposition, InstallDisposition::Fresh));
    assert!(
        home.path().join("registry.toml").exists(),
        "and the seed really did write it, inside this same invocation",
    );
}
