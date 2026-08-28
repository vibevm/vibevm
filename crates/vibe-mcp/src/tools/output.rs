//! The executed-result output of the [`McpTool`](super::McpTool) seam —
//! `ToolOutput`, the value a tool returns when the Rust call itself
//! succeeded, kept beside the seam it completes. Split from `tools.rs`
//! as its own cell-file for the same reason `query.rs` and `select.rs`
//! are: that registry file sits at its size ceiling, and this type
//! carries its own documentation burden (the preflight-vs-executed
//! distinction every tool author must know).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");

use std::ops::Deref;

use serde_json::Value;
use specmark::spec;

/// What a tool returns when the operation **executed** — the `Ok` half
/// of [`McpTool::run`](super::McpTool::run)'s contract.
///
/// A tool leaves the dispatcher through exactly two channels, and the
/// choice between them is *did it run?*, not *did it succeed?*:
///
/// - **Preflight / tool failure** — `Err(ToolError)`. The operation did
///   not execute: arguments failed validation, the target was not
///   found, or an I/O / internal error struck before any work happened.
///   There is no structured report to carry, so the dispatcher renders
///   `isError: true` with the error's text and **no** `structuredContent`.
///   That arm is unchanged by this type.
/// - **Executed** — `Ok(ToolOutput)`. The operation ran and produced a
///   structured report plus its human text projection. Within this
///   channel there are two shapes: [`ToolOutput::ok`] is ordinary
///   success (`isError: false`), and [`ToolOutput::executed_failure`]
///   is the **executed structured failure** — the operation ran,
///   produced this report, and the MCP result must still say
///   `isError: true`, but the report is retained as the single
///   `structuredContent` instead of collapsing to text.
///   "I ran and here is what happened" is a different statement from
///   "I never ran", and collapsing the former into the latter would
///   throw away exactly the report a caller asked for.
///
/// The text projection is **mandatory on both shapes**, and for a
/// reason: an executed failure must never fall back to a pretty-JSON
/// rendering of its report — that would bury the actionable typed
/// error chain a reader of the text channel needs. [`ToolOutput::ok`]
/// computes the text at construction with exactly the projection the
/// dispatcher applied before this seam existed (the raw string for a
/// JSON string value, pretty JSON for anything else), and
/// [`ToolOutput::executed_failure`] requires the caller to state the
/// text outright — there is no bare variant and no fallback.
///
/// Fields are private on purpose: `is_error` and the text are
/// constructor invariants (`ok` can never masquerade as a failure, and
/// the text can never contradict the shape it was built from), so the
/// type grants read access — [`Deref`] to the structured [`Value`]
/// plus the [`structured`](Self::structured) /
/// [`text`](Self::text) / [`is_error`](Self::is_error) accessors — and
/// no mutation.
///
/// ```
/// use serde_json::json;
/// use vibe_mcp::tools::ToolOutput;
///
/// // Ordinary success: the text is computed at construction — exactly
/// // the pre-seam dispatcher projection.
/// let ok = ToolOutput::ok(json!({ "status": "materialised" }));
/// assert!(!ok.is_error());
/// assert_eq!(
///     ok.text(),
///     serde_json::to_string_pretty(&json!({ "status": "materialised" })).unwrap()
/// );
/// assert_eq!(ok["status"], "materialised"); // Deref → read access
///
/// // Executed structured failure: the text is MANDATORY — an executed
/// // failure must never fall back to pretty JSON and lose the
/// // actionable error chain.
/// let failed = ToolOutput::executed_failure(
///     json!({ "status": "refused", "written": 0 }),
///     "the operation ran but refused the request",
/// );
/// assert!(failed.is_error());
/// assert_eq!(failed.text(), "the operation ran but refused the request");
/// assert_eq!(failed.structured()["status"], "refused");
/// ```
#[derive(Debug, Clone, PartialEq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ToolOutput {
    structured: Value,
    text: String,
    is_error: bool,
}

impl ToolOutput {
    /// Ordinary success: the operation executed and produced
    /// `structured`. The text projection is computed here, at
    /// construction, with exactly the deterministic projection the
    /// dispatcher applied before this seam existed — the raw string
    /// for a JSON string value, pretty JSON for anything else — so a
    /// constructed `ToolOutput` renders byte-identically to the
    /// pre-seam `Ok(Value)` it replaced.
    pub fn ok(structured: impl Into<Value>) -> Self {
        let structured = structured.into();
        let text = deterministic_text(&structured);
        ToolOutput {
            structured,
            text,
            is_error: false,
        }
    }

    /// Executed structured failure: the operation ran, produced
    /// `structured`, and the MCP result must carry `isError: true`
    /// with `structured` as its single `structuredContent` — and the
    /// human `text` stated outright. The text is mandatory and has no
    /// fallback on purpose: a bare variant would let an executed
    /// failure render as pretty JSON and lose the actionable typed
    /// error chain the text channel exists to carry.
    pub fn executed_failure(structured: impl Into<Value>, text: impl Into<String>) -> Self {
        ToolOutput {
            structured: structured.into(),
            text: text.into(),
            is_error: true,
        }
    }

    /// The structured report — what the dispatcher places in the
    /// result's single `structuredContent` field.
    pub fn structured(&self) -> &Value {
        &self.structured
    }

