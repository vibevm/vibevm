use std::path::{Path, PathBuf};

use serde_json::json;
use vibe_core::lifecycle::CompilePoint;
use vibe_native_loader::{NativeCompileInvocation, NativeLoadError, NativeLoader};
use vibe_wire::generated::native::e1::compile_reply::CompileReply;
use vibe_wire::generated::native::e1::compile_request::CompileRequest;

#[test]
fn real_compiler_fixture_loads_raw_statuses_and_recovers_after_panic() {
    assert_eq!(
        vibe_native_loader_compiler_fixture::fixture_marker(),
        "vibe-native-loader-compiler-fixture"
    );
    let library = fixture_library();
    let loader = NativeLoader::new();

    let ok = invoke(&loader, &library, "compiler-ok").expect("compiler ok");
    match decode(&ok) {
        CompileReply::Ok(reply) => {
            assert_eq!(reply.message.as_deref(), Some("handled compiler-ok"));
            assert_eq!(
                serde_json::to_value(&reply.payload).expect("reply payload JSON"),
                serde_json::to_value(request("compiler-ok").payload).expect("request payload JSON")
            );
        }
        other => panic!("expected typed ok reply, got {other:?}"),
    }

    let skip = invoke(&loader, &library, "compiler-skip").expect("compiler skip");
    assert!(matches!(decode(&skip), CompileReply::Skip(_)));
    let fail = invoke(&loader, &library, "compiler-fail").expect("compiler fail");
    assert!(matches!(decode(&fail), CompileReply::Fail(_)));

    let panic = invoke(&loader, &library, "compiler-panic").expect_err("panic is contained");
    assert!(matches!(
        panic,
        NativeLoadError::PluginStatus { status: 1, .. }
    ));

    let after = invoke(&loader, &library, "compiler-after").expect("loader survives panic");
    match decode(&after) {
        CompileReply::Ok(reply) => {
            assert_eq!(reply.message.as_deref(), Some("handled compiler-after"));
            assert_eq!(
                serde_json::to_value(&reply.payload).expect("reply payload JSON"),
                serde_json::to_value(request("compiler-after").payload)
                    .expect("request payload JSON")
            );
        }
        other => panic!("expected typed post-panic ok reply, got {other:?}"),
    }
}

fn invoke(loader: &NativeLoader, library: &Path, id: &str) -> Result<Vec<u8>, NativeLoadError> {
    let request = serde_json::to_vec(&request(id)).expect("compile request JSON");
    loader.invoke_compile(NativeCompileInvocation {
        library,
        extension_id: id,
        point: CompilePoint::Pass,
        request: &request,
    })
}

fn decode(bytes: &[u8]) -> CompileReply {
    serde_json::from_slice(bytes).expect("fixture emits strict compiler reply")
}

fn request(id: &str) -> CompileRequest {
    serde_json::from_value(json!({
        "envelope": 1,
        "execution": {"id": id, "package": "org.example/compiler", "config": {}},
        "io": {"scratch": ".vibe/compile/run"},
        "payload": {
            "shape": "source-document",
            "ir_schema": 1,
            "level": "source",
            "cardinality": "document",
            "doc": {
                "address": {
                    "kind": "static-entry",
                    "origin": "fixture",
                    "path": "fixture.md"
                },
                "format": "markdown",
                "subject": {
                    "declared_path": "fixture.md",
                    "provider": {"kind": "unclaimed"}
                },
                "text": "fixture compiler input"
            }
        },
        "point": "compile:pass",
        "project": {
            "root": ".", "name": "host", "version": "1.0.0", "kind": "flow",
            "manifest": "vibe.toml", "spec_roots": ["vibevm/vibespecs"]
        },
        "world": {"lockfile": "vibe.lock", "deps_root": "vibevm/vibedeps", "packages": []}
    }))
    .expect("fixture compile request matches generated root")
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
        "{}vibe_native_loader_compiler_fixture{}",
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
        "expected exactly one `{exact_name}` compiler fixture artifact in `{}` and its deps directory; found {candidates:?}",
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
