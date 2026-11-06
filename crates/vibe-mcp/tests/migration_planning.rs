//! Public tool boundary: exact retained plan meaning, explicit input and no effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");
use serde_json::json;
use vibe_mcp::tools::{McpTool, PlanManifestMigrationMcpTool};
use vibe_mcp::{ServerContext, ToolError};

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 2
)]
fn public_tool_keeps_exact_plan_questions_and_preflight_distinct() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("never-opened-project");
    let store = temp.path().join("never-opened-store");
    let context = ServerContext::with_store_root(&project, &store);
    let before = "[package]\nname=\"demo\"\ngroup=\"org.example\"\nkind=\"flow\"\nversion=\"1.1.0\"\nmaterialization=\"snapshot\"\n";
    let input = json!({"schema":1,"source_text":before,
        "source_profile":"materialization-legacy-absent-r1",
        "target_profile":"materialization-current-absent-r1","recipe":"snapshot-to-copy-r1"});
    let result = PlanManifestMigrationMcpTool.run(&input, &context).unwrap();
    assert!(!result.is_error());
    let plan = &result.structured()["outcome"]["plan"];
    assert_eq!(plan["before"], before);
    assert_eq!(plan["after"], before.replace("\"snapshot\"", "\"copy\""));
    assert_eq!(plan["edits"][0]["before_start"], "86");
    assert_eq!(
        plan["diff_identity"],
        "42faf5376728e4649c80657e6b0f93e9910a107261413a5d0c7f019bf5ab27e0"
    );
    let missing = PlanManifestMigrationMcpTool
        .run(&json!({"schema":1,"source_text":before}), &context)
        .unwrap();
    assert_eq!(missing.structured()["outcome"]["status"], "needs_input");
    assert_eq!(
        missing.structured()["outcome"]["questions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let malformed = PlanManifestMigrationMcpTool.run(
        &json!({"schema":1,"source_text":before,"recipe":null}),
        &context,
    );
    let Err(ToolError::StructuredPreflight { report, .. }) = malformed else {
        panic!("null must fail before planner entry")
    };
    assert_eq!(serde_json::to_value(report).unwrap()["stage"], "preflight");
    assert!(!project.exists());
    assert!(!store.exists());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}
