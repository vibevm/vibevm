//! The command cell's own reds: the exact field sets of the neutral
//! request and the opaque stages, the DERIVED chain a caller cannot
//! supply, and the laziness contract carried through the new API.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{AgentBackend, Phase, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

use super::{
    DefaultLifecyclePorts, DefaultLifecycleRequest, LeasedDefaultLifecycle,
    PreparedDefaultLifecycle, lease_default_lifecycle, prepare_default_lifecycle,
};
use crate::RitualPlan;
use crate::ports::{
    ConfirmGate, InstallNarration, InstallObserver, NoManifestMutation, PackageSource,
    PackageSourceBuild, PackageSourceFactory, RegistryEnvironment, RegistryEnvironmentSnapshot,
    RunObserver,
};

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

/// A workspace root plus one member, each with its own `vibe.toml` — the
/// fixture the two-root projections are proven apart on.
fn workspace_with_member() -> (tempfile::TempDir, std::path::PathBuf) {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join("vibe.toml"),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n[workspace]\nmembers = [\"member\"]\n",
    )
    .unwrap();
    let member = workspace.path().join("member");
    std::fs::create_dir_all(&member).unwrap();
    std::fs::write(
        member.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    (workspace, member)
}

fn request(requested: Phase) -> DefaultLifecycleRequest {
    DefaultLifecycleRequest {
        requested,
        force: false,
        agent_mode: RunAgentMode::Cli,
        assume_yes: true,
        trace_flag: false,
        install_inputs: Default::default(),
        policy: Default::default(),
    }
}

// ---- 1. the exact field sets ----------------------------------------------

/// The request carries SEVEN fields, and the compiler proves it.
///
/// Destructured with no `..`, so a caller-supplied `chain`, `phases`,
/// `path`, a second offline spelling or any config carrier is a compile
/// error here rather than a review question.
#[test]
fn the_request_carries_exactly_its_seven_neutral_fields() {
    fn destructure(request: DefaultLifecycleRequest) {
        let DefaultLifecycleRequest {
            requested,
            force,
            agent_mode,
            assume_yes,
            trace_flag,
            install_inputs,
            policy,
        } = request;
        let _ = (
            requested,
            force,
            agent_mode,
            assume_yes,
            trace_flag,
            install_inputs,
            policy,
        );
    }
    let _ = destructure;
}

/// The leased stage carries TWO fields and the prepared stage SEVEN, both
/// proven by exact destructuring — a hidden eighth carrier (a forged chain,
/// a second selection, a config value) is a compile error, not a review
/// question. This is the in-crate structural ratchet; the compiled EXTERNAL
/// privacy proof is the `compile_fail,E0451` doctests on the public items.
#[test]
fn the_opaque_stages_carry_exactly_their_fields() {
    fn destructure_leased(stage: LeasedDefaultLifecycle) {
        let LeasedDefaultLifecycle {
            selected_root,
            lease,
        } = stage;
        let _ = (selected_root, lease);
    }
    fn destructure_prepared(prepared: PreparedDefaultLifecycle) {
        let PreparedDefaultLifecycle {
            requested,
            phases,
            chain,
            metadata,
            install_inputs,
            policy,
            prelude,
        } = prepared;
        let _ = (
            requested,
            phases,
            chain,
            metadata,
            install_inputs,
            policy,
            prelude,
        );
    }
    let _ = (destructure_leased, destructure_prepared);
}

/// The textual twin: both opaque stages declare their fields PRIVATE and
/// expose no public constructor, so a foreign crate can neither forge a
/// stage nor reach into one — [`lease_default_lifecycle`] is the only way
/// in. An in-crate source ratchet, NOT the compiled external proof — that
/// is the `compile_fail,E0451` doctests on the two structs' docs.
#[test]
fn the_opaque_stages_stay_private_with_no_public_constructor() {
    let source = include_str!("../command.rs");
    for name in ["LeasedDefaultLifecycle", "PreparedDefaultLifecycle"] {
        let start = source
            .find(&format!("pub struct {name} {{"))
            .unwrap_or_else(|| panic!("{name} is declared"));
        let end = start + source[start..].find('}').expect("the struct body closes");
        let body = &source[start..end];
        assert!(
            !body.contains(" pub "),
            "{name}'s fields must stay private — the value is opaque by design"
        );
    }
    assert!(
        !source.contains("pub fn new"),
        "the stages are produced by lease_default_lifecycle / \
         prepare_default_lifecycle only — no public constructor",
    );
}

// ---- 2. the derived chain ---------------------------------------------------

/// A `Build` request prepares the EXACT canonical prefix
/// validate,install,generate,build — as phases, as the string chain, and in
/// the metadata — because all three are DERIVED from `requested` inside
/// the cell, never supplied by the caller.
#[test]
fn a_build_request_prepares_the_exact_chain() {
    let project = empty_project();
    let leased = lease_default_lifecycle(project.path()).expect("the fixture leases");
    let prepared = prepare_default_lifecycle(leased, request(Phase::Build))
        .expect("an empty project prepares");

    let expected = [
        Phase::Validate,
        Phase::Install,
        Phase::Generate,
        Phase::Build,
    ];
    assert_eq!(prepared.phases, expected, "the derived phases");
    assert_eq!(
        prepared.chain,
        ["validate", "install", "generate", "build"],
        "the derived string chain"
    );
    // Through the REQUIRED accessor: a surface pins the derived chain by
    // reading `metadata()`, never by supplying one.
    assert_eq!(
        prepared.metadata().chain,
        ["validate", "install", "generate", "build"],
        "the metadata carries the SAME derived chain"
    );
    assert_eq!(prepared.metadata().requested, "build");
    // The projections name the same roots the stages pinned.
    assert_eq!(prepared.selected_root(), prepared.workspace_root());
    assert!(prepared.selected_manifest().is_some());
    assert_eq!(prepared.metadata().selected, ".");
    drop(prepared);
}

/// A `Validate` request stays exactly one phase — the derivation is a
/// prefix, never a fixed full chain.
#[test]
fn a_validate_request_is_one_phase() {
    let project = empty_project();
    let leased = lease_default_lifecycle(project.path()).expect("the fixture leases");
    let prepared = prepare_default_lifecycle(leased, request(Phase::Validate))
        .expect("an empty project prepares");
    assert_eq!(prepared.phases, [Phase::Validate]);
    assert_eq!(prepared.chain, ["validate"]);
    assert_eq!(prepared.metadata().chain, ["validate"]);
}

/// A member invocation: the two root projections are DIFFERENT roots, and
/// each is the right one — the backend root is the leased WORKSPACE root
/// (where state, trace and prompt resolution live), the failure-family root
/// is the MEMBER the operator invoked.
#[test]
fn a_member_invocation_names_the_workspace_and_the_member_separately() {
    let (_workspace, member) = workspace_with_member();
    let leased = lease_default_lifecycle(&member).expect("the member leases");
    let prepared =
        prepare_default_lifecycle(leased, request(Phase::Validate)).expect("the member prepares");
    assert!(
        prepared.selected_root().ends_with("member"),
        "the failure-family root is the INVOKED member: {}",
        prepared.selected_root().display()
    );
    assert_eq!(
        prepared.workspace_root(),
        prepared
            .selected_root()
            .parent()
            .expect("a member has a parent"),
        "the backend root is the leased WORKSPACE root, not the member"
    );
    assert_eq!(prepared.metadata().selected, "member");
}

/// The Opus review's silent-regression case, made discriminating: the lease
/// is taken while the tree is intact, the workspace MANIFEST is then broken
/// (the selection's own load now carries a stored error and `loaded_root`
/// is `None`), and `workspace_root()` must STILL name the root the lease
/// pinned. A `loaded_root().unwrap_or(selected_root())` implementation
/// silently falls back to the member here — exactly the divergence the
/// two-stage seam exists to prevent, and only injectable BETWEEN the two
/// stages.
#[test]
fn the_workspace_root_stays_the_leases_answer_when_the_tree_broke_after_the_lease() {
    let (_workspace, member) = workspace_with_member();
    let leased = lease_default_lifecycle(&member).expect("the intact tree leases");
    // Corrupt the MEMBER manifest after the lease: the snapshot's stored
    // parse error defers the workspace load (its `loaded_root` is `None`),
    // but the lease already pinned the workspace root — and
    // workspace_root() must keep naming it, never fall back to the member.
    std::fs::write(member.join("vibe.toml"), "not [ valid").unwrap();
    let prepared = prepare_default_lifecycle(leased, request(Phase::Validate))
        .expect("the stored member snapshot still prepares (discovery failure is deferred)");
    assert_ne!(
        prepared.workspace_root(),
        prepared.selected_root(),
        "never the member: the lease is the authority"
    );
    assert_eq!(
        prepared.workspace_root(),
        prepared
            .selected_root()
            .parent()
            .expect("a member has a parent"),
        "the leased WORKSPACE root, still named after the tree broke"
    );
}

/// A PURE library consistency check — the two derivation ROUTES a live
/// surface could compose with (`LifecycleRequest::steps()` and
/// `inclusive_chain`) name the same default prefix for every phase. It
/// observes NO live composer; the discriminating red against the actual
/// clean region is the source RED in `vibe-cli`'s
/// `lifecycle_default_path_tests.rs`, which pins `execute` to this exact
/// steps→names route.
#[test]
fn the_clean_composer_and_the_cell_derive_equivalent_default_chains() {
    for phase in vibe_lifecycle::DEFAULT_PHASES {
        let via_steps = vibe_lifecycle::LifecycleRequest::Default(phase)
            .steps()
            .into_iter()
            .filter_map(|step| match step {
                vibe_lifecycle::LifecycleStep::Default(phase) => Some(phase.to_string()),
                vibe_lifecycle::LifecycleStep::Clean => None,
            })
            .collect::<Vec<_>>();
        let via_cell = vibe_lifecycle::inclusive_chain(phase)
            .iter()
            .map(Phase::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            via_steps, via_cell,
            "the two derivations agree for `{phase}`"
        );
    }
}

/// The metadata carries the run's policy-decided offline posture, not a
/// second spelling the caller could disagree with.
#[test]
fn the_metadata_offline_is_the_policy_offline() {
    let project = empty_project();
    let leased = lease_default_lifecycle(project.path()).expect("the fixture leases");
    let mut asked = request(Phase::Validate);
    asked.policy.offline = true;
    let prepared = prepare_default_lifecycle(leased, asked).expect("an empty project prepares");
    assert!(prepared.metadata.offline);
}

// ---- 6. the laziness contract through the new API ---------------------------

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

struct CountingEnvironment {
    prepares: AtomicUsize,
}
impl RegistryEnvironment for CountingEnvironment {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: None,
            global: vibe_core::GlobalRegistryConfig::load()?,
        })
    }
}

