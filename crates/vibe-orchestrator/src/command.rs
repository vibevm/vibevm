//! The lease-first shared default-lifecycle command API — the TWO-stage
//! composition every surface executes an ordinary phase verb through
//! (R7.4 A15b).
//!
//! Stage one, [`lease_default_lifecycle`], resolves the root and takes the
//! outermost mutation lease — nothing else. Stage two,
//! [`prepare_default_lifecycle`], takes the ONE selected-manifest snapshot,
//! chooses the durable identity, and hands back an opaque
//! [`PreparedDefaultLifecycle`] whose [`run`](PreparedDefaultLifecycle::run)
//! executes the derived chain. The split is load-bearing: a surface must
//! load its own user configuration BETWEEN the two calls, and a one-shot
//! `prepare_command(policy)` would force policy/config to be computed
//! BEFORE the lease — the exact reordering this API exists to make
//! unrepresentable.
//!
//! A surface supplies only its ports ([`DefaultLifecyclePorts`]) and the
//! neutral [`DefaultLifecycleRequest`]; the chain and phases are DERIVED
//! from `requested` inside, so no caller can forge them, reread the
//! selected manifest, or move user config ahead of the outer lease. No
//! provider, model, credential or rendering type crosses — the surface
//! injects an already-built [`AgentBackend`], and the outcome is the
//! neutral [`PhaseOutcome`] whose report family and trace finalisation stay
//! surface projections.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_lifecycle::inclusive_chain;
use vibe_lifecycle::{AgentBackend, LifecycleLease, Phase, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::shared::Timestamp;
use vibe_workspace::compile_trace::TraceRun;

use crate::install::{
    InstallInputs, InstallPolicy, SelectedManifest, acquire_lease, resolve_project_root,
};
use crate::phase::{PhaseOutcome, PhaseRun, run_phases};
use crate::ports::{
    ConfirmGate, InstallManifestMutation, InstallObserver, PackageSourceFactory,
    RegistryEnvironment, RunObserver,
};
use crate::prelude::{RunPrelude, run_prelude};
use crate::trace::{Clock, TracePreparation};

#[cfg(test)]
#[path = "command/tests.rs"]
mod tests;

/// The leased stage of a default-lifecycle command: the ONE canonical
/// selected root plus the outermost mutation lease over it — and nothing
/// else.
///
/// Produced only by [`lease_default_lifecycle`]; the fields are private and
/// the value is opaque, so it cannot be forged, and [`prepare_default_lifecycle`]
/// is the only consumer. It deliberately takes no execution-shaped selected
/// snapshot, user-config, state or run-id read: those belong to the second
/// stage, after the surface has loaded its user config. (The lease's own
/// read-only workspace-locator discovery does run here — that is what
/// decides WHICH root the lease pins.)
///
/// A foreign crate can neither forge the stage nor take it apart:
///
/// ```compile_fail,E0451
/// use std::path::PathBuf;
/// use vibe_orchestrator::LeasedDefaultLifecycle;
///
/// fn forge(selected_root: PathBuf, lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>)
///     -> LeasedDefaultLifecycle {
///     LeasedDefaultLifecycle { selected_root, lease }
/// }
/// # let _ = forge;
/// ```
///
/// ```compile_fail,E0451
/// use vibe_orchestrator::LeasedDefaultLifecycle;
///
/// fn take_apart(stage: LeasedDefaultLifecycle) {
///     let LeasedDefaultLifecycle { selected_root, lease } = stage;
///     let _ = (selected_root, lease);
/// }
/// # let _ = take_apart;
/// ```
///
/// ```no_run
/// use std::path::Path;
/// use vibe_orchestrator::LeasedDefaultLifecycle;
/// # fn staged(path: &Path) -> anyhow::Result<LeasedDefaultLifecycle> {
/// // Stage one resolves and leases; the surface's config load goes here,
/// // between the two calls.
/// vibe_orchestrator::lease_default_lifecycle(path)
/// # }
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct LeasedDefaultLifecycle {
    selected_root: PathBuf,
    lease: Arc<LifecycleLease>,
}

/// Stage one: resolve the root once and take the outermost lease once —
/// exactly two ordered calls, [`resolve_project_root`] then
/// [`acquire_lease`] (whose own workspace-locator discovery is read-only
/// and decides which root the lease pins). A contended workspace refuses
/// typed, before any run id, state row, config byte or selected-manifest
/// snapshot exists. Everything execution-shaped is loaded AFTER this
/// returns, by the second stage and the surface's own config load, so no
/// pre-lease snapshot can go stale under a concurrent mutator this lease
/// just refused.
///
/// ```no_run
/// use std::path::Path;
/// # fn lease(path: &Path) -> anyhow::Result<()> {
/// let leased = vibe_orchestrator::lease_default_lifecycle(path)?;
/// // … the surface loads its ONE user configuration here, then stage two:
/// # let _ = leased;
/// # Ok(())
/// # }
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn lease_default_lifecycle(path: &Path) -> Result<LeasedDefaultLifecycle> {
    let selected_root = resolve_project_root(path)?;
    let lease = acquire_lease(&selected_root)?;
    Ok(LeasedDefaultLifecycle {
        selected_root,
        lease,
    })
}

/// The neutral inputs of one default-lifecycle command — exactly what a
/// surface owns, and not one field more.
///
/// The chain and phases are NOT here: they are derived from `requested`
/// inside [`prepare_default_lifecycle`], so no caller can forge a chain,
/// skip a prior phase, or smuggle a second spelling of the offline posture
/// or the user config beside the policy that already decided it. The
/// exact-field structural RED in this crate's tests turns any eighth
/// carrier into a compile error.
///
/// ```
/// use vibe_lifecycle::Phase;
/// use vibe_orchestrator::DefaultLifecycleRequest;
///
/// let request = DefaultLifecycleRequest {
///     requested: Phase::Build,
///     force: false,
///     agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
///     assume_yes: true,
///     trace_flag: false,
///     install_inputs: Default::default(),
///     policy: Default::default(),
/// };
/// assert_eq!(request.requested, Phase::Build);
/// ```
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct DefaultLifecycleRequest {
    /// The last canonical phase this run executes.
    pub requested: Phase,
    /// The generic lifecycle force bit ("fresh run id, no probe, repark").
    pub force: bool,
    /// The surface's resolved agent mode.
    pub agent_mode: RunAgentMode,
    /// The surface's effective assume-yes posture (flags, unattended, JSON).
    pub assume_yes: bool,
    /// The RAW compile-trace flag request — the selected manifest's own
    /// `[compile] trace` vote is combined with it in stage two, and the
    /// metadata carries the EFFECTIVE bit the identity selector computed.
    pub trace_flag: bool,
    /// The prerequisite install's surface-neutral inputs.
    pub install_inputs: InstallInputs,
    /// The prerequisite install's narrow, already-resolved policy. Its
    /// `offline` is the run's offline posture — the surface collapsed its
    /// whole ladder (flags > env > config) into it before this call.
    pub policy: InstallPolicy,
}

/// Stage two's opaque result: everything a default-lifecycle run owns,
/// privately.
///
/// A surface reads projections ([`selected_root`](Self::selected_root),
/// [`workspace_root`](Self::workspace_root),
/// [`selected_manifest`](Self::selected_manifest),
/// [`metadata`](Self::metadata)) and hands its ports to the consuming
/// [`run`](Self::run). It cannot touch the prelude, the selection, the
/// lease identity, the derived chain or the metadata's construction — the
/// fields are private, and the exact-field structural RED in this crate's
/// tests turns any eighth carrier into a compile error. The compiled
/// external proof that the privacy holds is on [`LeasedDefaultLifecycle`];
/// this stage's fields are taken apart by the same law:
///
/// ```compile_fail,E0451
/// use vibe_orchestrator::PreparedDefaultLifecycle;
///
/// fn take_apart(prepared: PreparedDefaultLifecycle) {
///     let PreparedDefaultLifecycle { requested, .. } = prepared;
///     let _ = requested;
/// }
/// # let _ = take_apart;
/// ```
///
/// ```no_run
/// use vibe_orchestrator::PreparedDefaultLifecycle;
/// fn root_of(prepared: &PreparedDefaultLifecycle) -> &std::path::Path {
///     prepared.selected_root()
/// }
/// # let _ = root_of;
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct PreparedDefaultLifecycle {
    requested: Phase,
    phases: Vec<Phase>,
    chain: Vec<String>,
    metadata: RunMetadata,
    install_inputs: InstallInputs,
    policy: InstallPolicy,
    prelude: RunPrelude,
}

/// Stage two: one manifest snapshot, one identity, one metadata — over the
/// leased stage only.
///
/// `leased` is consumed by value, so a caller physically cannot prepare
/// twice over one lease or prepare over a root it forged. The order inside
/// is the order the CLI has always owed: the snapshot is taken at the
/// leased root, the effective trace request is computed from the request's
/// raw flag plus that snapshot, and the ONE identity selection — the first
/// step under the lease that can allocate (a scratch run directory, an
/// adopted run id) — runs only after both.
///
/// ```no_run
/// use std::path::Path;
/// use vibe_lifecycle::Phase;
/// use vibe_orchestrator::{DefaultLifecycleRequest, LeasedDefaultLifecycle};
/// # fn prepare(
/// #     leased: LeasedDefaultLifecycle,
/// # ) -> anyhow::Result<vibe_orchestrator::PreparedDefaultLifecycle> {
/// vibe_orchestrator::prepare_default_lifecycle(
///     leased,
///     DefaultLifecycleRequest {
///         requested: Phase::Build,
///         force: false,
///         agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
///         assume_yes: true,
///         trace_flag: false,
///         install_inputs: Default::default(),
///         policy: Default::default(),
///     },
/// )
/// # }
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn prepare_default_lifecycle(
    leased: LeasedDefaultLifecycle,
    request: DefaultLifecycleRequest,
) -> Result<PreparedDefaultLifecycle> {
    let LeasedDefaultLifecycle {
        selected_root,
        lease,
    } = leased;
    let DefaultLifecycleRequest {
        requested,
        force,
        agent_mode,
        assume_yes,
        trace_flag,
        install_inputs,
        policy,
    } = request;
    // The chain is DERIVED, never supplied: the canonical inclusive prefix
    // through `requested`, both as phases (for dispatch) and as the string
    // chain (for identity/state/reporting). A caller cannot forge either.
    let phases: Vec<Phase> = inclusive_chain(requested).to_vec();
    let chain: Vec<String> = phases.iter().map(Phase::to_string).collect();
    // The ONE selected-manifest snapshot, at the root the lease already
    // pinned — never at a raw `--path` again.
    let selection = SelectedManifest::read(&selected_root).prepare();
    // The effective trace request: the raw flag OR the snapshot's own
    // `[compile] trace` vote — combined once, here.
    let trace_request = selection.request(trace_flag);
    // The ONE identity selection, against the same lease.
    let prelude = run_prelude(
        selection,
        lease,
        &requested.to_string(),
        &chain,
        agent_mode.clone(),
        force,
        trace_request,
    )?;
    let metadata = RunMetadata {
        requested: requested.to_string(),
        chain: chain.clone(),
        offline: policy.offline,
        assume_yes,
        agent_mode,
        force,
        // The EFFECTIVE bit the one selector computed — an adopted run's
        // sticky trace bit, never the raw request.
        trace_compile: prelude.identity.compile_trace,
        run_id: prelude.identity.run_id.clone(),
        started: prelude.identity.started.clone(),
        selected: prelude.selected.clone(),
    };
    Ok(PreparedDefaultLifecycle {
        requested,
        phases,
        chain,
        metadata,
        install_inputs,
        policy,
        prelude,
    })
}

/// The ports one default-lifecycle run executes through — every surface
/// capability, and nothing either surface can compute itself.
///
/// Field-for-field the observation/confirmation/source/environment halves
/// of [`crate::ports`], plus the ONE already-built agent backend: the
/// surface injects it, this crate never constructs a provider.
///
/// ```no_run
/// use vibe_orchestrator::DefaultLifecyclePorts;
/// # fn takes<'a>(
/// #     observer: &'a dyn vibe_orchestrator::ports::RunObserver,
/// #     install_observer: &'a dyn vibe_orchestrator::ports::InstallObserver,
/// #     confirm: &'a dyn vibe_orchestrator::ports::ConfirmGate,
/// #     sources: &'a dyn vibe_orchestrator::ports::PackageSourceFactory,
/// #     environment: &'a dyn vibe_orchestrator::ports::RegistryEnvironment,
/// #     mutation: &'a dyn vibe_orchestrator::ports::InstallManifestMutation,
/// #     agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
/// # ) -> DefaultLifecyclePorts<'a> {
/// DefaultLifecyclePorts {
///     observer,
///     install_observer,
///     confirm_gate: confirm,
///     sources,
///     environment,
///     manifest_mutation: mutation,
///     agent,
/// }
/// # }
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct DefaultLifecyclePorts<'a> {
    /// The OUTER phase observation policy.
    pub observer: &'a dyn RunObserver,
    /// The CHILD install observation policy — never the same object.
    pub install_observer: &'a dyn InstallObserver,
    /// The surface's confirmation policy for the prerequisite install.
    pub confirm_gate: &'a dyn ConfirmGate,
    /// The surface's package-source composition root.
    pub sources: &'a dyn PackageSourceFactory,
    /// Where this run's registry environment is seeded and loaded — once.
    pub environment: &'a dyn RegistryEnvironment,
    /// The surface's own manifest mutation. A phase verb admits no
    /// manifest-mutating flag, so a surface injects the named no-op —
    /// stated in the call graph rather than assumed.
    pub manifest_mutation: &'a dyn InstallManifestMutation,
    /// The ONE agent backend every agent row of this run is served by.
    pub agent: Arc<dyn AgentBackend>,
}

