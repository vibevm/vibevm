use std::path::PathBuf;

use vibe_wire::behaviour::native_compile::{NativeCompileError, validate_request};
use vibe_wire::generated::native::e1::compile_request::CompileRequest;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(relative: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(relative)).unwrap()).unwrap()
}

fn request(point: &str) -> CompileRequest {
    let mut value = json("formats/corpora/native/e1/compile_request.valid.json");
    value["point"] = point.into();
    serde_json::from_value(value).unwrap()
}

#[test]
fn schema_generated_type_and_corpus_preserve_the_optional_stem() {
    let schema = json("schemas/native/e1/compile_request.jtd.json");
    assert_eq!(
        schema["optionalProperties"]["frontend_physical_stem"]["type"],
        "string"
    );
    assert!(
        schema["optionalProperties"]["frontend_physical_stem"]["metadata"]["x-default"].is_null()
    );
    let parsed = request("compile:pass");
    assert_eq!(
        parsed.frontend_physical_stem.as_deref(),
        Some("GUIDE-descriptive")
    );
    let mut absent = serde_json::to_value(&parsed).unwrap();
    absent
        .as_object_mut()
        .unwrap()
        .remove("frontend_physical_stem");
    assert_eq!(
        serde_json::from_value::<CompileRequest>(absent)
            .unwrap()
            .frontend_physical_stem,
        None
    );
}

#[test]
fn stem_is_safe_bounded_and_exclusive_to_compile_pass() {
    assert!(validate_request(&request("compile:pass")).is_ok());
    for stem in ["", "bad/name", "bad\\name", "bad\nstem"] {
        let mut invalid = request("compile:pass");
        invalid.frontend_physical_stem = Some(stem.to_owned());
        assert!(matches!(
            validate_request(&invalid),
            Err(NativeCompileError::FrontendPhysicalStem { .. })
        ));
    }
    let mut oversized = request("compile:pass");
    oversized.frontend_physical_stem = Some("x".repeat(129));
    assert!(matches!(
        validate_request(&oversized),
        Err(NativeCompileError::FrontendPhysicalStem { .. })
    ));
    let staged = request("compile:source");
    assert!(matches!(
        validate_request(&staged),
        Err(NativeCompileError::FrontendPhysicalStem { .. })
    ));
}
