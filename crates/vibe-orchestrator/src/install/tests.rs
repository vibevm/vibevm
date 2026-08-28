//! Producer-level reds for the shared install core's laziness contract.
//!
//! Constructing a multi-registry package source can inspect registry state and
//! credentials, so a lifecycle that needs no packages must never pay for one.
//! The empty-world fast path returns BEFORE the factory is built, and these
//! drive the real `execute_prepared` with a counting factory to prove it.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{AgentBackend, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

use super::{
    InstallDisposition, InstallExecution, InstallInputs, InstallPolicy, InstallRunContext,
    SelectedManifest, WorldCallbackOutcome, execute_prepared, generated_by, resolve_project_root,
};
use crate::RitualPlan;
use crate::ports::{
    AfterDurableWorld, ConfirmGate, InstallManifestMutation, InstallNarration, InstallObserver,
    NoManifestMutation, PackageSource, PackageSourceBuild, PackageSourceFactory,
    RegistryEnvironment, RegistryEnvironmentSnapshot, RunObserver,
};

/// A REAL lease over the fixture's OWN canonical root.
///
/// The public core proves lease/root/selected agreement before it mutates, so a
/// success test must own the tree it acts on. The retained process-wide helper
/// is rooted in a different temp directory and is therefore only for hand-built
/// values that never reach the public entry point.
fn lease_for(root: &Path) -> Arc<vibe_lifecycle::LifecycleLease> {
    Arc::new(vibe_lifecycle::LifecycleLease::acquire(root).expect("the fixture root is leasable"))
}

/// A manifest mutation that COUNTS, so a refusal can prove zero mutation.
#[derive(Default)]
struct CountingMutation {
    applied: AtomicUsize,
}

impl CountingMutation {
    fn applied(&self) -> usize {
        self.applied.load(Ordering::SeqCst)
    }
}

