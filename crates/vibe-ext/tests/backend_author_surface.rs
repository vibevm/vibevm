use std::ffi::CStr;

use vibe_ext::{BackendResponse, CompileRequest, Manifest, ManifestExtension};

fn manifest() -> Manifest {
    Manifest {
        extensions: vec![ManifestExtension {
            id: "backend".into(),
            point: "compile:pass".into(),
            ir_schema: Some(1),
        }],
    }
}

fn backend(_request: CompileRequest) -> BackendResponse {
    match BackendResponse::ok_with_message(vec![0, 255], Some("binary".into())) {
        Ok(reply) => reply,
        Err(error) => panic!("fixture backend reply: {error}"),
    }
}

vibe_ext::vibe_backend_extension!(manifest = manifest(), handler = backend);

fn request() -> Vec<u8> {
    let mut request: serde_json::Value = serde_json::from_str(include_str!(
        "../../../formats/corpora/native/e1/compile_request.valid.json"
    ))
    .unwrap();
    request["point"] = "compile:pass".into();
    request["payload"] = serde_json::from_str(include_str!(
        "../../../formats/corpora/compiler_ir/e1/valid/lane_artifact.json"
    ))
    .unwrap();
    serde_json::to_vec(&request).unwrap()
}

#[test]
fn bytes_only_backend_uses_the_existing_four_symbol_abi() {
    assert_eq!(vibe_ext_abi(), 1);
    let manifest = unsafe { CStr::from_ptr(vibe_ext_manifest()) };
    assert!(
        manifest
            .to_bytes()
            .windows(b"backend".len())
            .any(|part| part == b"backend")
    );

    let request = request();
    let mut response = std::ptr::null_mut();
    let mut response_len = usize::MAX;
    assert_eq!(
        vibe_ext_invoke(
            request.as_ptr(),
            request.len(),
            &mut response,
            &mut response_len,
        ),
        0
    );
    let reply = unsafe { std::slice::from_raw_parts(response, response_len) };
    let value: serde_json::Value = serde_json::from_slice(reply).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "status": "ok", "envelope": 1, "bytes_b64": "AP8=", "message": "binary"
        })
    );
    assert!(value.get("payload").is_none());
    assert!(value.get("provenance").is_none());
    vibe_ext_free(response, response_len);

    response = std::ptr::without_provenance_mut(1);
    response_len = usize::MAX;
    assert_eq!(
        vibe_ext_invoke(b"{".as_ptr(), 1, &mut response, &mut response_len),
        1
    );
    assert!(response.is_null());
    assert_eq!(response_len, 0);
}

#[test]
fn author_constructors_enforce_caps_without_exposing_emitted_ir() {
    assert!(BackendResponse::ok(vec![0, 1, 2]).is_ok());
    assert!(BackendResponse::fail("refused").is_ok());
    assert!(BackendResponse::fail(" \n ").is_err());
    let source = include_str!("../src/lib.rs");
    let macro_source = source
        .split("macro_rules! vibe_backend_extension")
        .nth(1)
        .unwrap()
        .split("macro_rules! vibe_mechanism_provider")
        .next()
        .unwrap();
    assert_eq!(macro_source.matches("__vibe_ext_emit_abi!").count(), 1);
    assert!(!macro_source.contains("EmittedArtifact"));
    assert!(!macro_source.contains("no_mangle"));
}