/// A factory that COUNTS its builds and refuses to produce a source:
/// if the empty-world path ever reached it, the run would fail loudly.
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

/// A package-free phase verb through the NEW command API still builds ZERO
/// package sources — the laziness contract survived the composition moving
/// into the cell. The request is `Install`, not `Validate`: only a chain
/// that reaches the install barrier exercises the fast path this RED
/// guards (a validate-only run never gets near the factory, so it would
/// prove nothing). The killing mutation — hoisting the factory build above
/// the empty-world fast path inside the install core — lives outside this
/// atom's perimeter; the counter/refusal oracle itself is proven live by
/// the eager-call mutation recorded in the worker report.
#[test]
fn a_package_free_command_builds_zero_package_sources() {
    let project = empty_project();
    let leased = lease_default_lifecycle(project.path()).expect("the fixture leases");
    let prepared = prepare_default_lifecycle(leased, request(Phase::Install))
        .expect("an empty project prepares");
    let environment = CountingEnvironment {
        prepares: AtomicUsize::new(0),
    };
    let factory = CountingFactory::default();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);

    let outcome = prepared.run(
        DefaultLifecyclePorts {
            observer: &SilentRun,
            install_observer: &SilentInstall,
            confirm_gate: &AlwaysYes,
            sources: &factory,
            environment: &environment,
            manifest_mutation: &NoManifestMutation,
            agent,
        },
        None,
        "2026-08-28T12:00:05Z".parse().expect("a fixture instant"),
    );

    assert!(
        matches!(outcome, crate::PhaseOutcome::Completed(_)),
        "an empty world completes without resolving anything"
    );
    assert_eq!(
        factory.builds(),
        0,
        "the empty-world fast path returns BEFORE any package source is built",
    );
}
