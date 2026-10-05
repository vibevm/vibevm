//! Input schemas mirror the strict argument records; checker semantics and
//! path containment remain in the shared runner.

specmark::scope!("spec://org.vibevm.ai-native/lean-ai-native-mcp/tools/lean-mcp#arguments");

use mcp_core::ToolDescriptor;
use serde_json::{Value, json};

use super::Operation;

pub(super) fn for_operation(operation: Operation) -> ToolDescriptor {
    let mut properties = serde_json::Map::new();
    properties.insert("carrier".into(), json!({
        "type": "string", "description": "Carrier manifest path relative to the configured project root."
    }));
    let mut required = vec!["carrier"];
    if matches!(operation, Operation::Check | Operation::VerifyWitness) {
        properties.insert(
            "subject".into(),
            json!({
                "type": "string", "description": "The exact spec subject declared by the carrier."
            }),
        );
        required.push("subject");
    }
    if matches!(operation, Operation::VerifyWitness) {
        properties.insert("witness".into(), json!({
            "type": "string", "description": "Witness path relative to the configured project root."
        }));
        required.push("witness");
    }
    let (name, description) = match operation {
        Operation::Scan => (
            "lean_ai_native_scan",
            "Read declared Lean subjects and theorem associations; inventory does not establish a checked proof.",
        ),
        Operation::Check => (
            "lean_ai_native_check",
            "Check one declared model proof through the configured Lean toolchain and return the shared engine reply.",
        ),
        Operation::VerifyWitness => (
            "lean_ai_native_verify_witness",
            "Verify a saved witness against its current subject using the same engine as the CLI.",
        ),
    };
    ToolDescriptor {
        name: name.into(),
        description: description.into(),
        input_schema: json!({
            "type": "object", "properties": Value::Object(properties),
            "required": required, "additionalProperties": false,
        }),
    }
}
