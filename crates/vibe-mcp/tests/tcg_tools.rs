//! Behavioural oracles for the tcg adapter cells (PROP-026 §2/§7):
//! descriptors mount vibe-tcg's faces verbatim, refusals surface as the
//! right ToolError classes with their recipes, `tools/list` carries the
//! family, and the not-installed path answers with the `[requires]`
//! recipe. No node/no cargo anywhere near these — the only test that
//! spawns the real chain is `live_chain_on_ts_demo`, ignored by
//! default (run with `--ignored` on a box with the slot built).

use serde_json::json;
use vibe_mcp::tcg::{TcgComplete, TcgScope, TcgType, TcgValidate};
use vibe_mcp::tools::McpTool;
use vibe_mcp::{ServerContext, ToolError, dispatch_one};

fn empty_project() -> (tempfile::TempDir, ServerContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .unwrap();
    let ctx = ServerContext::new(dir.path().to_path_buf());
    (dir, ctx)
}

#[test]
fn descriptors_mount_the_family_faces() {
    assert_eq!(TcgValidate.descriptor().name, "tcg_validate");
    assert_eq!(TcgScope.descriptor().name, "tcg_scope");
    assert_eq!(TcgComplete.descriptor().name, "tcg_complete");
    assert_eq!(TcgType.descriptor().name, "tcg_type");
    let schema = TcgValidate.descriptor().input_schema;
    assert_eq!(schema["properties"]["language"]["enum"][0], "typescript");
    assert_eq!(
        schema["properties"]["language"]["enum"][1], "rust",
        "the rust value rides the SAME four tools (PROP-026 §2 cashed)"
    );
}

#[test]
fn unsupported_language_is_invalid_arguments_naming_the_supported_set() {
    let (_dir, ctx) = empty_project();
    let err = TcgValidate
        .run(&json!({"language": "go", "file": "src/a.go"}), &ctx)
        .expect_err("unsupported");
    match err {
        ToolError::InvalidArguments(msg) => {
            assert!(msg.contains("typescript"), "{msg}");
            assert!(msg.contains("rust"), "both shipped languages named: {msg}");
        }
        other => panic!("wrong class: {other}"),
    }
}

#[test]
fn missing_stack_is_not_found_with_the_requires_recipe() {
    let (_dir, ctx) = empty_project();
    let err = TcgValidate
        .run(&json!({"language": "typescript", "file": "src/a.ts"}), &ctx)
        .expect_err("not installed");
    match err {
        ToolError::NotFound(msg) => {
            assert!(msg.contains("vibe install"), "{msg}");
            assert!(msg.contains("typescript-ai-native"), "{msg}");
        }
        other => panic!("wrong class: {other}"),
    }
}

#[test]
fn missing_rust_stack_names_its_own_requires_line() {
    let (_dir, ctx) = empty_project();
    let err = TcgValidate
        .run(&json!({"language": "rust", "file": "src/a.rs"}), &ctx)
        .expect_err("not installed");
    match err {
        ToolError::NotFound(msg) => {
            assert!(msg.contains("rust-ai-native\" = \"^0.5"), "{msg}");
            assert!(
                !msg.contains("typescript-ai-native"),
                "never another language's fix surface: {msg}"
            );
        }
        other => panic!("wrong class: {other}"),
    }
}

#[test]
fn tools_list_carries_the_family() {
    let (_dir, ctx) = empty_project();
    let response =
        dispatch_one(ctx, r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).expect("list");
    for name in ["tcg_validate", "tcg_scope", "tcg_complete", "tcg_type"] {
        assert!(response.contains(name), "tools/list misses {name}");
    }
}

#[test]
fn end_to_end_dispatch_surfaces_the_recipe_as_a_tool_error() {
    let (_dir, ctx) = empty_project();
    let response = dispatch_one(
        ctx,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tcg_validate","arguments":{"language":"typescript","file":"src/a.ts"}}}"#,
    )
    .expect("call");
    assert!(response.contains("\"isError\":true"), "{response}");
    assert!(response.contains("vibe install"), "{response}");
}

