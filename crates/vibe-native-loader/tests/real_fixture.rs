use std::path::{Path, PathBuf};

use serde_json::json;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_native_loader::{NativeInvocation, NativeLoader};
use vibe_wire::generated::native::e1::context::Context;

#[test]
fn real_sdk_fixture_loads_and_invokes_through_libloading() {
    assert_eq!(
        vibe_native_loader_fixture::fixture_marker(),
        "vibe-native-loader-fixture"
    );
    let library = fixture_library();
    let context = context();
    let loader = NativeLoader::new();
    let reply = loader
        .invoke(NativeInvocation {
            library: &library,
            extension_id: "fixture",
            point: "phase:build"
                .parse::<ExtensionPoint>()
                .expect("typed fixture point"),
            ir_schema: None,
            context: &context,
        })
        .expect("real fixture invocation succeeds");
    let reply = serde_json::to_value(reply).expect("reply serializes");
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["message"], "fixture handled phase:build");
}

fn fixture_library() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let executable_dir = executable.parent().expect("test executable directory");
    let profile_dir = if executable_dir
        .file_name()
        .is_some_and(|name| name == "deps")
    {
        executable_dir.parent().expect("Cargo profile directory")
    } else {
        executable_dir
    };
    let exact_name = format!(
        "{}vibe_native_loader_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let mut candidates = Vec::new();
    collect_exact(profile_dir, &exact_name, &mut candidates);
    collect_exact(&profile_dir.join("deps"), &exact_name, &mut candidates);
    candidates.sort();
    candidates.dedup();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one `{exact_name}` fixture artifact in `{}` and its deps directory; found {candidates:?}",
        profile_dir.display()
    );
    candidates.pop().expect("one fixture artifact")
}

fn collect_exact(directory: &Path, exact_name: &str, candidates: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let entry = entry.expect("read Cargo target entry");
        if entry.file_name() == exact_name && entry.file_type().expect("artifact type").is_file() {
            candidates.push(entry.path());
        }
    }
}

fn context() -> Context {
    serde_json::from_value(json!({
        "artifacts": [],
        "envelope": 1,
        "execution": {"id": "fixture", "package": "org.example/fixture", "config": {}},
        "io": {"scratch": ".vibe/lifecycle/run/fixture"},
        "point": "phase:build",
        "project": {
            "root": ".", "name": "host", "version": "1.0.0", "kind": "flow",
            "manifest": "vibe.toml", "spec_roots": ["vibevm/vibespecs"]
        },
        "run": {
            "requested": "build", "chain": ["validate", "install", "generate", "build"],
            "phase": "build", "offline": true, "assume_yes": false,
            "agent_mode": "cli", "force": false
        },
        "world": {"lockfile": "vibe.lock", "deps_root": "vibevm/vibedeps", "packages": []}
    }))
    .expect("fixture context matches generated wire")
}
