//! MCP adapter over the retained host route, with no adapter snapshot map.
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::{Value, json};
pub struct BootDebuggerMcpTool;
impl McpTool for BootDebuggerMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "boot_debugger".into(),
            description:
                "Captured boot inspection and explicit local scenarios. Call with {} to discover context, then send strict operation envelopes."
                    .into(),
            input_schema: json!({"type":"object","oneOf":[{"type":"object","maxProperties":0},{"type":"object","properties":{"schema":{"const":1},"operation_id":{"type":"string"},"domain_owner":{"type":"string"},"source_id":{"type":"string"},"evidence_id":{"type":"null"},"body":{"type":"object"}},"required":["schema","operation_id","domain_owner","source_id","evidence_id","body"],"additionalProperties":false}]}),
        }
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let route = ctx.boot_debugger_route().ok_or_else(|| {
            ToolError::InvalidArguments("boot debugger host is not configured".into())
        })?;
        route
            .compare_context_root(&ctx.project_root)
            .map_err(|e| ToolError::InvalidArguments(e.message))?;
        // Empty input discovers the host context and captures the admitted
        // inventory; subsequent commands use strict operation envelopes.
        let initial;
        let args = if args.as_object().is_some_and(|o| o.is_empty()) {
            use vibe_wire::generated::boot_debugger::e1::request as q;
            initial = serde_json::to_value(
                vibe_orchestrator::machine_operation::boot_debugger::input(&q::Request {
                    schema: 1,
                    context_id: route.context_id().into(),
                    command: q::Command::Capture(Box::new(q::CommandCapture {})),
                }),
            )
            .map_err(|e| ToolError::Internal(e.to_string()))?;
            &initial
        } else {
            args
        };
        let bytes = serde_json::to_vec(args).map_err(|e| ToolError::Internal(e.to_string()))?;
        let reply = route.exchange(&bytes);
        let is_error = matches!(
            reply.body,
            vibe_wire::generated::machine::e1::operation_envelope::Body::Error(_)
        );
        let report = serde_json::to_value(reply).map_err(|e| ToolError::Internal(e.to_string()))?;
        if is_error {
            Err(ToolError::StructuredPreflight {
                report,
                message: "boot inspection refused; see typed envelope".into(),
            })
        } else {
            Ok(ToolOutput::ok(report))
        }
    }
}
#[cfg(test)]
mod m_06_c_1_cases {
    use super::*;
    use std::sync::Arc;
    use vibe_orchestrator::{document_analysis::*, machine_operation::boot_debugger::*};
    use vibe_wire::generated::boot_debugger::e1::request::*;
    #[test]
    fn two_headless_clients_share_snapshot_and_matching_root_route() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("vibe.toml"), "[project]\n").unwrap();
        let host = DocumentAnalysisHost::new(
            root.path(),
            "vibe.toml",
            vec![
                ResourceDeclaration {
                    path: "vibe.toml".into(),
                    role: ResourceRole::Manifest,
                },
                ResourceDeclaration {
                    path: "STATIC.md".into(),
                    role: ResourceRole::Opaque,
                },
            ],
            AnalysisLimits {
                resource_bytes: 1024,
                capture_bytes: 4096,
                open_buffers: 2,
            },
        )
        .unwrap();
        let artifact = host.resource("STATIC.md").unwrap();
        let client = host.client().unwrap();
        let route = Arc::new(
            BootDebuggerRoute::new(
                host,
                artifact,
                vibe_workspace::boot_debugger::protocol::CapturedKind::Static,
            )
            .unwrap(),
        );
        let a = ServerContext::with_store_root(root.path(), root.path().join("unused"))
            .with_boot_debugger_route(route.clone())
            .unwrap();
        let b = ServerContext::with_store_root(root.path(), root.path().join("unused"))
            .with_boot_debugger_route(route.clone())
            .unwrap();
        let before = route.retain_capture(client.freeze().unwrap()).unwrap();
        let text = "<!-- vibe:static org.test/pkg — source.md -->\n<!-- embed: spec://org.test/pkg/part -->\noverlay\n<!-- /embed: spec://org.test/pkg/part -->\n";
        client
            .open(
                1,
                artifact,
                1,
                ByteEncoding::Utf8,
                Arc::from(text.as_bytes()),
                BufferIntent::Create,
            )
            .unwrap();
        let capture = client.freeze().unwrap();
        let id = capture.id().to_owned();
        route.retain_capture(capture).unwrap();
        let query = serde_json::to_value(input(&Request {
            schema: 1,
            context_id: route.context_id().into(),
            command: Command::Inspect(Box::new(CommandInspect {
                snapshot_id: id.clone(),
            })),
        }))
        .unwrap();
        let x = BootDebuggerMcpTool.run(&query, &a).unwrap();
        let y = BootDebuggerMcpTool.run(&query, &b).unwrap();
        assert_eq!(x.structured(), y.structured());
        use vibe_wire::generated::{
            boot_debugger::e1::result as r, machine::e1::operation_envelope as env,
        };
        let envelope: env::OperationEnvelope =
            serde_json::from_value(x.structured().clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!()
        };
        let result: r::Result =
            serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        let r::Body::Inspect(view) = result.body else {
            panic!()
        };
        assert!(view.nodes[0].artifact_lines.is_none());
        let span = view.nodes[1].artifact_lines.as_ref().unwrap();
        assert_eq!((&*span.start_line, &*span.end_line), ("2", "4"));
        assert_eq!(span.snapshot_id, id);
        assert_eq!(
            view.sources[view.nodes[1].source as usize].artifact_resource_id,
            span.resource_id
        );
        let compare = serde_json::to_value(input(&Request {
            schema: 1,
            context_id: route.context_id().into(),
            command: Command::Compare(Box::new(CommandCompare {
                snapshot_id: before,
                other_snapshot_id: id,
            })),
        }))
        .unwrap();
        let ca = BootDebuggerMcpTool.run(&compare, &a).unwrap();
        let cb = BootDebuggerMcpTool.run(&compare, &b).unwrap();
        assert_eq!(ca.structured(), cb.structured());
        let envelope: env::OperationEnvelope =
            serde_json::from_value(ca.structured().clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!()
        };
        let result: r::Result =
            serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        let r::Body::Compare(diff) = result.body else {
            panic!()
        };
        assert_eq!(diff.added, ["STATIC.md"]);
        assert!(diff.changed.is_empty() && diff.removed.is_empty());
        client
            .change(
                1,
                artifact,
                1,
                2,
                ByteEncoding::Utf8,
                Arc::from(&b"later overlay"[..]),
                BufferIntent::Create,
            )
            .unwrap();
        std::fs::write(root.path().join("STATIC.md"), "later disk").unwrap();
        assert_eq!(
            x.structured(),
            BootDebuggerMcpTool.run(&query, &a).unwrap().structured()
        );

        assert!(
            ServerContext::with_store_root(root.path().join("foreign"), root.path())
                .with_boot_debugger_route(route)
                .is_err()
        );
        assert!(
            BootDebuggerMcpTool
                .run(
                    &query,
                    &ServerContext::with_store_root(root.path(), root.path())
                )
                .is_err()
        );
        assert!(!root.path().join(".vibe").exists());
    }
}