/// The real chain: registry → slot artifact → `tcg-typescript serve` →
/// node oracle → enriched answer, against research/ts-demo. Needs the
/// slot artifact built (`vibe bin build tcg-typescript`) and node —
/// exactly what the campaign's acceptance runs on this box.
#[test]
#[ignore = "spawns the real slot binary + node; run with --ignored on a prepared box"]
fn live_chain_on_ts_demo() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let demo = repo_root.join("research").join("ts-demo");
    let ctx = ServerContext::new(demo);
    let out = TcgValidate
        .run(
            &json!({"language": "typescript", "file": "src/cells/greeting/index.ts"}),
            &ctx,
        )
        .expect("live validate");
    // enriched shape: diagnostics + conform_findings + advice
    assert_eq!(out["degraded"], false);
    let findings = out["conform_findings"].as_array().expect("findings");
    assert_eq!(findings.len(), 1, "{out}");
    assert_eq!(findings[0]["rule"], "ts-unsafe-in-domain");
    assert_eq!(findings[0]["baselined"], true, "the demo's frozen cast");
    let diags = out["diagnostics"].as_array().expect("diags");
    assert!(diags.iter().all(|d| d["category"] != "error"));

    // a second call reuses the SAME persistent relay (no respawn cost) —
    // and a hypothetical broken edit is caught without touching disk
    let original = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/ts-demo/src/cells/greeting/index.ts"),
    )
    .expect("read demo file");
    let seeded = format!("{original}\nconst bad: number = \"oops\";\n");
    let out2 = TcgValidate
        .run(
            &json!({
                "language": "typescript",
                "file": "src/cells/greeting/index.ts",
                "content": seeded,
            }),
            &ctx,
        )
        .expect("live overlay validate");
    let diags2 = out2["diagnostics"].as_array().expect("diags");
    assert!(
        diags2.iter().any(|d| d["code"] == 2322),
        "seeded TS2322 expected: {out2}"
    );
}

/// The RUST twin's real chain: registry → slot artifact → `tcg-rust
/// serve` → the consumer's rust-analyzer → enriched answer, against
/// research/rust-demo. Needs the slot artifact built (`vibe bin build
/// tcg-rust` from the demo) and the rust-analyzer component — the
/// stack prerequisite (ORACLE-RUST §1).
#[test]
#[ignore = "spawns the real slot binary + rust-analyzer; run with --ignored on a prepared box"]
fn live_chain_on_rust_demo() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let demo = repo_root.join("research").join("rust-demo");
    let ctx = ServerContext::new(demo.clone());

    // The clean file: zero diagnostics, zero findings (the demo's
    // baseline froze EMPTY — prediction §4.6).
    let out = TcgValidate
        .run(
            &json!({"language": "rust", "file": "crates/rust-demo/src/cells/greeting.rs"}),
            &ctx,
        )
        .expect("live validate");
    assert_eq!(out["degraded"], false, "{out}");
    let findings = out["conform_findings"].as_array().expect("findings");
    assert!(findings.is_empty(), "the demo is born conformant: {out}");
    let diags = out["diagnostics"].as_array().expect("diags");
    assert!(diags.iter().all(|d| d["category"] != "error"), "{out}");

    // A hypothetical broken edit through the SAME persistent relay —
    // caught without touching disk.
    let original = std::fs::read_to_string(demo.join("crates/rust-demo/src/cells/greeting.rs"))
        .expect("read demo file");
    let seeded = format!("{original}\npub const BAD: i32 = \"oops\";\n");
    let out2 = TcgValidate
        .run(
            &json!({
                "language": "rust",
                "file": "crates/rust-demo/src/cells/greeting.rs",
                "content": seeded,
            }),
            &ctx,
        )
        .expect("live overlay validate");
    let diags2 = out2["diagnostics"].as_array().expect("diags");
    assert!(
        diags2.iter().any(|d| d["code"] == "E0308"),
        "seeded E0308 expected: {out2}"
    );

    // And a discipline breach the compiler cannot see: the enrichment
    // hop reports it non-baselined, with advice.
    let unwrappy = format!("{original}\npub fn bad(v: Option<u32>) -> u32 {{ v.unwrap() }}\n");
    let out3 = TcgValidate
        .run(
            &json!({
                "language": "rust",
                "file": "crates/rust-demo/src/cells/greeting.rs",
                "content": unwrappy,
            }),
            &ctx,
        )
        .expect("live unwrap validate");
    let findings3 = out3["conform_findings"].as_array().expect("findings");
    assert!(
        findings3
            .iter()
            .any(|f| f["rule"] == "no-unwrap-in-domain" && f["baselined"] == false),
        "the gate's own rule answers at generation time: {out3}"
    );
    let disk = std::fs::read_to_string(demo.join("crates/rust-demo/src/cells/greeting.rs"))
        .expect("re-read");
    assert_eq!(disk, original, "overlays never touched the demo");
}
