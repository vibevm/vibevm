//! The report-neutral outcome values one lifecycle run produces.
//!
//! Nothing here renders: a surface attaches its own trace member, chooses its
//! registered root family and prints. These are the facts both surfaces agree
//! on.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use specmark::spec;
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleDelegation, LifecycleReport, LifecycleStepReport,
};
use vibe_wire::generated::shared::CompileTraceReport;

/// Everything the one lifecycle document reports, owned and surface-neutral.
///
/// ```
/// use vibe_orchestrator::values::LifecycleValues;
/// let failed = LifecycleValues::failed("build", vec!["build".into()], "build", Vec::new());
/// assert!(!failed.ok);
/// assert_eq!(failed.steps[0].status, "fail");
/// ```
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct LifecycleValues {
    /// Whether the run itself succeeded. A park is a SUCCESS with a handoff.
    pub ok: bool,
    /// The requested phase.
    pub requested: String,
    /// The complete requested chain.
    pub chain: Vec<String>,
    /// One row per step that really ran, truncated at a park.
    pub steps: Vec<LifecycleStepReport>,
    /// Every contribution row, in the order it happened.
    pub contributions: Vec<LifecycleContributionReport>,
    /// Non-fatal collection notices.
    pub notices: Vec<String>,
    /// The hosted handoff, already validated at the engine adapter.
    pub delegation: Option<LifecycleDelegation>,
}

impl LifecycleValues {
    /// A completed or parked phase run.
    ///
    /// The struct-level example builds the failed twin; this constructor takes
    /// the measured steps and rows instead.
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub fn completed(
        requested: &str,
        chain: Vec<String>,
        steps: Vec<LifecycleStepReport>,
        contributions: Vec<LifecycleContributionReport>,
        notices: Vec<String>,
        delegation: Option<LifecycleDelegation>,
    ) -> Self {
        Self {
            ok: true,
            requested: requested.to_string(),
            chain,
            steps,
            contributions,
            notices,
            delegation,
        }
    }

    /// A failed phase run: the rows measured up to the failure, one `fail` step
    /// for the phase it stopped at, and no handoff — a park is not a failure
    /// and a failure never parked.
    ///
    /// See the struct-level example.
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub fn failed(
        requested: &str,
        chain: Vec<String>,
        phase: &str,
        contributions: Vec<LifecycleContributionReport>,
    ) -> Self {
        Self {
            ok: false,
            requested: requested.to_string(),
            chain,
            steps: vec![LifecycleStepReport {
                phase: phase.to_string(),
                status: "fail".to_string(),
            }],
            contributions,
            notices: Vec::new(),
            delegation: None,
        }
    }

    /// The generated `cli-lifecycle-report` root, with the shared trace member
    /// attached — total, infallible, and the ONLY place a lifecycle report is
    /// built.
    ///
    /// It lives here rather than in a surface wrapper for the reason the whole
    /// boundary exists: the document is the SAME document whichever surface
    /// asked for the run, so a second constructor beside a second renderer is
    /// how two surfaces come to disagree about one run. The `trace` member is a
    /// parameter and never a field: the funnel decides it after the values are
    /// frozen, and `None` really means "the key is absent from the wire", which
    /// is the byte-for-byte law old corpora depend on.
    ///
    /// ```
    /// use vibe_orchestrator::values::LifecycleValues;
    /// let report = LifecycleValues::failed("build", vec!["build".into()], "build", Vec::new())
    ///     .into_report(None);
    /// assert_eq!(report.command, "lifecycle");
    /// assert!(!report.ok);
    /// assert!(report.trace.is_none(), "disabled omits the member entirely");
    /// ```
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub fn into_report(self, trace: Option<CompileTraceReport>) -> LifecycleReport {
        let Self {
            ok,
            requested,
            chain,
            steps,
            contributions,
            notices,
            delegation,
        } = self;
        LifecycleReport {
            chain,
            command: "lifecycle".to_string(),
            contributions,
            notices,
            ok,
            requested,
            steps,
            delegation,
            trace,
            // The R7.5 evidence member is attached by the verify
            // reconciliation that owns it (P2), never assembled here: a
            // report builder that could mint one would be a second
            // reference implementation of the identity.
            verification: None,
        }
    }
}

