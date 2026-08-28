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
///   channel there are three shapes, and the axis that separates them
///   is NOT success-vs-failure — it is *who writes the text*:
///   - [`ToolOutput::ok`] — ordinary success (`isError: false`) whose
///     text is the deterministic projection of the report;
///   - [`ToolOutput::executed`] — ordinary success (`isError: false`)
///     whose text the tool states outright, because the surface has
///     something to say that the report does not carry. A parked
///     lifecycle run is the motivating case: the wire report is
///     surface-identical and names the CLI resume command, while the
///     MCP-native guidance ("call this tool again with the same
///     phase") belongs only in the textual projection. Without this
///     shape that guidance is unrepresentable, because `ok` fixes its
///     own text and the only alternative says `isError: true`;
///   - [`ToolOutput::executed_failure`] — the **executed structured
///     failure**: the operation ran, produced this report, and the MCP
///     result must still say `isError: true`, but the report is
///     retained as the single `structuredContent` instead of
///     collapsing to text.
///
///   "I ran and here is what happened" is a different statement from
///   "I never ran", and collapsing the former into the latter would
///   throw away exactly the report a caller asked for. That distinction
///   is unchanged by the middle shape: `executed` is still the executed
///   channel, and a preflight refusal still has no `ToolOutput` at all.
///
/// The text projection is **mandatory on all three shapes**, and for a
/// reason: an executed result must never silently fall back to a
/// pretty-JSON rendering of its report where a surface had something
/// better to say — for a failure that would bury the actionable typed
/// error chain, and for a park it would bury the resume instruction.
/// [`ToolOutput::ok`] computes the text at construction with exactly
/// the projection the dispatcher applied before this seam existed (the
/// raw string for a JSON string value, pretty JSON for anything else),
/// while [`ToolOutput::executed`] and
/// [`ToolOutput::executed_failure`] require the caller to state the
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
/// // Executed success with explicit text: the same `isError: false`
/// // and the same structured report, but the surface writes the text
/// // itself. Nothing else in this type can say that.
/// let parked = ToolOutput::executed(
///     json!({ "ok": true, "requested": "build" }),
///     "parked for you — call `lifecycle_run` again with phase `build`",
/// );
/// assert!(!parked.is_error());
/// assert_eq!(
///     parked.text(),
///     "parked for you — call `lifecycle_run` again with phase `build`"
/// );
/// assert_eq!(parked.structured()["requested"], "build");
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

    /// Executed success with the text stated outright: the operation
    /// ran, produced `structured`, the MCP result carries
    /// `isError: false` and `structured` as its single
    /// `structuredContent` — and the human `text` is the caller's own,
    /// not a projection of the report.
    ///
    /// This is [`Self::ok`] in every respect except who writes the
    /// text, and it exists because that difference was previously
    /// unrepresentable. A tool whose report is deliberately
    /// surface-identical — the lifecycle report is the same document
    /// whichever surface asked for the run — still owes its own channel
    /// an MCP-native sentence: which tool to call again, with which
    /// argument. Reaching for [`Self::executed_failure`] to get an
    /// explicit text would report a successful park as
    /// `isError: true`; reaching for [`Self::ok`] would replace the
    /// sentence with pretty JSON the caller can already read in
    /// `structuredContent`.
    ///
    /// It is deliberately NOT a defaulting variant of [`Self::ok`]:
    /// the text is required, so a caller cannot construct this shape
    /// and then discover its text was silently derived.
    pub fn executed(structured: impl Into<Value>, text: impl Into<String>) -> Self {
        ToolOutput {
            structured: structured.into(),
            text: text.into(),
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

    /// The whole point of the third shape, stated at the constructor:
    /// `is_error` is `false` like [`ToolOutput::ok`], while the text is
    /// the caller's EXACT bytes and not the deterministic projection.
    ///
    /// Both halves are asserted against the same structured value, so a
    /// mutation in either direction is red: deriving the text makes the
    /// second assertion fail (the report pretty-prints to something
    /// else entirely), and flipping the flag makes the first fail.
    #[test]
    fn executed_keeps_the_callers_exact_text_and_is_not_an_error() {
        let structured = serde_json::json!({ "ok": true, "requested": "build" });
        let guidance = "parked — call `lifecycle_run` again with phase `build`";
        let parked = ToolOutput::executed(structured.clone(), guidance);

        assert!(!parked.is_error(), "an executed park is a SUCCESS");
        assert_eq!(parked.text(), guidance, "the caller's exact bytes");
        assert_eq!(parked.structured(), &structured);
        assert_ne!(
            parked.text(),
            deterministic_text(&structured),
            "and NOT the projection `ok` would have computed",
        );
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

    /// Drive one registered probe through the REAL dispatcher over a
    /// `MemoryTransport` and return the parsed JSON-RPC response. The
    /// rendering under test is `handle_tools_call`'s, not this cell's:
    /// a probe asserting its own projection would prove nothing.
    fn dispatch(name: &str, tool: Box<dyn McpTool>) -> Value {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": {} }
        })
        .to_string();
        let transport = MemoryTransport::with_input(request + "\n");
        let mut server = Server::new(transport, ServerContext::new("."));
        server.register_tool(tool);
        server.run().unwrap();
        let output = server.transport.take_output();
        serde_json::from_str(output.trim()).unwrap()
    }

    fn dispatch_probe() -> Value {
        dispatch("executed_failure_probe", Box::new(ExecutedFailureProbe))
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

    // --- the executed-success-with-explicit-text dispatch arm --------------
    //
    // The third arm, driven through the same real dispatcher. Before
    // `ToolOutput::executed` existed this answer was UNREPRESENTABLE:
    // `ok` fixes its own text (the caller's sentence would have been
    // replaced by pretty JSON of a report the caller can already read in
    // `structuredContent`), and the only constructor that accepts a text
    // renders `isError: true` — which would report a successful park as
    // a failure.

    /// The probe: a surface-identical report plus MCP-native guidance.
    struct ExecutedSuccessProbe;

    /// The exact report the probe returns — shared with the assertions so
    /// the test compares against ONE value rather than a retyped copy.
    fn parked_report() -> Value {
        serde_json::json!({ "ok": true, "requested": "build", "command": "lifecycle" })
    }

    /// The MCP-native sentence the wire report deliberately does not
    /// carry: the report's own `delegation.resume` is the CLI command,
    /// because the document is surface-identical.
    const PARKED_GUIDANCE: &str = "parked — call `lifecycle_run` again with phase `build`";

    impl McpTool for ExecutedSuccessProbe {
        fn descriptor(&self) -> ToolDescriptor {
            ToolDescriptor {
                name: "executed_success_probe".to_string(),
                description: "test probe: returns an executed success with explicit text"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            }
        }

        fn run(&self, _args: &Value, _ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::executed(parked_report(), PARKED_GUIDANCE))
        }
    }

    /// The RED this constructor exists for: `isError: false`, the report
    /// retained unchanged as the single `structuredContent`, and exactly
    /// ONE text row carrying the surface's own sentence — never the
    /// pretty-JSON projection of the report beside it.
    #[test]
    fn executed_success_renders_explicit_text_beside_the_same_structured_content() {
        let v = dispatch("executed_success_probe", Box::new(ExecutedSuccessProbe));

        assert!(
            v["error"].is_null(),
            "dispatched, not a transport error: {v}"
        );
        // A park is a SUCCESS with a handoff, not an error.
        assert_eq!(v["result"]["isError"], false);
        // The report crosses byte for byte — the same single root the
        // `ok` shape would have carried.
        assert_eq!(v["result"]["structuredContent"], parked_report());
        // Exactly one text row, and it is the surface's sentence.
        assert_eq!(v["result"]["content"].as_array().map(|a| a.len()), Some(1));
        assert_eq!(v["result"]["content"][0]["type"], "text");
        assert_eq!(v["result"]["content"][0]["text"], PARKED_GUIDANCE);
        // …stated as the negative too, because THIS is the mutation the
        // constructor exists to refuse: a text derived from the report.
        assert_ne!(
            v["result"]["content"][0]["text"],
            Value::String(deterministic_text(&parked_report())),
            "the dispatcher must not have re-derived the text from the report",
        );
    }
}
