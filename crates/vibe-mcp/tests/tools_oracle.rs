//! Behavioural oracles for the `McpTool` cells (PROP-015 §2.2, PROP-018
//! §2.8). Each cell is constructed and driven through the seam against a
//! lockfile fixture — both directly (`tool.run(...)`) and end-to-end
//! through the registered server (`dispatch_one`). This is the
//! cell-has-oracle net the replacement protocol requires (R-040): the four
//! tool cells are referenced here by name.

use serde_json::{Value, json};
use vibe_mcp::tools::{AgenticExplain, MaterialiseSubskill, McpTool, QueryPackage, ReadSubskill};
use vibe_mcp::{ServerContext, dispatch_one};

const LOCKFILE_FIXTURE: &str = r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-05-05T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.1.0"
registry = "vibespecs"
source_url = "https://github.com/vibespecs/flow-wal.git"
source_ref = "v0.1.0"
content_hash = "sha256:deadbeef"
boot_snippet = "10-flow-wal.md"
files_written = [
    "spec/flows/wal/PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]
features = ["default", "base-discipline"]
describes = "pkg:cargo/sqlx@0.8.0"
language = "ru"

[[package.subskills_active]]
path = "stack/rust"
delivery = "lazy-push"
files_written = [
    "spec/flows/wal/PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]
cache_files = [
    "spec/flows/wal/PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]

[[package.subskills_active]]
path = "sqlx/v08"
delivery = "lazy-pull"
describes = "pkg:cargo/sqlx@^0.8"
cache_files = [
    "spec/flows/wal/SQLX-NOTES.md",
]
"#;

fn project_with_locked(text: &str) -> (tempfile::TempDir, ServerContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("vibe.lock"), text).unwrap();
    let ctx = ServerContext::new(dir.path().to_path_buf());
    (dir, ctx)
}

// --- descriptors: the seam's `describe` half ----------------------------

#[test]
fn each_cell_descriptor_names_itself() {
    assert_eq!(QueryPackage.descriptor().name, "query_package");
    assert_eq!(ReadSubskill.descriptor().name, "read_subskill");
    assert_eq!(
        MaterialiseSubskill.descriptor().name,
        "materialise_subskill"
    );
    assert_eq!(AgenticExplain.descriptor().name, "agentic_explain");
}

// --- agentic_explain (PROP-018 §2.8 dual transport) ----------------------

#[test]
fn agentic_explain_cell_returns_inline_instruction() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let out = AgenticExplain.run(&json!({}), &ctx).unwrap();
    assert_eq!(out["source"], "agentic explain");
    assert_eq!(out["delivery"], "inline");
    let instruction = out["instruction"].as_str().unwrap();
    assert!(instruction.contains("three"));
    assert!(instruction.contains("vibe.toml"));
    // The MCP transport returns the intent inline and writes no mailbox
    // file (PROP-018 §2.8) — that is the CLI one-shot path's job.
    assert!(!ctx.project_root.join(".vibe/agentic/command.md").exists());
}

// --- query_package -------------------------------------------------------

#[test]
fn query_package_cell_returns_full_entry() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let out = QueryPackage
        .run(&json!({ "name": "org.vibevm.world/wal" }), &ctx)
        .unwrap();
    assert_eq!(out["kind"], "flow");
    assert_eq!(out["name"], "wal");
    assert_eq!(out["version"], "0.1.0");
    assert_eq!(out["content_hash"], "sha256:deadbeef");
    assert_eq!(out["describes"], "pkg:cargo/sqlx@0.8.0");
    assert_eq!(out["language"], "ru");
    let subs: Vec<&str> = out["subskills_active"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["path"].as_str().unwrap())
        .collect();
    assert!(subs.contains(&"stack/rust"));
    assert!(subs.contains(&"sqlx/v08"));
}

#[test]
fn query_package_cell_unknown_is_not_found() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let err = QueryPackage
        .run(&json!({ "name": "org.vibevm/nope" }), &ctx)
        .unwrap_err();
    assert!(err.to_string().contains("not in lockfile"));
}

#[test]
fn query_package_cell_invalid_pkgref_errors() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let err = QueryPackage
        .run(&json!({ "name": "no-group" }), &ctx)
        .unwrap_err();
    // bare short name → must be group-qualified
    assert!(err.to_string().contains("invalid arguments"));
}

// --- read_subskill -------------------------------------------------------

