//! Answered capture refuses every drift between selection and rendering.
use super::*;
#[test]
fn answered_doc_capture_refuses_definition_page_and_inventory_drift() {
    for mutation in [
        "definition",
        "page",
        "same-byte-page",
        "materialized",
        "inventory",
        "identity",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let registry = temp.path().join("packages");
        let root = registry.join("org.example/manual/v1.0.0");
        std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
        std::fs::write(root.join("vibe.toml"),"[package]\ngroup='org.example'\nname='manual'\nkind='doc'\nversion='1.0.0'\ntitle='Manual'\nabstract='Synthetic manual.'\n[[documents]]\npackage='org.example/subject'\nversion='*'\n").unwrap();
        std::fs::write(
            root.join("vibevm/vibespecs/guide.xml"),
            "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Manual</title></spec>",
        )
        .unwrap();
        let sources = SpecSources::new().with_in_tree(&registry);
        let address = SpecAddress::parse("spec://org.example/manual/guide").unwrap();
        let captured = sources
            .capture_document_with_choice(&address)
            .unwrap()
            .unwrap();
        match mutation {
            "definition" => std::fs::write(root.join("vibe.toml"), "[broken").unwrap(),
            "page" => std::fs::write(root.join("vibevm/vibespecs/guide.xml"), "changed").unwrap(),
            "same-byte-page" => {
                let page = root.join("vibevm/vibespecs/guide.xml");
                let bytes = std::fs::read(&page).unwrap();
                std::fs::rename(&page, temp.path().join("retained-page")).unwrap();
                std::fs::write(&page, bytes).unwrap();
            }
            "materialized" => {
                std::fs::write(
                    root.join(vibe_core::layout::SLOT_RECORD_FILENAME),
                    "synthetic",
                )
                .unwrap();
            }
            "inventory" => std::fs::create_dir(root.parent().unwrap().join("v2.0.0")).unwrap(),
            "identity" => {
                std::fs::rename(&root, temp.path().join("retained")).unwrap();
                std::fs::create_dir(&root).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            captured.recheck().is_err(),
            "answered capture missed {mutation} drift"
        );
    }
}
