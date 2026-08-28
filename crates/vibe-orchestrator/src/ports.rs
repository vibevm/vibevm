//! The surface ports one lifecycle run is executed through.
//!
//! Everything a surface owns — terminal/JSON rendering, dialoguer, credential
//! and provider construction, CLI argument grammar — reaches this crate ONLY
//! through these traits. The neutral package-source/registry-cell composition
//! is not a surface's either: it lives in the separate `vibe-package-source`
//! crate's implementation of the [`PackageSource`](PackageSource) port.
//! Nothing here names a report family, a provider, a model or a credential.
//!
//! There are deliberately TWO observation policies, not one. The phase run is
//! observed by the surface's OUTER context and the prerequisite install by its
//! CHILD context, and the two answer differently: the outer stream policy maps
//! quiet to a null stream, the child maps *suppressed* output to it, and the
//! machine-failure emission bit is computed from whichever context that failure
//! site has always belonged to. Merging them silently flips the emission bit of
//! a composed run's slot failure.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use specmark::spec;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use vibe_core::GlobalRegistryConfig;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_workspace::install::ResolvedDep;

use crate::RitualPlan;

/// The observation policy of the OUTER lifecycle surface.
///
/// Execution asks only for stream policy and typed plan/row observations. A
/// terminal adapter renders them; a hosted adapter can capture streams and keep
/// the typed result without importing terminal behavior.
///
/// ```
/// use vibe_orchestrator::ports::RunObserver;
/// fn takes(_: &dyn RunObserver) {}
/// ```
pub trait RunObserver: Send + Sync {
    /// How a handler's child process streams are wired.
    fn stream_mode(&self) -> StreamMode;

    /// Whether a `binary` handler's build output is suppressed.
    fn binary_quiet(&self) -> bool;

    /// Whether a carried executed failure owns an immediate machine document.
    fn emit_machine_failure(&self) -> bool;

    /// Observe the selected ritual before any contribution runs.
    fn observe_plan(
        &self,
        plan: &RitualPlan,
        metadata: &RunMetadata,
        emit_empty: bool,
    ) -> Result<()>;

    /// Observe one finished contribution row.
    fn observe_contribution(&self, report: &LifecycleContributionReport);

    /// Report the UNTRACKED clean epoch's failure document.
    ///
    /// That epoch keeps no state record and its wipe destroys the tree a trace
    /// would live in, so it never opens a session and has no outer funnel to
    /// hand a measurement to. This is the one place a surface owns the whole
    /// document for a run that will not reach a funnel at all.
    fn observe_untracked_failure(
        &self,
        metadata: &RunMetadata,
        phase: &str,
        contributions: &[LifecycleContributionReport],
    ) -> Result<()>;
}

/// One narration event of the prerequisite/direct install.
///
/// Every variant is a VALUE the surface renders in its own voice; no formatted
/// string crosses this boundary.
///
/// ```
/// use vibe_orchestrator::ports::InstallNarration;
/// let event = InstallNarration::FreshLock;
/// assert!(matches!(event, InstallNarration::FreshLock));
/// ```
#[derive(Debug)]
pub enum InstallNarration<'a> {
    /// Nothing is declared: the empty world is being regenerated.
    EmptyWorld,
    /// The lockfile is fresh, so resolution is skipped.
    FreshLock,
    /// The solved plan, immediately before the confirmation gate.
    Resolution(&'a [ResolvedDep]),
    /// PROP-050 `##VERIFY-LOCK-DIFF`, after a successful apply.
    ClosureDiff {
        /// The pre-apply lockfile snapshot.
        old: &'a Lockfile,
        /// The freshly written lockfile.
        new: &'a Lockfile,
        /// Watched boot-lane byte sizes sampled before the apply.
        lanes_before: &'a [(String, Option<u64>)],
        /// The same lanes sampled after it.
        lanes_after: &'a [(String, Option<u64>)],
    },
}

/// The observation policy of the CHILD install surface.
///
/// ```
/// use vibe_orchestrator::ports::InstallObserver;
/// fn takes(_: &dyn InstallObserver) {}
/// ```
pub trait InstallObserver: Send + Sync {
    /// How the slot lifecycle's child process streams are wired. This is the
    /// CHILD formula and is deliberately not [`RunObserver::stream_mode`].
    fn stream_mode(&self) -> StreamMode;

    /// Whether a measured slot failure owns an immediate machine document.
    fn emit_machine_failure(&self) -> bool;

