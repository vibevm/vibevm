#![deny(unsafe_code)]

use vibe_ext::{BackendResponse, CompileRequest, Ir, Manifest, ManifestExtension};

/// Identify the JSON backend fixture linked through the loader's dev graph.
///
/// ```
/// assert_eq!(
///     vibe_native_loader_json_backend_fixture::fixture_marker(),
///     "vibe-native-loader-json-backend-fixture"
/// );
/// ```
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
        return match BackendResponse::fail("deterministic JSON backend refusal") {
            Ok(response) => response,
            Err(error) => panic!("fixture failure is bounded: {error:?}"),
        };
    }
    let Ir::LaneArtifact(payload) = request.payload else {
        return match BackendResponse::fail("JSON backend requires lane IR") {
            Ok(response) => response,
            Err(error) => panic!("fixture failure is bounded: {error:?}"),
        };
    };
    let mut bytes = match vibe_ext::__serde_json::to_vec(&payload.lane) {
        Ok(bytes) => bytes,
        Err(error) => panic!("generated lane serializes deterministically: {error:?}"),
    };
    bytes.push(b'\n');
    match BackendResponse::ok(bytes) {
        Ok(response) => response,
        Err(error) => panic!("fixture bytes satisfy backend limits: {error:?}"),
    }
}

vibe_ext::vibe_backend_extension!(manifest = manifest(), handler = backend);