impl PreparedDefaultLifecycle {
    /// The canonical selected root this command acts on — the value a
    /// surface's failure-family projection names.
    ///
    /// Cloned rather than re-resolved: two canonicalisations are two
    /// answers to "which node did this command act on".
    #[must_use]
    pub fn selected_root(&self) -> &Path {
        self.prelude.selection.root()
    }

    /// The workspace mutation lease's root — the canonical root a surface's
    /// agent backend is built from. The LEASE is the authority: even when the
    /// selection's own workspace load failed after the lease was taken
    /// (its `loaded_root` is `None`), this still names the root the lease
    /// pinned — never a fallback re-derived from the selection.
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        self.prelude.lease.root()
    }

    /// The ONE selected-manifest snapshot, by borrow. A CLI surface may
    /// clone `[llm]` from it; a hosted surface (A15c) must not call this
    /// accessor at all.
    #[must_use]
    pub fn selected_manifest(&self) -> Option<&Manifest> {
        self.prelude.selection.parsed_ref()
    }

    /// The run's durable identity and effective posture — the REQUIRED
    /// projection a surface reads to pin what the cell derived (the chain a
    /// `Build` request prepares, the effective offline posture, the adopted
    /// run id) without ever being able to supply or alter any of it.
    #[must_use]
    pub fn metadata(&self) -> &RunMetadata {
        &self.metadata
    }

    /// Retain an owner of the mutation lease, so the surface holds the
    /// workspace through trace finalisation AND presentation — dropping it
    /// earlier would release the cooperative lock while the last bytes this
    /// invocation owes are still being written.
    #[must_use]
    pub fn retain_lease(&self) -> Arc<LifecycleLease> {
        self.prelude.lease.clone()
    }

    /// Open this command's compile-trace owner — or stand down honestly.
    /// The CLOCK stays injected; nothing here reads time.
    pub fn prepare_trace(&self, clock: Clock<'_>) -> TracePreparation {
        self.prelude.prepare_trace(clock)
    }

    /// Execute the derived chain: the ONE composition, with the empty clean
    /// prefix a plain phase verb has always run.
    ///
    /// Consuming: the prepared value is used exactly once, the ports are
    /// read for the run's lifetime, and the recorder is the surface's
    /// borrowed owner. The neutral [`PhaseOutcome`] comes back — report
    /// family and [`crate::trace::finalize`] remain surface projections.
    ///
    /// `observed_at` is the surface's injected verify instant — the same
    /// clock that opened the trace and will finalise it. It is also the
    /// COMPLETE epoch's permission for the engine-owned verify boundary; the
    /// partial install callback withholds it deliberately.
    ///
    /// ```no_run
    /// use vibe_orchestrator::{DefaultLifecyclePorts, PreparedDefaultLifecycle};
    /// use vibe_orchestrator::PhaseOutcome;
    /// # fn go(
    /// #     prepared: PreparedDefaultLifecycle,
    /// #     ports: DefaultLifecyclePorts<'_>,
    /// #     trace: Option<&vibe_workspace::compile_trace::TraceRun>,
    /// #     observed_at: vibe_wire::generated::shared::Timestamp,
    /// # ) -> PhaseOutcome {
    /// prepared.run(ports, trace, observed_at)
    /// # }
    /// # let _ = go;
    /// ```
    pub fn run(
        self,
        ports: DefaultLifecyclePorts<'_>,
        trace: Option<&TraceRun>,
        observed_at: Timestamp,
    ) -> PhaseOutcome {
        let DefaultLifecyclePorts {
            observer,
            install_observer,
            confirm_gate,
            sources,
            environment,
            manifest_mutation,
            agent,
        } = ports;
        // A plain phase verb owns no clean epoch: the prefix is empty, and
        // the clean-composed epoch stays a surface-side composition.
        run_phases(PhaseRun {
            requested: self.requested,
            phases: self.phases,
            chain: self.chain,
            metadata: self.metadata,
            install_args: self.install_inputs,
            policy: self.policy,
            lease: self.prelude.lease,
            selection: self.prelude.selection,
            steps: Vec::new(),
            contributions: Vec::new(),
            notices: Vec::new(),
            observer,
            install_observer,
            confirm_gate,
            sources,
            environment,
            manifest_mutation,
            agent,
            trace,
            observed_at,
        })
    }
}