    /// Render one narration event.
    fn narrate(&self, event: InstallNarration<'_>);

    /// Sample the watched boot lanes under `root` for the closure diff.
    fn lane_sizes(&self, root: &Path) -> Vec<(String, Option<u64>)>;

    /// The typed plan events of the install planner.
    fn plan_events(&self) -> &dyn vibe_install::PlanObserver;

    /// The slot-lifecycle observer this run narrates through.
    fn slot_observer(&self, metadata: &RunMetadata)
    -> Arc<dyn vibe_install::SlotLifecycleObserver>;
}

/// Confirmation policy supplied by the command surface.
///
/// The install core asks once, after it has a solved Ready plan and before any
/// materialisation. A headless adapter approves without importing a terminal
/// library.
///
/// ```
/// use vibe_orchestrator::ports::ConfirmGate;
/// fn takes(_: &dyn ConfirmGate) {}
/// ```
pub trait ConfirmGate: Send + Sync {
    /// Approve materialising `packages` package(s), or refuse typed.
    fn confirm_install(&self, packages: usize) -> Result<()>;
}

/// The registry environment ONE install run resolves — seeded, then loaded.
///
/// This replaced a pair of independent lookups whose ORDER was the defect. The
/// core used to load `GlobalRegistryConfig` early and ask the surface for its
/// embedded root much later, past the empty-world fast path; but on a source
/// install the surface's own closure is what SEEDS the machine-global defaults
/// the load then reads. On a fresh machine that ordering made the first run
/// fail and the second succeed — the classic "run it twice" bug, and one no
/// gate downstream could see, because both halves were individually correct.
///
/// So it is one call producing one snapshot, and the implementation owes the
/// order: seed first, load second. The core cannot get it wrong because the
/// core no longer performs either step.
///
/// It is deliberately NOT `Send + Sync`: the preparation is one synchronous
/// question asked inside one call, and the bound would force every surface to
/// make its own composition-root closure thread-safe for no gain.
///
/// ```
/// use vibe_orchestrator::ports::RegistryEnvironment;
/// fn takes(_: &dyn RegistryEnvironment) {}
/// ```
pub trait RegistryEnvironment {
    /// Seed whatever registry defaults this surface owns, THEN load the
    /// machine-global configuration, and hand both back as one value.
    fn prepare(&self) -> Result<RegistryEnvironmentSnapshot>;
}

/// The ONE registry environment snapshot an install run owns.
///
/// See [`RegistryEnvironment`] for why the two travel together.
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct RegistryEnvironmentSnapshot {
    /// The in-tree `packages/` root of a source install, when there is one.
    pub embedded_root: Option<PathBuf>,
    /// The machine-global registry configuration, loaded AFTER the seed.
    pub global: GlobalRegistryConfig,
}

/// Borrowed inputs at the one package-source construction point.
///
/// The surface's own argument grammar is deliberately absent: a factory closes
/// over whatever registry/solver flags it owns.
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct PackageSourceBuild<'a> {
    /// The selected node's manifest snapshot.
    pub manifest: &'a Manifest,
    /// The embedded-registry root, already resolved for this run.
    pub embedded_root: Option<&'a Path>,
    /// The one canonical selection of this command's project root.
    pub project_root: &'a Path,
    /// The machine-global registry configuration this command loaded.
    pub global: &'a GlobalRegistryConfig,
    /// The resolved offline posture.
    pub offline: bool,
    /// The pre-apply lockfile's members, as the provenance channel.
    pub locked: &'a [LockedPackage],
}

/// The opaque package source one install run owns.
///
/// It is the install substrate's own [`vibe_install::InstallSource`] plus the
/// one input-boundary capability the core needs and cannot own: short-name
/// qualification, which is a property of the surface's grammar and carries the
/// surface's own ambiguity exit code.
///
/// ```
/// use vibe_orchestrator::ports::PackageSource;
/// fn takes(_: &dyn PackageSource) {}
/// ```
pub trait PackageSource: vibe_install::InstallSource {
    /// Qualify one surface-supplied pkgref. An already-qualified reference
    /// passes through untouched; an ambiguous short name is the surface's own
    /// typed refusal, returned unchanged so its historical top-level wording
    /// and presentation survive. An ordinary context wrapper would NOT hide
    /// the typed error from a chain-walking downcast; replacing or
    /// translating the typed error is what would destroy its identity.
    fn qualify(
        &self,
        pkgref: &vibe_core::PackageRef,
        locked: &Lockfile,
    ) -> Result<vibe_core::PackageRef>;
}