#[cfg(test)]
mod m_06_d_2_cases {
    use super::*;
    use std::sync::Arc;
    use vibe_orchestrator::{document_analysis::*, machine_operation::boot_debugger::*};
    use vibe_wire::generated::{
        boot_debugger::e1::{request as q, result as r},
        machine::e1::operation_envelope as env,
    };
    #[test]
    pub(super) fn ordinary_mcp_route_prepares_local_source_and_reads_retained_bytes() {
        let tool = crate::tools::default_tools()
            .into_iter()
            .find(|tool| tool.descriptor().name == "boot_debugger")
            .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let registry = temp.path().join("registry");
        let cache = temp.path().join("preview");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&cache).unwrap();
        let package = registry.join("org.example/app/v1.0.0");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("vibe.toml"), "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n").unwrap();
        std::fs::write(package.join("boot.md"), b"real producer bytes").unwrap();
        std::fs::write(
            root.join("vibe.toml"),
            "[project]\nname='fixture'\nversion='1.0.0'\n[requires.packages]\n'org.example/app'={version='=1.0.0',link='static'}\n",
        )
        .unwrap();
        for name in ["vibe.lock", "STATIC.md", "INDEX.md"] {
            std::fs::write(root.join(name), format!("canonical {name}")).unwrap();
        }
        let before: Vec<_> = ["vibe.toml", "vibe.lock", "STATIC.md", "INDEX.md"]
            .into_iter()
            .map(|n| (n, std::fs::read(root.join(n)).unwrap()))
            .collect();
        let host = DocumentAnalysisHost::new(
            &root,
            "vibe.toml",
            vec![
                ResourceDeclaration {
                    path: "vibe.toml".into(),
                    role: ResourceRole::Manifest,
                },
                ResourceDeclaration {
                    path: "STATIC.md".into(),
                    role: ResourceRole::Opaque,
                },
            ],
            AnalysisLimits {
                resource_bytes: 4096,
                capture_bytes: 8192,
                open_buffers: 2,
            },
        )
        .unwrap();
        let artifact = host.resource("STATIC.md").unwrap();
        let capture = host.client().unwrap().freeze().unwrap();
        let route = Arc::new(BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap());
        let id = route.retain_capture(capture).unwrap();
        let resolver = vibe_package_source::InstallResolver::Local(
            vibe_registry::LocalRegistry::new(&registry).unwrap(),
            None,
        );
        let (service, plan) = resolver
            .bind_boot_preparation(&route, &id, &cache, "mcp-host".into(), None)
            .unwrap();
        let context = ServerContext::with_store_root(&root, temp.path().join("unused"))
            .with_boot_debugger_route(route.clone())
            .unwrap();
        let call = |command| {
            let query = serde_json::to_value(input(&q::Request {
                schema: 1,
                context_id: route.context_id().into(),
                command,
            }))
            .unwrap();
            let answer = tool.run(&query, &context).unwrap();
            let reply: env::OperationEnvelope =
                serde_json::from_value(answer.structured().clone()).unwrap();
            let env::Body::Result(body) = reply.body else {
                panic!()
            };
            let result: r::Result =
                serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
            (body.effect_state, result)
        };
        let (_, planned) = call(plan.command);
        let r::Body::Preparation(p) = planned.body else {
            panic!()
        };
        assert!(p.plan_json.contains("content=sha256:"));
        let command = || {
            q::Command::Prepare(Box::new(q::CommandPrepare {
                plan_id: service.id().into(),
            }))
        };
        let (effects, refused) = call(command());
        assert_eq!(effects, env::BodyResultEffectState::NoEffects);
        let r::Body::Preparation(p) = refused.body else {
            panic!()
        };
        assert_eq!(p.status, r::BodyPreparationStatus::NeedsPreparation);
        assert!(!cache.join("sources").exists());
        // Configure real producer through its standard host constructor on a second
        // route: the transport itself cannot issue this grant.
        let host = DocumentAnalysisHost::new(
            &root,
            "vibe.toml",
            vec![
                ResourceDeclaration {
                    path: "vibe.toml".into(),
                    role: ResourceRole::Manifest,
                },
                ResourceDeclaration {
                    path: "STATIC.md".into(),
                    role: ResourceRole::Opaque,
                },
            ],
            AnalysisLimits {
                resource_bytes: 4096,
                capture_bytes: 8192,
                open_buffers: 2,
            },
        )
        .unwrap();
        let artifact = host.resource("STATIC.md").unwrap();
        let capture = host.client().unwrap().freeze().unwrap();
        let route = Arc::new(BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap());
        let id = route.retain_capture(capture).unwrap();
        let (service, _) = resolver
            .bind_boot_preparation(&route, &id, &cache, "mcp-host".into(), Some(60_000))
            .unwrap();
        let context = ServerContext::with_store_root(&root, temp.path().join("unused"))
            .with_boot_debugger_route(route.clone())
            .unwrap();
        let call = |command| {
            let query = serde_json::to_value(input(&q::Request {
                schema: 1,
                context_id: route.context_id().into(),
                command,
            }))
            .unwrap();
            let answer = tool.run(&query, &context).unwrap();
            let reply: env::OperationEnvelope =
                serde_json::from_value(answer.structured().clone()).unwrap();
            let env::Body::Result(body) = reply.body else {
                panic!()
            };
            (body.effect_state, body.payload_json.unwrap())
        };
        let (effects, first) = call(q::Command::Prepare(Box::new(q::CommandPrepare {
            plan_id: service.id().into(),
        })));
        assert_eq!(effects, env::BodyResultEffectState::Completed);
        let result: r::Result = serde_json::from_str(&first).unwrap();
        let r::Body::Preparation(p) = result.body else {
            panic!()
        };
        assert_eq!(p.status, r::BodyPreparationStatus::Complete, "{p:?}");
        assert!(cache.join("sources").exists());
        service.revoke().unwrap();
        std::fs::remove_dir_all(&registry).unwrap();
        for _ in 0..2 {
            let (effects, retained) = call(q::Command::PreparationRead(Box::new(
                q::CommandPreparationRead {
                    plan_id: service.id().into(),
                },
            )));
            assert_eq!(effects, env::BodyResultEffectState::NoEffects);
            assert_eq!(retained, first);
            let (effects, _) = call(q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: id.clone(),
            })));
            assert_eq!(effects, env::BodyResultEffectState::NoEffects);
            let (effects, _) = call(q::Command::Cost(Box::new(q::CommandCost {
                snapshot_id: id.clone(),
            })));
            assert_eq!(effects, env::BodyResultEffectState::NoEffects);
            let (effects, _) = call(q::Command::Compare(Box::new(q::CommandCompare {
                snapshot_id: id.clone(),
                other_snapshot_id: id.clone(),
            })));
            assert_eq!(effects, env::BodyResultEffectState::NoEffects);
        }
        for (name, bytes) in before {
            assert_eq!(bytes, std::fs::read(root.join(name)).unwrap());
        }
        assert!(!root.join(".vibe").exists());
        assert!(!root.join("vibevm").exists());
    }
}

