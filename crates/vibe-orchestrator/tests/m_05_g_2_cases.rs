//! G2 drives the same prepared selection/source route used by CLI install.
#[path = "../../vibe-install/tests/support/g2.rs"]
mod fixture;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use vibe_lifecycle::RunMetadata;
use vibe_orchestrator::{ports::*, *};
use vibe_workspace::configuration_transaction::{
    ConfigurationTransaction, with_before_boundary_fault, with_boundary_fault,
};

struct Host;
impl RegistryEnvironment for Host {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: None,
            global: Default::default(),
        })
    }
}
impl InstallObserver for Host {
    fn stream_mode(&self) -> vibe_lifecycle::process::StreamMode {
        vibe_lifecycle::process::StreamMode::Capture
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn narrate(&self, _: InstallNarration<'_>) {}
    fn lane_sizes(&self, _: &Path) -> Vec<(String, Option<u64>)> {
        vec![]
    }
    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        &vibe_install::NullObserver
    }
    fn slot_observer(&self, _: &RunMetadata) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(vibe_install::NoSlotLifecycleObserver)
    }
}
impl ConfirmGate for Host {
    fn confirm_install(&self, _: usize) -> anyhow::Result<()> {
        Ok(())
    }
}
struct Factory(PathBuf);
struct Qualified;
impl vibe_package_source::PackageQualifier for Qualified {
    fn qualify(
        &self,
        _: &vibe_package_source::InstallResolver,
        p: &vibe_core::PackageRef,
        _: &vibe_core::manifest::Lockfile,
    ) -> anyhow::Result<vibe_core::PackageRef> {
        anyhow::ensure!(p.is_qualified(), "G2 inputs must already be qualified");
        Ok(p.clone())
    }
}
impl PackageSourceFactory for Factory {
    fn build(&self, input: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        let resolver = vibe_package_source::build_install_resolver(
            &vibe_package_source::PackageSourceOptions {
                registry: Some(self.0.clone()),
                ..Default::default()
            },
            input.manifest,
            input.embedded_root,
            input.project_root,
            input.global,
            input.offline,
            input.locked,
        )?;
        Ok(Box::new(vibe_package_source::RegistryPackageSource::new(
            resolver,
            Box::new(Qualified),
        )))
    }
}
fn run(f: &fixture::Fixture) -> anyhow::Result<InstallRun> {
    run_mode(f, false)
}
fn run_mode(f: &fixture::Fixture, hosted: bool) -> anyhow::Result<InstallRun> {
    run_inputs(f, hosted, false)
}
fn run_inputs(
    f: &fixture::Fixture,
    hosted: bool,
    manifest_delta: bool,
) -> anyhow::Result<InstallRun> {
    let root = resolve_project_root(&f.root)?;
    let lease = Arc::new(vibe_lifecycle::LifecycleLease::acquire(&root)?);
    let metadata = RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: if hosted {
            vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Agent
        } else {
            vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli
        },
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(&root)?,
        started: vibe_core::timestamp::now_utc(),
        selected: ".".into(),
    };
    execute_prepared(
        InstallExecution {
            args: InstallInputs {
                packages: if manifest_delta {
                    vec!["org.vibevm.world/wal@=1.1.0".into()]
                } else {
                    vec![]
                },
                ..Default::default()
            },
            environment: &Host,
            policy: InstallPolicy {
                offline: true,
                ..Default::default()
            },
            lease,
            selection: SelectedManifest::read(&root).prepare(),
            metadata,
            manifest_mutation: &NoManifestMutation,
            sources: &Factory(f.registry.clone()),
            confirm_gate: &Host,
            observer: &Host,
            agent: Arc::new(ParkOnly),
            trace: None,
        },
        &mut NoAfterDurableWorld,
    )
}

struct ParkOnly;
fn workspace_error(error: &vibe_install::Error) -> Option<&vibe_workspace::WorkspaceError> {
    match error {
        vibe_install::Error::Workspace(error) => Some(error),
        vibe_install::Error::SlotFailed { source, .. } => workspace_error(source),
        _ => None,
    }
}
impl vibe_lifecycle::AgentBackend for ParkOnly {
    fn resolve_prompt(
        &self,
        _: &vibe_lifecycle::agent::PromptRequest,
    ) -> Result<vibe_lifecycle::agent::ResolvedPrompt, String> {
        Ok(vibe_lifecycle::agent::ResolvedPrompt {
            text: "Write the isolated declared slot output.".into(),
            unsupported: vec![],
        })
    }
    fn complete(
        &self,
        _: &vibe_lifecycle::agent::AgentRequest,
    ) -> Result<vibe_lifecycle::agent::AgentCompletion, String> {
        panic!("G2 hosted fixture must never call a model")
    }
}

