//! Queries an explicitly prepared M16 basis. No per-call preparation authority.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05E1-DISCOVERY"
);
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::{Value, json};
use std::sync::Arc;
use vibe_orchestrator::document_analysis::PreparedConfigurationDiscovery;

#[derive(Default)]
#[specmark::cell(seam = "McpTool", variant = "configuration_discovery")]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05E1-DISCOVERY"
)]
pub struct ConfigurationDiscoveryMcpTool {
    basis: Option<Arc<PreparedConfigurationDiscovery>>,
}
impl ConfigurationDiscoveryMcpTool {
    pub fn prepared(basis: Arc<PreparedConfigurationDiscovery>) -> Self {
        Self { basis: Some(basis) }
    }
}
impl McpTool for ConfigurationDiscoveryMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "configuration_discovery".into(),
            description: "Query one host-prepared frozen configuration basis. Does not prepare, choose, solve, fetch or write. Unconfigured returns unavailable.".into(),
            input_schema: json!({"type":"object","properties":{"capture":{"type":"string"}},"additionalProperties":false}),
        }
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let object = args
            .as_object()
            .ok_or_else(|| ToolError::InvalidArguments("expected object".into()))?;
        if object.keys().any(|k| k != "capture")
            || object.get("capture").is_some_and(|v| !v.is_string())
        {
            return Err(ToolError::InvalidArguments(
                "expected optional capture identity only".into(),
            ));
        }
        let Some(basis) = &self.basis else {
            return Ok(ToolOutput::ok(
                serde_json::to_value(vibe_workspace::configuration_discovery::discover([]))
                    .map_err(|e| ToolError::Internal(e.to_string()))?,
            ));
        };
        if ctx.project_root != basis.root()
            || object.get("capture").and_then(Value::as_str) != Some(basis.capture_id())
        {
            return Err(ToolError::InvalidArguments(
                "missing or foreign prepared capture".into(),
            ));
        }
        Ok(ToolOutput::ok(
            serde_json::to_value(basis.query()).map_err(|e| ToolError::Internal(e.to_string()))?,
        ))
    }
}
#[cfg(test)]
mod m_05_e_1_cases {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Arc<PreparedConfigurationDiscovery>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname='empty'\nversion='1.0.0'\n",
        )
        .unwrap();
        let capture =
            vibe_orchestrator::document_analysis::prepare_configuration_discovery(dir.path(), &[])
                .unwrap();
        let basis = Arc::new(PreparedConfigurationDiscovery::admit(capture, vec![]));
        (dir, basis)
    }
    #[test]
    fn same_service_result_as_cli() {
        let (dir, basis) = fixture();
        let tool = ConfigurationDiscoveryMcpTool::prepared(basis.clone());
        let result = tool
            .run(
                &json!({"capture":basis.capture_id()}),
                &ServerContext::new(dir.path()),
            )
            .unwrap();
        assert_eq!(
            result.structured(),
            &serde_json::to_value(basis.query()).unwrap()
        );
    }
    #[test]
    fn empty_cache_no_provider() {
        let dir = tempfile::tempdir().unwrap();
        let result = ConfigurationDiscoveryMcpTool::default()
            .run(&json!({}), &ServerContext::new(dir.path()))
            .unwrap();
        assert_eq!(result.structured()["status"], "unavailable");
        assert_eq!(result.structured()["counters"]["provider_calls"], 0);
        assert!(
            ConfigurationDiscoveryMcpTool::default()
                .run(
                    &json!({"cached_manifests":[]}),
                    &ServerContext::new(dir.path())
                )
                .is_err()
        );
    }
    #[test]
    fn purity_counters_and_no_mutation() {
        let (dir, basis) = fixture();
        let before = std::fs::read(dir.path().join("vibe.toml")).unwrap();
        let tool = ConfigurationDiscoveryMcpTool::prepared(basis.clone());
        tool.run(
            &json!({"capture":basis.capture_id()}),
            &ServerContext::new(dir.path()),
        )
        .unwrap();
        assert_eq!(std::fs::read(dir.path().join("vibe.toml")).unwrap(), before);
        assert_eq!(
            (
                basis.query().counters.provider_calls,
                basis.query().counters.fetches,
                basis.query().counters.solves,
                basis.query().counters.authored_writes,
                basis.query().counters.cache_writes
            ),
            (0, 0, 0, 0, 0)
        );
    }
    #[test]
    fn unresolved_returns_needs_selection_once() {
        // Admitted unresolved MCP parity is also exercised by the CLI target,
        // which already owns the normalized resolver fixture dependencies.
        let (dir, basis) = fixture();
        let tool = ConfigurationDiscoveryMcpTool::prepared(basis.clone());
        let args = json!({"capture":basis.capture_id()});
        let first = tool.run(&args, &ServerContext::new(dir.path())).unwrap();
        std::fs::remove_file(dir.path().join("vibe.toml")).unwrap();
        assert_eq!(
            first.structured(),
            tool.run(&args, &ServerContext::new(dir.path()))
                .unwrap()
                .structured()
        );
        assert!(
            tool.run(
                &json!({"capture":"foreign"}),
                &ServerContext::new(dir.path())
            )
            .is_err()
        );
        assert!(
            tool.run(&json!({}), &ServerContext::new(dir.path()))
                .is_err()
        );
    }
}
