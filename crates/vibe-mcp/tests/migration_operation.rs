//! Public operation cell oracle: independent grant, retained intent and exact restore.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL");
use serde_json::json;
use vibe_mcp::tools::{ManifestMigrationOperationMcpTool, McpTool, PlanManifestMigrationMcpTool};
use vibe_mcp::{ServerContext, ToolError};

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn public_operation_cell_preserves_explicit_grant_and_recovery_meaning() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let recovery = temp.path().join("recovery");
    let cache = temp.path().join("cache");
    let before = "# exact café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.0.0'\r\nmaterialization='snapshot'\r\n";
    std::fs::write(project.join("vibe.toml"), before).unwrap();
    let ctx = ServerContext::with_store_root(&project, &cache);
    let planning = json!({"schema":1,"source_text":before,
        "source_profile":"materialization-legacy-absent-r1", "target_profile":"materialization-current-absent-r1", "recipe":"snapshot-to-copy-r1"});
    let plan = PlanManifestMigrationMcpTool.run(&planning, &ctx).unwrap();
    let plan = &plan["outcome"]["plan"];
    let authority = json!({"project_root":project,"recovery_store_root":recovery,"target_relative":"vibe.toml","scope":"owned_authored_file"});
    let apply = json!({"schema":1,"operation":"apply","operation_id":"public-cell","authority":authority,
        "approved_request_json":planning.to_string(),"before_content_id":plan["before_identity"],"after_content_id":plan["after_identity"],"diff_id":plan["diff_identity"]});
    assert!(matches!(
        ManifestMigrationOperationMcpTool.run(&apply, &ctx),
        Err(ToolError::OperationPreflight { .. })
    ));
    assert!(!recovery.exists());
    let ctx = ctx
        .with_manifest_migration_authority(&recovery, ["vibe.toml"])
        .unwrap();
    let applied = ManifestMigrationOperationMcpTool.run(&apply, &ctx).unwrap();
    assert!(!applied.is_error());
    assert_eq!(applied["outcome"], "completed");
    assert_eq!(
        std::fs::read_to_string(project.join("vibe.toml")).unwrap(),
        before.replace("'snapshot'", "'copy'")
    );
    let replay = ManifestMigrationOperationMcpTool.run(&apply, &ctx).unwrap();
    assert_eq!(replay["outcome"], "no_op");
    let restore = json!({"schema":1,"operation":"recover","operation_id":"public-cell","authority":authority,
        "action":"restore","expected_intent_id":applied["intent_id"]});
    assert!(
        !ManifestMigrationOperationMcpTool
            .run(&restore, &ctx)
            .unwrap()
            .is_error()
    );
    assert_eq!(
        std::fs::read_to_string(project.join("vibe.toml")).unwrap(),
        before
    );
    assert_eq!(
        ManifestMigrationOperationMcpTool.run(&apply, &ctx).unwrap()["outcome"],
        "no_op"
    );
    assert!(!cache.exists());
}