impl InstallManifestMutation for CountingMutation {
    fn apply(
        &self,
        _manifest: &mut vibe_core::manifest::Manifest,
        _workspace: &mut vibe_workspace::Workspace,
        _project_root: &Path,
    ) -> anyhow::Result<()> {
        self.applied.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// A project with NOTHING declared: the empty-world fast path.
fn empty_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    dir
}

struct SilentRun;

impl RunObserver for SilentRun {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Null
    }
    fn binary_quiet(&self) -> bool {
        true
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn observe_plan(
        &self,
        _plan: &RitualPlan,
        _metadata: &RunMetadata,
        _emit_empty: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    fn observe_contribution(&self, _report: &LifecycleContributionReport) {}
    fn observe_untracked_failure(
        &self,
        _metadata: &RunMetadata,
        _phase: &str,
        _contributions: &[LifecycleContributionReport],
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

struct SilentInstall;

impl InstallObserver for SilentInstall {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Null
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn narrate(&self, _event: InstallNarration<'_>) {}
    fn lane_sizes(&self, _root: &Path) -> Vec<(String, Option<u64>)> {
        Vec::new()
    }
    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        &NoPlanEvents
    }
    fn slot_observer(
        &self,
        _metadata: &RunMetadata,
    ) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(NoSlotEvents)
    }
}

struct NoPlanEvents;
impl vibe_install::PlanObserver for NoPlanEvents {
    fn on(&self, _event: vibe_install::PlanEvent) {}
}

struct NoSlotEvents;
impl vibe_install::SlotLifecycleObserver for NoSlotEvents {
    fn observe(&self, _plan: &vibe_install::SlotLifecyclePlan) -> Result<(), String> {
        Ok(())
    }
    fn outcome(&self, _report: &vibe_install::SlotLifecycleReport) -> Result<(), String> {
        Ok(())
    }
}

struct AlwaysYes;
impl ConfirmGate for AlwaysYes {
    fn confirm_install(&self, _packages: usize) -> anyhow::Result<()> {
        Ok(())
    }
}

/// A registry environment that COUNTS its preparations and records the ORDER
/// in which it seeded and loaded.
///
/// The seed→load order is the whole contract: on a source install the surface's
/// embedded lookup is what seeds the machine-global defaults the load then
/// reads, so a core that loaded first failed on a fresh machine and succeeded on
/// the second run.
#[derive(Default)]
struct CountingEnvironment {
    prepares: AtomicUsize,
    steps: std::sync::Mutex<Vec<&'static str>>,
}

impl CountingEnvironment {
    fn prepares(&self) -> usize {
        self.prepares.load(Ordering::SeqCst)
    }
    fn steps(&self) -> Vec<&'static str> {
        self.steps.lock().unwrap().clone()
    }
}

impl RegistryEnvironment for CountingEnvironment {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        let mut steps = self.steps.lock().unwrap();
        // The order a real surface owes, stated where a test can read it.
        steps.push("seed");
        let embedded_root: Option<PathBuf> = None;
        steps.push("load");
        let global = vibe_core::GlobalRegistryConfig::load()?;
        Ok(RegistryEnvironmentSnapshot {
            embedded_root,
            global,
        })
    }
}

/// A named post-durability stage that COUNTS its invocations.
///
/// The port is one-shot BY USE: the core consumes it on the one branch that
/// completes, and never fabricates a call on a failure or a park. A closure
/// could not be counted from outside without capturing a cell, which is the
/// whole reason this seam is a named type.
#[derive(Default)]
struct CountingStage {
    calls: AtomicUsize,
}

impl CountingStage {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl AfterDurableWorld for CountingStage {
    fn after(
        &mut self,
        _project_root: &Path,
        _run: InstallRunContext,
        _workspace: &vibe_workspace::Workspace,
    ) -> anyhow::Result<WorldCallbackOutcome> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(WorldCallbackOutcome::default())
    }
}

/// A factory that COUNTS its builds and refuses to produce a source.
///
/// Refusing is the point: if the empty-world path ever reached it, the run
/// would fail loudly rather than pass with a silent extra construction.
#[derive(Default)]
struct CountingFactory {
    builds: AtomicUsize,
}

impl CountingFactory {
    fn builds(&self) -> usize {
        self.builds.load(Ordering::SeqCst)
    }
}

impl PackageSourceFactory for CountingFactory {
    fn build(&self, _input: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        self.builds.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("the package source was constructed")
    }
}

fn metadata(root: &Path) -> RunMetadata {
    RunMetadata {
        requested: "install".to_string(),
        chain: vec!["validate".to_string(), "install".to_string()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root).unwrap(),
        started: vibe_core::timestamp::now_utc(),
        selected: ".".into(),
    }
}

/// A package-free lifecycle builds ZERO package sources.
///
/// The mutation this kills is hoisting the factory build above the empty-world
/// return "for symmetry": the counting factory refuses on construction, so an
/// early build turns this green run red.
#[test]
fn a_package_free_run_never_constructs_a_package_source() {
    let project = empty_project();
    let root = resolve_project_root(project.path()).unwrap();
    let selection = SelectedManifest::read(&root).prepare();
    let environment = CountingEnvironment::default();
    let factory = CountingFactory::default();
    let mut stage = CountingStage::default();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);

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
            sources: &factory,
            confirm_gate: &AlwaysYes,
            observer: &SilentInstall,
            agent,
            trace: None,
        },
        &mut stage,
    )
    .expect("an empty world regenerates without resolving anything");

    assert_eq!(
        factory.builds(),
        0,
        "the empty-world fast path returns BEFORE any package source is built",
    );
    assert_eq!(
        stage.calls(),
        1,
        "and the post-durability stage is consumed EXACTLY once on completion",
    );
    assert!(
        matches!(run.disposition, InstallDisposition::Fresh),
        "and reports the fresh shape",
    );
    assert_eq!(run.packages_resolved, 0);

    // The provenance stamp is the PRODUCT's, not this crate's identity: the
    // extraction moved `generated_by` between crates that share
    // `version.workspace = true`, so the bytes cannot move with it.
    assert!(generated_by().starts_with("vibe "));
}

/// A run that DOES declare a package reaches the factory exactly once.
///
/// Together with the red above this pins the whole laziness contract: not
/// "never built", but "built exactly when the world needs resolving".
#[test]
fn a_declaring_run_constructs_exactly_one_package_source() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.demo/tools\" = \"^1.0\"\n",
    )
    .unwrap();
    let root = resolve_project_root(project.path()).unwrap();
    let selection = SelectedManifest::read(&root).prepare();
    let environment = CountingEnvironment::default();
    let factory = CountingFactory::default();
    let mut stage = CountingStage::default();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);

    let outcome = execute_prepared(
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
    );
    let Err(error) = outcome else {
        panic!("the counting factory refuses to produce a source");
    };

    assert_eq!(
        factory.builds(),
        1,
        "exactly one construction, at the one composition point",
    );
    assert_eq!(
        stage.calls(),
        0,
        "a run that never reached durability never fabricates a stage call",
    );
    assert!(
        format!("{error:#}").contains("the package source was constructed"),
        "and the factory's own error travels unchanged: {error:#}",
    );
}

/// The OUTER phase port and the CHILD install port are separate objects with
/// separate policies — the whole reason there are two traits.
#[test]
fn the_two_observation_ports_are_distinct_traits() {
    let phase: &dyn RunObserver = &SilentRun;
    let install: &dyn InstallObserver = &SilentInstall;
    assert_eq!(phase.stream_mode(), StreamMode::Null);
    assert_eq!(install.stream_mode(), StreamMode::Null);
    assert!(!phase.emit_machine_failure());
    assert!(!install.emit_machine_failure());
}

/// The agreement gates and the registry epoch — split out at the line budget.
#[path = "tests/gates.rs"]
mod gates;
