use super::*;

const FIXTURE: &str = "# Document {#root}\n\n## Solution {#solution}\n\n@fact:PARA Paragraph. @status:impl/done\n\n- @fact:LIST List item. @status:impl/done\n- Other list item.\n\n| Header | Value |\n| --- | --- |\n| @fact:CC-A-4-1 First cell. @status:impl/done | @fact:CC-A-4-2 Second cell. @status:impl/done |\n\n> @fact:QUOTE Quoted fact. @status:impl/done\n\n### Child {#child}\n\n@fact:DEEP Child fact. @status:impl/done\n\n## Sibling {#sibling}\n\n@fact:OTHER Other. @status:impl/done\n";

fn assert_literal_hash(anchor: &str) {
    use sha2::{Digest, Sha256};
    let doc = crate::from_xml(&format!("<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"{anchor}\" title=\"Section\"><p>Body.</p></section></spec>")).unwrap();
    let addressed = SpecDoc {
        sections: vec![doc.sections[0].clone()],
        ..SpecDoc::default()
    };
    let expected = format!(
        "sha256-specdoc-node/1:{:x}",
        Sha256::digest(crate::to_xml(&addressed).as_bytes())
    );
    assert_eq!(
        crate::canonical_node::canonical_node_hash(&doc, anchor).unwrap(),
        Some(expected)
    );
}

#[test]
fn actual_parser_numeric_heading_keeps_literal_recipe_one_hash() {
    assert_literal_hash("123");
}

#[test]
fn actual_parser_literal_dot_heading_keeps_literal_recipe_one_hash() {
    assert_literal_hash("literal.dot");
}

#[test]
fn literal_dot_and_tree_path_collision_refuses_general_selection() {
    let doc = crate::from_xml("<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"literal.dot\" title=\"Literal\"><p>Literal body.</p></section><section id=\"literal\" title=\"Parent\"><section id=\"dot\" title=\"Child\"><p>Nested body.</p></section></section></spec>").unwrap();
    let error = select_node(&doc, Some("literal.dot"), true, Vocabulary::Spec).unwrap_err();
    assert!(matches!(error, SelectionError::Ambiguous { candidates, .. } if candidates.len() == 2));
    let exact = select_exact_anchor(&doc, "literal.dot", true, Vocabulary::Spec).unwrap();
    assert!(crate::to_markdown(&exact.ir).contains("Literal body"));
    assert!(!crate::to_markdown(&exact.ir).contains("Nested body"));
}

#[test]
fn exact_nested_subtree_and_children_false() {
    let doc = crate::from_markdown(FIXTURE).unwrap();
    let selected = select_node(&doc, Some("root.solution"), true, Vocabulary::Spec).unwrap();
    assert_eq!(selected.heading_chain, ["Document", "Solution"]);
    assert_eq!(selected.ir.sections.len(), 1);
    assert_eq!(selected.ir.sections[0].sections.len(), 1);
    assert!(!crate::to_markdown(&selected.ir).contains("OTHER"));
    let direct = select_node(&doc, Some("solution"), false, Vocabulary::Spec).unwrap();
    assert!(direct.ir.sections[0].sections.is_empty());
    assert!(crate::to_markdown(&direct.ir).contains("CC-A-4-1"));
    assert!(!crate::to_markdown(&direct.ir).contains("DEEP"));
    let outline = outline(&doc, Vocabulary::Spec).unwrap();
    let entry = outline
        .iter()
        .find(|entry| entry.path.join(".") == "root.solution")
        .unwrap();
    assert_eq!(entry.bytes_with_children, selected.selected_bytes);
    assert_eq!(entry.bytes_without_children, direct.selected_bytes);
    assert!(entry.bytes_with_children > entry.bytes_without_children);
}

