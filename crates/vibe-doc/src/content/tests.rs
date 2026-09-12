//! The address map is deterministic, so it is testable without a site.

use super::*;

/// A bare address means `latest`, a pinned one means that version, and
/// the page path ends with a slash before the fragment
/// (PROP-057 `##SITE-TRAILING-SLASH`).
#[test]
fn the_address_map_is_the_one_the_spec_writes() {
    let c = Content::new();
    assert_eq!(
        c.link("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#X")
            .as_deref(),
        Some("/doc/org.vibevm.core/vibevm/latest/modules/vibe-registry/PROP-002/#X")
    );
    assert_eq!(
        c.link("spec://org.demo/lib@0.2.0/guide#A").as_deref(),
        Some("/doc/org.demo/lib/0.2.0/guide/#A")
    );
}

/// A dotted tree path keeps its dots in the fragment — it is one anchor
/// spelled in segments, not several.
#[test]
fn a_dotted_tree_path_stays_one_fragment() {
    let c = Content::new();
    assert_eq!(
        c.link("spec://org.demo/lib/guide#a.b.c").as_deref(),
        Some("/doc/org.demo/lib/latest/guide/#a.b.c")
    );
}

/// An address with no anchor points at the page, not at a fragment of it.
#[test]
fn an_address_without_an_anchor_points_at_the_page() {
    let c = Content::new();
    assert_eq!(
        c.link("spec://org.demo/lib/guide").as_deref(),
        Some("/doc/org.demo/lib/latest/guide/")
    );
}

/// A reader served from somewhere else gets links from there.
#[test]
fn another_base_moves_every_link_with_it() {
    let c = Content::new().with_base("http://127.0.0.1:7777/");
    assert_eq!(
        c.link("spec://org.demo/lib/guide#A").as_deref(),
        Some("http://127.0.0.1:7777/org.demo/lib/latest/guide/#A")
    );
}

/// Nothing that is not an address becomes a link — an invented one would
/// be worse than none.
#[test]
fn a_string_that_is_not_an_address_gets_no_link() {
    let c = Content::new();
    assert_eq!(c.link("not an address"), None);
    assert_eq!(c.link("spec://vibevm/common/PROP-001#X"), None);
}

/// The `derived` key is one spelling, shared by whoever fills the bundle
/// and whoever reads it.
#[test]
fn a_derived_block_is_found_by_its_kind_and_reference() {
    let mut c = Content::new();
    c.derived.insert(
        Content::derived_key(DerivedKind::CliHelp, "vibe list --help"),
        "Usage: vibe list".to_owned(),
    );
    assert_eq!(
        c.derived_text(DerivedKind::CliHelp, "vibe list --help"),
        Some("Usage: vibe list")
    );
    assert_eq!(
        c.derived_text(DerivedKind::JtdSchema, "vibe list --help"),
        None
    );
}
