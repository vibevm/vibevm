specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#EXECUTED-DIAGNOSTIC-FAILURE"
);
use serde_json::{Value, json};
use std::{
    io,
    sync::{Arc, Mutex},
};
use vibe_mcp::{
    McpTool, MemoryTransport, Server, ServerContext, ToolDescriptor, ToolError, ToolOutput,
    Transport,
};

struct OutputProbe(ToolOutput);
impl McpTool for OutputProbe {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "neutral_output_probe".into(),
            description: "generic output test".into(),
            input_schema: json!({"type":"object","additionalProperties":false}),
        }
    }
    fn run(&self, _args: &Value, _ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        Ok(self.0.clone())
    }
}
struct SharedMemory(Arc<Mutex<MemoryTransport>>);
impl Transport for SharedMemory {
    fn read_line(&mut self) -> io::Result<Option<String>> {
        self.0.lock().unwrap().read_line()
    }
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        self.0.lock().unwrap().write_line(line)
    }
}
fn dispatch(output: ToolOutput) -> Value {
    let temp = tempfile::tempdir().unwrap();
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call", "params":{"name":"neutral_output_probe","arguments":{}}});
    let memory = Arc::new(Mutex::new(MemoryTransport::with_input(format!(
        "{request}\n"
    ))));
    let ctx = ServerContext::with_store_root(
        temp.path().join("absent-project"),
        temp.path().join("absent-cache"),
    );
    let mut server = Server::new(SharedMemory(memory.clone()), ctx);
    server.register_tool(Box::new(OutputProbe(output)));
    server.run().unwrap();
    let response: Value =
        serde_json::from_str(memory.lock().unwrap().take_output().trim()).unwrap();
    assert!(response.get("error").is_none());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    response["result"].clone()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#EXECUTED-DIAGNOSTIC-FAILURE",
    r = 1
)]
fn executed_diagnostic_failure_omits_structured_content() {
    let out = ToolOutput::executed_failure_text("unvalidated retained evidence: diagnostic");
    assert!(!out.has_structured());
    assert_eq!(out.structured(), &Value::Null);
    let (value, text, error) = out.clone().into_parts();
    assert_eq!(value, Value::Null);
    assert!(error);
    assert_eq!(text, out.text());
    let result = dispatch(out);
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["content"],
        json!([{"type":"text","text":"unvalidated retained evidence: diagnostic"}])
    );
    assert!(result.get("structuredContent").is_none());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#EXECUTED-DIAGNOSTIC-FAILURE",
    r = 1
)]
fn explicit_null_structured_outputs_retain_the_field() {
    for output in [
        ToolOutput::ok(Value::Null),
        ToolOutput::executed(Value::Null, "success"),
        ToolOutput::executed_failure(Value::Null, "failure"),
    ] {
        assert!(output.has_structured());
        let error = output.is_error();
        let text = output.text().to_owned();
        let result = dispatch(output);
        assert_eq!(result.get("structuredContent"), Some(&Value::Null));
        assert_eq!(result["isError"], error);
        assert_eq!(result["content"], json!([{"type":"text","text":text}]));
    }
}
