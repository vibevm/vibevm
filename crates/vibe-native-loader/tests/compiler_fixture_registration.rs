#[test]
fn compiler_fixture_keeps_abi_one_with_current_and_controlled_old_schema_rows() {
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
            "compiler-manager-source",
            "compiler-minify",
            "compiler-counter",
            "compiler-schema-one",
            "compiler-sleep",
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
            .filter(|extension| extension.id != "compiler-schema-one")
            .all(|extension| extension.ir_schema == Some(2))
    );
}

#[test]
fn controlled_schema_one_row_is_explicit_and_ffi_stays_one() {
    let manifest = vibe_native_loader_compiler_fixture::fixture_manifest();
    let old = manifest
        .extensions
        .iter()
        .find(|e| e.id == "compiler-schema-one")
        .unwrap();
    assert_eq!(old.ir_schema, Some(1));
    assert_eq!(old.point, "compile:pass");
    assert_eq!(vibe_native_loader_compiler_fixture::vibe_ext_abi(), 1);
}
