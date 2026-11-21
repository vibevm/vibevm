//! What one install execution is handed by the command that owns it.
//!
//! Split from the install core along the seam it already carried: the
//! request vocabulary changes when a caller gains a decision to pass down,
//! the core when the install sequence itself changes.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use specmark::spec;
use vibe_lifecycle::{AgentBackend, RunMetadata};
use vibe_workspace::compile_trace::TraceRun;

use super::{InstallInputs, InstallPolicy, PreparedSelection};
use crate::ports::{
    ConfirmGate, InstallManifestMutation, InstallObserver, PackageSourceFactory,
    RegistryEnvironment,
};

/// Everything one install execution needs that its caller already decided.
///
/// The identity is NOT selected here any more. A caller either owns the
/// command (and selected one identity together with its trace request, before
/// anything was allocated) or is chained inside one (and carries that outer
/// metadata unchanged). A fallback selection at this depth was a second
/// selector: it ran after the config load, could allocate a second run
/// directory, and had no way to know the effective trace bit its caller had
/// already committed to.
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct InstallExecution<'a> {
    /// The surface-neutral install inputs.
    pub args: InstallInputs,
    /// Where this run's registry environment is seeded and loaded — once.
    pub environment: &'a dyn RegistryEnvironment,
    /// The narrow execution policy: the already-resolved offline posture, the
    /// slot-integrity strategy and the operator's spec-format default.
    pub policy: InstallPolicy,
    /// The command's mutation lease — the outermost lock, acquired by the
    /// owning boundary (direct install, a phase verb, update, reinstall)
    /// before anything execution-shaped was read. The install substrate
    /// never acquires; it consumes this proof and shares it onward by Arc.
    pub lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    /// The command's ONE selected-world provenance bundle: the canonical root,
    /// the manifest snapshot taken at it, and the tree built from THAT
    /// snapshot — see [`PreparedSelection`]. One value rather than three
    /// fields, so a caller cannot hand this boundary a manifest from one moment
    /// and a tree from another. Proven below, at the boundary that historically
    /// performed the read.
    pub selection: PreparedSelection,
    /// The invocation's durable identity and effective posture.
    pub metadata: RunMetadata,
    /// The surface's own manifest mutation, applied at its one position: the
    /// grammar and the exit codes stay in the surface, the position stays here.
    pub manifest_mutation: &'a dyn InstallManifestMutation,
    /// The caller-selected package-source composition root. Borrowed, not
    /// recreated inside the install kernel, so a hosted surface can supply a
    /// credential-free policy without a surface back-edge.
    pub sources: &'a dyn PackageSourceFactory,
    /// The surface's one confirmation policy. The core invokes it only for a
    /// solved Ready plan, immediately before materialisation.
    pub confirm_gate: &'a dyn ConfirmGate,
    /// The CHILD install narration policy — deliberately not the phase
    /// observer's.
    pub observer: &'a dyn InstallObserver,
    /// The agent backend every agent row of this run is served by: the slot
    /// barrier, the phase dispatch and any resume share this ONE injection.
    pub agent: std::sync::Arc<dyn AgentBackend>,
    /// The command owner's recorder, borrowed. `None` is not "off" here — it
    /// is "this caller's command is not tracing", which is the same thing to
    /// every layer below.
    pub trace: Option<&'a TraceRun>,
}
