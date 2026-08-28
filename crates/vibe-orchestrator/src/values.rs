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
use vibe_wire::generated::shared::{CompileTraceReport, VerificationEvidence};

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
    /// The ONE verification-evidence member the lifecycle library produced,
    /// carried verbatim.
    ///
    /// Present exactly when this run reached engine-owned verify
    /// reconciliation — INCLUDING the `stale`, `missing` and `unstable`
    /// outcomes, which is why it rides the failure carrier too. It is an
    /// independent axis from [`Self::ok`]: a matched identity may sit beside a
    /// failed verify contribution, and neither rewrites the other.
    pub verification: Option<VerificationEvidence>,
}

impl LifecycleValues {
    /// A completed or parked phase run that reached no verify reconciliation.
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
        Self::completed_with_verification(
            requested,
            chain,
            steps,
            contributions,
            notices,
            delegation,
            None,
        )
    }

    /// The same completed or parked run, carrying whatever the verify boundary
    /// reconciled.
    ///
    /// Deliberately not named `*_verified`: the member's status may be any of
    /// the five evidence words, and `matched` is only one of them.
    ///
    /// ```
    /// use vibe_orchestrator::values::LifecycleValues;
    /// let values = LifecycleValues::completed_with_verification(
    ///     "verify", vec!["verify".into()], Vec::new(), Vec::new(), Vec::new(), None, None,
    /// );
    /// assert!(values.ok && values.verification.is_none());
    /// ```
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES")]
    pub fn completed_with_verification(
        requested: &str,
        chain: Vec<String>,
        steps: Vec<LifecycleStepReport>,
        contributions: Vec<LifecycleContributionReport>,
        notices: Vec<String>,
        delegation: Option<LifecycleDelegation>,
        verification: Option<VerificationEvidence>,
    ) -> Self {
        Self {
            ok: true,
            requested: requested.to_string(),
            chain,
            steps,
            contributions,
            notices,
            delegation,
            verification,
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
        Self::failed_with_verification(requested, chain, phase, contributions, None)
    }

    /// The same failed run, carrying the member the failing site had already
    /// measured — a stale/missing/unstable stop, or a matched identity a later
    /// verify handler then failed beside.
    ///
    /// ```
    /// use vibe_orchestrator::values::LifecycleValues;
    /// let failed = LifecycleValues::failed_with_verification(
    ///     "verify", vec!["verify".into()], "verify", Vec::new(), None,
    /// );
    /// assert!(!failed.ok && failed.verification.is_none());
    /// ```
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES")]
    pub fn failed_with_verification(
        requested: &str,
        chain: Vec<String>,
        phase: &str,
        contributions: Vec<LifecycleContributionReport>,
        verification: Option<VerificationEvidence>,
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
            verification,
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
    /// assert!(report.verification.is_none(), "and so does an unreconciled run");
    /// ```
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES")]
    pub fn into_report(self, trace: Option<CompileTraceReport>) -> LifecycleReport {
        let Self {
            ok,
            requested,
            chain,
            steps,
            contributions,
            notices,
            delegation,
            verification,
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
            // ATTACHED, never assembled: the member arrives exactly as the
            // lifecycle library minted it, and this builder neither mints,
            // reinterprets nor reshapes one. A report builder that could
            // build an identity would be a second reference implementation
            // of it.
            verification,
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

#[cfg(test)]
mod verification_tests {
    use super::LifecycleValues;
    use vibe_wire::behaviour::verification_evidence::validate;
    use vibe_wire::generated::shared::{
        DigestWitness, EvidenceRun, EvidenceStatus, InputMeasurement, VerificationEvidence,
    };

    fn witness(byte: char) -> DigestWitness {
        DigestWitness {
            algorithm: "sha256:vibe-input-manifest-v1".into(),
            bytes: Some("3".into()),
            digest: format!("sha256:{}", byte.to_string().repeat(64)),
            files: Some(1),
        }
    }

    /// A member whose ROW status carries the root's — the wire refuses a root
    /// that speaks for itself, which is exactly the law that makes `matched`
    /// mean "every row matched".
    fn member(status: EvidenceStatus) -> VerificationEvidence {
        let stale = status == EvidenceStatus::Stale;
        VerificationEvidence {
            artifacts: Vec::new(),
            evidence: 1,
            evidence_id: format!("sha256:{}", "b".repeat(64)),
            inputs: vec![InputMeasurement {
                declaration_fingerprint: format!("sha256:{}", "c".repeat(64)),
                execution: "org.demo/tools#compile".into(),
                patterns: vec!["data/**".into()],
                phase: "build".into(),
                status: status.clone(),
                measured: Some(witness('1')),
                measured_run_id: Some("0".repeat(32)),
                observed: Some(witness(if stale { '2' } else { '1' })),
                reason_code: None,
            }],
            observed_at: "2026-08-28T12:00:05Z".parse().expect("a fixture instant"),
            run: EvidenceRun {
                chain: vec!["build".into(), "verify".into()],
                requested: "verify".into(),
                run_id: "0".repeat(32),
                selected: ".".into(),
                started: "2026-08-28T11:59:40Z".into(),
            },
            status,
        }
    }

    /// The ONE builder ATTACHES: what goes in comes out, byte for byte, on
    /// both the success and the failure road. A builder that reshaped it would
    /// be a second reference implementation of the identity.
    #[test]
    fn the_report_builder_attaches_the_member_it_was_handed() {
        for (status, ok) in [
            (EvidenceStatus::Matched, true),
            (EvidenceStatus::Stale, false),
        ] {
            let expected = member(status.clone());
            validate(&expected).expect("the fixture member is itself valid");
            let report = if ok {
                LifecycleValues::completed_with_verification(
                    "verify",
                    vec!["verify".into()],
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    None,
                    Some(expected.clone()),
                )
            } else {
                LifecycleValues::failed_with_verification(
                    "verify",
                    vec!["verify".into()],
                    "verify",
                    Vec::new(),
                    Some(expected.clone()),
                )
            }
            .into_report(None);

            assert_eq!(report.ok, ok, "the command's own axis");
            assert_eq!(
                report.verification,
                Some(expected),
                "and the identity axis, untouched by it",
            );
        }
    }

    /// A run that reached no boundary omits the KEY, not merely its value:
    /// every pre-R7.5 document stays byte-shape compatible.
    #[test]
    fn an_unreconciled_run_omits_the_member_from_the_wire() {
        let report = LifecycleValues::completed(
            "build",
            vec!["build".into()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
        )
        .into_report(None);
        assert!(report.verification.is_none());
        let json = serde_json::to_string(&report).expect("the root serialises");
        assert!(
            !json.contains("verification"),
            "an absent member is an absent key: {json}",
        );
    }
}
