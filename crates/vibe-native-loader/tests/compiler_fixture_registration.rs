#[test]
fn compiler_fixture_is_registered_as_one_homogeneous_schema_one_image() {
    assert_eq!(
        vibe_native_loader_compiler_fixture::fixture_marker(),
        "vibe-native-loader-compiler-fixture"
    );
    let manifest = vibe_native_loader_compiler_fixture::fixture_manifest();
    let ids: Vec<&str> = manifest
        .extensions
        .iter()
        .map(|extension| extension.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "compiler-ok",
            "compiler-skip",
            "compiler-fail",
            "compiler-panic",
            "compiler-after",
        ]
    );
    assert!(
        manifest
            .extensions
            .iter()
            .all(|extension| extension.point.starts_with("compile:"))
    );
    assert!(
        manifest
            .extensions
            .iter()
            .all(|extension| extension.ir_schema == Some(1))
    );
}
