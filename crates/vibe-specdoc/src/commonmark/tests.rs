//! Independently expected source slices and ordinary grammar discriminators.
use super::*;
fn path(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).into()).collect()
}
fn sections(doc: &CommonMarkDoc) -> Vec<&str> {
    doc.nodes()
        .iter()
        .filter_map(|node| match &node.kind {
            NodeKind::Section { title, .. } => Some(title.as_str()),
            _ => None,
        })
        .collect()
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-045#ORDINARY-COMMONMARK-PARSER",
    r = 1
)]
fn exact_sections_nested_fence_and_links_are_from_original_bytes() {
    let source = "# Outer\n\n````md\n## Hidden\n```rust\n[not a link](nowhere)\n```\n````\n\n## Child\nSee [guide](docs/guide.md).\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(doc.source(), source);
    assert_eq!(sections(&doc), ["Outer", "Child"]);
    let child = doc.resolve_headings(&path(&["Outer", "Child"])).unwrap();
    assert_eq!(child.raw(), "## Child\nSee [guide](docs/guide.md).\n");
    let NodeKind::Section { heading_span, .. } = &child.node().kind else {
        panic!()
    };
    assert_eq!(&source[heading_span.clone()], "## Child\n");
    let fences: Vec<_> = doc
        .nodes()
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Fence { .. }))
        .collect();
    assert_eq!(fences.len(), 1);
    assert_eq!(
        &source[fences[0].span.clone()],
        "````md\n## Hidden\n```rust\n[not a link](nowhere)\n```\n````"
    );
    let links: Vec<_> = doc
        .nodes()
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Link { .. }))
        .collect();
    assert_eq!(links.len(), 1);
    assert_eq!(&source[links[0].span.clone()], "[guide](docs/guide.md)");
    assert!(
        matches!(&links[0].kind, NodeKind::Link{destination,label} if destination=="docs/guide.md" && label=="guide")
    );
}
#[test]
fn quote_section_closes_at_container_and_outside_text_stays_outside() {
    let source = "# Outer\n> ## Quoted\n> own body\n\noutside body\n\n## Next\nnext\n";
    let doc = CommonMarkDoc::parse(source);
    let selected = doc.resolve_headings(&path(&["Outer", "Quoted"])).unwrap();
    assert_eq!(selected.raw(), "## Quoted\n> own body\n");
    let outer = doc.resolve_headings(&path(&["Outer"])).unwrap();
    let outside = doc
        .nodes()
        .iter()
        .find(|node| matches!(&node.kind, NodeKind::Text(text) if text=="outside body"))
        .unwrap();
    assert_eq!(outside.parent, Some(outer.node().id));
    assert!(!selected.raw().contains("outside"));
}
#[test]
fn list_items_bound_sections_and_do_not_adopt_following_root_heading() {
    let source = "- ## One\n  first\n- ## Two\n  second\n# Outside\nout\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(
        doc.resolve_headings(&path(&["One"])).unwrap().raw(),
        "## One\n  first\n"
    );
    assert_eq!(
        doc.resolve_headings(&path(&["Two"])).unwrap().raw(),
        "## Two\n  second\n"
    );
    assert_eq!(
        doc.resolve_headings(&path(&["Outside"])).unwrap().raw(),
        "# Outside\nout\n"
    );
    assert!(matches!(
        doc.resolve_headings(&path(&["One", "Two"])),
        Err(HeadingSelectionError::Missing { index: 1, .. })
    ));
}
#[test]
fn nested_quote_section_does_not_swallow_outer_tail() {
    let source = "> # Q\n> > ## N\n> > deep\n>\n> tail\n\noutside\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(
        doc.resolve_headings(&path(&["Q", "N"])).unwrap().raw(),
        "## N\n> > deep\n"
    );
    assert_eq!(
        doc.resolve_headings(&path(&["Q"])).unwrap().raw(),
        "# Q\n> > ## N\n> > deep\n>\n> tail\n"
    );
}
#[test]
fn setext_utf8_crlf_offsets_and_image_code_visible_titles() {
    let source = "Équipe `code` ![alt **image**](pic.png)\r\n====================================\r\ncorps café\r\n\r\n# Next\r\nfin\r\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(sections(&doc), ["Équipe code alt image", "Next"]);
    let selected = doc
        .resolve_headings(&path(&["Équipe code alt image"]))
        .unwrap();
    assert_eq!(
        selected.raw(),
        "Équipe `code` ![alt **image**](pic.png)\r\n====================================\r\ncorps café\r\n\r\n"
    );
    let NodeKind::Section { heading_span, .. } = &selected.node().kind else {
        panic!()
    };
    assert_eq!(
        &source[heading_span.clone()],
        "Équipe `code` ![alt **image**](pic.png)\r\n====================================\r\n"
    );
    let image = doc
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, NodeKind::Image { .. }))
        .unwrap();
    assert!(
        matches!(&image.kind,NodeKind::Image{label,destination} if label=="alt image" && destination=="pic.png")
    );
    for node in doc.nodes() {
        assert!(source.get(node.span.clone()).is_some(), "{:?}", node);
    }
}
#[test]
fn duplicate_siblings_refuse_while_distinct_parents_resolve_exactly() {
    let source = "# A\n## Same\nfirst\n# B\n## Same\nsecond\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(
        doc.resolve_headings(&path(&["A", "Same"])).unwrap().raw(),
        "## Same\nfirst\n"
    );
    assert_eq!(
        doc.resolve_headings(&path(&["B", "Same"])).unwrap().raw(),
        "## Same\nsecond\n"
    );
    let duplicates = CommonMarkDoc::parse("# A\n## Same\none\n## Same\ntwo\n");
    assert!(matches!(
        duplicates.resolve_headings(&path(&["A", "Same"])),
        Err(HeadingSelectionError::Ambiguous { index: 1, .. })
    ));
    let containers = CommonMarkDoc::parse("> # Same\n> one\n\n# Same\ntwo\n");
    assert!(matches!(
        containers.resolve_headings(&path(&["Same"])),
        Err(HeadingSelectionError::Ambiguous { index: 0, .. })
    ));
    assert_eq!(duplicates.source(), "# A\n## Same\none\n## Same\ntwo\n");
}
#[test]
fn selectors_are_exact_normalized_paths_and_lookup_is_only_positional() {
    let doc = CommonMarkDoc::parse("# A   title\nbody\n");
    assert_eq!(sections(&doc), ["A title"]);
    for invalid in [
        vec![],
        path(&[""]),
        path(&[" A title"]),
        path(&["A  title"]),
        path(&["A\ttitle"]),
        path(&["A title\n"]),
    ] {
        assert_eq!(
            doc.resolve_headings(&invalid).unwrap_err(),
            HeadingSelectionError::MalformedSelector
        );
    }
    assert!(matches!(
        doc.resolve_headings(&path(&["a title"])),
        Err(HeadingSelectionError::Missing { index: 0, .. })
    ));
    assert!(doc.node(NodeId::from_index(usize::MAX)).is_none());
    assert!(doc.children(NodeId::from_index(usize::MAX)).is_none());
    let other = CommonMarkDoc::parse("# Different\n");
    let foreign_position = other
        .resolve_headings(&path(&["Different"]))
        .unwrap()
        .node()
        .id;
    assert!(
        doc.node(foreign_position).is_some(),
        "in-bounds lookup never proves a position's source identity"
    );
    let selected = doc.resolve_headings(&path(&["A title"])).unwrap();
    assert!(std::ptr::eq(selected.document(), &doc));
    assert_eq!(selected.raw(), &doc.source()[selected.span()]);
}
#[test]
fn vibe_markers_html_and_heading_attributes_are_ordinary_in_all_contexts() {
    let source = "# Notes {#id .class}\n\n@fact:REAL @requires:ONE #use pkg <status stage=\"impl\"/> are literal. `@fact:SAME`\n\n| Example |\n| --- |\n| `@fact:SAME` @requires:X #use pkg <x/> |\n\n```text\n# Hidden {#other}\n@fact:SAME\n#use other\n```\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(sections(&doc), ["Notes {#id .class}"]);
    assert_eq!(doc.source(), source);
    let codes: Vec<_> = doc
        .nodes()
        .iter()
        .filter_map(|node| match &node.kind {
            NodeKind::InlineCode(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(codes, ["@fact:SAME", "@fact:SAME"]);
    assert!(
        doc.nodes()
            .iter()
            .any(|node| matches!(&node.kind,NodeKind::Text(text) if text.contains("@fact:REAL")))
    );
    assert!(
        doc.nodes()
            .iter()
            .any(|node| matches!(&node.kind,NodeKind::Text(text) if text.contains("<status")))
    );
    assert_eq!(
        doc.resolve_headings(&path(&["Notes {#id .class}"]))
            .unwrap()
            .raw(),
        source
    );
}
#[test]
fn all_outline_edges_and_spans_are_checked_without_rendering_or_durable_ids() {
    for source in [
        "",
        "no heading café",
        "# A\n> - ## B\n>   ![x `y`](asset) and [z](javascript:alert)\n\nout\n",
        "# A\n\n    # literal\n\n# B\n",
    ] {
        let doc = CommonMarkDoc::parse(source);
        assert_eq!(doc, CommonMarkDoc::parse(source));
        for node in doc.nodes() {
            assert_eq!(doc.node(node.id), Some(node));
            assert!(source.get(node.span.clone()).is_some());
            if let Some(parent) = node.parent {
                assert!(doc.node(parent).unwrap().children.contains(&node.id));
            }
            for child in &node.children {
                assert_eq!(doc.node(*child).unwrap().parent, Some(node.id));
            }
        }
    }
}

#[test]
fn lazy_quote_continuation_remains_inside_the_standard_container() {
    let source = "> # Q\n> > ## N\n> > deep\n> tail\n\noutside\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(
        doc.resolve_headings(&path(&["Q", "N"])).unwrap().raw(),
        "## N\n> > deep\n> tail\n"
    );
}

#[test]
fn incumbent_native_scanner_keeps_real_fact_and_repeated_inline_quotes_distinct() {
    let source = "# H {#h}\n\n@fact:X real\n\nQuoted `@fact:X` and `@fact:X`.\n\n| Example |\n| --- |\n| `@fact:X` `@fact:X` |\n";
    let native = progress_core::parse::parse_document("quoted.md", source);
    assert_eq!(native.error_count(), 0, "{:?}", native.issues);
    // Native grammar counts unmarked paragraph/cell units too; only X is an authored ID.
    assert_eq!(native.fact_count, 3);
    let ids: Vec<_> = native
        .blocks
        .iter()
        .flat_map(|block| &block.facts)
        .filter_map(|fact| fact.id.as_deref())
        .collect();
    assert_eq!(ids, ["X"]);
    assert!(crate::from_markdown(source).is_ok());
    let ordinary = CommonMarkDoc::parse(source);
    assert_eq!(sections(&ordinary), ["H {#h}"]);
    assert_eq!(
        ordinary
            .nodes()
            .iter()
            .filter(|node| matches!(&node.kind, NodeKind::InlineCode(value) if value=="@fact:X"))
            .count(),
        4
    );
    assert_eq!(ordinary.source(), source);
}

#[test]
fn visible_heading_text_keeps_image_alt_code_and_entities_without_html_markup() {
    let source = "# Hello <b>World</b> ![a &amp; b](image) `x   y`\nbody\n";
    let doc = CommonMarkDoc::parse(source);
    assert_eq!(sections(&doc), ["Hello World a & b x y"]);
    assert_eq!(
        doc.resolve_headings(&path(&["Hello World a & b x y"]))
            .unwrap()
            .raw(),
        source
    );
}
