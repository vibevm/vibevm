//! The tcg tool family's MCP adapter — THIN by design (PROP-026 §3):
//! four newtype cells implementing [`McpTool`] by delegating to
//! `vibe-tcg` (descriptors, schemas, run logic, registry all live
//! THERE, with zero vibe-mcp imports), and one error mapping. This
//! file is the ONLY place the two crates meet; a standalone tcg MCP
//! server later mounts the same `vibe-tcg` with its own copy of this
//! ~hundred lines.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-026#tools");

use std::path::Path;

use serde_json::Value;
use specmark::spec;

use crate::{ServerContext, ToolDescriptor, ToolError, tools::McpTool};

struct HostAdapter<'a>(&'a ServerContext);

impl vibe_tcg::TcgHost for HostAdapter<'_> {
    fn project_root(&self) -> &Path {
        &self.0.project_root
    }
}

fn map_error(e: vibe_tcg::TcgError) -> ToolError {
    use vibe_tcg::TcgError::*;
    match e {
        BadArguments { .. } | LanguageUnsupported { .. } => {
            ToolError::InvalidArguments(e.to_string())
        }
        StackNotInstalled { .. } => ToolError::NotFound(e.to_string()),
        // The Display strings already carry the PROP-026 recipes.
        NotBuiltThirdParty { .. } | BuildFailed { .. } | OracleGone { .. } | Protocol { .. } => {
            ToolError::Internal(e.to_string())
        }
    }
}

#[spec(
    deviates = "spec://vibevm/modules/vibe-mcp/PROP-026#tools",
    reason = "compile-time invariant: the tcg_tool_cell! macro instantiates \
              only the literal names vibe_tcg::tool_specs() declares; a miss \
              is unreachable by construction and a panic here is a build \
              defect, not a runtime input"
)]
fn descriptor_for(name: &str) -> ToolDescriptor {
    let spec = vibe_tcg::tool_specs()
        .into_iter()
        .find(|s| s.name == name)
        .expect("vibe-tcg declares every family tool this adapter mounts");
    ToolDescriptor {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        input_schema: spec.input_schema,
    }
}

fn run_family_tool(name: &str, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
    vibe_tcg::run_tool(name, args, &HostAdapter(ctx), &ctx.tcg).map_err(map_error)
}

macro_rules! tcg_tool_cell {
    ($cell:ident, $tool_name:literal) => {
        #[doc = concat!("The `", $tool_name, "` adapter cell (PROP-026 §2).")]
        pub struct $cell;

        impl McpTool for $cell {
            fn descriptor(&self) -> ToolDescriptor {
                descriptor_for($tool_name)
            }

            fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
                run_family_tool($tool_name, args, ctx)
            }
        }
    };
}

tcg_tool_cell!(TcgValidate, "tcg_validate");
tcg_tool_cell!(TcgScope, "tcg_scope");
tcg_tool_cell!(TcgComplete, "tcg_complete");
tcg_tool_cell!(TcgType, "tcg_type");
