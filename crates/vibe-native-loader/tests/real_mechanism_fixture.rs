use std::path::{Path, PathBuf};

use serde_json::json;
use vibe_core::manifest::MechanismKey;
use vibe_native_loader::{NativeLoader, NativeMechanismInvocation};
use vibe_wire::generated::native::e1::deploy_reply::{DeployReply, RemoveResult};
use vibe_wire::generated::native::e1::deploy_request::DeployRequest;

#[test]
fn real_sdk_mechanism_fixture_loads_selects_and_invokes() {
    assert_eq!(
        vibe_native_loader_mechanism_fixture::fixture_marker(),
        "vibe-native-loader-mechanism-fixture"
    );
    let library = fixture_library();
    let request: DeployRequest = serde_json::from_value(json!({
        "operation": "remove", "envelope": 1, "protocol": 1,
        "identity": {"provider": "org.example/provider", "mechanism": "fixture-deploy",
            "target": "tool", "profile": "default"},
        "resources": ["bin/tool"]
    }))
    .expect("generated mechanism request");
    let logical_key: MechanismKey = "deploy:fixture".parse().expect("logical mechanism key");
    let reply = NativeLoader::new()
        .invoke_mechanism(NativeMechanismInvocation {
            library: &library,
            provider: "org.example/provider",
            mechanism_id: "fixture-deploy",
            logical_key: &logical_key,
            request: &request,
        })
        .expect("real fixture invocation succeeds");
    let DeployReply::Remove(reply) = reply else {
        panic!("remove request returns remove reply")
    };
    let RemoveResult::Ok(result) = reply.result else {
        panic!("fixture remove succeeds")
    };
    assert_eq!(result.removed, ["bin/tool"]);
    assert_eq!(result.evidence, "real mechanism fixture");
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
        "{}vibe_native_loader_mechanism_fixture{}",
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
        "expected one `{exact_name}`, found {candidates:?}"
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
