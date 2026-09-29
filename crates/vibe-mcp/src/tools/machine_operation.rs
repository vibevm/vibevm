//! MCP channel adapter for the one shared pure-planner machine exchange.
//! Outer shape remains generated; domain decoding and publication live in the
//! neutral service, never in this transport cell.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");

use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_orchestrator::machine_operation::{
    MachineOperationService, PurePlannerAdmission,
    exchange::{
        ExchangeDiagnosticEvidence, ExchangePreflightCause, PublishedPlannerStatus,
        PurePlannerExchange,
    },
};
use vibe_wire::generated::machine::e1::operation_envelope::{
    Body, BodyError, BodyErrorCompleteness, BodyErrorEffectState, OperationEnvelope,
};
use vibe_workspace::migration_planning::MigrationPlanningResponse;

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

const OPERATION: &str = "rust:vibe-workspace:migration_planning";
const SOURCE: &str = "pure-migration-transport-adapter";
const INPUT: &str = "migration-plan-request";
const REPORT: &str = "migration-plan-report";

#[cell(seam = "McpTool", variant = "plan_manifest_migration_envelope")]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
    r = 1
)]
pub struct PlanManifestMigrationEnvelopeMcpTool;

/// Only malformed outer JSON lacks a generated Input DTO. The shared owner
/// still validates this fixed null-identity preflight before publication.
fn invalid_shape(message: impl Into<String>) -> ToolError {
    let candidate = OperationEnvelope {
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
    };
    let service = match MachineOperationService::pure_text_planner(
        PurePlannerAdmission::Unavailable,
    ) {
        Ok(service) => service,
        Err(error) => {
            return ToolError::Internal(format!(
                "INTERNAL MACHINE PREFLIGHT SERVICE FAILURE: {error}; UNVALIDATED original envelope: {candidate:?}"
            ));
        }
    };
    let validated = match service.validate_pure_planner_publication(None, candidate) {
        Ok(envelope) => envelope,
        Err(error) => {
            return ToolError::Internal(format!(
                "INTERNAL MACHINE PREFLIGHT VALIDATION FAILURE: UNVALIDATED original envelope: {:?}",
                error.original
            ));
        }
    };
    match serde_json::to_value(validated) {
        Ok(report) => ToolError::StructuredPreflight {
            report,
            message: message.into(),
        },
        Err(error) => {
            ToolError::Internal(format!("machine preflight serialization failed: {error}"))
        }
    }
}

fn diagnostic_is_preflight(evidence: &ExchangeDiagnosticEvidence) -> bool {
    match evidence {
        ExchangeDiagnosticEvidence::DomainResponse(MigrationPlanningResponse::Preflight(_)) => true,
        ExchangeDiagnosticEvidence::UnvalidatedPublication(envelope) => {
            matches!(&envelope.body, Body::Error(row) if row.format_id.as_deref() != Some(REPORT))
        }
        ExchangeDiagnosticEvidence::DomainResponse(MigrationPlanningResponse::Report(_))
        | ExchangeDiagnosticEvidence::Service(_) => false,
    }
}

fn diagnostic_text(reason: &str, evidence: &ExchangeDiagnosticEvidence) -> String {
    let original_label = if matches!(
        evidence,
        ExchangeDiagnosticEvidence::DomainResponse(MigrationPlanningResponse::Preflight(_))
    ) {
        "UNVALIDATED original error"
    } else {
        "original evidence is UNVALIDATED"
    };
    let original = match evidence {
        ExchangeDiagnosticEvidence::DomainResponse(response) => {
            serde_json::to_string(response).unwrap_or_else(|_| format!("{response:?}"))
        }
        ExchangeDiagnosticEvidence::UnvalidatedPublication(envelope) => {
            serde_json::to_string(envelope).unwrap_or_else(|_| format!("{envelope:?}"))
        }
        ExchangeDiagnosticEvidence::Service(error) => error.to_string(),
    };
    format!(
        "INTERNAL MACHINE VALIDATION FAILURE: {reason}; {original_label}, not a normal report or replay authority: {original}"
    )
}

fn map_exchange(exchange: PurePlannerExchange) -> Result<ToolOutput, ToolError> {
    match exchange {
        PurePlannerExchange::Published { status, envelope } => {
            let structured = serde_json::to_value(envelope).map_err(|error| {
                ToolError::Internal(format!(
                    "validated machine envelope serialization failed: {error}"
                ))
            })?;
            Ok(match status {
                PublishedPlannerStatus::Success => ToolOutput::executed(
                    structured,
                    "Pure migration plan produced; no filesystem effects were authorized.",
                ),
                PublishedPlannerStatus::NeedsInput => ToolOutput::executed(
                    structured,
                    "Pure migration planning needs the named choices; no filesystem effects were authorized.",
                ),
                PublishedPlannerStatus::Refused => ToolOutput::executed_failure(
                    structured,
                    "Pure migration planning refused; exact owning report is retained without effects.",
                ),
            })
        }
        PurePlannerExchange::Preflight {
            cause, envelope, ..
        } => {
            let report = serde_json::to_value(envelope).map_err(|error| {
                ToolError::Internal(format!(
                    "validated machine preflight serialization failed: {error}"
                ))
            })?;
            let message = match cause {
                ExchangePreflightCause::OuterRoute => {
                    "Machine envelope epoch, route or source refused before execution"
                }
                ExchangePreflightCause::EmbeddedRequest => {
                    "Embedded planning request refused before execution"
                }
            };
            Err(ToolError::StructuredPreflight {
                report,
                message: message.to_owned(),
            })
        }
        PurePlannerExchange::Unavailable { error, .. } => Err(invalid_shape(format!(
            "Pure planner route unavailable before execution: {error}"
        ))),
        PurePlannerExchange::Diagnostic {
            evidence, reason, ..
        } => {
            let preflight = diagnostic_is_preflight(&evidence);
            let text = diagnostic_text(reason, &evidence);
            if preflight {
                Err(ToolError::Internal(text))
            } else {
                Ok(ToolOutput::executed_failure_text(text))
            }
        }
    }
}

impl McpTool for PlanManifestMigrationEnvelopeMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "plan_manifest_migration_envelope".to_owned(),
            description: "One pure, explicitly selected migration planner route through the generated machine envelope; no filesystem, provider or generic eval authority.".to_owned(),
            input_schema: json!({
                "type":"object", "properties":{
                    "schema":{"type":"integer","const":1},
                    "operation_id":{"type":"string","const":OPERATION},
                    "domain_owner":{"type":"string","const":OPERATION},
                    "source_id":{"type":"string","const":SOURCE},
                    "evidence_id":{"type":["string","null"]},
                    "body":{"type":"object","properties":{
                        "stage":{"type":"string","const":"input"},
                        "format_id":{"type":"string","const":INPUT},
                        "payload_json":{"type":"string"}
                    },"required":["stage","format_id","payload_json"],"additionalProperties":false}
                },"required":["schema","operation_id","domain_owner","source_id","evidence_id","body"],
                "additionalProperties":false
            }),
        }
    }

    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
        r = 1
    )]
    fn run(&self, args: &Value, _ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let incoming: OperationEnvelope = serde_json::from_value(args.clone())
            .map_err(|_| invalid_shape("Machine envelope shape refused before execution"))?;
        let service = MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted)
            .map_err(|error| {
                invalid_shape(format!(
                    "Trusted pure planner service composition refused: {error}"
                ))
            })?;
        map_exchange(service.exchange_pure_planner(incoming))
    }
}

#[cfg(test)]
#[path = "machine_operation/tests.rs"]
mod tests;