#[cfg(test)]
mod m_06_e_2_cases {
    use super::*;
    use vibe_orchestrator::machine_operation::boot_debugger::{
        debugger_request as q, debugger_result as r, input,
    };
    use vibe_wire::generated::machine::e1::operation_envelope as env;
    use vibe_workspace::boot_debugger::{
        scenario::*,
        viewer::{Action, Service, Viewer},
    };
    fn answer(tool: &dyn McpTool, ctx: &ServerContext, value: Value) -> r::Result {
        let output = tool.run(&value, ctx).unwrap();
        let envelope: env::OperationEnvelope =
            serde_json::from_value(output.structured().clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!()
        };
        assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
    }
    struct Client<'a> {
        tool: &'a dyn McpTool,
        ctx: &'a ServerContext,
        context: String,
    }
    impl Service for Client<'_> {
        fn query(&self, command: q::Command) -> Result<r::Result, String> {
            Ok(answer(
                self.tool,
                self.ctx,
                serde_json::to_value(input(&q::Request {
                    schema: 1,
                    context_id: self.context.clone(),
                    command,
                }))
                .unwrap(),
            ))
        }
    }
    fn files(root: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        fn walk(at: &std::path::Path, out: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
            for e in std::fs::read_dir(at).unwrap() {
                let e = e.unwrap();
                if e.file_type().unwrap().is_dir() {
                    walk(&e.path(), out);
                } else {
                    out.push((e.path(), std::fs::read(e.path()).unwrap()));
                }
            }
        }
        let mut out = vec![];
        walk(root, &mut out);
        out.sort();
        out
    }
    #[test]
    fn registered_default_tool_produces_controls_and_replays_viewer_comparison_without_writes() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("vibevm/vibepacks/org.example/app/v1.0.0");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("vibe.toml"), "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n[[boot_snippet.fragment]]\nsource='future.md'\nwhen='os:linux'\n").unwrap();
        std::fs::write(package.join("boot.md"), "real static bytes").unwrap();
        std::fs::write(package.join("future.md"), "potential future bytes").unwrap();
        std::fs::write(
            root.path().join("vibe.toml"),
            "[project]\nname='fixture'\nversion='1.0.0'\n[boot]\ndefault_link='static'\n",
        )
        .unwrap();
        let before = files(root.path());
        let context = ServerContext::new(root.path());
        let tool = crate::tools::default_tools()
            .into_iter()
            .find(|t| t.descriptor().name == "boot_debugger")
            .unwrap();
        // Standard empty-input discovery, no manually constructed route/map.
        let initial = answer(tool.as_ref(), &context, json!({}));
        let client = Client {
            tool: tool.as_ref(),
            ctx: &context,
            context: initial.context_id,
        };
        let mut viewer = Viewer::new(client).unwrap();
        for edit in [
            Edit::Add {
                package: "org.example/app@=1.0.0".into(),
                loading: Some(vibe_core::manifest::LinkType::Static),
            },
            Edit::Toggle {
                package: "org.example/app@=1.0.0".into(),
                enabled: true,
                loading: Some(vibe_core::manifest::LinkType::Static),
            },
            Edit::Reconfigure {
                selections: Selections::default(),
            },
        ] {
            let draft = Draft {
                baseline_selections: Selections::default(),
                edits: vec![edit],
            };
            let first = viewer
                .apply(Action::Scenario {
                    target: "os:linux".into(),
                    draft_json: serde_json::to_string(&draft).unwrap(),
                })
                .unwrap()
                .unwrap();
            let r::Body::Preview(p) = &first.body else {
                panic!()
            };
            assert_eq!(p.status, r::BodyPreviewStatus::Complete, "{p:?}");
            let comparison: Value =
                serde_json::from_str(p.comparison_json.as_deref().unwrap()).unwrap();
            assert_eq!(
                comparison["candidate"]["runtime_reads"]["status"],
                "unavailable"
            );
            let selectors = q::CommandPreview {
                scenario_id: p.scenario_id.clone(),
                expected_baseline: p.baseline_world.clone(),
                expected_candidate: p.candidate_world.clone(),
                expected_catalog: p.candidate_catalog.clone(),
                loading_overrides: Default::default(),
            };
            let again = viewer
                .apply(Action::Preview(selectors.clone()))
                .unwrap()
                .unwrap();
            assert_eq!(
                serde_json::to_value(&first.body).unwrap(),
                serde_json::to_value(again.body).unwrap()
            );
            if !p.placements.is_empty() {
                let mut illegal = selectors;
                illegal
                    .loading_overrides
                    .insert("org.example/app".into(), "dynamic".into());
                let result = viewer.apply(Action::Preview(illegal)).unwrap().unwrap();
                let r::Body::Preview(p) = result.body else {
                    panic!()
                };
                assert_eq!(p.status, r::BodyPreviewStatus::Incompatible);
            }
        }
        assert_eq!(files(root.path()), before);
        let missing = viewer
            .apply(Action::Preview(q::CommandPreview {
                scenario_id: "unprepared".into(),
                expected_baseline: "none".into(),
                expected_candidate: "none".into(),
                expected_catalog: "none".into(),
                loading_overrides: Default::default(),
            }))
            .unwrap()
            .unwrap();
        let r::Body::Preview(p) = missing.body else {
            panic!()
        };
        assert_eq!(p.status, r::BodyPreviewStatus::Incomplete);
        assert_eq!(files(root.path()), before);
    }
}

