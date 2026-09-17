//! Values accumulated by one tracked dispatch pass.

use std::collections::BTreeMap;
use vibe_lifecycle::Delegation;
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::shared::VerificationEvidence;

/// What one dispatch pass produced.
#[derive(Debug, Default)]
pub(crate) struct DispatchOutcome {
    pub(crate) reports: Vec<LifecycleContributionReport>,
    pub(crate) parked: Option<(String, Delegation)>,
    pub(crate) verification: Option<VerificationEvidence>,
    /// Empty engine-owned phases terminalized at their fence. The outer phase
    /// owner uses these exact statuses and must not observe a second terminal.
    pub(crate) phase_terminals: BTreeMap<String, String>,
}

/// Rows and verification evidence measured before a dispatch stopped.
#[derive(Debug, Default)]
pub(super) struct MeasuredDispatch {
    pub(super) rows: Vec<LifecycleContributionReport>,
    pub(super) verification: Option<VerificationEvidence>,
}
