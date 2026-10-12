//! C3: actual materialized-source line spans, separate from carried source decoration.

#[test]
fn xml_native_section_and_fact_ranges_follow_selected_bytes() {
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <title id=\"root\">Document</title>\n",
        "  <section id=\"law\" title=\"Law\">\n",
        "    <p><fact id=\"ITEM\" status=\"impl/done\">Body</fact></p>\n",
        "  </section>\n",
        "</spec>\n",
    );
    assert_eq!(
        crate::xmlspec::native_anchor_range(xml, "root"),
        Ok(Some((2, 6)))
    );
    assert_eq!(
        crate::xmlspec::native_anchor_range(xml, "law"),
        Ok(Some((3, 5)))
    );
    assert_eq!(
        crate::xmlspec::native_anchor_range(xml, "ITEM"),
        Ok(Some((4, 4)))
    );
    assert_eq!(crate::xmlspec::native_anchor_range(xml, "absent"), Ok(None));
}

#[test]
fn markdown_native_section_range_ends_at_next_peer() {
    let md = "# Document {#root}\n\n## Law {#law}\n\nBody\n\n## Next {#next}\n";
    assert_eq!(
        crate::mdspec::native_anchor_range(md, "law"),
        Ok(Some((3, 6)))
    );
    assert_eq!(
        crate::mdspec::native_anchor_range(md, "root"),
        Ok(Some((1, 7)))
    );
}

#[test]
fn vibe_markdown_fact_spelling_has_a_native_anchor_line() {
    let md = "# Document {#root}\n\n@fact:ITEM The law. @status:impl/done\n";
    assert_eq!(
        crate::mdspec::native_anchor_range(md, "ITEM"),
        Ok(Some((3, 3)))
    );
}

#[test]
fn duplicate_xml_anchor_cannot_be_resolved_by_first_match() {
    let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"x\" title=\"A\"/><section id=\"x\" title=\"B\"/></spec>";
    assert!(crate::xmlspec::native_anchor_range(xml, "x").is_err());
}