#[test]
fn read_subskill_cell_returns_paths_and_content() {
    let (dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let p = dir.path().join("spec/flows/wal/PROTOCOL.md");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "Russian-localised PROTOCOL bytes here.").unwrap();
    let b = dir.path().join("spec/boot/10-flow-wal.md");
    std::fs::create_dir_all(b.parent().unwrap()).unwrap();
    std::fs::write(&b, "boot snippet bytes here.").unwrap();

    let out = ReadSubskill
        .run(
            &json!({ "package": "org.vibevm.world/wal", "subskill_path": "stack/rust" }),
            &ctx,
        )
        .unwrap();
    let content = out["content"].as_str().unwrap();
    assert!(content.contains("PROTOCOL bytes"));
    assert!(content.contains("boot snippet bytes"));
    let paths: Vec<&str> = out["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("PROTOCOL.md")));
}

#[test]
fn read_subskill_cell_unknown_subskill_errors() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let err = ReadSubskill
        .run(
            &json!({ "package": "org.vibevm.world/wal", "subskill_path": "made/up" }),
            &ctx,
        )
        .unwrap_err();
    assert!(err.to_string().contains("not active"));
}

// --- materialise_subskill ------------------------------------------------

#[test]
fn materialise_subskill_cell_copies_lazy_pull_content() {
    let (dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let cache_root = dir
        .path()
        .join(".vibe/cache/flow/wal/v0.1.0/subskills/sqlx/v08/spec/flows/wal");
    std::fs::create_dir_all(&cache_root).unwrap();
    std::fs::write(
        cache_root.join("SQLX-NOTES.md"),
        "sqlx 0.8.x specific notes",
    )
    .unwrap();

    let out = MaterialiseSubskill
        .run(
            &json!({ "package": "org.vibevm.world/wal", "subskill_path": "sqlx/v08" }),
            &ctx,
        )
        .unwrap();
    assert_eq!(out["status"], "materialised");
    let written: Vec<&str> = out["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert!(written.contains(&"spec/flows/wal/SQLX-NOTES.md"));
    let materialised = dir.path().join("spec/flows/wal/SQLX-NOTES.md");
    assert!(materialised.is_file());
    assert!(
        std::fs::read_to_string(&materialised)
            .unwrap()
            .contains("sqlx 0.8.x")
    );
}

#[test]
fn materialise_subskill_cell_no_op_for_non_lazy_pull() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let out = MaterialiseSubskill
        .run(
            &json!({ "package": "org.vibevm.world/wal", "subskill_path": "stack/rust" }),
            &ctx,
        )
        .unwrap();
    assert_eq!(out["status"], "no-op");
}

#[test]
fn materialise_subskill_cell_refuses_overwrite_without_force() {
    let (dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let cache_root = dir
        .path()
        .join(".vibe/cache/flow/wal/v0.1.0/subskills/sqlx/v08/spec/flows/wal");
    std::fs::create_dir_all(&cache_root).unwrap();
    std::fs::write(cache_root.join("SQLX-NOTES.md"), "from-cache").unwrap();
    let target_dir = dir.path().join("spec/flows/wal");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(target_dir.join("SQLX-NOTES.md"), "user-edit").unwrap();

    let out = MaterialiseSubskill
        .run(
            &json!({ "package": "org.vibevm.world/wal", "subskill_path": "sqlx/v08" }),
            &ctx,
        )
        .unwrap();
    assert_eq!(out["status"], "skipped");
    assert_eq!(
        std::fs::read_to_string(target_dir.join("SQLX-NOTES.md")).unwrap(),
        "user-edit",
        "user file must survive when force is unset"
    );
}

// --- end-to-end through the registered server ----------------------------

#[test]
fn dispatch_routes_tools_call_through_the_seam() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "query_package", "arguments": { "name": "org.vibevm.world/wal" } }
    })
    .to_string();
    let resp = dispatch_one(ctx, &req).unwrap();
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["result"]["isError"], false);
    assert_eq!(v["result"]["structuredContent"]["name"], "wal");
}

#[test]
fn dispatch_tools_list_includes_every_cell() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let req =
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }).to_string();
    let resp = dispatch_one(ctx, &req).unwrap();
    let v: Value = serde_json::from_str(&resp).unwrap();
    let names: Vec<&str> = v["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"query_package"));
    assert!(names.contains(&"read_subskill"));
    assert!(names.contains(&"materialise_subskill"));
    assert!(names.contains(&"agentic_explain"));
}
