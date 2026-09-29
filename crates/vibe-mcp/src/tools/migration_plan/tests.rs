use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::manifest::{
    CURRENT_ABSENT, CURRENT_EPOCH1, IDENTITY, LEGACY_ABSENT, SNAPSHOT_TO_COPY,
};
use vibe_wire::generated::migration::e1::{plan_error::PlanError, plan_report::PlanReport};
use vibe_workspace::migration_planning::{MigrationPlanningResponse, plan_migration_value};

use super::*;
use crate::{MemoryTransport, Server, dispatch_one};

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                out.insert(path.strip_prefix(root).unwrap().to_owned(), None);
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    Some(std::fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn context(root: &Path) -> ServerContext {
    ServerContext::with_store_root(root.join("absent-project"), root.join("absent-store"))
}

fn isolated() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("keep.txt"), b"unchanged sentinel").unwrap();
    root
}

fn dispatch(root: &Path, method: &str, params: Value) -> Value {
    let before = snapshot(root);
    let request = json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params});
    let text = dispatch_one(context(root), &request.to_string()).unwrap();
    let response: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        snapshot(root),
        before,
        "actual dispatch changed fixture files"
    );
    for path in [
        "absent-project",
        "absent-store",
        ".vibe",
        "vibe.toml",
        "vibe.lock",
    ] {
        assert!(!root.join(path).exists(), "dispatch created {path}");
    }
    response
}

fn call(root: &Path, args: Value) -> Value {
    let response = dispatch(
        root,
        "tools/call",
        json!({"name":"plan_manifest_migration", "arguments":args}),
    );
    assert!(
        response["error"].is_null(),
        "tool was not dispatched: {response}"
    );
    response["result"].clone()
}

fn source(mode: &str) -> String {
    format!(
        "# café\n[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.1.0'\nmaterialization='{mode}' # keep\n"
    )
}

fn request(text: String, source_profile: &str, target_profile: &str, recipe: &str) -> Value {
    json!({"schema":1, "source_text":text, "source_profile":source_profile, "target_profile":target_profile, "recipe":recipe})
}

fn assert_text(result: &Value) {
    let rows = result["content"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["type"], "text");
    assert!(!rows[0]["text"].as_str().unwrap().is_empty());
}

#[test]
fn registered_descriptor_is_strict_without_context_files() {
    let root = isolated();
    let response = dispatch(root.path(), "tools/list", json!({}));
    let tools = response["result"]["tools"].as_array().unwrap();
    let found: Vec<_> = tools
        .iter()
        .filter(|tool| tool["name"] == "plan_manifest_migration")
        .collect();
    assert_eq!(found.len(), 1);
    let schema = &found[0]["inputSchema"];
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["required"], json!(["schema", "source_text"]));
    assert_eq!(
        schema["properties"]["schema"],
        json!({"type":"integer", "const":1})
    );
    let fields: BTreeSet<_> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        fields,
        BTreeSet::from([
            "schema",
            "source_text",
            "source_profile",
            "target_profile",
            "recipe"
        ])
    );
    for field in ["source_text", "source_profile", "target_profile", "recipe"] {
        assert_eq!(schema["properties"][field]["type"], "string");
    }
}

