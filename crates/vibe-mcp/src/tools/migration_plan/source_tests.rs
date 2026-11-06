//! Real MCP source-media conversation; pure planner still has no filesystem grant.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");
use super::*;
use crate::dispatch_one;
use vibe_core::manifest::{BOOT_SOURCE_INSERT, SOURCE_BOOT_CURRENT, SOURCE_BOOT_LEGACY};
fn request(path: &str) -> Value {
    json!({"schema":1,"source_text":format!("# café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.0.0'\r\nformat='normal'\r\n[boot_snippet]\r\nsource='{path}'"),
        "source_profile":SOURCE_BOOT_LEGACY,"target_profile":SOURCE_BOOT_CURRENT,"recipe":BOOT_SOURCE_INSERT})
}
fn decision(path: &str, media: &str) -> Value {
    json!({"file":path,"media":media,"interpretation":"vibe-spec","role":"spec","loading":"explicit"})
}
fn call(root: &std::path::Path, input: Value) -> Value {
    let context =
        ServerContext::with_store_root(root.join("absent-project"), root.join("absent-cache"));
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"plan_manifest_migration","arguments":input}});
    let response: Value =
        serde_json::from_str(&dispatch_one(context, &request.to_string()).unwrap()).unwrap();
    assert!(response["error"].is_null(), "{response}");
    assert_eq!(
        std::fs::read_dir(root).unwrap().count(),
        0,
        "planning must not create context paths"
    );
    assert_eq!(response["result"]["content"].as_array().unwrap().len(), 1);
    response["result"].clone()
}
#[test]
fn actual_source_media_question_answer_and_missing_choice_are_lossless() {
    let temp = tempfile::tempdir().unwrap();
    for (path, media) in [
        ("boot/source.md", "application/xml"),
        ("boot/source.xml", "text/markdown"),
    ] {
        let mut input = request(path);
        let questions = call(temp.path(), input.clone());
        assert_eq!(questions["isError"], false);
        assert!(
            questions["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("requested choices")
        );
        assert_eq!(
            questions["structuredContent"]["outcome"],
            json!({"status":"needs_input","questions":[{
            "field":"source_media","resource_path":path,"choices":["application/xml","text/markdown"]}]})
        );
        input["source_decisions"] = json!([decision(path, media)]);
        let answer = call(temp.path(), input.clone());
        assert_eq!(answer["isError"], false);
        assert_eq!(
            answer["structuredContent"],
            serde_json::to_value(plan_migration_value(input.clone())).unwrap()
        );
        let plan = &answer["structuredContent"]["outcome"]["plan"];
        assert_eq!(plan["before"], input["source_text"]);
        assert!(
            plan["after"]
                .as_str()
                .unwrap()
                .starts_with(input["source_text"].as_str().unwrap())
        );
        let manifest =
            vibe_core::manifest::Manifest::parse_str(plan["after"].as_str().unwrap()).unwrap();
        assert_eq!(
            manifest
                .require_package()
                .unwrap()
                .sources
                .as_ref()
                .unwrap()[0]
                .media(),
            media
        );
        input["source_decisions"] = json!([]);
        assert_eq!(call(temp.path(), input), questions);
    }
}
#[test]
fn malformed_answers_are_typed_preflight_and_unsupported_answers_stay_planner_refusals() {
    let temp = tempfile::tempdir().unwrap();
    let path = "boot/source.xml";
    let valid = decision(path, "application/xml");
    let mut bad_rows = vec![
        Value::Null,
        json!({}),
        json!([valid.clone(), valid.clone()]),
    ];
    for field in ["file", "media", "interpretation", "role", "loading"] {
        let mut row = valid.clone();
        row[field] = Value::Null;
        bad_rows.push(json!([row]));
        let mut row = valid.clone();
        row.as_object_mut().unwrap().remove(field);
        bad_rows.push(json!([row]));
    }
    let mut row = valid;
    row["trust"] = json!("verified");
    bad_rows.push(json!([row]));
    for bad in bad_rows {
        let mut input = request(path);
        input["source_decisions"] = bad;
        let direct = PlanManifestMigrationMcpTool
            .run(&input, &ServerContext::with_store_root("absent", "absent"));
        assert!(matches!(direct, Err(ToolError::StructuredPreflight { .. })));
        let result = call(temp.path(), input);
        assert_eq!(result["isError"], true);
        assert_eq!(result["structuredContent"]["stage"], "preflight");
        assert_eq!(
            result["structuredContent"]["error"]["code"],
            "invalid_request"
        );
    }
    let mut input = request(path);
    input["source_decisions"] = json!([decision("boot/foreign.xml", "application/xml")]);
    let result = call(temp.path(), input);
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["stage"], "planner");
    assert_eq!(result["structuredContent"]["outcome"]["status"], "refused");
}
#[test]
fn report_question_jtd_excludes_null_while_optional_serde_is_not_presence_evidence() {
    let schema: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/migration/e1/plan_report.jtd.json"
    )))
    .unwrap();
    let question = &schema["definitions"]["question"];
    assert_eq!(
        question["optionalProperties"]["resource_path"]["type"],
        "string"
    );
    assert_ne!(
        question["optionalProperties"]["resource_path"]["nullable"],
        true
    );
    let raw: vibe_wire::generated::migration::e1::plan_report::Question =
        serde_json::from_value(json!({
        "field":"source_media","choices":["application/xml"],"resource_path":null}))
        .unwrap();
    assert_eq!(
        raw.resource_path, None,
        "typed Option does not retain raw null provenance"
    );
    let temp = tempfile::tempdir().unwrap();
    let result = call(temp.path(), request("boot/source.xml"));
    assert_eq!(
        result["structuredContent"]["outcome"]["questions"][0]["resource_path"],
        "boot/source.xml"
    );
    let result = call(temp.path(), json!({"schema":1,"source_text":""}));
    for question in result["structuredContent"]["outcome"]["questions"]
        .as_array()
        .unwrap()
    {
        assert!(question.get("resource_path").is_none());
    }
}
