//! C3: the semantic node identity consumes selected representation bytes.

use std::fs;

use vibe_core::manifest::{SourceInterpretation, SpecFormat};
use vibe_safefs::Project;

use super::{MaterializedVibeMedia, RecordedOutput, canonical_node_of_output};

#[test]
fn equivalent_materialized_forms_have_equal_ir_identity_but_distinct_raw_digests() {
    let root = tempfile::tempdir().unwrap();
    let md = "# Document {#root}\n\n## Law {#law}\n\n@fact:ITEM Body. @status:impl/done\n";
    let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(md).unwrap());
    fs::write(root.path().join("DOC.md"), md).unwrap();
    fs::write(root.path().join("DOC.xml"), xml).unwrap();
    let project = Project::open(root.path()).unwrap();
    let md_snapshot = project
        .read_file_snapshot_bounded("DOC.md", 4096)
        .unwrap()
        .unwrap();
    let xml_snapshot = project
        .read_file_snapshot_bounded("DOC.xml", 4096)
        .unwrap()
        .unwrap();
    assert_ne!(md_snapshot.sha256, xml_snapshot.sha256);
    let markdown = RecordedOutput {
        slot_root: root.path(),
        snapshot: &md_snapshot,
        source_path: "DOC.md",
        slot_format: SpecFormat::Markdown,
        media: Some(MaterializedVibeMedia::Markdown),
        interpretation: Some(SourceInterpretation::VibeSpec),
        converter_recipe: None,
        overlay_hash: None,
        disposition: None,
    };
    let xml = RecordedOutput {
        slot_root: root.path(),
        snapshot: &xml_snapshot,
        source_path: "DOC.md",
        slot_format: SpecFormat::Xml,
        media: Some(MaterializedVibeMedia::Xml),
        interpretation: Some(SourceInterpretation::VibeSpec),
        converter_recipe: None,
        overlay_hash: None,
        disposition: None,
    };
    for anchor in ["root", "law", "ITEM"] {
        let left = canonical_node_of_output(&markdown, anchor)
            .unwrap()
            .unwrap();
        let right = canonical_node_of_output(&xml, anchor).unwrap().unwrap();
        assert_eq!(left, right);
        assert_ne!(left.hash, format!("sha256:{}", md_snapshot.sha256));
    }
}

#[test]
fn unselected_ordinary_data_cannot_become_a_canonical_spec_node() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("data.md"), "# Data {#root}\n").unwrap();
    let project = Project::open(root.path()).unwrap();
    let snapshot = project
        .read_file_snapshot_bounded("data.md", 4096)
        .unwrap()
        .unwrap();
    let ordinary = RecordedOutput {
        slot_root: root.path(),
        snapshot: &snapshot,
        source_path: "data.md",
        slot_format: SpecFormat::Markdown,
        media: None,
        interpretation: None,
        converter_recipe: None,
        overlay_hash: None,
        disposition: None,
    };
    assert!(canonical_node_of_output(&ordinary, "root").is_err());
}