#[test]
fn planner_outcomes_match_pure_adapter_and_correct_channels() {
    let root = isolated();
    let inputs = [
        (
            request(
                source("snapshot"),
                LEGACY_ABSENT,
                CURRENT_ABSENT,
                SNAPSHOT_TO_COPY,
            ),
            "planned",
            false,
        ),
        (
            request(source("copy"), CURRENT_ABSENT, CURRENT_ABSENT, IDENTITY),
            "no_op",
            false,
        ),
        (
            json!({"schema":1,"source_text":source("snapshot")}),
            "needs_input",
            false,
        ),
        (
            request(
                source("snapshot"),
                "unknown-r2",
                CURRENT_ABSENT,
                SNAPSHOT_TO_COPY,
            ),
            "refused",
            true,
        ),
        (
            request(
                "[package".to_owned(),
                LEGACY_ABSENT,
                CURRENT_ABSENT,
                SNAPSHOT_TO_COPY,
            ),
            "refused",
            true,
        ),
    ];
    for (args, status, is_error) in inputs {
        let expected = plan_migration_value(args.clone());
        assert!(matches!(&expected, MigrationPlanningResponse::Report(_)));
        let result = call(root.path(), args);
        assert_eq!(
            result["structuredContent"],
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(result["isError"], is_error);
        assert_eq!(result["structuredContent"]["stage"], "planner");
        assert_eq!(result["structuredContent"]["outcome"]["status"], status);
        let _: PlanReport = serde_json::from_value(result["structuredContent"].clone()).unwrap();
        assert_text(&result);
    }
}

#[test]
fn epoch_and_utf8_crlf_source_are_carried_losslessly() {
    let root = isolated();
    let text = source("copy")
        .replace("[package]\n", "[package]\nepoch=1\n")
        .replace('\n', "\r\n");
    let args = request(text.clone(), CURRENT_EPOCH1, CURRENT_EPOCH1, IDENTITY);
    let result = call(root.path(), args.clone());
    assert_eq!(
        result["structuredContent"],
        serde_json::to_value(plan_migration_value(args)).unwrap()
    );
    let plan = &result["structuredContent"]["outcome"]["plan"];
    assert_eq!(plan["before"], text);
    assert_eq!(plan["after"], plan["before"]);
    assert_eq!(plan["source_epoch"], json!({"kind":"epoch_1"}));
    assert_eq!(plan["target_epoch"], plan["source_epoch"]);
    assert_eq!(plan["edits"], json!([]));
    assert_eq!(plan["before_identity"], plan["after_identity"]);
}

#[test]
fn malformed_null_unknown_fields_and_schema_are_structured_preflight() {
    let root = isolated();
    let base = json!({"schema":1,"source_text":source("snapshot")});
    let mut inputs = vec![
        Value::Null,
        json!([]),
        json!({}),
        json!({"schema":"1","source_text":""}),
        json!({"schema":1,"source_text":false}),
    ];
    for field in ["source_profile", "target_profile", "recipe"] {
        for value in [Value::Null, json!(1), json!(true), json!([])] {
            let mut args = base.clone();
            args[field] = value;
            inputs.push(args);
        }
    }
    let mut extra = base.clone();
    extra["project"] = json!("PRIVATE_PATH_SENTINEL");
    inputs.push(extra);
    let mut unsupported = base;
    unsupported["schema"] = json!(2);
    inputs.push(unsupported);
    for args in inputs {
        let expected = plan_migration_value(args.clone());
        assert!(matches!(&expected, MigrationPlanningResponse::Preflight(_)));
        let direct = PlanManifestMigrationMcpTool.run(&args, &context(root.path()));
        assert!(
            matches!(direct, Err(ToolError::StructuredPreflight { .. })),
            "preflight must not become an executed ToolOutput"
        );
        let result = call(root.path(), args);
        assert_eq!(
            result["structuredContent"],
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(result["isError"], true);
        assert_eq!(result["structuredContent"]["stage"], "preflight");
        let _: PlanError = serde_json::from_value(result["structuredContent"].clone()).unwrap();
        assert_text(&result);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("PROP-015#STRUCTURED-TOOL-PREFLIGHT"));
        assert!(text.contains("fix:"));
        assert!(!text.contains("PRIVATE_PATH_SENTINEL"));
    }
}

struct GenericFailureProbe {
    structured: bool,
}

impl McpTool for GenericFailureProbe {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: if self.structured {
                "other_preflight"
            } else {
                "ordinary_failure"
            }
            .to_owned(),
            description: "test probe".to_owned(),
            input_schema: json!({"type":"object"}),
        }
    }
    fn run(&self, _args: &Value, _ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        if self.structured {
            let report = serde_json::to_value(plan_migration_value(Value::Null)).unwrap();
            Err(ToolError::StructuredPreflight {
                report,
                message: "generic preflight probe".to_owned(),
            })
        } else {
            Err(ToolError::InvalidArguments("ordinary probe".to_owned()))
        }
    }
}

#[test]
fn generic_preflight_and_ordinary_errors_remain_distinct() {
    let root = isolated();
    let before = snapshot(root.path());
    let requests = ["other_preflight", "ordinary_failure"].map(|name| json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":{}}}).to_string()).join("\n");
    let transport = MemoryTransport::with_input(requests + "\n");
    let mut server = Server::new(transport, context(root.path()));
    server.register_tool(Box::new(GenericFailureProbe { structured: true }));
    server.register_tool(Box::new(GenericFailureProbe { structured: false }));
    server.run().unwrap();
    let responses: Vec<Value> = server
        .transport
        .take_output()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 2);
    for response in &responses {
        assert_eq!(response["result"]["isError"], true);
        assert_text(&response["result"]);
    }
    assert_eq!(
        responses[0]["result"]["structuredContent"],
        serde_json::to_value(plan_migration_value(Value::Null)).unwrap()
    );
    assert!(responses[1]["result"].get("structuredContent").is_none());
    assert_eq!(snapshot(root.path()), before);
}
