//! One law per form of address an island writes.

use super::*;

/// The base every case below is served under, spelled once.
const BASE: &str = "/doc/";

/// A page's neighbour is addressed as the site addresses it: the
/// directory, not the file, one level further up than the source said.
#[test]
fn a_page_link_becomes_the_address_of_that_page() {
    let links = Links::page(BASE);
    for (written, served) in [
        // The manual's commonest link, 274 of them: a glossary term.
        ("../glossary/index.xml#term", "../../glossary/index/#term"),
        // A sibling of the page, named without a prefix.
        ("first-project.xml", "../first-project/"),
        // A page in another directory of the same package.
        ("../model/two-trees.xml", "../../model/two-trees/"),
        // A page deeper than the one linking to it.
        ("reference/tree.xml", "../reference/tree/"),
        // `./` says nothing an address has to keep.
        ("./what-vibevm-is.xml", "../what-vibevm-is/"),
        // An anchor with no address is a place on this page.
        ("#p07", "#p07"),
    ] {
        assert_eq!(links.href(written).as_deref(), Some(served), "{written}");
    }
}

/// `index.xml` is a page like any other: the address map carries no
/// notion of a directory's index, and inventing one here would be a
/// second opinion about where the site serves a page from.
#[test]
fn the_index_of_a_directory_is_addressed_as_the_page_it_is() {
    assert_eq!(
        Links::page(BASE).href("../faq/index.xml").as_deref(),
        Some("../../faq/index/")
    );
}

/// A file the edition carries beside its pages keeps its name and takes
/// the same climb — which lands the card's pictures on `media/` at the
/// root of the edition, wherever in the tree the page sits.
#[test]
fn a_file_beside_the_pages_keeps_its_name_and_takes_the_same_climb() {
    let links = Links::page(BASE);
    assert_eq!(
        links.href("../media/2778b64929450ffd.svg").as_deref(),
        Some("../../media/2778b64929450ffd.svg")
    );
    // A page one level deeper climbs one level more, and the source said
    // so: the path was written beside the file, not beside the mount.
    assert_eq!(
        links.href("../../media/2778b64929450ffd.svg").as_deref(),
        Some("../../../media/2778b64929450ffd.svg")
    );
    // A projection of a page is a file too.
    assert_eq!(
        links.href("../reference/tree.md").as_deref(),
        Some("../../reference/tree.md")
    );
}

/// A citation is handed to the resolver whole, fragment and all — the
/// one address that knows what the mount in front of it carries.
#[test]
fn a_citation_is_handed_to_the_resolver_with_its_fragment_inside_the_parameter() {
    let links = Links::page(BASE);
    assert_eq!(
        links
            .href("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT")
            .as_deref(),
        Some("/doc/resolve/?uri=spec://org.vibevm.core/vibevm/common/PROP-057%23SITE-MOUNT")
    );
    // A pinned version is part of the citation and travels with it.
    assert_eq!(
        links.href("spec://org.demo/lib@2.1.0/guide").as_deref(),
        Some("/doc/resolve/?uri=spec://org.demo/lib@2.1.0/guide")
    );
    // Another base moves the resolver with it, which is what makes one
    // island right on the site and right on the loopback.
    assert_eq!(
        Links::page("/read/")
            .href("spec://org.demo/lib/guide#X")
            .as_deref(),
        Some("/read/resolve/?uri=spec://org.demo/lib/guide%23X")
    );
}

/// The links inside a quoted fact belong to the document that wrote
/// them, and are read against its address rather than against the page
/// that shows the quotation.
#[test]
fn a_link_inside_a_quotation_is_read_against_the_document_quoted() {
    let links = Links::quoting(
        BASE,
        "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-TUPLE",
    );
    for (written, served) in [
        (
            "../../common/PROP-024-code-bearing-packages.xml#build",
            "/doc/resolve/?uri=spec://org.vibevm.core/vibevm/common/\
             PROP-024-code-bearing-packages%23build",
        ),
        (
            "PROP-008-qualified-naming.xml",
            "/doc/resolve/?uri=spec://org.vibevm.core/vibevm/modules/vibe-registry/\
             PROP-008-qualified-naming",
        ),
        (
            "../vibe-index/PROP-005-package-index.xml",
            "/doc/resolve/?uri=spec://org.vibevm.core/vibevm/modules/vibe-index/\
             PROP-005-package-index",
        ),
        // An anchor alone is a place in the document being quoted.
        (
            "#R",
            "/doc/resolve/?uri=spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002%23R",
        ),
    ] {
        assert_eq!(links.href(written).as_deref(), Some(served), "{written}");
    }
}

/// An address this build cannot place gets none: a file of a repository
/// is not a document, and a path that climbs out of the package names
/// nothing the site could serve.
#[test]
fn a_target_the_island_cannot_place_gets_no_address() {
    let links = Links::quoting(BASE, "spec://org.demo/lib/common/PROP-000#token");
    assert!(
        links
            .href("../../crates/vibe-core/src/manifest.rs")
            .is_none()
    );
    assert!(links.href("../../LICENSE.md").is_none());
    assert!(
        links
            .href("../../../legacy-spec/research/PROP-004.xml")
            .is_none()
    );
}

/// The web, an address already written against the site's root, and a
/// scheme of any other kind are left exactly as the author wrote them.
#[test]
fn an_address_that_is_already_one_is_left_alone() {
    let links = Links::page(BASE);
    for target in [
        "https://vibevm.org/doc/",
        "http://127.0.0.1:8413/doc/",
        "mailto:someone@example.org",
        "//example.org/x",
        "/doc/org.demo/lib/latest/guide/",
    ] {
        assert_eq!(links.href(target).as_deref(), Some(target), "{target}");
    }
}

/// With nowhere to point, a citation gets no address — while the page
/// links around it, which depend on no base at all, are written as ever.
#[test]
fn without_a_base_a_citation_has_nowhere_to_point_and_a_page_link_still_does() {
    let links = Links::page("");
    assert!(links.href("spec://org.demo/lib/guide#X").is_none());
    assert_eq!(
        links.href("../glossary/index.xml").as_deref(),
        Some("../../glossary/index/")
    );
}

/// Only the citation form is escaped, and the one character that must be
/// is the fragment marker: a browser reading `#` in a query would keep
/// it for the resolver's own page and hand the resolver half an address.
#[test]
fn the_parameter_escapes_what_a_query_cannot_carry() {
    assert_eq!(
        encode("spec://org.demo/lib@1.0/a-b_c.d#A~B"),
        "spec://org.demo/lib@1.0/a-b_c.d%23A~B"
    );
    assert_eq!(encode("a b&c=d+e?f%g"), "a%20b%26c%3Dd%2Be%3Ff%25g");
}

/// The climb is folded, never straightened: a path that already climbed
/// keeps every level it asked for.
#[test]
fn the_climb_is_folded_and_never_straightened() {
    assert_eq!(normalise("../start/../first-project/"), "../first-project/");
    assert_eq!(normalise(".././x/"), "../x/");
    assert_eq!(normalise("../../glossary/index/"), "../../glossary/index/");
    assert_eq!(normalise("../a/b.md"), "../a/b.md");
}
