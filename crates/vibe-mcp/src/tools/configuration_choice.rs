//! The default registered tool delegates to the context's explicitly prepared host.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05E4-PREPARED-OPERATIONS"
);
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::{Value, json};
#[cfg(test)]
use vibe_orchestrator;
#[cfg(test)]
#[path = "../../../vibe-orchestrator/src/machine_operation/configuration_choice_fixture.rs"]
mod fixture;
#[cfg(test)]
mod m_05_e_4_cases {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn default_registered_server_prepares_same_graph_and_applies_intent() {
        let (_dir, host) = fixture::fixture();
        let q = host.questions();
        let args = json!({"operation":"prepare","capture":host.capture(),"shown":q["shown"],
            "request":q["request"],"allow_defaults":true});
        let expected = host
            .dispatch(serde_json::from_value(args.clone()).unwrap())
            .unwrap();
        let host = Arc::new(host);
        let ctx = ServerContext::new(host.root())
            .with_configuration_choices(host.clone())
            .unwrap();
        let input = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":"configuration_choice","arguments":args}})
        .to_string()
            + "\n";
        let mut server = crate::Server::new(crate::MemoryTransport::with_input(input), ctx);
        server.run().unwrap();
        let output: Value = serde_json::from_str(server.transport.take_output().trim()).unwrap();
        assert_eq!(output["result"]["structuredContent"], expected);
        let ctx = ServerContext::new(host.root())
            .with_configuration_choices(host.clone())
            .unwrap();
        let applied = ConfigurationChoiceMcpTool
            .run(
                &json!({"operation":"apply","capture":host.capture(),"plan":expected["plan"]}),
                &ctx,
            )
            .unwrap();
        assert_eq!(applied.structured()["status"], "completed");
        assert!(host.root().join("vibe.lock").exists());
    }
    #[test]
    fn machine_missing_unknown_and_unconfigured_never_prompt_or_write() {
        let (_dir, host) = fixture::fixture();
        let q = host.questions();
        let host = Arc::new(host);
        let ctx = ServerContext::new(host.root())
            .with_configuration_choices(host.clone())
            .unwrap();
        let mut args = json!({"operation":"prepare","capture":host.capture(),"shown":q["shown"],"request":q["request"],"allow_defaults":false});
        assert_eq!(
            ConfigurationChoiceMcpTool
                .run(&args, &ctx)
                .unwrap()
                .structured()["status"],
            "needs_selection"
        );
        args["request"]["edges"][0]["selections"]["choice"] =
            json!({"intent":"exact","members":["unknown"],"provenance":"mcp:explicit"});
        assert!(ConfigurationChoiceMcpTool.run(&args, &ctx).is_err());
        assert!(!host.root().join("vibe.lock").exists());
        assert_eq!(
            ConfigurationChoiceMcpTool
                .run(
                    &json!({"operation":"questions","capture":"absent"}),
                    &ServerContext::new(host.root())
                )
                .unwrap()
                .structured()["status"],
            "unavailable"
        );
    }
}
#[specmark::cell(seam = "McpTool", variant = "configuration_choice")]
pub struct ConfigurationChoiceMcpTool;
impl McpTool for ConfigurationChoiceMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor { name:"configuration_choice".into(),
            description:"Questions, explicit preparation, and separately confirmed application over a host-admitted captured configuration. No terminal input.".into(),
            input_schema:json!({"type":"object","required":["operation","capture"],"properties":{
                "operation":{"enum":["questions","evaluate","prepare","apply"]},"capture":{"type":"string"},
                "shown":{"type":"string"},"request":{"type":"object"},"allow_defaults":{"type":"boolean"},"plan":{"type":"string"}},"additionalProperties":false}) }
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let operation = serde_json::from_value(args.clone())
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let Some(host) = ctx.configuration_choice_host() else {
            return Ok(ToolOutput::ok(
                json!({"status":"unavailable","reason":"prepared configuration authority unavailable"}),
            ));
        };
        if host.root() != ctx.project_root {
            return Err(ToolError::InvalidArguments(
                "configuration choice root differs".into(),
            ));
        }
        host.dispatch(operation)
            .map(ToolOutput::ok)
            .map_err(ToolError::InvalidArguments)
    }
}
