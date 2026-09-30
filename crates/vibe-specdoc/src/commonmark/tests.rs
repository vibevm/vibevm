use super::{CommonMarkDoc, ExportResolutionError, HeadingLookupError, NodeKind};
use sha2::{Digest, Sha256};

#[test]
fn structural_sections_fences_and_links_keep_exact_source_spans() {
    let source = "# Outer\n\n````md\n## Hidden\n```rust\n[not a link](nowhere)\n```\n````\n\n## Child\nSee [guide](docs/guide.md).\n";
    let doc = CommonMarkDoc::parse(source);
    let outer = doc.heading("Outer").unwrap();
    let child = doc.heading("Child").unwrap();

    assert!(
        matches!(&doc.node(outer).kind, NodeKind::Section { heading_span, .. }
        if source[heading_span.clone()].trim_end() == "# Outer"),
        "{:?}",
        doc.node(outer)
    );
    assert!(
        matches!(&doc.node(child).kind, NodeKind::Section { heading_span, .. }
        if source[heading_span.clone()].trim_end() == "## Child"),
        "{:?}",
        doc.node(child)
    );

    assert_eq!(doc.node(child).parent, Some(outer));
    assert!(matches!(
        doc.heading("Hidden"),
        Err(HeadingLookupError::Missing(_))
    ));
    assert_eq!(
        doc.nodes()
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Section { .. }))
            .count(),
        2
    );

    let fence = doc
        .nodes()
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Fence { .. }))
        .unwrap();
    assert_eq!(
        &source[fence.span.clone()],
        "````md\n## Hidden\n```rust\n[not a link](nowhere)\n```\n````"
    );
    assert_eq!(fence.parent, Some(outer));
    let links = doc
        .nodes()
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Link { .. }))
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 1);
    assert_eq!(&source[links[0].span.clone()], "[guide](docs/guide.md)");
    assert_eq!(links[0].parent, Some(child));
    assert!(
        matches!(&links[0].kind, NodeKind::Link { destination, label } if destination == "docs/guide.md" && label == "guide")
    );

    let again = CommonMarkDoc::parse(source);
    assert_eq!(doc, again);
    for node in doc.nodes() {
        assert!(
            source.get(node.span.clone()).is_some(),
            "invalid UTF-8 byte span for {:?}",
            node.id
        );
    }
}

#[test]
fn duplicate_headings_are_ambiguous_and_setext_is_a_section() {
    let source = "Équipe\n======\n\n# Repeat\n\n## Repeat\n";
    let doc = CommonMarkDoc::parse(source);
    let setext = doc.heading("Équipe").unwrap();
    assert_eq!(
        &source[doc.node(setext).span.start..source.find("# Repeat").unwrap()],
        "Équipe\n======\n\n"
    );
    assert!(
        matches!(doc.heading("Repeat"), Err(HeadingLookupError::Ambiguous { matches, .. }) if matches.len() == 2)
    );
    assert!(
        matches!(doc.heading("Absent"), Err(HeadingLookupError::Missing(name)) if name == "Absent")
    );
}

#[test]
fn vibe_shaped_words_remain_ordinary_text_even_in_tables_and_code() {
    let source = "# Notes\n\n@requires:ONE and #use pkg are prose. `@fact:INLINE`\n\n| Example |\n| --- |\n| `@fact:TABLE` |\n";
    let doc = CommonMarkDoc::parse(source);
    assert!(doc.nodes().iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t.contains("@requires:ONE") && t.contains("#use pkg"))));
    let code = doc
        .nodes()
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::InlineCode(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(code, ["@fact:INLINE", "@fact:TABLE"]);
    assert_eq!(
        doc.nodes()
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Section { .. }))
            .count(),
        1
    );
}

#[test]
fn named_export_walks_exact_nested_sections_and_binds_captured_bytes() {
    let source = "# Guide\n\n## Introduction\nText and [link](target).\n\n## Other\nRest.\n";
    let path = ["Guide".to_owned(), "Introduction".to_owned()];
    let doc = CommonMarkDoc::parse(source);
    let selected = doc.resolve_export("intro.v1", &path).unwrap();
    assert_eq!(selected.name, "intro.v1");
    assert_eq!(selected.node, doc.heading("Introduction").unwrap());
    assert_eq!(
        &source[selected.span.clone()],
        "## Introduction\nText and [link](target).\n\n"
    );
    assert_eq!(selected.source_sha256.len(), 71);
    assert_eq!(
        selected.source_sha256,
        format!("sha256:{:x}", Sha256::digest(source.as_bytes()))
    );
    assert_eq!(doc.resolve_export("intro.v1", &path).unwrap(), selected);

    let changed = CommonMarkDoc::parse(&source.replace("Text and", "Changed text and"));
    let changed_export = changed.resolve_export("intro.v1", &path).unwrap();
    assert_ne!(selected.source_sha256, changed_export.source_sha256);
    assert_ne!(selected.span, changed_export.span);
}

#[test]
fn named_export_refuses_sibling_ambiguity_missing_and_malformed_selectors() {
    let doc = CommonMarkDoc::parse("# Guide\n## Repeat\nA.\n## Repeat\nB.\n");
    let repeated = ["Guide".to_owned(), "Repeat".to_owned()];
    assert!(matches!(
        doc.resolve_export("repeat", &repeated),
        Err(ExportResolutionError::Ambiguous { index: 1, .. })
    ));
    assert!(matches!(
        doc.resolve_export("missing", &["Guide".into(), "Absent".into()]),
        Err(ExportResolutionError::Missing { index: 1, .. })
    ));
    assert!(matches!(
        doc.resolve_export("1bad", &["Guide".into()]),
        Err(ExportResolutionError::MalformedName)
    ));
    for bad in [
        vec![],
        vec![" Guide".into()],
        vec!["Guide  Name".into()],
        vec!["Guide", ""].into_iter().map(str::to_owned).collect(),
    ] {
        assert!(
            matches!(
                doc.resolve_export("valid", &bad),
                Err(ExportResolutionError::MalformedHeadings)
            ),
            "{bad:?}"
        );
    }
}