/// The surface's own manifest mutation, applied at the ONE position the
/// install core has always applied it: after the manifest and workspace are
/// consumed, and before the global registry config is loaded.
///
/// `vibe install --git <url> --tag/branch/rev <ref>` records a git-source
/// declaration on `[requires.packages]` before resolving. That grammar — which
/// flags exist, how they combine, what an unknown auth kind is called — is the
/// SURFACE's, and its refusals carry the surface's own exit codes, so nothing
/// about it belongs here. The core only knows that a surface may mutate its
/// manifest at that exact point, and that a failure there is the surface's
/// error unchanged.
///
/// ```
/// use vibe_orchestrator::ports::InstallManifestMutation;
/// fn takes(_: &dyn InstallManifestMutation) {}
/// ```
pub trait InstallManifestMutation {
    /// Apply the surface's mutation to the snapshot and replay it onto the
    /// loaded tree. A surface with nothing to record does nothing.
    fn apply(
        &self,
        manifest: &mut Manifest,
        workspace: &mut vibe_workspace::Workspace,
        project_root: &Path,
    ) -> Result<()>;
}

/// The named no-op: a surface that records nothing.
///
/// Lifecycle phase verbs and a hosted surface never admit a manifest-mutating
/// flag, so this is what they inject — a named type rather than an empty
/// closure, so "this surface records nothing" is a statement in the call graph.
///
/// ```
/// use vibe_orchestrator::ports::{InstallManifestMutation, NoManifestMutation};
/// fn takes(_: &dyn InstallManifestMutation) {}
/// takes(&NoManifestMutation);
/// ```
pub struct NoManifestMutation;

impl InstallManifestMutation for NoManifestMutation {
    fn apply(
        &self,
        _manifest: &mut Manifest,
        _workspace: &mut vibe_workspace::Workspace,
        _project_root: &Path,
    ) -> Result<()> {
        Ok(())
    }
}

/// The post-durability stage a surface runs once the install's world exists.
///
/// Named and typed rather than a closure: the callback is where a surface used
/// to smuggle its whole rendering context into the shared core, and a
/// `FnOnce` cannot say which surface state it captured. It is one-shot BY USE —
/// the core consumes it exactly once, on the one branch that completes — and
/// `&mut self` lets an implementation collect what it saw.
///
/// ```
/// use vibe_orchestrator::ports::AfterDurableWorld;
/// fn takes(_: &mut dyn AfterDurableWorld) {}
/// ```
pub trait AfterDurableWorld {
    /// Run the stage against the world this install just made durable, over
    /// the CURRENT workspace by borrow — the one this execution loaded and, on
    /// a `--git` run, mutated in place.
    fn after(
        &mut self,
        project_root: &Path,
        run: crate::InstallRunContext,
        workspace: &vibe_workspace::Workspace,
    ) -> Result<crate::WorldCallbackOutcome>;
}

/// The named no-op post-durability stage.
///
/// `vibe update --all` delegates to the install core but owns no lifecycle
/// stage of its own; this is what it injects.
///
/// ```
/// use vibe_orchestrator::ports::{AfterDurableWorld, NoAfterDurableWorld};
/// fn takes(_: &mut dyn AfterDurableWorld) {}
/// takes(&mut NoAfterDurableWorld);
/// ```
pub struct NoAfterDurableWorld;

impl AfterDurableWorld for NoAfterDurableWorld {
    fn after(
        &mut self,
        _project_root: &Path,
        _run: crate::InstallRunContext,
        _workspace: &vibe_workspace::Workspace,
    ) -> Result<crate::WorldCallbackOutcome> {
        Ok(crate::WorldCallbackOutcome::default())
    }
}

/// Construction port for the package source an install run owns.
///
/// Deliberately invoked only after the empty-world fast path: constructing a
/// multi-registry source can inspect registry state and credentials, so a
/// lifecycle which needs no packages must never pay for it.
///
/// ```
/// use vibe_orchestrator::ports::PackageSourceFactory;
/// fn takes(_: &dyn PackageSourceFactory) {}
/// ```
pub trait PackageSourceFactory: Send + Sync {
    /// Build the source for one run.
    fn build(&self, input: PackageSourceBuild<'_>) -> Result<Box<dyn PackageSource>>;
}
