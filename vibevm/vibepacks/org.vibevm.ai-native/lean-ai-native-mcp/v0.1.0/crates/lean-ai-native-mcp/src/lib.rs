//! The Lean MCP surface calls the same checking library as the CLI.
//! Startup fixes the consumer root and optional trusted toolchain directory.
//! Tool arguments can select only root-relative carriers and witnesses.

specmark::scope!("spec://org.vibevm.ai-native/lean-ai-native-mcp/tools/lean-mcp#root");

use lean_ai_native_check::{Context, Reply, execute};
use mcp_core::{Tool, ToolDescriptor, ToolOutput, ToolSet};
use serde_json::Value;

mod arguments;
mod descriptor;

/// The registered MCP server name, shared with `vibe.toml`.
pub const SERVER_NAME: &str = "lean-ai-native";

/// The exact tool inventory in the transport's stable name order.
pub const TOOL_NAMES: [&str; 3] = [
    "lean_ai_native_check",
    "lean_ai_native_scan",
    "lean_ai_native_verify_witness",
];

#[derive(Clone, Copy)]
enum Operation {
    Scan,
    Check,
    VerifyWitness,
}

struct LeanTool {
    operation: Operation,
    context: Context,
}

impl Tool for LeanTool {
    fn descriptor(&self) -> ToolDescriptor {
        descriptor::for_operation(self.operation)
    }

    fn run(&mut self, args: &Value) -> ToolOutput {
        let reply = match arguments::parse(self.operation, args) {
            Ok(request) => execute(&self.context, request),
            Err(error) => Reply::invalid_request(error.to_string()),
        };
        // Reply is the engine's JSON wire type. No report is reconstructed
        // from a log, checker exit code or an independent status heuristic.
        let is_error = !reply.ok;
        let report = serde_json::to_string(&reply)
            .expect("the shared checking Reply contains JSON-serializable wire values");
        ToolOutput { report, is_error }
    }
}

/// Build all three tools with immutable startup authority.
///
/// Each tool owns the same root/toolchain values, and rejects attempts to
/// override them through tool arguments. The engine owns path containment.
///
/// ```
/// let context = lean_ai_native_check::Context {
///     project_root: std::path::PathBuf::from("."),
///     toolchain_dir: None,
/// };
/// let tools = lean_ai_native_mcp::tool_set(&context);
/// let names: Vec<_> = tools.descriptors().into_iter().map(|t| t.name).collect();
/// assert_eq!(names, lean_ai_native_mcp::TOOL_NAMES);
/// ```
pub fn tool_set(context: &Context) -> ToolSet {
    let mut tools = ToolSet::new();
    for operation in [Operation::Scan, Operation::Check, Operation::VerifyWitness] {
        tools.register(Box::new(LeanTool {
            operation,
            context: Context {
                project_root: context.project_root.clone(),
                toolchain_dir: context.toolchain_dir.clone(),
            },
        }));
    }
    tools
}
