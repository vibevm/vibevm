//! Standalone fixture validator using existing pure APIs, not derivative tests.
use std::fs;
use std::path::Path;
use vibe_specdoc::commonmark::CommonMarkDoc;
use vibe_specdoc::xml_data::{DataInspection, inspect_xml_data, preserve_opaque};

fn main() {
    let root = Path::new("fixtures/next/derivatives");
    let canonical = fs::read_to_string(
        "vibevm/vibespecs/common/PROP-074-local-derivatives.xml",
    ).unwrap();
    vibe_specdoc::from_xml(&canonical).expect("canonical Vibe XML must parse");
    let mut hashes = Vec::new();
    for label in ["base", "local", "expected"] {
        let cm = fs::read_to_string(root.join(format!("commonmark/{label}/guide.md"))).unwrap();
        let doc = CommonMarkDoc::parse(&cm);
        let export = doc.resolve_export("usage.v1", &["Guide".into(), "Usage".into()]).unwrap();
        assert!(cm[export.span.clone()].contains("printf DO_NOT_EXECUTE"));
        assert!(!cm[export.span].contains("Keep this section."));
        hashes.push(export.source_sha256);
        let xml = fs::read(root.join(format!("xml-data/{label}/catalog.xml"))).unwrap();
        let data = inspect_xml_data(&xml);
        assert_eq!(data.raw(), xml);
        assert!(matches!(data.inspection, DataInspection::Xml(_)));
        let opaque = fs::read(root.join(format!("opaque-file/{label}/payload.bin"))).unwrap();
        let data = preserve_opaque(&opaque);
        assert_eq!(data.raw(), opaque);
        assert_eq!(data.inspection, DataInspection::NotRequested);
        let spec = fs::read_to_string(root.join(format!(
            "vibe-spec/{label}/vibevm/vibespecs/common/POLICY.xml"
        ))).unwrap();
        let parsed = vibe_specdoc::from_xml(&spec).unwrap();
        let md = vibe_specdoc::to_markdown(&parsed);
        assert!(md.contains("RETRY") && md.contains("printf DO_NOT_EXECUTE"));
        let expected_body = if label == "base" { "Retry at most three times." }
            else { "Retry at most once in the local profile." };
        assert!(md.contains(expected_body));
        let tree_script = fs::read(root.join(format!("nested-tree/{label}/scripts/helper.sh"))).unwrap();
        assert_eq!(preserve_opaque(&tree_script).raw(), tree_script);
    }
    assert_ne!(hashes[0], hashes[1]);
    assert_eq!(hashes[1], hashes[2]);
    println!("pure seams passed: 1 canonical XML; 3 CommonMark exports; 3 XML-data raw checks; 3 opaque files; 3 Vibe-spec parses/projections; 3 inert tree script checks; 0 derivative service tests");
}