#[test]
fn execute_prepared_parks_pre_and_post_with_measured_partial_effects() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    for point in ["pre-install", "post-install"] {
        let f = fixture::Fixture::new(Some("mup"));
        f.add_hook();
        std::fs::write(f.registry.join("org.g2/hooked/v1.1.0/vibe.toml"), format!("[package]\ngroup='org.g2'\nname='hooked'\nkind='flow'\nversion='1.1.0'\n[[extension]]\nid='produce'\npoint='slot:{point}'\nhandler={{kind='agent',prompt='spec://org.g2/hooked/common/prompt#root'}}\neffects=['build-artifact']\napplies_to={{packages=['org.g2/hooked']}}\nconfig.outputs=[{{path='.vibe/g2-output.txt',kind='file',accept='non-empty file'}}]\n")).unwrap();
        let run = run_mode(&f, true).unwrap();
        assert_eq!(run.disposition, InstallDisposition::Parked);
        assert!(run.parked.is_some());
        assert!(!run.slot_reports.is_empty());
        assert_eq!(f.root.join("vibe.lock").exists(), point == "post-install");
        assert!(!f.root.join(".vibe/g2-output.txt").exists());
        let (_, path, journal) = fixture::journal(&f.root);
        assert_eq!(
            journal["phase"],
            if point == "pre-install" {
                "slot"
            } else {
                "post-install"
            }
        );
        assert_eq!(journal["external_effects_possible"], true);
        assert!(path.is_file());
    }
}

#[test]
fn execute_prepared_applies_real_redbook_and_independent_roots() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    for branch in ["mup", "wal"] {
        let f = fixture::Fixture::new(Some(branch));
        let result = run(&f).unwrap();
        assert_eq!(result.disposition, InstallDisposition::Applied);
        let lock = vibe_core::manifest::Lockfile::read(f.root.join("vibe.lock")).unwrap();
        assert_eq!(lock.meta.root_dependencies.len(), 3);
        for name in ["wal", "multi-user-planning", "redbook"] {
            assert!(lock.packages.iter().any(|p| p.name == name));
        }
        let index = fixture::boot_text(&f.root);
        assert!(index.contains("10-flow-wal") && index.contains("12-flow-multi-user-planning"));
        let (_, resolved) = vibe_wire::behaviour::lock_configuration::decode(
            lock.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        let selected = resolved
            .packages
            .iter()
            .find(|p| p.package == "org.vibevm.world/redbook@1.1.0")
            .unwrap();
        assert!(selected.feature_closure.contains(&branch.to_owned()));
    }
}

#[test]
fn execute_prepared_propagates_each_before_after_interruption_and_retains_evidence() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    for phase in ["manifest", "slot", "boot", "lock", "post-install"] {
        for before in [true, false] {
            let f = fixture::Fixture::new(Some("mup"));
            let p = f.plan();
            let slots = fixture::Fixture::slots(&p);
            let workspace = vibe_workspace::Workspace::discover(&f.root).unwrap();
            let result = if before {
                with_before_boundary_fault(&f.root, phase, || run_inputs(&f, false, true))
            } else {
                with_boundary_fault(&f.root, phase, || run_inputs(&f, false, true))
            };
            let error = match result {
                Err(e) => e,
                Ok(_) => panic!("{phase}/{before}: must interrupt"),
            };
            assert!(
                error.to_string().contains("injected interruption"),
                "{error:#}"
            );
            let carried =
                vibe_orchestrator::failure::take::<vibe_orchestrator::failure::Measurement>(error)
                    .unwrap();
            assert!(
                carried
                    .original
                    .downcast_ref::<vibe_install::Error>()
                    .and_then(workspace_error)
                    .is_some(),
                "typed original: {:#}",
                carried.original
            );
            let vibe_orchestrator::failure::Measurement::InstallBarrier {
                progress,
                packages_resolved,
                ..
            } = carried.evidence
            else {
                panic!("install barrier measurement required")
            };
            assert_eq!(packages_resolved, p.resolution.len());
            assert_eq!(progress.complete, phase == "post-install");
            let (operation, path, journal) = fixture::journal(&f.root);
            assert_eq!(journal["phase"], phase);
            assert_eq!(journal["in_flight"], before);
            let tx = ConfigurationTransaction::reopen(&workspace, &slots, &operation).unwrap();
            assert_eq!(tx.report()["complete_rollback"], false);
            assert!(path.is_file());
        }
    }
}

#[test]
fn execute_prepared_missing_input_never_treats_confirmation_as_choice() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    let f = fixture::Fixture::new(None);
    let before = fixture::image(&f.root);
    let error = match run(&f) {
        Err(e) => e,
        Ok(_) => panic!("missing choice"),
    };
    assert!(error.to_string().contains("continuity"), "{error:#}");
    // Run bookkeeping is allowed; product inputs, slots and lock are not.
    assert_eq!(
        std::fs::read(f.root.join("vibe.toml")).unwrap(),
        before[Path::new("vibe.toml")]
    );
    assert!(!f.root.join("vibe.lock").exists());
    assert!(
        !f.root
            .join(vibe_core::layout::current_vibedeps_root())
            .exists()
    );
}
