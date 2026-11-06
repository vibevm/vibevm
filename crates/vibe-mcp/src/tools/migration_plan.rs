//! Text-only migration planning; server context is not an input or effect grant.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");

use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_wire::generated::migration::e1::plan_report::Outcome;
use vibe_workspace::migration_planning::{MigrationPlanningResponse, plan_migration_value};

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

/// Plan supplied text through the same pure domain owner as the CLI.
/// ```
/// use vibe_mcp::tools::{McpTool, PlanManifestMigrationMcpTool};
/// use vibe_mcp::ServerContext;
/// let request = serde_json::json!({"schema":1,"source_text":""});
/// let output = PlanManifestMigrationMcpTool.run(&request, &ServerContext::new("unread-project")).unwrap();
/// assert_eq!(output.structured()["outcome"]["status"], "needs_input");
/// ```
#[cell(seam = "McpTool", variant = "plan_manifest_migration")]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 2
)]
pub struct PlanManifestMigrationMcpTool;

impl McpTool for PlanManifestMigrationMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "plan_manifest_migration".to_owned(),
            description: "Plan a supported authored-manifest migration from supplied TOML text and explicit profiles/recipe. Returns exact in-memory edits or typed questions/refusal without reading or writing a project or invoking providers.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "schema": { "type": "integer", "const": 1 },
                    "source_text": { "type": "string" },
                    "source_profile": { "type": "string" },
                    "target_profile": { "type": "string" },
                    "recipe": { "type": "string" },
                    "source_decisions": {"type":"array","items":{
                        "type":"object","additionalProperties":false,
                        "required":["file","media","interpretation","role","loading"],
                        "properties": {
                            "file":{"type":"string"},"media":{"type":"string"},
                            "interpretation":{"type":"string","enum":["vibe-spec","vibe-doc","commonmark","xml-data","opaque"]},
                            "role":{"type":"string","enum":["spec","documentation","resource"]},
                            "loading":{"type":"string","enum":["scan","explicit"]}
                        }
                    }}
                },
                "required": ["schema", "source_text"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, _ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        match plan_migration_value(args.clone()) {
            MigrationPlanningResponse::Preflight(error) => {
                Err(ToolError::StructuredPreflight {
                    report: Box::new(error),
                    message: "Migration request refused before planner entry; supply schema 1, source_text and string selections when present.".to_owned(),
                })
            }
            MigrationPlanningResponse::Report(report) => {
                let (text, is_error) = match &report.outcome {
                    Outcome::Planned(_) => (
                        "Migration plan prepared in memory. Inspect its exact edits before separately authorized application.",
                        false,
                    ),
                    Outcome::NoOp(_) => (
                        "The selected source already matches the requested profile. No changes were applied.",
                        false,
                    ),
                    Outcome::NeedsInput(_) => (
                        "Provide the requested choices and call plan_manifest_migration again. No changes were applied.",
                        false,
                    ),
                    Outcome::Refused(_) => (
                        "Migration planning refused. Inspect the typed refusal and supply a supported source/profile/recipe. No changes were applied.",
                        true,
                    ),
                };
                let value = serde_json::to_value(report).map_err(|_| {
                    ToolError::Internal("migration planner report serialization failed".to_owned())
                })?;
                Ok(if is_error {
                    ToolOutput::executed_failure(value, text)
                } else {
                    ToolOutput::executed(value, text)
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "migration_plan/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "migration_plan/source_tests.rs"]
mod source_tests;
