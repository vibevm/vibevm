//! One pure planner machine-envelope cell; descriptors are not effect grants.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");

use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_orchestrator::machine_operation::{
    MachineOperationError, MachineOperationService, PurePlannerAdmission,
};
use vibe_wire::generated::{
    machine::e1::operation_envelope::{
        Body, BodyError, BodyErrorCompleteness, BodyErrorEffectState, BodyResult,
        BodyResultCompleteness, BodyResultEffectState, OperationEnvelope,
    },
    migration::e1::{
        plan_error::PlanError,
        plan_report::{Outcome, PlanReport},
        plan_request::PlanRequest,
    },
};
use vibe_workspace::migration_planning::{
    MigrationPlanningResponse, decode_migration_request, plan_migration_json,
    validate_preflight_error, validate_report_for_request,
};

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

const OPERATION: &str = "rust:vibe-workspace:migration_planning";
const SOURCE: &str = "pure-migration-transport-adapter";
const INPUT: &str = "migration-plan-request";
const REPORT: &str = "migration-plan-report";
const ERROR: &str = "migration-plan-error";

#[cell(seam = "McpTool", variant = "plan_manifest_migration_envelope")]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
    r = 1
)]
pub struct PlanManifestMigrationEnvelopeMcpTool;

fn envelope(body: Body) -> OperationEnvelope {
    OperationEnvelope {
        schema: 1,
        operation_id: Some(OPERATION.to_owned()),
        domain_owner: Some(OPERATION.to_owned()),
        source_id: Some(SOURCE.to_owned()),
        evidence_id: None,
        body,
    }
}
fn empty_error() -> OperationEnvelope {
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
fn typed_error(error: &PlanError) -> Result<OperationEnvelope, ToolError> {
    validate_preflight_error(error)
        .map_err(|_| ToolError::Internal("owning preflight validation failed".to_owned()))?;
    Ok(envelope(Body::Error(Box::new(BodyError {
        format_id: Some(ERROR.to_owned()),
        payload_json: Some(serde_json::to_string(error).map_err(|_| {
            ToolError::Internal("generated preflight serialization failed".to_owned())
        })?),
        effect_state: BodyErrorEffectState::NoEffects,
        completeness: BodyErrorCompleteness::Complete,
        provenance_id: None,
    }))))
}
fn invalid(message: &'static str, error: Option<&PlanError>) -> ToolError {
    let document = match error {
        Some(error) => match typed_error(error) {
            Ok(document) => document,
            Err(_) => {
                let original =
                    serde_json::to_string(error).unwrap_or_else(|_| format!("{error:?}"));
                return ToolError::Internal(format!(
                    "INTERNAL MACHINE PREFLIGHT VALIDATION FAILURE: UNVALIDATED original error, not a normal report or replay authority: {original}"
                ));
            }
        },
        None => empty_error(),
    };
    // The fixed preflight shape is checked before exposing structuredContent.
    if validate_output(&document, None).is_err() {
        return ToolError::Internal("machine preflight envelope invariant failed".to_owned());
    }
    match serde_json::to_value(document) {
        Ok(report) => ToolError::StructuredPreflight {
            report,
            message: message.to_owned(),
        },
        Err(_) => ToolError::Internal("machine preflight envelope serialization failed".to_owned()),
    }
}

/// Local publication law for this ONE pure route; the domain owner separately
/// replays the entire report against the admitted request.
fn validate_output(
    value: &OperationEnvelope,
    request: Option<&PlanRequest>,
) -> Result<(), &'static str> {
    if value.schema != 1 || value.evidence_id.is_some() {
        return Err("machine envelope identity or epoch mismatch");
    }
    let unknown_preflight = matches!(
        (&value.body, request),
        (Body::Error(row), None)
            if row.format_id.is_none() && row.payload_json.is_none()
    );
    if unknown_preflight {
        if value.operation_id.is_some() || value.domain_owner.is_some() || value.source_id.is_some()
        {
            return Err("unestablished preflight identity must be null");
        }
    } else if value.operation_id.as_deref() != Some(OPERATION)
        || value.domain_owner.as_deref() != Some(OPERATION)
        || value.source_id.as_deref() != Some(SOURCE)
    {
        return Err("machine envelope identity mismatch");
    }
    match &value.body {
        Body::Result(row) => {
            let request = request.ok_or("result has no admitted request")?;
            if row.format_id.as_deref() != Some(REPORT)
                || row.effect_state != BodyResultEffectState::NoEffects
                || row.provenance_id.is_some()
            {
                return Err("result owner, format or effect mismatch");
            }
            let report: PlanReport = serde_json::from_str(
                row.payload_json
                    .as_deref()
                    .ok_or("result has no domain bytes")?,
            )
            .map_err(|_| "result does not carry the generated domain root")?;
            validate_report_for_request(request, &report)
                .map_err(|_| "result differs from owning replay")?;
            match (&report.outcome, &row.completeness) {
                (Outcome::Planned(_) | Outcome::NoOp(_), BodyResultCompleteness::Complete)
                | (Outcome::NeedsInput(_), BodyResultCompleteness::Partial) => Ok(()),
                _ => Err("result status and completeness mismatch"),
            }
        }
        Body::Error(row) => {
            if row.effect_state != BodyErrorEffectState::NoEffects || row.provenance_id.is_some() {
                return Err("error effect or provenance mismatch");
            }
            match (
                row.format_id.as_deref(),
                row.payload_json.as_deref(),
                request,
            ) {
                (None, None, None) if row.completeness == BodyErrorCompleteness::Unavailable => {
                    Ok(())
                }
                (Some(ERROR), Some(payload), None)
                    if row.completeness == BodyErrorCompleteness::Complete =>
                {
                    let error: PlanError =
                        serde_json::from_str(payload).map_err(|_| "error has wrong domain root")?;
                    validate_preflight_error(&error).map_err(|_| "error has wrong domain code")
                }
                (Some(REPORT), Some(payload), Some(request))
                    if row.completeness == BodyErrorCompleteness::Complete =>
                {
                    let report: PlanReport =
                        serde_json::from_str(payload).map_err(|_| "error has wrong report root")?;
                    validate_report_for_request(request, &report)
                        .map_err(|_| "error differs from owning replay")?;
                    if matches!(report.outcome, Outcome::Refused(_)) {
                        Ok(())
                    } else {
                        Err("error arm does not carry refusal")
                    }
                }
                _ => Err("error format, payload and completeness mismatch"),
            }
        }
        _ => Err("only result/error are published by pure planner"),
    }
}
fn publish(
    value: OperationEnvelope,
    request: &PlanRequest,
    is_error: bool,
    message: &'static str,
) -> Result<ToolOutput, ToolError> {
    if validate_output(&value, Some(request)).is_err() {
        let evidence = serde_json::to_string(&value).unwrap_or_else(|_| format!("{value:?}"));
        return Ok(ToolOutput::executed_failure_text(format!(
            "INTERNAL MACHINE RESULT VALIDATION FAILURE: UNVALIDATED original envelope (not a normal report or replay authority): {evidence}"
        )));
    }
    let structured = serde_json::to_value(value)
        .map_err(|_| ToolError::Internal("machine envelope serialization failed".to_owned()))?;
    Ok(if is_error {
        ToolOutput::executed_failure(structured, message)
    } else {
        ToolOutput::executed(structured, message)
    })
}
fn diagnostic(error: MachineOperationError) -> ToolOutput {
    match error {
        MachineOperationError::InvalidDomainResponse { original } => {
            let evidence =
                serde_json::to_string(&original).unwrap_or_else(|_| format!("{original:?}"));
            ToolOutput::executed_failure_text(format!(
                "INTERNAL MACHINE DOMAIN VALIDATION FAILURE: original response is UNVALIDATED, not a replay authority; retained evidence: {evidence}"
            ))
        }
        other => ToolOutput::executed_failure_text(format!(
            "INTERNAL MACHINE SERVICE FAILURE: {other}; no validated normal response was published"
        )),
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
            .map_err(|_| invalid("Machine envelope shape refused before execution", None))?;
        if incoming.schema != 1
            || incoming.operation_id.as_deref() != Some(OPERATION)
            || incoming.domain_owner.as_deref() != Some(OPERATION)
            || incoming.source_id.as_deref() != Some(SOURCE)
            || incoming.evidence_id.is_some()
        {
            return Err(invalid(
                "Machine envelope epoch, route or source refused before execution",
                None,
            ));
        }
        let Body::Input(input) = incoming.body else {
            return Err(invalid("Only the pure planner input arm is callable", None));
        };
        if input.format_id != INPUT {
            return Err(invalid(
                "Unregistered input format for the selected pure planner",
                None,
            ));
        }
        let body: Value = match serde_json::from_str(&input.payload_json) {
            Ok(body) => body,
            Err(_) => {
                let MigrationPlanningResponse::Preflight(error) =
                    plan_migration_json(&input.payload_json)
                else {
                    return Err(ToolError::Internal(
                        "owning JSON parser disagreed on malformed input".to_owned(),
                    ));
                };
                return Err(invalid(
                    "Embedded planning JSON refused before execution",
                    Some(&error),
                ));
            }
        };
        let request = decode_migration_request(body).map_err(|error| {
            invalid(
                "Embedded planning request refused before execution",
                Some(&error),
            )
        })?;
        let service = MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted)
            .map_err(|error| {
                invalid("Trusted pure planner service composition refused", None)
                    .with_machine_context(error)
            })?;
        let response = match service.plan_migration(OPERATION, request.clone()) {
            Ok(response) => response,
            Err(error @ MachineOperationError::InvalidDomainResponse { .. }) => {
                return Ok(diagnostic(error));
            }
            Err(error) => {
                return Err(
                    invalid("Pure planner route unavailable before execution", None)
                        .with_machine_context(error),
                );
            }
        };
        match response {
            MigrationPlanningResponse::Report(report) => {
                let (body, is_error, message) = match &report.outcome {
                    Outcome::Planned(_) | Outcome::NoOp(_) => (
                        Body::Result(Box::new(BodyResult {
                            format_id: Some(REPORT.to_owned()),
                            payload_json: Some(serde_json::to_string(&report).map_err(|_| {
                                ToolError::Internal("domain report serialization failed".to_owned())
                            })?),
                            effect_state: BodyResultEffectState::NoEffects,
                            completeness: BodyResultCompleteness::Complete,
                            provenance_id: None,
                        })),
                        false,
                        "Pure migration plan produced; no filesystem effects were authorized.",
                    ),
                    Outcome::NeedsInput(_) => (
                        Body::Result(Box::new(BodyResult {
                            format_id: Some(REPORT.to_owned()),
                            payload_json: Some(serde_json::to_string(&report).map_err(|_| {
                                ToolError::Internal(
                                    "domain questions serialization failed".to_owned(),
                                )
                            })?),
                            effect_state: BodyResultEffectState::NoEffects,
                            completeness: BodyResultCompleteness::Partial,
                            provenance_id: None,
                        })),
                        false,
                        "Pure migration planning needs the named choices; no filesystem effects were authorized.",
                    ),
                    Outcome::Refused(_) => (
                        Body::Error(Box::new(BodyError {
                            format_id: Some(REPORT.to_owned()),
                            payload_json: Some(serde_json::to_string(&report).map_err(|_| {
                                ToolError::Internal(
                                    "domain refusal serialization failed".to_owned(),
                                )
                            })?),
                            effect_state: BodyErrorEffectState::NoEffects,
                            completeness: BodyErrorCompleteness::Complete,
                            provenance_id: None,
                        })),
                        true,
                        "Pure migration planning refused; exact owning report is retained without effects.",
                    ),
                };
                publish(envelope(body), &request, is_error, message)
            }
            MigrationPlanningResponse::Preflight(error) => {
                // A validated typed service can still return its owning preflight.
                Err(invalid(
                    "Pure planner reported preflight before execution",
                    Some(&error),
                ))
            }
        }
    }
}

trait MachineContextText {
    fn with_machine_context(self, error: MachineOperationError) -> Self;
}
impl MachineContextText for ToolError {
    fn with_machine_context(self, error: MachineOperationError) -> Self {
        match self {
            ToolError::StructuredPreflight { report, message } => ToolError::StructuredPreflight {
                report,
                message: format!("{message}: {error}"),
            },
            other => other,
        }
    }
}

#[cfg(test)]
#[path = "machine_operation/tests.rs"]
mod tests;
