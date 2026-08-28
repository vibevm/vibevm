//! The shared fixture: a real project whose host coordinate also carries a
//! specmap config, so a `relations = true` call performs a GENUINE map scan
//! (config load → in-memory index build over a scannable tree) rather than
//! short-circuiting on an absent config. That is the whole point of the
//! stdio cell next door — a scan that walks a tree is exactly the moment a
//! stray `println!` below would corrupt the JSON-RPC stream.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

pub(crate) const ADDRESS: &str = "spec://org.example/demo/RULE#FIRST";
pub(crate) const HOST: &str = "org.example/demo";
/// The one authored sentence — bounded metadata may never carry it.
pub(crate) const PROSE: &str = "The one authored sentence";
pub(crate) const REQUEST_ID: i64 = 11;

pub(crate) fn project_with_map() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let specs = vibe_core::layout::current_specs_root()
        .to_string_lossy()
        .replace('\\', "/");
    write(
        root.path(),
        "vibe.toml",
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    write(
        root.path(),
        &format!("{specs}/RULE.md"),
        &format!("# Rules\n\n@fact:FIRST {PROSE}. @status:impl/done\n"),
    );
    write(
        root.path(),
        "specmap.toml",
        &format!(
            "namespace = \"{HOST}\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"{specs}\"]\n"
        ),
    );
    write(
        root.path(),
        "crates/demo/src/lib.rs",
        "//! A scannable unit, so the map build walks real bytes.\npub fn demo() {}\n",
    );
    root
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

/// One `tools/call` request line for this tool.
pub(crate) fn request(arguments: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": REQUEST_ID,
        "method": "tools/call",
        "params": {
            "name": "requirements_query",
            "arguments": arguments,
        }
    })
    .to_string()
}
