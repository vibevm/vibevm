use super::*;
use clap::Parser;
use std::{
    io::{BufReader, Cursor},
    sync::Arc,
};
use vibe_mcp::{ServerContext, ToolError};
use vibe_wire::generated::{
    adapter_plan::e1::{request as q, result as r},
    machine::e1::operation_envelope as env,
};
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[package]\ngroup='org.example'\nname='fixture'\nkind='tool'\nversion='1.0.0'\nauthors=['Fixture']\nlicense='MIT'\ndescription='Fixture'\nkeywords=['fixture']\n[[skill]]\nname='demo'\npath='skill'\nagents=['claude']\n").unwrap();
    std::fs::create_dir(root.path().join("skill")).unwrap();
    std::fs::write(root.path().join("skill/SKILL.md"), "body").unwrap();
    root
}
fn request(context: &str, target: &str, capture: Option<&str>) -> env::OperationEnvelope {
    adapter_plan::input(&q::Request {
        schema: 1,
        context_id: context.into(),
        target_id: target.into(),
        command: q::Command::Plan(Box::new(q::CommandPlan {
            capture_id: capture.map(str::to_owned),
        })),
    })
}
fn result(envelope: env::OperationEnvelope) -> r::Result {
    let env::Body::Result(body) = envelope.body else {
        panic!("expected result")
    };
    assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
    let value = serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
    vibe_wire::behaviour::adapter_plan::result(&value).unwrap();
    value
}
#[test]
fn clap_admits_offline_prepare_and_rejects_incompatible_targets() {
    let cli = crate::cli::Cli::try_parse_from([
        "vibe",
        "--offline",
        "adapter-plan",
        "--root",
        ".",
        "--kind",
        "skills",
        "--prepare",
    ])
    .unwrap();
    assert!(cli.offline);
    let crate::cli::Command::AdapterPlan(args) = cli.command else {
        panic!()
    };
    assert!(args.prepare);
    args.target().unwrap();
    assert!(
        crate::cli::Cli::try_parse_from([
            "vibe",
            "adapter-plan",
            "--root",
            ".",
            "--kind",
            "deploy"
        ])
        .is_err()
    );
    assert!(
        crate::cli::Cli::try_parse_from([
            "vibe",
            "adapter-plan",
            "--root",
            ".",
            "--kind",
            "deploy",
            "--profile",
            "x",
            "--scope",
            "user"
        ])
        .is_err()
    );
    let cli = crate::cli::Cli::try_parse_from([
        "vibe",
        "adapter-plan",
        "--root",
        ".",
        "--kind",
        "skills",
        "--profile",
        "x",
    ])
    .unwrap();
    let crate::cli::Command::AdapterPlan(args) = cli.command else {
        panic!()
    };
    assert!(args.target().is_err());
}
#[test]
fn serve_and_registered_mcp_share_positive_and_missing_payloads() {
    let root = fixture();
    for prepare in [false, true] {
        let host = AdapterPlanHost::new(
            root.path(),
            AdapterTarget::Skills {
                scope: AdapterScope::Project,
                user_home: None,
            },
        )
        .unwrap();
        let prepared = prepare.then(|| host.prepare(true).unwrap());
        let route = Arc::new(AdapterPlanRoute::new(host, prepared).unwrap());
        let input = request(route.context_id(), route.target_id(), route.capture_id());
        let mut bytes = serde_json::to_vec(&input).unwrap();
        bytes.push(b'\n');
        let mut output = Vec::new();
        serve(&route, Cursor::new(bytes), &mut output).unwrap();
        let lines: Vec<_> = output
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .collect();
        assert_eq!(lines.len(), 2);
        result(serde_json::from_slice(lines[0]).unwrap());
        let ctx = ServerContext::with_store_root(root.path(), root.path().join("absent-cache"))
            .with_adapter_plan_route(route.clone())
            .unwrap();
        let tool = vibe_mcp::tools::default_tools()
            .into_iter()
            .find(|tool| tool.descriptor().name == "adapter_plan")
            .unwrap();
        let mcp = match tool.run(&serde_json::to_value(input).unwrap(), &ctx) {
            Ok(output) => output.structured().clone(),
            Err(ToolError::StructuredPreflight { report, .. }) => report,
            Err(error) => panic!("{error}"),
        };
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(lines[1]).unwrap(),
            mcp
        );
        if prepare {
            let plan = result(serde_json::from_value(mcp).unwrap());
            let r::Body::Plan(body) = plan.body else {
                panic!()
            };
            assert!(!body.rows.is_empty());
        }
        assert!(!root.path().join("absent-cache").exists());
    }
}
#[test]
fn spawned_prepared_offline_cli_emits_only_protocol_and_retains_plan() {
    let binary = std::env::var_os("VIBE_M08_B1_TEST_BINARY")
        .expect("inject exact scoped cargo build output");
    let root = fixture();
    let mut child = std::process::Command::new(binary)
        .args(["--offline", "adapter-plan", "--root"])
        .arg(root.path())
        .args(["--kind", "skills", "--prepare"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let session = result(serde_json::from_str(&line).unwrap());
    let payload = request(
        &session.context_id,
        &session.target_id,
        session.capture_id.as_deref(),
    );
    let before = std::fs::read(root.path().join("skill/SKILL.md")).unwrap();
    let mut replies = Vec::new();
    for _ in 0..2 {
        serde_json::to_writer(&mut input, &payload).unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
        line.clear();
        output.read_line(&mut line).unwrap();
        replies.push(result(serde_json::from_str(&line).unwrap()));
    }
    drop(input);
    assert!(child.wait().unwrap().success());
    assert_eq!(replies[0], replies[1]);
    let r::Body::Plan(plan) = &replies[0].body else {
        panic!()
    };
    assert!(!plan.rows.is_empty());
    assert_eq!(
        std::fs::read(root.path().join("skill/SKILL.md")).unwrap(),
        before
    );
    assert!(!root.path().join(".vibe").exists());
    line.clear();
    assert_eq!(output.read_line(&mut line).unwrap(), 0);
}

use vibe_orchestrator::machine_operation::adapter_plan as adapter_owner;
#[cfg(unix)]
#[path = "../../../../vibe-orchestrator/src/machine_operation/adapter_plan/test_fixture.rs"]
mod shared_fixture;
#[cfg(unix)]
#[test]
fn configured_deploy_cli_mcp_share_positive_stale_and_unavailable_currentness() {
    for qualified in [true, false] {
        let (root, owned, route, trace) = shared_fixture::deploy_route(qualified);
        let ctx = ServerContext::with_store_root(root.path(), owned.path().join("cache"))
            .with_adapter_plan_route(route.clone())
            .unwrap();
        let tool = vibe_mcp::tools::default_tools()
            .into_iter()
            .find(|t| t.descriptor().name == "adapter_plan")
            .unwrap();
        for drift in ["none", "state", "executable"] {
            if drift == "state" {
                std::fs::write(owned.path().join("home/list.json"), "[]").unwrap();
            }
            if drift == "executable" {
                std::fs::remove_file(owned.path().join("client")).unwrap();
                std::fs::create_dir(owned.path().join("client")).unwrap();
            }
            let before = std::fs::read(&trace).unwrap();
            let input = request(route.context_id(), route.target_id(), route.capture_id());
            let mut bytes = serde_json::to_vec(&input).unwrap();
            bytes.push(b'\n');
            let mut output = Vec::new();
            serve(&route, Cursor::new(bytes), &mut output).unwrap();
            let lines: Vec<_> = output
                .split(|b| *b == b'\n')
                .filter(|l| !l.is_empty())
                .collect();
            assert_eq!(lines.len(), 2);
            let cli: serde_json::Value = serde_json::from_slice(lines[1]).unwrap();
            let mcp = match tool.run(&serde_json::to_value(input).unwrap(), &ctx) {
                Ok(out) => out.structured().clone(),
                Err(ToolError::StructuredPreflight { report, .. }) => report,
                Err(e) => panic!("{e}"),
            };
            assert_eq!(cli, mcp);
            if qualified && drift == "none" {
                let answer = result(serde_json::from_value(cli).unwrap());
                assert!(matches!(answer.body, r::Body::Plan(_)));
            } else {
                let envelope: env::OperationEnvelope = serde_json::from_value(cli).unwrap();
                let env::Body::Error(body) = envelope.body else {
                    panic!("expected error")
                };
                assert_eq!(body.effect_state, env::BodyErrorEffectState::NoEffects);
                let error: vibe_wire::generated::adapter_plan::e1::error::Error =
                    serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
                assert_eq!(
                    serde_json::to_value(error.reason.unwrap()).unwrap(),
                    serde_json::json!(if drift == "executable" {
                        "client-executable-unavailable"
                    } else if qualified {
                        "client-capture-stale"
                    } else {
                        "client-state-unavailable"
                    })
                );
            }
            assert_eq!(std::fs::read(&trace).unwrap(), before);
            assert!(!owned.path().join("cache").exists());
            assert!(!owned.path().join("settings").exists());
        }
    }
}

mod m_08_c_1_cases {
    use super::*;
    use sha2::{Digest, Sha256};
    mod fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/next/adapters/m-08-c1/fixture.rs"
        ));
    }
    fn query(binary: &std::path::Path, root: &std::path::Path) -> env::OperationEnvelope {
        let before = fixture::inventory(root);
        let out = std::process::Command::new(binary)
            .env("HOME", root.join("home"))
            .env("VIBE_SETTINGS", root.join("settings"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("XDG_CACHE_HOME", root.join("cache"))
            .args(["--offline", "adapter-plan", "--root"])
            .arg(root)
            .args(["--kind", "clients", "--prepare"])
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(before, fixture::inventory(root));
        serde_json::from_slice(&out.stdout).unwrap()
    }
    #[test]
    fn same_binary_adds_client_then_rejects_duplicate_and_unknown_operation() {
        let binary =
            PathBuf::from(std::env::var_os("VIBE_M08_C1_TEST_BINARY").expect("V7 exact binary"));
        let digest =
            |p: &std::path::Path| format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()));
        let hash = digest(&binary);
        let root = tempfile::tempdir().unwrap();
        fixture::host(root.path());
        let before = query(&binary, root.path());
        let env::Body::Result(body) = before.body else {
            panic!("before refusal");
        };
        let result: vibe_wire::generated::client_capability::e1::result::Result =
            serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        assert!(result.capabilities.is_empty());
        // An unselected directory cannot introduce an identity.
        std::fs::create_dir(root.path().join("unselected")).unwrap();
        std::fs::write(
            root.path().join("unselected/vibe.toml"),
            fixture::DESCRIPTOR,
        )
        .unwrap();
        let unselected = query(&binary, root.path());
        assert!(matches!(unselected.body, env::Body::Result(_)));
        let slot = fixture::add(root.path());
        let after = query(&binary, root.path());
        let env::Body::Result(body) = after.body else {
            panic!("after refusal: {after:?}");
        };
        let result: serde_json::Value =
            serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        assert_eq!(
            result["capabilities"]["org.example/new-client"]["bindings"][0]["destination"],
            ".new-client/settings.json"
        );
        assert_eq!(
            result["capabilities"]["org.example/new-client"]["bindings"][0]["key"][1],
            "new.key"
        );
        let declaration = fixture::DESCRIPTOR.split("[[mechanism]]").nth(1).unwrap();
        std::fs::write(
            slot.join("vibe.toml"),
            format!(
                "{}\n[[mechanism]]{}",
                fixture::DESCRIPTOR,
                declaration.replace("id = 'client'", "id = 'duplicate'")
            ),
        )
        .unwrap();
        assert!(matches!(
            query(&binary, root.path()).body,
            env::Body::Error(_)
        ));
        std::fs::write(
            slot.join("vibe.toml"),
            fixture::DESCRIPTOR.replace("reconcile-json-member", "launch-arbitrary"),
        )
        .unwrap();
        assert!(matches!(
            query(&binary, root.path()).body,
            env::Body::Error(_)
        ));
        assert_eq!(hash, digest(&binary));
        eprintln!(
            "M08C1 same_binary_sha256={hash} phases=before,unselected,selected,duplicate,unknown-operation mutation_census=unchanged queries=5"
        );
    }
}