#[cfg(test)]
mod m_10_d_1_cases;

#[cfg(test)]
mod m_06_f_1_cases {
    use super::*;
    use vibe_orchestrator::machine_operation::boot_debugger::input;
    use vibe_wire::generated::{
        boot_debugger::e1::{request as q, result as r},
        machine::e1::operation_envelope as env,
    };
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/next/derivatives/inspection_route_fixture.rs"
    ));

    #[test]
    fn revoked_preparation_provider_retains_captured_data_without_execution() {
        super::m_06_d_2_cases::ordinary_mcp_route_prepares_local_source_and_reads_retained_bytes();
    }

    #[test]
    fn default_registered_host_retains_full_derivative_inspection_after_provider_removal() {
        let f = InspectionFixture::new("vibe-spec");
        let ctx = ServerContext::with_store_root(
            f.source.output.root_path(),
            f.source._home.path().join("unused"),
        )
        .with_source_derivatives(f.catalog.clone());
        let server = std::cell::RefCell::new(crate::Server::new(
            crate::MemoryTransport::with_input(""),
            ctx,
        ));
        let rpc = |arguments: Value| {
            let mut server = server.borrow_mut();
            server.transport = crate::MemoryTransport::with_input(format!(
                "{}\n",
                json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
                    "params":{"name":"boot_debugger", "arguments":arguments}})
            ));
            server.run().unwrap();
            let output: Value = serde_json::from_str(&server.transport.take_output()).unwrap();
            assert_eq!(output["result"]["isError"], false, "{output}");
            let envelope: env::OperationEnvelope =
                serde_json::from_value(output["result"]["structuredContent"].clone()).unwrap();
            let env::Body::Result(body) = envelope.body else {
                panic!()
            };
            assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
            serde_json::from_str::<r::Result>(body.payload_json.as_deref().unwrap()).unwrap()
        };
        let initial = rpc(json!({}));
        let call = |command| {
            rpc(serde_json::to_value(input(&q::Request {
                schema: 1,
                context_id: initial.context_id.clone(),
                command,
            }))
            .unwrap())
        };
        let prepared = call(q::Command::Scenario(Box::new(q::CommandScenario {
            snapshot_id: initial.snapshot_id.unwrap(),
            target: "os:linux".into(),
            draft_json: "{\"baseline_selections\":{\"features\":[],\"no_defaults\":false,\"all_features\":false,\"configuration\":null},\"edits\":[]}".into(),
        })));
        let r::Body::Preview(prepared) = prepared.body else {
            panic!()
        };
        assert_eq!(prepared.status, r::BodyPreviewStatus::Complete);
        let id = prepared.produced_snapshot_id.unwrap();
        let inspect = || {
            call(q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: id.clone(),
            })))
        };
        let retained = inspect();
        let r::Body::Inspect(view) = &retained.body else {
            panic!()
        };
        assert!(view.compiler_carrier_json.is_some());
        assert!(view.sources.iter().any(|s| s.derivative_read.is_some()));
        fs::remove_dir_all(f.source.output.root_path().join("vibevm/vibepacks")).unwrap();
        fs::remove_dir_all(f.source.base.root_path()).unwrap();
        fs::remove_dir_all(f.source.local.root_path()).unwrap();
        let before = inspection_files(f.source._home.path());
        assert_eq!(inspect(), retained);
        let cost = call(q::Command::Cost(Box::new(q::CommandCost {
            snapshot_id: id.clone(),
        })));
        let r::Body::Cost(cost) = cost.body else {
            panic!()
        };
        assert!(cost.raw_bytes.exact.is_some());
        assert!(cost.tokens.unavailable.is_some());
        for resource in &view.resources {
            let mut offset = "0".to_string();
            let mut bytes = 0;
            loop {
                let result = call(q::Command::Chunk(Box::new(q::CommandChunk {
                    snapshot_id: id.clone(),
                    resource_id: resource.resource_id.clone(),
                    filter: q::CommandChunkFilter::Raw,
                    offset,
                    limit: 7,
                })));
                let r::Body::Chunk(chunk) = result.body else {
                    panic!()
                };
                bytes += chunk.hex_bytes.len() / 2;
                if let Some(next) = chunk.next_offset {
                    offset = next;
                } else {
                    assert_eq!(bytes.to_string(), chunk.total);
                    break;
                }
            }
        }
        assert_eq!(inspection_files(f.source._home.path()), before);
    }
}
