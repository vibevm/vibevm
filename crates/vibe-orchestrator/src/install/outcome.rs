//! What one install invocation DID, in the shape its caller renders — and the
//! typed envelopes the post-durability stage is handed and hands back.
//!
//! Split out of `install/mod.rs` at its real seam: that cell is the DECISION
//! (empty world / fresh fast path / ready apply) and this one is the vocabulary
//! the decision reports in. Nothing here executes.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use specmark::spec;

use std::path::{Path, PathBuf};

use vibe_install::SlotLifecycleReport;
use vibe_lifecycle::{LifecycleRunHandle, RunMetadata};

/// Whether the existing install implementation applied a plan or proved the
/// materialised world fresh. Lifecycle callers consume this instead of
/// inferring machine state from rendered text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub enum InstallDisposition {
    /// The materialised world was already fresh.
    Fresh,
    /// A solved plan was applied.
    Applied,
    /// A hosted `agent` row parked for the hosting agent. The install stopped
    /// AT THAT ROW's point and did NOT render: a park travels outward as a
    /// value so the outermost command, and only it, emits the one document.
    ///
    /// How much is durable when that happens is point-dependent, and nothing
    /// here assumes. A `slot:pre-install` park precedes the remaining
    /// materialisation, the lockfile barrier and every post-barrier row; a
    /// `slot:post-install` or `phase:install` park follows a COMPLETE, durable
    /// apply and stops only what came after it. The accompanying
    /// [`vibe_install::InstallProgress`] is the boundary-measured record of
    /// which of those it was.
    Parked,
}

/// What one install invocation did, in the shape its caller renders.
///
/// Nothing in the install substrate prints a report any more: `vibe install`
/// renders a `cli-install-report`, a phase verb renders its own
/// `cli-lifecycle-report`, and update/reinstall render theirs. Returning the
/// outcome instead of printing it is what makes "exactly one document" a
/// property of the call graph rather than a hope at each call site.
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct InstallRun {
    /// Whether this invocation applied a plan, proved the world fresh, or
    /// parked.
    pub disposition: InstallDisposition,
    /// The boundary-measured record of what became durable.
    pub progress: vibe_install::InstallProgress,
    /// How many packages this invocation RESOLVED — the solved graph's size,
    /// counted where the plan is produced. Not `materialised.len()`: a slot
    /// that was already present is resolved and skipped, and reading the
    /// count off the materialised list would silently under-report exactly
    /// the runs that changed the least. Zero on the fresh fast path, which
    /// resolves nothing at all.
    pub packages_resolved: usize,
    /// The hook reports this apply produced.
    pub hooks: Vec<vibe_workspace::hooks::HookReport>,
    /// The slot-lifecycle rows this apply produced.
    pub slot_reports: Vec<SlotLifecycleReport>,
    /// The phase-ritual rows the post-durability callback produced.
    pub contributions: Vec<vibe_wire::generated::lifecycle_report::LifecycleContributionReport>,
    /// Non-fatal notices.
    pub notices: Vec<String>,
    /// The typed handoff, when a hosted row parked.
    pub parked: Option<vibe_lifecycle::Delegation>,
    /// The narration counts of the post-durability observer.
    pub world_summary: WorldCallbackSummary,
    /// The one canonical project root this run acted on.
    pub project_root: PathBuf,
}

impl InstallRun {
    /// An empty run at `project_root` with the given disposition.
    ///
    /// ```
    /// use vibe_orchestrator::{InstallDisposition, InstallRun};
    /// let run = InstallRun::new(std::path::PathBuf::from("."), InstallDisposition::Fresh);
    /// assert_eq!(run.packages_resolved, 0);
    /// ```
    #[must_use]
    pub fn new(project_root: PathBuf, disposition: InstallDisposition) -> Self {
        Self {
            disposition,
            progress: vibe_install::InstallProgress::default(),
            packages_resolved: 0,
            hooks: Vec::new(),
            slot_reports: Vec::new(),
            contributions: Vec::new(),
            notices: Vec::new(),
            parked: None,
            world_summary: WorldCallbackSummary::default(),
            project_root,
        }
    }
}

/// The run one FRESH (or empty-world) path produces.
pub(crate) fn fresh_run(
    project_root: &Path,
    nodes: Vec<String>,
    world: WorldCallbackOutcome,
) -> InstallRun {
    let mut run = InstallRun::new(
        project_root.to_path_buf(),
        if world.parked.is_some() {
            InstallDisposition::Parked
        } else {
            InstallDisposition::Fresh
        },
    );
    run.progress = vibe_install::InstallProgress::fresh(nodes);
    run.contributions = world.contributions;
    run.notices = world.notices;
    run.parked = world.parked;
    run.world_summary = world.summary;
    run
}

/// Effective invocation facts the durable-world lifecycle callback needs in
/// the canonical handler envelope.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct InstallRunContext {
    /// The invocation's durable identity and effective posture.
    pub metadata: RunMetadata,
    /// The command's mutation lease, shared by Arc into the callback: the
    /// post-durability world dispatch reuses this proof and never
    /// reacquires. Present on every path — the callback may run when no slot
    /// lifecycle exists at all (the empty-world no-op, the fresh fast path),
    /// and its dispatch still needs the one owner.
    pub lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    /// The slot run this install built, when it built one.
    pub lifecycle_run: Option<LifecycleRunHandle>,
    /// The slot rows it produced.
    pub lifecycle_reports: Vec<SlotLifecycleReport>,
}

/// Counts produced by an additive post-durability observer. Keeping them
/// typed prevents quiet rendering from dropping a class of ritual output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct WorldCallbackSummary {
    /// Contributions the ritual selected.
    pub selected_contributions: usize,
    /// Contributions that really executed.
    pub executed_contributions: usize,
    /// Contributions that succeeded.
    pub successful_contributions: usize,
    /// Contributions that reused a fingerprint.
    pub fresh_contributions: usize,
    /// Non-fatal notices.
    pub notices: usize,
}

/// What the post-durability observer produced. The counts are the narration;
/// the rows and the handoff are machine facts the OUTERMOST command folds into
/// its single document — the observer itself renders nothing.
#[derive(Debug, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct WorldCallbackOutcome {
    /// The narration counts.
    pub summary: WorldCallbackSummary,
    /// The phase-ritual rows.
    pub contributions: Vec<vibe_wire::generated::lifecycle_report::LifecycleContributionReport>,
    /// Non-fatal notices.
    pub notices: Vec<String>,
    /// The typed handoff, when a hosted row parked.
    pub parked: Option<vibe_lifecycle::Delegation>,
}
