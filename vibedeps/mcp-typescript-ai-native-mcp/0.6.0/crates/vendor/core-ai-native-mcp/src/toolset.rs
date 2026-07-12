//! The tool registry seam (MCP-CORE §3): a server is a [`ToolSet`] —
//! named tools with JSON-schema'd inputs — mounted on the transport
//! loop. Servers built on this crate implement [`Tool`] per command and
//! register them; the loop owns dispatch, the tools own semantics.

specmark::scope!("spec://org.vibevm.ai-native.core-ai-native/mechanisms/MCP-CORE-v0.1#toolset");

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

/// What `tools/list` shows an agent for one tool.
///
/// ```
/// let d = core_ai_native_mcp::ToolDescriptor {
///     name: "floor".into(),
///     description: "run the verification floor (expect minutes)".into(),
///     input_schema: serde_json::json!({"type": "object", "properties": {}}),
/// };
/// assert_eq!(serde_json::to_value(&d).unwrap()["inputSchema"]["type"], "object");
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// What one tool call produced. `report` becomes the MCP text content;
/// `is_error` marks a tool-level failure (the gate found findings, the
/// oracle refused) — distinct from a PROTOCOL error, which only fires
/// for unknown tools or malformed params (MCP-CORE §3).
///
/// ```
/// let out = core_ai_native_mcp::ToolOutput::ok("floor: all green");
/// assert!(!out.is_error);
/// let red = core_ai_native_mcp::ToolOutput::failed("conform: 3 new finding(s)");
/// assert!(red.is_error);
/// ```
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub report: String,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn ok(report: impl Into<String>) -> Self {
        ToolOutput {
            report: report.into(),
            is_error: false,
        }
    }

    pub fn failed(report: impl Into<String>) -> Self {
        ToolOutput {
            report: report.into(),
            is_error: true,
        }
    }

    /// The MCP `tools/call` result value for this output.
    pub fn into_result_value(self) -> Value {
        serde_json::json!({
            "content": [{ "type": "text", "text": self.report }],
            "isError": self.is_error,
        })
    }
}

/// One mounted tool. `run` returns [`ToolOutput`] — tools NEVER prompt
/// (MCP-CORE §5: a server has no interactive channel; anything that
/// would ask becomes an explicit parameter) and NEVER panic for
/// domain-level failure.
pub trait Tool {
    fn descriptor(&self) -> ToolDescriptor;
    fn run(&mut self, args: &Value) -> ToolOutput;
}

/// The registry: insertion is by descriptor name; `tools/list` renders
/// in stable (sorted) order.
///
/// ```
/// use core_ai_native_mcp::{Tool, ToolDescriptor, ToolOutput, ToolSet};
/// use serde_json::Value;
///
/// struct Ping;
/// impl Tool for Ping {
///     fn descriptor(&self) -> ToolDescriptor {
///         ToolDescriptor {
///             name: "ping".into(),
///             description: "answers pong".into(),
///             input_schema: serde_json::json!({"type": "object"}),
///         }
///     }
///     fn run(&mut self, _args: &Value) -> ToolOutput {
///         ToolOutput::ok("pong")
///     }
/// }
///
/// let mut set = ToolSet::new();
/// set.register(Box::new(Ping));
/// assert_eq!(set.descriptors().len(), 1);
/// assert_eq!(set.run("ping", &Value::Null).unwrap().report, "pong");
/// assert!(set.run("ghost", &Value::Null).is_none());
/// ```
#[derive(Default)]
pub struct ToolSet {
    tools: BTreeMap<String, Box<dyn Tool>>,
}

impl ToolSet {
    pub fn new() -> Self {
        ToolSet::default()
    }

    /// Mount a tool; a repeated name overwrites the previous entry (the
    /// last registration wins, mirroring the product server).
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.descriptor().name, tool);
    }

    /// Stable-order descriptors for `tools/list`.
    pub fn descriptors(&self) -> Vec<ToolDescriptor> {
        self.tools.values().map(|t| t.descriptor()).collect()
    }

    /// Dispatch one call; `None` = no such tool (the loop answers
    /// method-not-found).
    pub fn run(&mut self, name: &str, args: &Value) -> Option<ToolOutput> {
        self.tools.get_mut(name).map(|t| t.run(args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(&'static str);
    impl Tool for Fixed {
        fn descriptor(&self) -> ToolDescriptor {
            ToolDescriptor {
                name: self.0.into(),
                description: String::new(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        fn run(&mut self, _args: &Value) -> ToolOutput {
            ToolOutput::ok(self.0)
        }
    }

    #[test]
    fn descriptors_are_sorted_and_last_registration_wins() {
        let mut set = ToolSet::new();
        set.register(Box::new(Fixed("zeta")));
        set.register(Box::new(Fixed("alpha")));
        set.register(Box::new(Fixed("zeta")));
        let names: Vec<String> = set.descriptors().into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["alpha", "zeta"]);
    }

    #[test]
    fn tool_output_renders_the_mcp_result_shape() {
        let v = ToolOutput::failed("3 findings").into_result_value();
        assert_eq!(v["isError"], true);
        assert_eq!(v["content"][0]["type"], "text");
        assert_eq!(v["content"][0]["text"], "3 findings");
    }
}
