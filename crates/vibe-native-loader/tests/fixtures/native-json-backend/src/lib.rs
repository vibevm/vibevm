#![deny(unsafe_code)]

use vibe_ext::{BackendResponse, CompileRequest, Ir, Manifest, ManifestExtension};

pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-json-backend-fixture"
}

fn manifest() -> Manifest {
    Manifest {
        extensions: vec![ManifestExtension {
            id: "json".to_owned(),
            point: "compile:pass".to_owned(),
            ir_schema: Some(1),
        }],
    }
}

fn backend(request: CompileRequest) -> BackendResponse {
    if request
        .execution
        .config
        .get("fail")
        .and_then(Option::as_ref)
        == Some(&vibe_ext::__serde_json::Value::Bool(true))
    {
        return BackendResponse::fail("deterministic JSON backend refusal")
            .expect("fixture failure is bounded");
    }
    let Ir::LaneArtifact(payload) = request.payload else {
        return BackendResponse::fail("JSON backend requires lane IR")
            .expect("fixture failure is bounded");
    };
    let mut bytes = vibe_ext::__serde_json::to_vec(&payload.lane)
        .expect("generated lane serializes deterministically");
    bytes.push(b'\n');
    BackendResponse::ok(bytes).expect("fixture bytes satisfy backend limits")
}

vibe_ext::vibe_backend_extension!(manifest = manifest(), handler = backend);
