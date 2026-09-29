//! Pure-planner publication checks over the generated outer and domain roots.
//! This is one owned route, not a generic machine-envelope interpreter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION");

use std::fmt;

use vibe_wire::generated::machine::e1::operation_envelope::{
    Body, BodyErrorCompleteness, BodyErrorEffectState, BodyResultCompleteness,
    BodyResultEffectState, OperationEnvelope,
};
use vibe_wire::generated::migration::e1::{
    plan_error::PlanError,
    plan_report::{Outcome, PlanReport},
    plan_request::PlanRequest,
};
use vibe_workspace::migration_planning::{validate_preflight_error, validate_report_for_request};

use super::{
    MachineOperationService, PLANNER_ID, PLANNER_SOURCE, PurePlannerAdmission, recognized_planner,
    recognized_planner_identity,
};

const REPORT_FORMAT: &str = "migration-plan-report";
const ERROR_FORMAT: &str = "migration-plan-error";

/// Rejected publication keeps the original typed envelope for diagnostic-only
/// handling. It must never become a normal result or replay authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineEnvelopeValidationError {
    pub reason: &'static str,
    pub original: OperationEnvelope,
}

impl fmt::Display for MachineEnvelopeValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pure planner envelope refused: {}", self.reason)
    }
}
impl std::error::Error for MachineEnvelopeValidationError {}

impl MachineOperationService {
    /// Validate this service's exact pure-planner Result/Error before a library
    /// or transport publishes it. A generated outer DTO alone does not decode
    /// its embedded JSON or establish domain/effect provenance.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
        r = 1
    )]
    pub fn validate_pure_planner_publication(
        &self,
        request: Option<&PlanRequest>,
        envelope: OperationEnvelope,
    ) -> Result<OperationEnvelope, MachineEnvelopeValidationError> {
        match self.check_planner_publication(request, &envelope) {
            Ok(()) => Ok(envelope),
            Err(reason) => Err(MachineEnvelopeValidationError {
                reason,
                original: envelope,
            }),
        }
    }

    fn check_planner_publication(
        &self,
        request: Option<&PlanRequest>,
        envelope: &OperationEnvelope,
    ) -> Result<(), &'static str> {
        if envelope.schema != 1 || envelope.evidence_id.is_some() {
            return Err("unsupported epoch or fabricated envelope evidence");
        }
        if let Some(request) = request {
            if request.schema != 1
                || self.planner_admission != PurePlannerAdmission::Admitted
                || !self.descriptors.iter().any(recognized_planner)
            {
                return Err("pure planner request lacks an admitted service route");
            }
        }
        let has_route = envelope.operation_id.as_deref() == Some(PLANNER_ID)
            && envelope.domain_owner.as_deref() == Some(PLANNER_ID)
            && envelope.source_id.as_deref() == Some(PLANNER_SOURCE);
        match &envelope.body {
            Body::Result(row) => {
                let request = request.ok_or("result has no admitted request")?;
                if !has_route
                    || row.format_id.as_deref() != Some(REPORT_FORMAT)
                    || row.effect_state != BodyResultEffectState::NoEffects
                    || row.provenance_id.is_some()
                {
                    return Err("result route, format or effect mismatch");
                }
                let report: PlanReport = serde_json::from_str(
                    row.payload_json
                        .as_deref()
                        .ok_or("result has no domain bytes")?,
                )
                .map_err(|_| "result is not the generated domain root")?;
                validate_report_for_request(request, &report)
                    .map_err(|_| "result differs from owning planner replay")?;
                match (&report.outcome, &row.completeness) {
                    (Outcome::Planned(_) | Outcome::NoOp(_), BodyResultCompleteness::Complete)
                    | (Outcome::NeedsInput(_), BodyResultCompleteness::Partial) => Ok(()),
                    _ => Err("result outcome and completeness differ"),
                }
            }
            Body::Error(row) => {
                if row.effect_state != BodyErrorEffectState::NoEffects
                    || row.provenance_id.is_some()
                {
                    return Err("error effect or provenance mismatch");
                }
                match (
                    row.format_id.as_deref(),
                    row.payload_json.as_deref(),
                    request,
                ) {
                    (None, None, None)
                        if row.completeness == BodyErrorCompleteness::Unavailable
                            && envelope.operation_id.is_none()
                            && envelope.domain_owner.is_none()
                            && envelope.source_id.is_none() =>
                    {
                        Ok(())
                    }
                    (Some(ERROR_FORMAT), Some(payload), None)
                        if row.completeness == BodyErrorCompleteness::Complete && has_route =>
                    {
                        if !self.descriptors.iter().any(recognized_planner_identity) {
                            return Err("typed preflight has no recognized planner descriptor");
                        }
                        let error: PlanError = serde_json::from_str(payload)
                            .map_err(|_| "error is not the generated preflight root")?;
                        validate_preflight_error(&error)
                            .map_err(|_| "error has invalid owning preflight semantics")
                    }
                    (Some(REPORT_FORMAT), Some(payload), Some(request))
                        if row.completeness == BodyErrorCompleteness::Complete && has_route =>
                    {
                        let report: PlanReport = serde_json::from_str(payload)
                            .map_err(|_| "error is not the generated report root")?;
                        validate_report_for_request(request, &report)
                            .map_err(|_| "error differs from owning planner replay")?;
                        if matches!(report.outcome, Outcome::Refused(_)) {
                            Ok(())
                        } else {
                            Err("error arm does not carry a domain refusal")
                        }
                    }
                    _ => Err("error route, format, payload or completeness mismatch"),
                }
            }
            _ => Err("pure planner publishes only Result or Error"),
        }
    }
}

#[cfg(test)]
#[path = "publication/tests.rs"]
mod tests;
