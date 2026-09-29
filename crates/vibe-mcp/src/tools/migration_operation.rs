//! Explicit context-bound migration application; no payload-derived host grant.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL");

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_wire::generated::migration::e1::{
    operation_report::{
        ApplicationErrorCode as Code, OperationReport, OperationReportOperation as Operation,
        OperationReportOutcome as Outcome,
    },
    operation_request::OperationRequest,
};
use vibe_workspace::migration_application::{
    ApplicationValidationError as Fault, ReportBuilder, decode_operation_json,
    execute_manifest_migration_json, report::ReportValidationFailure,
};
use vibe_workspace::migration_planning::{MigrationPlanningResponse, plan_migration_json};

#[cell(seam = "McpTool", variant = "manifest_migration_operation")]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
pub struct ManifestMigrationOperationMcpTool;

impl McpTool for ManifestMigrationOperationMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        let authority = json!({"type":"object","properties":{
            "project_root":{"type":"string"},"recovery_store_root":{"type":"string"},
            "target_relative":{"type":"string"},"scope":{"type":"string","const":"owned_authored_file"}},
            "required":["project_root","recovery_store_root","target_relative","scope"],"additionalProperties":false});
        let branch = |operation: &str, extra: Value, required: &[&str]| {
            let mut properties = json!({"schema":{"type":"integer","const":1},
                "operation":{"type":"string","const":operation},"operation_id":{"type":"string"},"authority":authority});
            if let Some(extra) = extra.as_object() {
                for (key, value) in extra {
                    properties[key] = value.clone();
                }
            }
            let mut fields = vec!["schema", "operation", "operation_id", "authority"];
            fields.extend_from_slice(required);
            json!({"type":"object","properties":properties,"required":fields,"additionalProperties":false})
        };
        ToolDescriptor {
            name: "manifest_migration_operation".to_owned(),
            description: "Apply, inspect or recover one explicitly granted authored manifest using the server's finite project/file/recovery-store authority. Returns exact effects, uncertainty and residue; a payload path or scope label grants no write authority.".to_owned(),
            input_schema: json!({"oneOf":[
                branch("apply",json!({"approved_request_json":{"type":"string"},"before_content_id":{"type":"string"},"after_content_id":{"type":"string"},"diff_id":{"type":"string"}}), &["approved_request_json","before_content_id","after_content_id","diff_id"]),
                branch("inspect",json!({}),&[]),
                branch("recover",json!({"expected_intent_id":{"type":"string"},"action":{"type":"string","enum":["complete","restore"]}}),&["expected_intent_id","action"])
            ]}),
        }
    }

    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
        r = 2
    )]
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let known_operation = args
            .get("operation")
            .and_then(Value::as_str)
            .and_then(|value| match value {
                "apply" => Some(Operation::Apply),
                "inspect" => Some(Operation::Inspect),
                "recover" => Some(Operation::Recover),
                _ => None,
            });
        let known_id = args
            .get("operation_id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned);
        let refuse = |code, field, reason| {
            preflight(Fault {
                code,
                field,
                reason,
                operation: known_operation.clone(),
                operation_id: known_id.clone(),
            })
        };
        let decoded: OperationRequest = serde_json::from_value(args.clone()).map_err(|_| {
            refuse(
                Code::InvalidRequest,
                "request",
                "operation request does not match the registered shape",
            )
        })?;
        let (schema, id, authority) = match &decoded {
            OperationRequest::Apply(row) => (row.schema, &row.operation_id, &row.authority),
            OperationRequest::Inspect(row) => (row.schema, &row.operation_id, &row.authority),
            OperationRequest::Recover(row) => (row.schema, &row.operation_id, &row.authority),
        };
        if schema != 1 {
            return Err(refuse(
                Code::UnsupportedSchema,
                "schema",
                "unsupported operation schema",
            ));
        }
        if id.trim().is_empty() {
            return Err(refuse(
                Code::InvalidRequest,
                "operation_id",
                "operation id must be nonblank",
            ));
        }
        if let OperationRequest::Apply(row) = &decoded
            && matches!(
                plan_migration_json(&row.approved_request_json),
                MigrationPlanningResponse::Preflight(_)
            )
        {
            return Err(refuse(
                Code::InvalidRequest,
                "approved_request_json",
                "embedded planning request failed strict preflight",
            ));
        }
        let grant = ctx
            .grant_for_manifest_migration(authority)
            .map_err(|mut fault| {
                fault.operation = known_operation.clone();
                fault.operation_id = known_id.clone();
                preflight(fault)
            })?;
        let text = serde_json::to_string(args).map_err(|_| {
            ToolError::Internal("operation request serialization failed".to_owned())
        })?;
        decode_operation_json(&text, &grant).map_err(preflight)?;
        render_result(execute_manifest_migration_json(&text, &grant))
    }
}

fn preflight(fault: Fault) -> ToolError {
    // Generated fields contain only caller-supplied recognized identity and
    // fixed owning diagnostics; no private configuration/parser text is added.
    let report = ReportBuilder::preflight(&fault);
    match serde_json::to_value(report) {
        Ok(report) => ToolError::StructuredPreflight { report,
            message:"Migration operation refused before runtime. Inspect the typed input/admission error and configured server grant.".to_owned() },
        Err(_) => ToolError::Internal("migration preflight report serialization failed".to_owned()),
    }
}

fn render_result(
    result: Result<OperationReport, ReportValidationFailure>,
) -> Result<ToolOutput, ToolError> {
    match result {
        Ok(report) => {
            let (message, is_error) = match report.outcome {
                Outcome::Completed => (
                    "Migration operation completed. Inspect its actual retained effects and guarantees.",
                    false,
                ),
                Outcome::NoOp => (
                    "Exact migration replay made no new target write; verified prior effects remain reported.",
                    false,
                ),
                Outcome::Pending => (
                    "Migration inspection is pending explicit continuation. Inspect retained intent and residue before recovery.",
                    false,
                ),
                Outcome::Uncertain => (
                    "Migration effects are uncertain. Inspect retained evidence; do not infer that the target was untouched.",
                    true,
                ),
                Outcome::Refused => (
                    "Migration operation refused. Earlier effects and residue remain in the report.",
                    true,
                ),
            };
            let value = serde_json::to_value(report).map_err(|_| {
                ToolError::Internal("migration operation report serialization failed".to_owned())
            })?;
            Ok(if is_error {
                ToolOutput::executed_failure(value, message)
            } else {
                ToolOutput::executed(value, message)
            })
        }
        Err(failure) => {
            let evidence = match serde_json::to_string(&failure.report) {
                Ok(evidence) => evidence,
                Err(_) => format!(
                    "diagnostic serialization failed; retained Rust evidence: {:?}",
                    failure.report
                ),
            };
            Ok(ToolOutput::executed_failure_text(format!(
                "INTERNAL MIGRATION REPORT VALIDATION FAILURE: {}\nUNVALIDATED RETAINED EVIDENCE (not a normal report or replay authority): {evidence}",
                failure.fault
            )))
        }
    }
}

#[cfg(test)]
#[path = "migration_operation/tests.rs"]
mod tests;
