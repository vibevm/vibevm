specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-B");

use super::*;

fn contract_with(extra: &str) -> Contract {
    Contract::parse(
        format!(
            r#"schema = 1
id = "rewrite-reds"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "preserve"
[[classify]]
id = "keep-src"
kind = "keep"
patterns = ["src/**"]
owner = "project"
require_match = false
{extra}
[[assert]]
id = "never"
kind = "paths-absent-v1"
patterns = ["never"]
[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "deny"
max_stdout_bytes = 1
max_stderr_bytes = 1
max_result_bytes = 1048576
termination_grace_seconds = 1
[[healthcheck]]
id = "health"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = []
protocol = "exit-code"
reads = []
writes = []
spawn = false
network = "deny"
timeout_seconds = 1
"#
        )
        .as_bytes(),
    )
    .unwrap()
}

fn inventory_for(project: &Project, files: &[(&str, &[u8])]) -> Vec<InventoryEntry> {
    files
        .iter()
        .map(|(path, bytes)| {
            let snapshot = project
                .read_file_snapshot_bounded(path, bytes.len())
                .unwrap()
                .unwrap();
            InventoryEntry {
                path: (*path).to_owned(),
                kind: EntryKind::File,
                sha256: Some(digest(bytes)),
                bytes: Some(bytes.len() as u64),
                unix_mode: snapshot.unix_mode,
                identity: Some(snapshot.identity),
            }
        })
        .collect()
}

include!("tests/syntax_adapters.rs");
include!("tests/cargo_topology.rs");
include!("tests/lock_projected.rs");
