//! The authored-fact consumer observes definitions, not quoted examples.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS");
use vibe_facts::{SourceKind, scan_authored_facts};

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS"
)]
fn quoted_anchors_do_not_duplicate_authored_addresses_or_change_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root
        .path()
        .join(vibe_core::layout::current_specs_root())
        .join("RULE.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let source = "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `@fact:SAME` and ``@fact/code:SAME``\n";
    std::fs::write(&path, source).unwrap();
    let facts = scan_authored_facts(root.path(), "org.example/quotes", SourceKind::Host).unwrap();
    let addresses: Vec<_> = facts.iter().map(|f| f.address.as_str()).collect();
    assert_eq!(
        addresses,
        [
            "spec://org.example/quotes/RULE#OTHER",
            "spec://org.example/quotes/RULE#SAME"
        ]
    );
    assert_eq!(std::fs::read(&path).unwrap(), source.as_bytes());
}

#[test]
fn authored_source_still_refuses_real_duplicate() {
    let root = tempfile::tempdir().unwrap();
    let path = root
        .path()
        .join(vibe_core::layout::current_specs_root())
        .join("RULE.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "# H {#h}\n\n@fact:SAME one\n\n@fact:SAME two\n").unwrap();
    assert!(
        scan_authored_facts(root.path(), "org.example/quotes", SourceKind::Host)
            .unwrap_err()
            .to_string()
            .contains("defined twice")
    );
}
