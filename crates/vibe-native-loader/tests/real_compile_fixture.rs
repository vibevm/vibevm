use std::path::{Path, PathBuf};

use serde_json::json;
use vibe_core::lifecycle::CompilePoint;
use vibe_native_loader::{NativeCompileInvocation, NativeLoadError, NativeLoader};
use vibe_wire::generated::native::e2::compile_reply::CompileReply;
use vibe_wire::generated::native::e2::compile_request::{CompileRequest, IrV2 as Ir};

// One process may keep the DLL cached across tests. Counter comparisons are
// serialized against every real invocation in this test binary.
static INVOCATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn real_compiler_fixture_loads_raw_statuses_and_recovers_after_panic() {
    let _serial = INVOCATIONS.lock().unwrap();
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

#[test]
fn real_compiler_frontend_derives_title_from_physical_stem() {
    let _serial = INVOCATIONS.lock().unwrap();
    let loader = NativeLoader::new();
    let library = fixture_library();
    let mut request = request("compiler-ok");
    request.frontend_physical_stem = Some("NOTE-descriptive".to_owned());
    let Ir::SourceDocument(source) = &mut request.payload else {
        unreachable!("fixture request carries source IR")
    };
    source.doc.format = "txt".to_owned();
    source.doc.text = "first\n\nsecond\n".to_owned();

    let raw = invoke_request(&loader, &library, "compiler-ok", &request).unwrap();
    let CompileReply::Ok(reply) = decode(&raw) else {
        panic!("frontend returns an ok document")
    };
    let Ir::DocumentDocument(document) = reply.payload else {
        panic!("frontend returns document IR")
    };
    assert_eq!(document.doc.tree.nodes[1].heading, "NOTE-descriptive");
    assert_eq!(
        document.doc.tree.lines,
        ["# NOTE-descriptive {#root}", "", "first", "", "second"]
    );
}

fn invoke(loader: &NativeLoader, library: &Path, id: &str) -> Result<Vec<u8>, NativeLoadError> {
    invoke_request(loader, library, id, &request(id))
}

fn invoke_request(
    loader: &NativeLoader,
    library: &Path,
    id: &str,
    request: &CompileRequest,
) -> Result<Vec<u8>, NativeLoadError> {
    let request = serde_json::to_vec(request).expect("compile request JSON");
    loader.invoke_compile(NativeCompileInvocation {
        library,
        extension_id: id,
        point: CompilePoint::Pass,
        request: &request,
    })
}

fn decode(bytes: &[u8]) -> CompileReply {
    let reply = serde_json::from_slice(bytes).expect("fixture emits strict current compiler reply");
    vibe_wire::behaviour::native_compile::validate_reply(&reply)
        .expect("envelope2 and IR2 behavior");
    reply
}

fn request(id: &str) -> CompileRequest {
    serde_json::from_value(json!({
        "envelope": 2,
        "execution": {"id": id, "package": "org.example/compiler", "config": {}},
        "io": {"scratch": ".vibe/compile/run"},
        "payload": {
            "shape": "source-document",
            "ir_schema": 2,
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

fn raw_invocations(loader: &NativeLoader, library: &Path) -> usize {
    let bytes = invoke(loader, library, "compiler-counter").expect("real counter query");
    let CompileReply::Ok(reply) = decode(&bytes) else {
        panic!("counter reply");
    };
    reply
        .message
        .as_deref()
        .unwrap()
        .strip_prefix("raw invocations: ")
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn advertised_schema_one_refuses_before_actual_ffi_invoke_and_current_two_runs() {
    let _serial = INVOCATIONS.lock().unwrap();
    let library = fixture_library();
    let loader = NativeLoader::new();
    let before = raw_invocations(&loader, &library);
    let error = invoke(&loader, &library, "compiler-schema-one")
        .expect_err("old advertised schema must refuse before invocation");
    assert!(
        matches!(&error, NativeLoadError::IrSchemaMismatch { id, actual, expected, .. }
        if id == "compiler-schema-one" && actual == "1" && expected == "2"),
        "{error}"
    );
    let after = raw_invocations(&loader, &library);
    assert_eq!(
        after,
        before + 1,
        "only this counter query may enter the actual DLL dispatcher"
    );

    let unknown = invoke(&loader, &library, "compiler-absent").unwrap_err();
    assert!(matches!(
        unknown,
        NativeLoadError::MissingExtensionId { .. }
    ));
    let bytes = serde_json::to_vec(&request("compiler-ok")).unwrap();
    let wrong_point = loader
        .invoke_compile(NativeCompileInvocation {
            library: &library,
            extension_id: "compiler-ok",
            point: CompilePoint::Source,
            request: &bytes,
        })
        .unwrap_err();
    assert!(matches!(
        wrong_point,
        NativeLoadError::ExtensionPointMismatch { .. }
    ));
    let after_admission = raw_invocations(&loader, &library);
    assert_eq!(
        after_admission,
        after + 1,
        "id/point refusals also precede actual invoke"
    );

    // A current manifest DOES invoke ABI1. Unknown request points are then
    // refused by the real generated/behavior dispatcher, before the handler.
    let mut bad = request("compiler-ok");
    bad.point = "compile:future".into();
    assert!(matches!(
        invoke_request(&loader, &library, "compiler-ok", &bad),
        Err(NativeLoadError::PluginStatus { status: 1, .. })
    ));
    let after_bad_request = raw_invocations(&loader, &library);
    assert_eq!(
        after_bad_request,
        after_admission + 2,
        "bad request entered dispatcher, plus query"
    );

    let raw = invoke(&loader, &library, "compiler-after").unwrap();
    let CompileReply::Ok(reply) = decode(&raw) else {
        panic!("current invocation after refusals");
    };
    assert_eq!(reply.envelope, 2);
    let Ir::SourceDocument(source) = reply.payload else {
        panic!("current source payload");
    };
    assert_eq!(source.ir_schema, 2);
    assert_eq!(source.doc.text, "fixture compiler input");
    assert_eq!(raw_invocations(&loader, &library), after_bad_request + 2);
}

#[test]
fn actual_native_minifier_preserves_generated_comments_inside_annotated_xml() {
    let _serial = INVOCATIONS.lock().unwrap();
    let library = fixture_library();
    let loader = NativeLoader::new();
    let paragraph = "<p>alpha\n<!-- vibe:c1 embed: spec://org.probe/p/payload#x -->\nβ\n<!-- vibe:c1 /embed: spec://org.probe/p/payload#x -->\nomega</p>";
    let literal =
        "<fence><![CDATA[\n<!-- vibe:c1 vibe:static literal — docs/example -->\n]]></fence>";
    let input = format!(
        "<!-- vibe:c1 frame-one -->\n<?xml version=\"1.0\"?><spec xmlns=\"https://vibevm.org/spec/1\">\n  {paragraph}\n  {literal}\n</spec>\n<!-- vibe:c1 frame-two -->\n<?xml version=\"1.0\"?><spec xmlns=\"https://vibevm.org/spec/1\">\n  <p>second</p>\n</spec>\n"
    );
    let expected = format!(
        "<!-- vibe:c1 frame-one -->\n<?xml version=\"1.0\"?><spec xmlns=\"https://vibevm.org/spec/1\">{paragraph}{literal}</spec>\n<!-- vibe:c1 frame-two -->\n<?xml version=\"1.0\"?><spec xmlns=\"https://vibevm.org/spec/1\"><p>second</p></spec>\n"
    );
    let before = raw_invocations(&loader, &library);
    let mut input_request = request("compiler-minify");
    input_request.point = "compile:emitted".into();
    input_request.payload = vibe_native_loader_compiler_fixture::emitted_payload_for_test(&input)
        .expect("synthetic emitted DTO has the current structural shape");
    let bytes = serde_json::to_vec(&input_request).unwrap();
    let raw = loader
        .invoke_compile(NativeCompileInvocation {
            library: &library,
            extension_id: "compiler-minify",
            point: CompilePoint::Emitted,
            request: &bytes,
        })
        .expect("real dynamically loaded annotated minifier succeeds");
    let CompileReply::Ok(reply) = decode(&raw) else {
        panic!("actual minifier returns ok");
    };
    let Ir::EmittedArtifact(actual) = reply.payload else {
        panic!("actual emitted reply");
    };
    let Ir::EmittedArtifact(expected) =
        vibe_native_loader_compiler_fixture::emitted_payload_for_test(&expected)
            .expect("expected synthetic emitted DTO has the current structural shape")
    else {
        unreachable!();
    };
    assert_eq!(
        actual.emitted.bytes_b64, expected.emitted.bytes_b64,
        "exact authored oracle, not another minifier call"
    );
    assert_eq!(
        actual.emitted.provenance.bytes_digest,
        expected.emitted.provenance.bytes_digest
    );
    assert_eq!(
        raw_invocations(&loader, &library),
        before + 2,
        "actual minifier invocation plus counter query"
    );
}
