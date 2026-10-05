//! The two selected-subject proof tools project the shared engine Reply.
//! They execute uncached checks, with root and toolchain policy fixed by
//! the server context and trusted startup environment.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-EVIDENCE-IDENTITY");

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_trace::proof::{Context, Request, execute};

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

/// Check both selected evidence streams and write their joined witness.
#[cell(seam = "McpTool", variant = "proof_check")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ProofCheckMcpTool;

/// Verify a saved joined witness against fresh selected-subject checks.
#[cell(seam = "McpTool", variant = "proof_verify")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ProofVerifyMcpTool;

#[derive(Clone, Copy)]
enum Operation {
    Check,
    Verify,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    subject: String,
    witness: PathBuf,
}

fn descriptor(name: &str, description: &str) -> ToolDescriptor {
    ToolDescriptor {
        name: name.into(),
        description: description.into(),
        input_schema: json!({
            "type":"object",
            "properties": {
                "subject": {"type":"string", "description":"Exact spec address selected by the composition policy."},
                "witness": {"type":"string", "description":"Witness path relative to the configured consumer root."}
            },
            "required":["subject", "witness"],
            "additionalProperties":false
        }),
    }
}

impl McpTool for ProofCheckMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        descriptor(
            "proof_check",
            "Run uncached checks for the selected Rust implementation and Lean G1 model and write their joined witness. Returns the same Reply as vibe proof-check; model evidence does not establish Rust refinement.",
        )
    }

    fn run(&self, args: &Value, context: &ServerContext) -> Result<ToolOutput, ToolError> {
        run(Operation::Check, args, context)
    }
}

impl McpTool for ProofVerifyMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        descriptor(
            "proof_verify",
            "Verify the selected saved witness through fresh uncached checks. Returns the same Reply as vibe proof-verify; root and toolchain configuration cannot be overridden by the call.",
        )
    }

    fn run(&self, args: &Value, context: &ServerContext) -> Result<ToolOutput, ToolError> {
        run(Operation::Verify, args, context)
    }
}

fn run(
    operation: Operation,
    args: &Value,
    context: &ServerContext,
) -> Result<ToolOutput, ToolError> {
    if !args.is_object() {
        return Err(ToolError::InvalidArguments(
            "proof tools require an object with subject and witness".into(),
        ));
    }
    let args: Arguments = serde_json::from_value(args.clone()).map_err(|error| {
        ToolError::InvalidArguments(format!(
            "proof tools accept only subject and witness: {error}"
        ))
    })?;
    let request = match operation {
        Operation::Check => Request::Check {
            subject: args.subject,
            witness: args.witness,
        },
        Operation::Verify => Request::Verify {
            subject: args.subject,
            witness: args.witness,
        },
    };
    let reply = execute(
        &Context {
            project_root: context.project_root.clone(),
            lean_toolchain_dir: None,
            rust_toolchain_dir: None,
        },
        request,
    );
    let body = serde_json::to_value(&reply).map_err(|error| {
        ToolError::Internal(format!("serializing the selected proof reply: {error}"))
    })?;
    let text = serde_json::to_string_pretty(&reply).map_err(|error| {
        ToolError::Internal(format!("serializing the selected proof text: {error}"))
    })?;
    Ok(if reply.ok {
        ToolOutput::executed(body, text)
    } else {
        ToolOutput::executed_failure(body, text)
    })
}
