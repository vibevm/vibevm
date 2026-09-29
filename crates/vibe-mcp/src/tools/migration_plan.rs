//! Text-only migration planning; server context is not an input or effect grant.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");

use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_wire::generated::migration::e1::plan_report::Outcome;
use vibe_workspace::migration_planning::{MigrationPlanningResponse, plan_migration_value};

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

#[cell(seam = "McpTool", variant = "plan_manifest_migration")]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
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
                    "recipe": { "type": "string" }
                },
                "required": ["schema", "source_text"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, _ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        match plan_migration_value(args.clone()) {
            MigrationPlanningResponse::Preflight(error) => {
                let report = serde_json::to_value(error).map_err(|_| {
                    ToolError::Internal(
                        "migration preflight report serialization failed".to_owned(),
                    )
                })?;
                Err(ToolError::StructuredPreflight {
                    report,
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
                        "Select the listed profile and recipe choices, then call plan_manifest_migration again. No changes were applied.",
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