/// The status one default-lifecycle step reports.
///
/// ```
/// use vibe_orchestrator::values::StepStatus;
/// assert_eq!(StepStatus::Delegated.as_str(), "delegated");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub enum StepStatus {
    /// The step ran and changed something.
    Ok,
    /// Every contribution of the step reused a fingerprint.
    Fresh,
    /// The step selected no contribution.
    NoOp,
    /// The chain parked at this phase for the hosting agent; no later phase
    /// ran, and none is reported.
    Delegated,
}

impl StepStatus {
    /// The exact wire spelling.
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Fresh => "fresh",
            Self::NoOp => "no-op",
            Self::Delegated => "delegated",
        }
    }
}

/// One step row.
///
/// ```
/// use vibe_orchestrator::values::{StepStatus, step_report};
/// assert_eq!(step_report("build", StepStatus::Ok).phase, "build");
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn step_report(phase: &str, status: StepStatus) -> LifecycleStepReport {
    LifecycleStepReport {
        phase: phase.to_string(),
        status: status.as_str().to_string(),
    }
}

/// One slot-lifecycle row, projected into the shared contribution shape.
///
/// ```
/// use vibe_orchestrator::values::contribution_report;
/// fn takes(row: vibe_install::SlotLifecycleReport) {
///     let projected = contribution_report(row);
///     assert_eq!(projected.phase, "install");
/// }
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn contribution_report(
    report: vibe_install::SlotLifecycleReport,
) -> LifecycleContributionReport {
    LifecycleContributionReport {
        flagged: report.flagged.then_some(true),
        handler: report.handler,
        key: report.key,
        message: report.message,
        stderr: report.stderr,
        stderr_truncated: report.stderr_truncated.then_some(true),
        stdout: report.stdout,
        stdout_truncated: report.stdout_truncated.then_some(true),
        phase: "install".into(),
        point: report.point,
        provider: report.provider,
        reference: Some(report.reference),
        slot_target: report.slot_target.map(|target| {
            vibe_wire::generated::lifecycle_report::SlotTarget {
                group: target.group,
                kind: target.kind,
                name: target.name,
                root: target.root,
                version: target.version,
            }
        }),
        status: report.status,
        tier: report.tier,
        version: report.version,
    }
}

/// Validate a typed handoff at the engine adapter: a non-empty task list, and
/// every task the exact deterministic path the reported run owns. A machine
/// fact this load-bearing is never smuggled into a prose notice.
///
/// ```
/// use vibe_orchestrator::values::delegation_member;
/// let refused = delegation_member(vibe_lifecycle::Delegation {
///     resume: "vibe build".into(),
///     run_id: "0".repeat(32),
///     tasks: Vec::new(),
/// });
/// assert!(refused.is_err(), "an empty task list is never a handoff");
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub fn delegation_member(
    delegation: vibe_lifecycle::Delegation,
) -> anyhow::Result<LifecycleDelegation> {
    if delegation.tasks.is_empty() {
        anyhow::bail!(
            "internal: run `{}` reported a hosted handoff with no task file",
            delegation.run_id
        );
    }
    let home = format!("{}/{}/", vibe_lifecycle::OUTBOX_RELATIVE, delegation.run_id);
    for task in &delegation.tasks {
        let owned = task
            .strip_prefix(&home)
            .is_some_and(|name| !name.contains('/') && name.ends_with(".md"));
        anyhow::ensure!(
            owned,
            "internal: task `{task}` does not live directly under run `{}`",
            delegation.run_id,
        );
    }
    Ok(LifecycleDelegation {
        resume: delegation.resume,
        run_id: delegation.run_id,
        tasks: delegation.tasks,
    })
}

/// Validate a typed handoff without keeping the member.
///
/// ```
/// use vibe_orchestrator::values::check_delegation;
/// let refused = check_delegation(&vibe_lifecycle::Delegation {
///     resume: "vibe build".into(),
///     run_id: "0".repeat(32),
///     tasks: vec!["elsewhere/a.md".into()],
/// });
/// assert!(refused.is_err(), "a foreign task path is never a handoff");
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub fn check_delegation(delegation: &vibe_lifecycle::Delegation) -> anyhow::Result<()> {
    delegation_member(delegation.clone()).map(|_| ())
}