    /// The human text projection — what the dispatcher places in the
    /// text channel. Mandatory on every shape.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the rendered MCP result must carry `isError: true`.
    pub fn is_error(&self) -> bool {
        self.is_error
    }

    /// Consume into the `(structured, text, is_error)` triple — the
    /// dispatcher's path, so the structured [`Value`] moves into the
    /// rendered result instead of being cloned out.
    pub fn into_parts(self) -> (Value, String, bool) {
        (self.structured, self.text, self.is_error)
    }
}

/// The one deterministic text projection for executed results — the
/// exact expression the dispatcher applied to every `Ok(Value)` before
/// this seam existed. Kept here (not in the dispatcher) because the
/// constructor owns the invariant "text is fixed at construction".
fn deterministic_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// Read-only ergonomics over the structured value: indexing and the
/// `Value` readers (`as_str`, `as_array`, …) work directly on a
/// `ToolOutput`. Deliberately **not** `DerefMut` — mutating the value
/// after construction could contradict the constructor invariants.
impl Deref for ToolOutput {
    type Target = Value;

    fn deref(&self) -> &Value {
        &self.structured
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::McpTool;
    use crate::{MemoryTransport, Server, ServerContext, ToolDescriptor, ToolError};

    // --- constructor invariants -------------------------------------------

    /// `ok` fixes the text at construction with the exact pre-seam
    /// projection: pretty JSON for objects, the raw string for strings.
    #[test]
    fn ok_stores_the_pre_seam_text_at_construction() {
        let obj = ToolOutput::ok(serde_json::json!({ "a": 1 }));
        assert!(!obj.is_error());
        assert_eq!(obj.text(), "{\n  \"a\": 1\n}");
        assert_eq!(obj.structured(), &serde_json::json!({ "a": 1 }));

        let s = ToolOutput::ok(Value::String("plain".to_string()));
        assert_eq!(s.text(), "plain");
        assert_eq!(s.structured(), &Value::String("plain".to_string()));
    }

    /// The failure constructor requires its text outright — there is no
    /// bare variant, so nothing can fall back to pretty JSON.
    #[test]
    fn executed_failure_requires_and_keeps_its_text() {
        let failed = ToolOutput::executed_failure(serde_json::json!({ "a": 1 }), "typed chain");
        assert!(failed.is_error());
        assert_eq!(failed.text(), "typed chain");
        assert_eq!(failed.structured(), &serde_json::json!({ "a": 1 }));
    }

    /// The dispatcher's path moves the parts, not a clone.
    #[test]
    fn into_parts_yields_the_triple() {
        let out = ToolOutput::executed_failure(serde_json::json!({ "a": 1 }), "typed chain");
        let (structured, text, is_error) = out.into_parts();
        assert_eq!(structured, serde_json::json!({ "a": 1 }));
        assert_eq!(text, "typed chain");
        assert!(is_error);
    }

    // --- the executed-structured-failure dispatch arm ----------------------
    //
    // One probe cell behind the seam, driven end-to-end through the
    // real dispatcher over a MemoryTransport. Under the pre-seam
    // contract (`run -> Result<Value, ToolError>`) this probe's answer
    // was UNREPRESENTABLE: its only failure channel was
    // `Err(ToolError)`, which renders text-only with no
    // `structuredContent` — the third arm (isError true AND the
    // structured report retained) did not exist.

    /// The probe: executes, then reports failure with an explicit text.
    struct ExecutedFailureProbe;

    impl McpTool for ExecutedFailureProbe {
        fn descriptor(&self) -> ToolDescriptor {
            ToolDescriptor {
                name: "executed_failure_probe".to_string(),
                description: "test probe: returns an executed structured failure".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            }
        }

        fn run(&self, _args: &Value, _ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::executed_failure(
                serde_json::json!({ "report": "the operation executed" }),
                "executed, but reported failure",
            ))
        }
    }

    fn dispatch_probe() -> Value {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "executed_failure_probe", "arguments": {} }
        })
        .to_string();
        let transport = MemoryTransport::with_input(request + "\n");
        let mut server = Server::new(transport, ServerContext::new("."));
        server.register_tool(Box::new(ExecutedFailureProbe));
        server.run().unwrap();
        let output = server.transport.take_output();
        serde_json::from_str(output.trim()).unwrap()
    }

    /// The RED this seam exists for: an executed structured failure keeps
    /// its report as the single `structuredContent` under `isError: true`,
    /// with its explicit text in the text channel. Before the seam the
    /// only way to say "failure" was `Err(ToolError)`, which the
    /// dispatcher renders with no `structuredContent` at all.
    #[test]
    fn executed_structured_failure_retains_structured_content() {
        let v = dispatch_probe();
        assert!(
            v["error"].is_null(),
            "dispatched, not a transport error: {v}"
        );
        assert_eq!(v["result"]["isError"], true);
        assert_eq!(
            v["result"]["structuredContent"],
            serde_json::json!({ "report": "the operation executed" })
        );
        assert_eq!(v["result"]["content"].as_array().map(|a| a.len()), Some(1));
        assert_eq!(v["result"]["content"][0]["type"], "text");
        assert_eq!(
            v["result"]["content"][0]["text"],
            "executed, but reported failure"
        );
    }
}
