//! Link-destination opacity regressions for terminal requirements.

use super::parse_document;
use super::requires::mask_link_destinations;

#[test]
fn only_complete_inline_links_make_destinations_opaque() {
    let linked = "[label](https://example/é\\)@requires:plan)";
    let masked = mask_link_destinations(linked);
    assert_eq!(masked.len(), linked.len());
    assert!(!masked.contains("@requires:"), "{masked:?}");
    assert!(std::str::from_utf8(masked.as_bytes()).is_ok());

    for visible in [
        "stray ](@requires:plan)",
        "[label](@requires:plan",
        "escaped \\[label](@requires:plan)",
        "[label\\](@requires:plan)",
    ] {
        assert_eq!(mask_link_destinations(visible), visible);
    }

    for source in [
        "@fact:A stray ]( @requires:plan @status:spec/done",
        "@fact:A [label](unclosed @requires:plan @status:spec/done",
        "@fact:A [label\\](@requires:research) prose @requires:plan @status:spec/done",
    ] {
        let doc = parse_document("x.md", source);
        assert_eq!(doc.error_count(), 0, "{source}: {:#?}", doc.issues);
        assert_eq!(
            doc.blocks[0].facts[0]
                .requirements
                .as_ref()
                .unwrap()
                .to_csv(),
            "plan"
        );
    }
}
