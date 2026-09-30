use vibe_specdoc::{from_markdown, to_xml};

#[test]
fn nested_item_starting_with_a_multibyte_char_converts() {
    let md = "# probe {#root}\n\n- item\n  * `ΓFoldRef.edition?`\n  - `Φ/Ψ policy-ids?`\n";
    let doc = from_markdown(md).expect("the nested items parse");
    let xml = to_xml(&doc);
    assert!(xml.contains("ΓFoldRef.edition?"), "{xml}");
    assert!(xml.contains("Φ/Ψ policy-ids?"), "{xml}");
}

#[test]
fn nested_task_box_is_restored_at_the_head_of_its_item() {
    let md = "# probe {#root}\n\n- item\n  - [ ] Γ nested task\n";
    let doc = from_markdown(md).expect("the nested task item parses");
    let xml = to_xml(&doc);
    assert!(xml.contains("[ ] Γ nested task"), "{xml}");
    assert!(
        !xml.contains("[ ] [ ]"),
        "the box must not be doubled: {xml}"
    );
}
