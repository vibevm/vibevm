//! One generated Input → pure migration planner → validated Result/Error.
//! No other operation, tool, effect or transport is admitted here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");

use serde_json::Value;
use vibe_wire::generated::machine::e1::operation_envelope::{
    Body, BodyError, BodyErrorCompleteness, BodyErrorEffectState, BodyResult,
    BodyResultCompleteness, BodyResultEffectState, OperationEnvelope,
};
use vibe_wire::generated::migration::e1::{
    plan_error::PlanError,
    plan_report::{Outcome, PlanReport},
    plan_request::PlanRequest,
};
use vibe_workspace::migration_planning::{
    MigrationPlanningResponse, decode_migration_request, plan_migration_json,
    validate_preflight_error,
};

use super::{MachineOperationError, MachineOperationService, PLANNER_ID, PLANNER_SOURCE};

const INPUT_FORMAT: &str = "migration-plan-request";
const REPORT_FORMAT: &str = "migration-plan-report";
const ERROR_FORMAT: &str = "migration-plan-error";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishedPlannerStatus {
    Success,
    NeedsInput,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangePreflightCause {
    OuterRoute,
    EmbeddedRequest,
}

/// Original evidence is retained in Rust but never promoted to a normal
/// machine report when an owner or publication invariant fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeDiagnosticEvidence {
    DomainResponse(MigrationPlanningResponse),
    UnvalidatedPublication(OperationEnvelope),
    Service(MachineOperationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurePlannerExchange {
    Published {
        status: PublishedPlannerStatus,
        envelope: OperationEnvelope,
    },
    Preflight {
        cause: ExchangePreflightCause,
        envelope: OperationEnvelope,
        original_input: OperationEnvelope,
    },
    Unavailable {
        original_input: OperationEnvelope,
        error: MachineOperationError,
    },
    Diagnostic {
        original_input: OperationEnvelope,
        evidence: ExchangeDiagnosticEvidence,
        reason: &'static str,
    },
}

fn envelope(body: Body) -> OperationEnvelope {
    OperationEnvelope {
        schema: 1,
        operation_id: Some(PLANNER_ID.to_owned()),
        domain_owner: Some(PLANNER_ID.to_owned()),
        source_id: Some(PLANNER_SOURCE.to_owned()),
        evidence_id: None,
        body,
    }
}

fn null_identity_preflight() -> OperationEnvelope {
    OperationEnvelope {
        schema: 1,
        operation_id: None,
        domain_owner: None,
        source_id: None,
        evidence_id: None,
        body: Body::Error(Box::new(BodyError {
            format_id: None,
            payload_json: None,
            effect_state: BodyErrorEffectState::NoEffects,
            completeness: BodyErrorCompleteness::Unavailable,
            provenance_id: None,
        })),
    }
}

fn preflight_publication(
    service: &MachineOperationService,
    original_input: OperationEnvelope,
    cause: ExchangePreflightCause,
    candidate: OperationEnvelope,
) -> PurePlannerExchange {
    match service.validate_pure_planner_publication(None, candidate) {
        Ok(envelope) => PurePlannerExchange::Preflight {
            cause,
            envelope,
            original_input,
        },
        Err(error) => PurePlannerExchange::Diagnostic {
            original_input,
            evidence: ExchangeDiagnosticEvidence::UnvalidatedPublication(error.original),
            reason: "preflight publication failed owning validation",
        },
    }
}

fn typed_preflight(
    service: &MachineOperationService,
    original_input: OperationEnvelope,
    error: PlanError,
) -> PurePlannerExchange {
    if validate_preflight_error(&error).is_err() {
        return PurePlannerExchange::Diagnostic {
            original_input,
            evidence: ExchangeDiagnosticEvidence::DomainResponse(
                MigrationPlanningResponse::Preflight(error),
            ),
            reason: "owning preflight root failed validation",
        };
    }
    let payload = match serde_json::to_string(&error) {
        Ok(payload) => payload,
        Err(_) => {
            return PurePlannerExchange::Diagnostic {
                original_input,
                evidence: ExchangeDiagnosticEvidence::DomainResponse(
                    MigrationPlanningResponse::Preflight(error),
                ),
                reason: "generated preflight serialization failed",
            };
        }
    };
    preflight_publication(
        service,
        original_input,
        ExchangePreflightCause::EmbeddedRequest,
        envelope(Body::Error(Box::new(BodyError {
            format_id: Some(ERROR_FORMAT.to_owned()),
            payload_json: Some(payload),
            effect_state: BodyErrorEffectState::NoEffects,
            completeness: BodyErrorCompleteness::Complete,
            provenance_id: None,
        }))),
    )
}

fn output_for_report(
    report: &PlanReport,
) -> Result<(PublishedPlannerStatus, OperationEnvelope), ()> {
    let payload = serde_json::to_string(report).map_err(|_| ())?;
    let (status, body) = match &report.outcome {
        Outcome::Planned(_) | Outcome::NoOp(_) => (
            PublishedPlannerStatus::Success,
            Body::Result(Box::new(BodyResult {
                format_id: Some(REPORT_FORMAT.to_owned()),
                payload_json: Some(payload),
                effect_state: BodyResultEffectState::NoEffects,
                completeness: BodyResultCompleteness::Complete,
                provenance_id: None,
            })),
        ),
        Outcome::NeedsInput(_) => (
            PublishedPlannerStatus::NeedsInput,
            Body::Result(Box::new(BodyResult {
                format_id: Some(REPORT_FORMAT.to_owned()),
                payload_json: Some(payload),
                effect_state: BodyResultEffectState::NoEffects,
                completeness: BodyResultCompleteness::Partial,
                provenance_id: None,
            })),
        ),
        Outcome::Refused(_) => (
            PublishedPlannerStatus::Refused,
            Body::Error(Box::new(BodyError {
                format_id: Some(REPORT_FORMAT.to_owned()),
                payload_json: Some(payload),
                effect_state: BodyErrorEffectState::NoEffects,
                completeness: BodyErrorCompleteness::Complete,
                provenance_id: None,
            })),
        ),
    };
    Ok((status, envelope(body)))
}

fn publish_report(
    service: &MachineOperationService,
    original_input: OperationEnvelope,
    request: &PlanRequest,
    report: PlanReport,
) -> PurePlannerExchange {
    let (status, candidate) = match output_for_report(&report) {
        Ok(value) => value,
        Err(_) => {
            return PurePlannerExchange::Diagnostic {
                original_input,
                evidence: ExchangeDiagnosticEvidence::DomainResponse(
                    MigrationPlanningResponse::Report(report),
                ),
                reason: "generated domain report serialization failed",
            };
        }
    };
    match service.validate_pure_planner_publication(Some(request), candidate) {
        Ok(envelope) => PurePlannerExchange::Published { status, envelope },
        Err(error) => PurePlannerExchange::Diagnostic {
            original_input,
            evidence: ExchangeDiagnosticEvidence::UnvalidatedPublication(error.original),
            reason: "result publication failed owning replay validation",
        },
    }
}

impl MachineOperationService {
    /// Exact accepted planner route. Structural outer decoding belongs to the
    /// generated reader; embedded request semantics belong to workspace.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
        r = 1
    )]
    pub fn exchange_pure_planner(&self, input: OperationEnvelope) -> PurePlannerExchange {
        let valid_outer = input.schema == 1
            && input.operation_id.as_deref() == Some(PLANNER_ID)
            && input.domain_owner.as_deref() == Some(PLANNER_ID)
            && input.source_id.as_deref() == Some(PLANNER_SOURCE)
            && input.evidence_id.is_none();
        let Body::Input(body) = &input.body else {
            return preflight_publication(
                self,
                input,
                ExchangePreflightCause::OuterRoute,
                null_identity_preflight(),
            );
        };
        if !valid_outer || body.format_id != INPUT_FORMAT {
            return preflight_publication(
                self,
                input,
                ExchangePreflightCause::OuterRoute,
                null_identity_preflight(),
            );
        }
        let payload = body.payload_json.clone();
        let value: Value = match serde_json::from_str(&payload) {
            Ok(value) => value,
            Err(_) => {
                let owner = plan_migration_json(&payload);
                let MigrationPlanningResponse::Preflight(error) = owner else {
                    return PurePlannerExchange::Diagnostic {
                        original_input: input,
                        evidence: ExchangeDiagnosticEvidence::DomainResponse(owner),
                        reason: "owning parser disagreed on malformed embedded JSON",
                    };
                };
                return typed_preflight(self, input, error);
            }
        };
        let request = match decode_migration_request(value) {
            Ok(request) => request,
            Err(error) => return typed_preflight(self, input, error),
        };
        match self.plan_migration(PLANNER_ID, request.clone()) {
            Ok(MigrationPlanningResponse::Report(report)) => {
                publish_report(self, input, &request, report)
            }
            Ok(response @ MigrationPlanningResponse::Preflight(_)) => {
                PurePlannerExchange::Diagnostic {
                    original_input: input,
                    evidence: ExchangeDiagnosticEvidence::DomainResponse(response),
                    reason: "admitted typed request produced unexpected preflight",
                }
            }
            Err(MachineOperationError::InvalidDomainResponse { original }) => {
                PurePlannerExchange::Diagnostic {
                    original_input: input,
                    evidence: ExchangeDiagnosticEvidence::DomainResponse(original),
                    reason: "owner response failed validation",
                }
            }
            Err(
                error @ (MachineOperationError::UnavailableOperation(_)
                | MachineOperationError::UnknownOperation(_)),
            ) => PurePlannerExchange::Unavailable {
                original_input: input,
                error,
            },
            Err(error) => PurePlannerExchange::Diagnostic {
                original_input: input,
                evidence: ExchangeDiagnosticEvidence::Service(error),
                reason: "pure planner service refused after typed admission",
            },
        }
    }
}

#[cfg(test)]
mod tests;
