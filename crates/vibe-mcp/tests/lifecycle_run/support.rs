use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_mcp::tools::{LifecycleRunMcpTool, McpTool, ToolOutput};
use vibe_mcp::{ServerContext, ToolError, dispatch_one};
use vibe_orchestrator::InstallPolicy;
use vibe_wire::generated::lifecycle_report::LifecycleReport;

pub(crate) const BASE_MANIFEST: &str = r#"[package]
group = "org.demo"
name = "demo"
kind = "flow"
version = "0.1.0"
"#;

pub(crate) fn project(extra: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        format!("{BASE_MANIFEST}{extra}"),
    )
    .unwrap();
    dir
}

pub(crate) fn append(root: &Path, text: &str) {
    let path = root.join("vibe.toml");
    let mut body = fs::read_to_string(&path).unwrap();
    body.push_str(text);
    fs::write(path, body).unwrap();
}

pub(crate) fn context(root: &Path) -> ServerContext {
    let policy = InstallPolicy {
        offline: true,
        ..InstallPolicy::default()
    };
    ServerContext::new(root).with_lifecycle_execution(policy, None, false)
}

pub(crate) fn run(ctx: &ServerContext, phase: &str) -> Result<ToolOutput, ToolError> {
    LifecycleRunMcpTool.run(&json!({ "phase": phase }), ctx)
}

pub(crate) fn report(output: &ToolOutput) -> LifecycleReport {
    serde_json::from_value(output.structured().clone()).unwrap()
}

pub(crate) fn dispatch(ctx: ServerContext, arguments: Value) -> Value {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "lifecycle_run",
            "arguments": arguments,
        }
    })
    .to_string();
    serde_json::from_str(&dispatch_one(ctx, &request).unwrap()).unwrap()
}

pub(crate) fn state_bytes(root: &Path) -> Vec<u8> {
    fs::read(root.join(".vibe/lifecycle.toml")).unwrap()
}

pub(crate) fn task_bytes(root: &Path, relative: &str) -> Vec<u8> {
    fs::read(root.join(relative)).unwrap()
}

pub(crate) fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, at: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(at)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, into);
            } else {
                let relative = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                // The live lease file is the permitted infrastructure and is
                // unreadable while this process holds its byte-range lock on
                // Windows. The oracle is every state/outbox/product byte.
                if relative == ".vibe/lifecycle.lock" {
                    continue;
                }
                into.insert(relative, fs::read(path).unwrap());
            }
        }
    }
    let mut snapshot = BTreeMap::new();
    walk(root, root, &mut snapshot);
    snapshot
}

pub(crate) fn hosted_project(rows: &str) -> tempfile::TempDir {
    let project = project(rows);
    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Documentation prompt {#root}\n\nWrite the declared documentation.\n",
    )
    .unwrap();
    project
}

pub(crate) const ONE_AGENT_ROW: &str = r#"
[[extension]]
id = "produce-docs"
point = "phase:create"
handler = { kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/guide.md", kind = "file", accept = "non-empty file" },
]

[[extension]]
id = "after-agent"
point = "phase:create"
handler = { kind = "builtin", name = "log" }
config = { message = "SENTINEL-AFTER-AGENT" }
"#;

pub(crate) fn write_registry_package(
    root: &Path,
    group: &str,
    name: &str,
    version: &str,
) -> PathBuf {
    let slot = root
        .join(vibe_core::layout::current_packages_root())
        .join(group)
        .join(name)
        .join(format!("v{version}"));
    fs::create_dir_all(&slot).unwrap();
    fs::write(
        slot.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"tool\"\nversion = \"{version}\"\n"
        ),
    )
    .unwrap();
    slot
}
