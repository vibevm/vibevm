//! Resolve and inspect through the same selected-source request owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-NATIVE-LOCATION");
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::Value;

pub struct SpecResolveMcpTool;
impl McpTool for SpecResolveMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor{name:"spec_resolve".into(),description:"Resolve a source-neutral spec locator against the exact captured selected slot, returning native location and observation identities. Uses the same generated epoch-one source request/refusal as read_spec; no physical root or effect authority from arguments.".into(),input_schema:vibe_wire::behaviour::source_query::request_json_schema()}
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        super::read_spec::run_query(args, ctx)
    }
}
