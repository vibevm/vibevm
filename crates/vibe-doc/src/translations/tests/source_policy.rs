//! Declared source admission for direct single-page and bulk borrowing.
use super::*;

#[test]
fn source_policy_borrowed_page_has_no_raw_fallback() {
    for (suffix, empty) in [
        ("sources=[]\n", true),
        ("source_exports=[]\n", true),
        ("sources='invalid'\n", false),
        (
            "[[package.sources]]\nfile='vibevm/vibespecs/raw.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
            false,
        ),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let adaptation = tmp.path().join("adaptation");
        std::fs::create_dir_all(source.join(crate::pages::SPEC_ROOT)).unwrap();
        std::fs::create_dir_all(&adaptation).unwrap();
        std::fs::write(
            source.join(crate::pages::SPEC_ROOT).join("raw.xml"),
            "<foreign/>",
        )
        .unwrap();
        std::fs::write(source.join("vibe.toml"), format!("[package]\nname='guard'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n{suffix}")).unwrap();
        std::fs::write(
            adaptation.join("vibe.toml"),
            "[translates]\npackage='org.example/guard'\nversion='^1.0'\n",
        )
        .unwrap();
        let world = SpecSources::for_checkout(&source, Some("org.example"), "guard");
        let result = borrowed_by(&adaptation, "raw.xml", &world);
        if empty {
            assert!(result.unwrap().is_empty());
        } else {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("SOURCE-ADMISSION-BOUNDARY")
            );
        }
        let result = borrowed(&adaptation, &world);
        if empty {
            assert!(result.unwrap().is_empty());
        } else {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("SOURCE-ADMISSION-BOUNDARY")
            );
        }
    }
}
