//! The configured Arc is the sole adapter session owner.
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::{Value, json};
use vibe_orchestrator::machine_operation::adapter_plan;
pub struct AdapterPlanMcpTool;
impl McpTool for AdapterPlanMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "adapter_plan".into(),
            description: "Query the host-bound prepared adapter session.".into(),
            input_schema: json!({"type":"object","properties":{"schema":{"const":1},"operation_id":{"type":"string"},"domain_owner":{"type":"string"},"source_id":{"type":"string"},"evidence_id":{"type":"null"},"body":{"type":"object"}},"required":["schema","operation_id","domain_owner","source_id","evidence_id","body"],"additionalProperties":false}),
        }
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let reply = match ctx.adapter_plan_route() {
            None => adapter_plan::refusal(adapter_plan::missing()),
            Some(route) => match route.compare_context_root(&ctx.project_root) {
                Err(error) => adapter_plan::refusal(error),
                Ok(()) => route.exchange(
                    &serde_json::to_vec(args).map_err(|e| ToolError::Internal(e.to_string()))?,
                ),
            },
        };
        let failed = matches!(
            reply.body,
            vibe_wire::generated::machine::e1::operation_envelope::Body::Error(_)
        );
        let report = serde_json::to_value(reply).map_err(|e| ToolError::Internal(e.to_string()))?;
        if failed {
            Err(ToolError::StructuredPreflight {
                report,
                message: "adapter planning refused; see typed reason".into(),
            })
        } else {
            Ok(ToolOutput::ok(report))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use vibe_orchestrator::machine_operation::adapter_plan::{
        AdapterPlanHost, AdapterPlanRoute, AdapterScope, AdapterTarget,
    };
    use vibe_wire::generated::adapter_plan::e1::{error as e, request as q};
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("vibe.toml"), "[package]\ngroup='org.example'\nname='fixture'\nkind='tool'\nversion='1.0.0'\nauthors=['Fixture']\nlicense='MIT'\ndescription='Fixture'\nkeywords=['fixture']\n[[skill]]\nname='demo'\npath='skill'\nagents=['claude']\n").unwrap();
        std::fs::create_dir(root.path().join("skill")).unwrap();
        std::fs::write(root.path().join("skill/SKILL.md"), "body").unwrap();
        root
    }
    fn report(result: Result<ToolOutput, ToolError>) -> Value {
        match result {
            Ok(output) => output.structured().clone(),
            Err(ToolError::StructuredPreflight { report, .. }) => report,
            Err(error) => panic!("{error}"),
        }
    }
    #[test]
    fn registered_configured_arc_matches_owner_and_refuses_source_drift() {
        let root = fixture();
        let host = AdapterPlanHost::new(
            root.path(),
            AdapterTarget::Skills {
                scope: AdapterScope::Project,
                user_home: None,
            },
        )
        .unwrap();
        let prepared = host.prepare(true).unwrap();
        let route = Arc::new(AdapterPlanRoute::new(host, Some(prepared)).unwrap());
        let mut ctx = ServerContext::with_store_root(root.path(), root.path().join("absent-cache"))
            .with_adapter_plan_route(route.clone())
            .unwrap();
        assert!(Arc::ptr_eq(ctx.adapter_plan_route().unwrap(), &route));
        let tool = super::super::default_tools()
            .into_iter()
            .find(|tool| tool.descriptor().name == "adapter_plan")
            .unwrap();
        let request = adapter_plan::input(&q::Request {
            schema: 1,
            context_id: route.context_id().into(),
            target_id: route.target_id().into(),
            command: q::Command::Plan(Box::new(q::CommandPlan {
                capture_id: route.capture_id().map(str::to_owned),
            })),
        });
        let value = serde_json::to_value(request).unwrap();
        let expected =
            serde_json::to_value(route.exchange(&serde_json::to_vec(&value).unwrap())).unwrap();
        assert_eq!(report(tool.run(&value, &ctx)), expected);
        std::fs::write(root.path().join("skill/SKILL.md"), "drift").unwrap();
        let changed = report(tool.run(&value, &ctx));
        let payload: e::Error =
            serde_json::from_str(changed["body"]["payload_json"].as_str().unwrap()).unwrap();
        assert_eq!(payload.reason, Some(e::ErrorReason::SourceCaptureStale));
        assert!(!root.path().join("absent-cache").exists());
        ctx.project_root = root.path().join("foreign");
        let wrong = report(tool.run(&value, &ctx));
        let error: e::Error =
            serde_json::from_str(wrong["body"]["payload_json"].as_str().unwrap()).unwrap();
        assert_eq!(error.code, e::ErrorCode::WrongContext);
    }
    #[test]
    fn unconfigured_and_wrong_root_return_generated_errors() {
        let root = fixture();
        let ctx = ServerContext::with_store_root(root.path(), root.path().join("absent-cache"));
        let value = report(AdapterPlanMcpTool.run(&json!({}), &ctx));
        let payload: e::Error =
            serde_json::from_str(value["body"]["payload_json"].as_str().unwrap()).unwrap();
        assert_eq!(payload, adapter_plan::missing());
        let host = AdapterPlanHost::new(
            root.path(),
            AdapterTarget::Skills {
                scope: AdapterScope::Project,
                user_home: None,
            },
        )
        .unwrap();
        let route = Arc::new(AdapterPlanRoute::new(host, None).unwrap());
        let other = tempfile::tempdir().unwrap();
        let result = ServerContext::with_store_root(other.path(), other.path().join("cache"))
            .with_adapter_plan_route(route);
        match result {
            Err(error) => assert_eq!(error.code, e::ErrorCode::WrongContext),
            Ok(_) => panic!("foreign root admitted"),
        }
    }
    use vibe_orchestrator::machine_operation::adapter_plan as adapter_owner;
    #[cfg(unix)]
    mod shared_fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vibe-orchestrator/src/machine_operation/adapter_plan/test_fixture.rs"
        ));
    }
    #[test]
    #[cfg(unix)]
    fn configured_deploy_currentness_unavailable_is_generated_no_effects() {
        let (root, owned, route, trace) = shared_fixture::deploy_route(false);
        let ctx = ServerContext::with_store_root(root.path(), owned.path().join("cache"))
            .with_adapter_plan_route(route.clone())
            .unwrap();
        let input = adapter_plan::input(&q::Request {
            schema: 1,
            context_id: route.context_id().into(),
            target_id: route.target_id().into(),
            command: q::Command::Plan(Box::new(q::CommandPlan {
                capture_id: route.capture_id().map(str::to_owned),
            })),
        });
        let before = std::fs::read(&trace).unwrap();
        let answer = report(AdapterPlanMcpTool.run(&serde_json::to_value(&input).unwrap(), &ctx));
        assert_eq!(
            answer,
            serde_json::to_value(route.exchange(&serde_json::to_vec(&input).unwrap())).unwrap()
        );
        let error: e::Error =
            serde_json::from_str(answer["body"]["payload_json"].as_str().unwrap()).unwrap();
        assert_eq!(error.reason, Some(e::ErrorReason::ClientStateUnavailable));
        assert_eq!(std::fs::read(&trace).unwrap(), before);
    }
}
