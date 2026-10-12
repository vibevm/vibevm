//! Source-format-neutral identity for one addressed SpecDoc IR node.
//!
//! This is a new versioned semantic hash, separate from a source span's
//! specmap `contentHash` and from either file's raw SHA-256.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#CANONICAL-NODE-LOCATION");

use sha2::{Digest, Sha256};

use crate::doc::SpecDoc;

/// The canonical IR hash recipe. Changing the selected IR node or its
/// serialization requires a new recipe identifier.
pub const NODE_HASH_RECIPE: &str = "sha256-specdoc-node/1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeHashError {
    AmbiguousAnchor,
    SelectionLimitExceeded,
}

/// Hash the addressed title, section or fact as canonical SpecDoc IR.
/// No source location or representation bytes enter this value.
pub fn canonical_node_hash(doc: &SpecDoc, anchor: &str) -> Result<Option<String>, NodeHashError> {
    match crate::selection::select_exact_anchor(doc, anchor, true, crate::doc::Vocabulary::Doc) {
        Ok(selected) => Ok(Some(format!(
            "{NODE_HASH_RECIPE}:{:x}",
            Sha256::digest(crate::to_xml(&selected.ir).as_bytes())
        ))),
        Err(crate::selection::SelectionError::Ambiguous { .. }) => {
            Err(NodeHashError::AmbiguousAnchor)
        }
        Err(crate::selection::SelectionError::LimitExceeded(_)) => {
            Err(NodeHashError::SelectionLimitExceeded)
        }
        Err(
            crate::selection::SelectionError::Unknown { .. }
            | crate::selection::SelectionError::InvalidPath(_),
        ) => Ok(None),
    }
}

#[cfg(test)]
mod m_01_c_3_cases {
    use super::*;

    #[test]
    fn equivalent_markdown_and_xml_nodes_keep_one_semantic_hash() {
        let markdown =
            "# Document {#root}\n\n## Section {#section}\n\n@fact:ITEM Body. @status:impl/done\n";
        let authored = crate::from_markdown(markdown).unwrap();
        let xml = crate::to_xml(&authored);
        let projected = crate::from_xml(&xml).unwrap();
        for anchor in ["root", "section", "ITEM"] {
            let left = canonical_node_hash(&authored, anchor).unwrap().unwrap();
            let right = canonical_node_hash(&projected, anchor).unwrap().unwrap();
            assert_eq!(left, right);
            assert!(left.starts_with(NODE_HASH_RECIPE));
        }
        assert_eq!(canonical_node_hash(&authored, "missing"), Ok(None));
        assert_ne!(
            canonical_node_hash(&authored, "root"),
            canonical_node_hash(&authored, "section")
        );
    }

    #[test]
    fn duplicate_anchor_cannot_mint_semantic_identity() {
        let mut doc =
            crate::from_markdown("# Document {#root}\n\n## Section {#section}\n").unwrap();
        doc.sections.push(doc.sections[0].clone());
        assert_eq!(
            canonical_node_hash(&doc, "section"),
            Err(NodeHashError::AmbiguousAnchor)
        );
    }

    #[test]
    fn bound_fence_content_is_part_of_fact_identity() {
        let first = crate::from_markdown(
            "# D {#root}\n\n@fact/code:RUN Run it @status:impl/done\n\n```sh\nprintf one\n```\n",
        )
        .unwrap();
        let second = crate::from_markdown(
            "# D {#root}\n\n@fact/code:RUN Run it @status:impl/done\n\n```sh\nprintf two\n```\n",
        )
        .unwrap();
        assert_ne!(
            canonical_node_hash(&first, "RUN"),
            canonical_node_hash(&second, "RUN")
        );
    }

    #[test]
    fn fact_hash_keeps_its_carrier_and_ignores_other_list_items() {
        let first = crate::from_markdown(
            "# D {#root}\n\n- @fact:ITEM Body. @status:impl/done\n- sibling one\n",
        )
        .unwrap();
        let sibling_changed = crate::from_markdown(
            "# D {#root}\n\n- @fact:ITEM Body. @status:impl/done\n- sibling two\n",
        )
        .unwrap();
        let ordered = crate::from_markdown(
            "# D {#root}\n\n1. @fact:ITEM Body. @status:impl/done\n2. sibling one\n",
        )
        .unwrap();
        assert_eq!(
            canonical_node_hash(&first, "ITEM"),
            canonical_node_hash(&sibling_changed, "ITEM")
        );
        assert_ne!(
            canonical_node_hash(&first, "ITEM"),
            canonical_node_hash(&ordered, "ITEM")
        );
    }

    #[test]
    fn noncanonical_markdown_layout_matches_xml_ir_not_raw_bytes() {
        let markdown = "# D {#root}\n\n\n## Law {#law}\n\n* @fact:ITEM Body. @status:impl/done\n";
        let authored = crate::from_markdown(markdown).unwrap();
        let xml = crate::to_xml(&authored);
        let reparsed = crate::from_xml(&xml).unwrap();
        assert_ne!(markdown.as_bytes(), xml.as_bytes());
        for anchor in ["law", "ITEM"] {
            assert_eq!(
                canonical_node_hash(&authored, anchor),
                canonical_node_hash(&reparsed, anchor)
            );
        }
    }

    #[test]
    fn doc_vocabulary_native_range_keeps_section_around_note() {
        let xml = "<spec xmlns=\"https://vibevm.org/spec/1\">\n  <title id=\"root\">Document</title>\n  <section id=\"law\" title=\"Law\">\n    <note kind=\"tip\">A doc-only tip.</note>\n  </section>\n</spec>\n";
        assert_eq!(
            crate::native_anchor_range_with(xml, crate::doc::Vocabulary::Doc, "law").unwrap(),
            Some((3, 5))
        );
        assert_eq!(
            crate::native_anchor_range_with(xml, crate::doc::Vocabulary::Doc, "root").unwrap(),
            Some((2, 6))
        );
        assert!(crate::native_anchor_range_with(xml, crate::doc::Vocabulary::Spec, "law").is_err());
    }

    #[test]
    fn multiline_xml_opening_reports_its_first_native_line() {
        let xml = "<spec xmlns=\"https://vibevm.org/spec/1\">\n  <title id=\"root\">Document</title>\n  <section\n    id=\"law\"\n    title=\"Law\">\n    <note kind=\"tip\">Tip.</note>\n  </section>\n</spec>\n";
        assert_eq!(
            crate::native_anchor_range_with(xml, crate::doc::Vocabulary::Doc, "law").unwrap(),
            Some((3, 7))
        );
    }
}
