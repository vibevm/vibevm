//! Neutral prerequisite-resume failures absorbed by the lifecycle owner.

use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::shared::VerificationEvidence;

use crate::failure::{MeasuredFailure, Measurement, take};
use crate::values::contribution_report;

/// Rows and verification evidence measured so far by the outer phase run.
#[derive(Default)]
pub(super) struct Measured {
    pub(super) contributions: Vec<LifecycleContributionReport>,
    pub(super) verification: Option<VerificationEvidence>,
}

/// Absorb a neutral resume failure into this command's accumulator while the
/// original error continues to the lifecycle-family fallback unchanged.
pub(super) fn absorb_resume_failure(
    error: anyhow::Error,
    measured: &mut Measured,
) -> anyhow::Error {
    match take(error) {
        Ok(MeasuredFailure {
            original,
            evidence: Measurement::Slot { reports, .. },
            ..
        }) => {
            measured
                .contributions
                .extend(reports.into_iter().map(contribution_report));
            original
        }
        Ok(other) => crate::failure::carry(other),
        Err(error) => error,
    }
}
