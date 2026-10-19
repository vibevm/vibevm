use super::*;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

#[test]
fn five_families_reopen_exact_sources_and_refuse_post_plan_drift() {
    for family in [
        "commonmark",
        "xml-data",
        "vibe-spec",
        "opaque-file",
        "nested-tree",
    ] {
        for side in ["base", "local"] {
            let f = Fixture::new(family);
            let materialized =
                MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result")
                    .unwrap();
            let entry = f.output.subproject(&["result"]).unwrap();
            let read = || MaterializedDerivative::read(&entry, &f.base, &f.local, 1 << 20);
            let current = read().unwrap();
            assert_eq!(current.result().record(), materialized.result().record());
            let shown = current.show().unwrap();
            assert_eq!(shown.effects, Effects::NoExternalEffects);
            for file in &f.expected {
                assert_eq!(
                    fs::read(current.tree().root_path().join(&file.path)).unwrap(),
                    file.bytes
                );
            }
            let (source, retained) = if side == "base" {
                (&f.base, f.binding.base())
            } else {
                (&f.local, f.binding.local())
            };
            source
                .write_atomic(&retained.files[0].path, b"post-plan drift")
                .unwrap();
            // Output validity cannot replace independent source freshness.
            materialized.verify_tree().unwrap();
            assert!(read().is_err(), "{family} {side}");
            assert!(!f._home.path().join("DO_NOT_EXECUTE").exists());
        }
    }
}
