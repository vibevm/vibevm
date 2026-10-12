//! Actual MCP dispatcher over the same pure source owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-PURITY-EVIDENCE");
use crate::{ServerContext, dispatch_one};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use vibe_workspace::source_query::SourceQueryService;
fn state(root: &Path) -> Vec<(String, String, Vec<u8>)> {
    fn walk(root: &Path, p: &Path, out: &mut Vec<(String, String, Vec<u8>)>) {
        let m = fs::symlink_metadata(p).unwrap();
        out.push((
            p.strip_prefix(root).unwrap().display().to_string(),
            if m.is_file() {
                format!("{:x}", Sha256::digest(fs::read(p).unwrap()))
            } else {
                format!("{:?}", m.file_type())
            },
            if m.is_file() {
                fs::read(p).unwrap()
            } else {
                Vec::new()
            },
        ));
        if m.is_dir() {
            let mut paths: Vec<_> = fs::read_dir(p)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            paths.sort();
            for path in paths {
                walk(root, &path, out);
            }
        }
    }
    let mut out = vec![];
    walk(root, root, &mut out);
    out
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("vibevm/vibespecs")).unwrap();
    fs::create_dir_all(root.path().join("home/.vibe/cache")).unwrap();
    fs::write(
        root.path().join("home/.vibe/config.toml"),
        " # E2 unchanged empty configuration\n",
    )
    .unwrap();
    fs::write(root.path().join("vibe.toml"),"[package]\ngroup='org.demo'\nname='self'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    fs::write(
        root.path().join("vibevm/vibespecs/DOC.md"),
        "# Doc {#root}\n\n@fact:ITEM Selected. @status:impl/done\n",
    )
    .unwrap();
    root
}
fn request() -> Value {
    json!({"schema":1,"address":"spec://org.demo/self/DOC#ITEM","mode":"read","format":"md","children":true})
}
fn parity(root: &Path, args: &Value, name: &str) -> Value {
    let before = state(root);
    eprintln!("E2_QUERY_BEGIN mcp-{name}");
    let direct = SourceQueryService::execute_json(root, args).into_value();
    let response=dispatch_one(ServerContext::with_store_root(root,root.join("home/.vibe/cache")),&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_spec","arguments":args}}).to_string()).unwrap();
    eprintln!("E2_QUERY_END mcp-{name}");
    let response: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(response["result"]["structuredContent"], direct);
    assert_eq!(response["result"]["isError"], direct.get("code").is_some());
    let after = state(root);
    assert_eq!(after, before);
    println!(
        "E2_SNAPSHOT {name} {}",
        json!({"before":before,"after":after})
    );
    direct
}
#[test]
fn dispatcher_offline_repeat_and_expectation_keep_empty_store_unchanged() {
    let root = fixture();
    let a = parity(root.path(), &request(), "repeat-a");
    let b = parity(root.path(), &request(), "repeat-b");
    assert_eq!(a, b);
    assert_eq!(a["observation"]["strength"], "observed");
    let mut stale = request();
    stale["expected_document_hash"] = json!(format!("sha256:{}", "b".repeat(64)));
    let error = parity(root.path(), &stale, "stale");
    assert_eq!(error["code"], "stale_expectation");
    assert_eq!(error["boundary"], "document expectation");
    let mut strict = request();
    strict["proof_strength"] = json!("source-proven");
    let error = parity(root.path(), &strict, "unsupported-proof");
    assert_eq!(error["code"], "invalid_request");
    assert_eq!(error["boundary"], "request shape");
}
#[test]
fn dispatcher_missing_capture_never_fetches_or_recreates() {
    let root = fixture();
    fs::remove_file(root.path().join("vibevm/vibespecs/DOC.md")).unwrap();
    let error = parity(root.path(), &request(), "missing");
    assert_eq!(error["code"], "unknown_document");
    assert_eq!(error["boundary"], "selected address");
    assert!(!root.path().join("vibevm/vibespecs/DOC.md").exists());
}