#[test]
fn paragraph_list_table_and_quote_facts_keep_exact_carrier() {
    let doc = crate::from_markdown(FIXTURE).unwrap();
    let xml = crate::to_xml(&doc);
    let reparsed = crate::from_xml(&xml).unwrap();
    for id in ["PARA", "LIST", "CC-A-4-1", "CC-A-4-2", "QUOTE"] {
        let left = select_node(&doc, Some(id), true, Vocabulary::Spec).unwrap();
        let right = select_node(
            &reparsed,
            Some(&format!("root.solution.{id}")),
            true,
            Vocabulary::Spec,
        )
        .unwrap();
        assert_eq!(left.ir, right.ir, "{id}");
        assert_eq!(left.kind, SelectedNodeKind::Fact);
        assert_eq!(left.ir.preamble.len(), 1);
        assert!(!crate::to_markdown(&left.ir).contains("OTHER"));
    }
    let cell = select_node(&doc, Some("CC-A-4-1"), true, Vocabulary::Spec).unwrap();
    let Block::Table { rows } = &cell.ir.preamble[0].block else {
        panic!("table carrier")
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 1);
    assert!(!crate::to_markdown(&cell.ir).contains("CC-A-4-2"));
}

#[test]
fn dotted_ambiguity_and_unknown_return_candidates() {
    let mut doc = crate::from_markdown(FIXTURE).unwrap();
    let error =
        select_node(&doc, Some("root.solution.MISSING"), true, Vocabulary::Spec).unwrap_err();
    let SelectionError::Unknown { candidates, .. } = error else {
        panic!("unknown refusal")
    };
    assert!(candidates.contains(&"root.solution.PARA".into()));
    doc.sections.push(doc.sections[0].clone());
    let error = select_node(&doc, Some("root.solution.PARA"), true, Vocabulary::Spec).unwrap_err();
    let SelectionError::Ambiguous { candidates, .. } = error else {
        panic!("ambiguous refusal")
    };
    assert_eq!(candidates.len(), 2);
    assert!(select_node(&doc, Some("root..solution"), true, Vocabulary::Spec).is_err());
}

#[test]
fn authored_doc_p01_is_selected_without_minting_plain_paragraph_fact() {
    let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Page</title><section id=\"solution\" title=\"Solution\"><p><p01 fact=\"true\">Authored paragraph.</p01></p><p>Unanchored prose.</p><note kind=\"tip\">Tip.</note></section></spec>";
    let doc = crate::from_xml_with(xml, Vocabulary::Doc).unwrap();
    let selected = select_node(&doc, Some("root.solution.p01"), false, Vocabulary::Doc).unwrap();
    assert!(crate::to_markdown(&selected.ir).contains("Authored paragraph"));
    assert!(!crate::to_markdown(&selected.ir).contains("Unanchored prose"));
    assert!(matches!(
        select_node(&doc, Some("p02"), true, Vocabulary::Doc),
        Err(SelectionError::Unknown { .. })
    ));
}

#[test]
fn selected_table_fact_roundtrips_as_a_body_cell_not_a_header() {
    let doc = crate::from_markdown(FIXTURE).unwrap();
    let selected = select_node(&doc, Some("CC-A-4-1"), true, Vocabulary::Spec).unwrap();
    let markdown = crate::to_markdown(&selected.ir);
    let reparsed = crate::from_markdown(&markdown).unwrap();
    let selected_again = select_node(&reparsed, Some("CC-A-4-1"), true, Vocabulary::Spec).unwrap();
    assert_eq!(selected_again.ir, selected.ir);
    assert!(!markdown.contains("CC-A-4-2"));
}

#[test]
fn inline_code_marker_never_becomes_selected_fact() {
    let doc = crate::from_markdown("# Document {#root}\n\n| Header | Value |\n| --- | --- |\n| `@fact:FAKE` example | Ordinary prose. |\n").unwrap();
    assert!(matches!(
        select_node(&doc, Some("FAKE"), true, Vocabulary::Spec),
        Err(SelectionError::Unknown { .. })
    ));
    assert!(
        outline(&doc, Vocabulary::Spec)
            .unwrap()
            .iter()
            .all(|entry| entry.kind != SelectedNodeKind::Fact)
    );
}
