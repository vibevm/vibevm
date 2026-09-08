use std::path::Path;

use vibe_core::manifest::SpecFormat;

use super::support::{Fixture, compiler_manifest, node_manifest, write};

#[derive(Clone, Copy)]
enum MinifyFixture {
    Builtin,
    Native,
    NativeFailOnce,
}

impl Fixture {
    fn minify(kind: MinifyFixture) -> Self {
        let mut fixture = Self::new(false);
        fixture.spec_format = SpecFormat::Xml;
        write(
            &fixture.root.path().join("vibe.toml"),
            &node_manifest("root", Some("org.vibevm/vibe#cargo"), false)
                .replace("version='0.1.0'\n", "version='0.1.0'\nspec_format='xml'\n"),
        );
        write(
            &fixture.root.path().join("observer/build.rs"),
            "fn main(){let root=std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();assert!(!std::fs::read_to_string(root.join(\"vibevm/vibespecs/boot/STATIC.xml\")).unwrap_or_default().contains(\"vibe:transforms-pending\"));std::fs::write(root.join(\"observer-ran\"),\"ok\").unwrap();}\n",
        );
        let slot = fixture
            .root
            .path()
            .join("vibevm/vibedeps/org.demo.compiler/1.0.0");
        let handler = match kind {
            MinifyFixture::Builtin => "handler={kind='builtin',name='xml-minify'}",
            MinifyFixture::Native | MinifyFixture::NativeFailOnce => {
                "handler={kind='native',crate_dir='native'}"
            }
        };
        write(&slot.join("vibe.toml"), &compiler_manifest(handler));
        if !matches!(kind, MinifyFixture::Builtin) {
            write_minify_source(
                fixture.root.path(),
                matches!(kind, MinifyFixture::NativeFailOnce),
            );
        }
        fixture
    }
}

fn write_minify_source(root: &Path, fail_once: bool) {
    let slot = root.join("vibevm/vibedeps/org.demo.compiler/1.0.0/native");
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let vibe_ext = crates
        .join("vibe-ext")
        .display()
        .to_string()
        .replace('\\', "/");
    let fixture = crates
        .join("vibe-native-loader/tests/fixtures/native-compiler-plugin")
        .display()
        .to_string()
        .replace('\\', "/");
    write(
        &slot.join("Cargo.toml"),
        &format!(
            "[package]\nname='compiler-fixture'\nversion='1.0.0'\nedition='2024'\nbuild='build.rs'\n[lib]\ncrate-type=['cdylib']\n[dependencies]\nvibe-ext={{path={vibe_ext:?}}}\nfixture={{package='vibe-native-loader-compiler-fixture',path={fixture:?},default-features=false}}\n"
        ),
    );
    let fail = if fail_once {
        "if count==1{panic!(\"intentional first build refusal\");}"
    } else {
        ""
    };
    write(
        &slot.join("build.rs"),
        &format!(
            "fn main(){{println!(\"cargo:rerun-if-changed=build.rs\");let path=std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"build-count\");let count=std::fs::read_to_string(&path).ok().and_then(|v|v.parse::<u32>().ok()).unwrap_or(0)+1;std::fs::write(path,count.to_string()).unwrap();{fail}}}\n"
        ),
    );
    write(
        &slot.join("src/lib.rs"),
        "use vibe_ext::{CompileReply,CompileRequest,Manifest,ManifestExtension};fn manifest()->Manifest{Manifest{extensions:vec![ManifestExtension{id:\"native\".into(),point:\"compile:emitted\".into(),ir_schema:Some(1)}]}}fn handle(request:CompileRequest)->CompileReply{fixture::handle(request)}vibe_ext::vibe_compile_extension!(manifest=manifest(),handler=handle);\n",
    );
}

fn contains_pending(outputs: &super::support::NativeOutputs) -> bool {
    outputs.iter().any(|(_, files)| {
        files
            .iter()
            .any(|(_, bytes)| String::from_utf8_lossy(bytes).contains("vibe:transforms-pending"))
    })
}

fn assert_outputs_equal(
    actual: &super::support::NativeOutputs,
    expected: &super::support::NativeOutputs,
) {
    assert_eq!(actual.len(), expected.len());
    for ((actual_owner, actual_files), (expected_owner, expected_files)) in
        actual.iter().zip(expected)
    {
        assert_eq!(actual_owner, expected_owner);
        assert_eq!(actual_files.len(), expected_files.len());
        for ((actual_name, actual_bytes), (expected_name, expected_bytes)) in
            actual_files.iter().zip(expected_files)
        {
            assert_eq!(actual_name, expected_name);
            if actual_name != "STATIC.xml" {
                continue;
            }
            if actual_bytes != expected_bytes {
                let offset = actual_bytes
                    .iter()
                    .zip(expected_bytes)
                    .position(|(left, right)| left != right)
                    .unwrap_or(actual_bytes.len().min(expected_bytes.len()));
                panic!(
                    "{actual_owner}/{actual_name} differs at {offset}; native={} builtin={}",
                    String::from_utf8_lossy(actual_bytes),
                    String::from_utf8_lossy(expected_bytes)
                );
            }
        }
    }
}

#[test]
fn native_xml_minify_matches_builtin_for_all_owners_and_stays_cargo_fresh() {
    let builtin = Fixture::minify(MinifyFixture::Builtin);
    drop(builtin.native_context());
    let expected = builtin.native_outputs();
    assert!(!contains_pending(&expected));
    assert_eq!(builtin.build_count(), 0);

    let native = Fixture::minify(MinifyFixture::Native);
    let context = native.native_context();
    let pending = native.native_outputs();
    assert!(contains_pending(&pending));
    native
        .build_and_replay(context)
        .expect("first native build and strict replay");
    assert_outputs_equal(&native.native_outputs(), &expected);
    assert_eq!(native.build_count(), 1, "one real Cargo compilation");

    let converged = native.native_outputs();
    let mtimes = native.output_mtimes();
    let fresh = native.native_context();
    native
        .build_and_replay(fresh)
        .expect("second native build remains fresh");
    assert_eq!(native.build_count(), 1, "Cargo did not rerun build.rs");
    assert_eq!(native.native_outputs(), converged);
    assert_eq!(native.output_mtimes(), mtimes);
}

#[test]
fn failed_native_build_preserves_pending_then_retry_converges_exactly() {
    let builtin = Fixture::minify(MinifyFixture::Builtin);
    drop(builtin.native_context());
    let expected = builtin.native_outputs();

    let native = Fixture::minify(MinifyFixture::NativeFailOnce);
    let first = native.native_context();
    let pending = native.native_outputs();
    assert!(contains_pending(&pending));
    assert!(native.build_and_replay(first).is_err());
    assert_eq!(native.build_count(), 1);
    assert_eq!(native.native_outputs(), pending);

    let retry = native.native_context();
    native
        .build_and_replay(retry)
        .expect("retry after the same-source Cargo refusal");
    assert_eq!(native.build_count(), 2);
    assert_outputs_equal(&native.native_outputs(), &expected);
}
