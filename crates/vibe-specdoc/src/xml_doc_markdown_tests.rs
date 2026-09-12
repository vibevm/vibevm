//! The genre's Markdown projection (##DOC-VOCAB-MD-ONE-WAY).
//!
//! A READING surface and not a round trip: the host scanners see the
//! documentation only through it, so what it says is pinned here, and the
//! fact that it does not come back is pinned with it.
//!
//! Sibling test cell of [`crate::xml_doc_tests`], which owns the shared
//! fixtures it reads.

use crate::doc::Block;
use crate::xml_doc_tests::doc;
use crate::{from_markdown, to_markdown};

/// The projection of every element, pinned. It is a READING surface: the
/// host scanners see the documentation only through it, so what it says is
/// what `facts check` and `progress` count.
#[test]
fn the_markdown_projection_of_each_element_is_pinned() {
    let d = doc(
        "  <example id=\"e\" fixture=\"none\">\n    <run>vibe --version</run>\n    \
         <expect>vibe 1.0.0</expect>\n    <stderr>warning</stderr>\n  </example>\n  \
         <example ref=\"e\"/>\n  \
         <rule ref=\"spec://g/n/p#ANCHOR\"/>\n  \
         <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n  \
         <note kind=\"warning\">Mind the gap.</note>\n  \
         <figure src=\"media/a.png\" alt=\"A tree\">\n    <caption>The tree.</caption>\n  </figure>\n  \
         <prompt id=\"p\">Install vibe.\n    <needs>network</needs>\n    \
         <outcome>a version prints</outcome>\n    <assert>vibe --version</assert>\n  </prompt>\n",
    );
    let md = to_markdown(&d);
    assert!(md.contains("```sh\nvibe --version\n```"), "{md}");
    assert!(md.contains("```output\nvibe 1.0.0\n```"), "{md}");
    assert!(md.contains("```stderr\nwarning\n```"), "{md}");
    assert!(
        md.contains("Example `e` is copied from the source page"),
        "{md}"
    );
    assert!(md.contains("> <spec://g/n/p#ANCHOR>"), "{md}");
    assert!(
        md.contains("```text\ngenerated from cli-help: vibe --help\n```"),
        "{md}"
    );
    assert!(md.contains("> **Warning**\n> Mind the gap."), "{md}");
    assert!(md.contains("![A tree](media/a.png)\n\nThe tree."), "{md}");
    assert!(md.contains("```prompt\nInstall vibe.\n```"), "{md}");
    assert!(md.contains("- needs: network"), "{md}");
    assert!(md.contains("outcome: a version prints"), "{md}");
    assert!(md.contains("- assert: `vibe --version`"), "{md}");
    // The projection must PARSE, because the scanners read it.
    from_markdown(&md).unwrap_or_else(|e| panic!("{e}\n{md}"));
}

/// A guarded slot projects as a sub-heading naming the condition, one
/// level under its section (##ROW-DOCVOCAB-WHEN-MD); a guarded SECTION's
/// own heading names it.
#[test]
fn a_guarded_slot_projects_as_a_sub_heading() {
    let d = doc(
        "  <install title=\"Install\">\n    <p when=\"os:windows\">Unpack the archive.</p>\n  </install>\n  \
         <mac title=\"On macOS\" when=\"os:macos\">\n    <p>Use the script.</p>\n  </mac>\n",
    );
    let md = to_markdown(&d);
    assert!(md.contains("### os:windows\n\nUnpack the archive."), "{md}");
    assert!(md.contains("## On macOS (os:macos) {#mac}"), "{md}");
    from_markdown(&md).unwrap_or_else(|e| panic!("{e}\n{md}"));
}

/// `assert="none"` survives into the projection, so the style linter reads
/// the same fact from the Markdown as from the XML.
#[test]
fn an_illustrative_prompt_projects_its_missing_assert() {
    let md = to_markdown(&doc(
        "  <prompt id=\"p\" assert=\"none\">Ask for a tour.</prompt>\n",
    ));
    assert!(md.contains("- assert: none"), "{md}");
}

/// The recorded degradation, on one page rather than the whole corpus:
/// the genre's projection re-parses to a DIFFERENT IR. It is the law, and
/// it is why authoring documentation is XML only.
#[test]
fn doc_genre_is_not_round_trippable_through_markdown() {
    let d = doc(
        "  <example id=\"e\" fixture=\"none\">\n    <run>vibe --version</run>\n    \
         <expect>vibe 1.0.0</expect>\n  </example>\n",
    );
    let md = to_markdown(&d);
    let back = from_markdown(&md).expect("the projection parses");
    assert_ne!(back, d, "an example cannot come back from Markdown");
    // What comes back is the honest thing: two fences.
    assert_eq!(back.preamble.len(), 2);
    assert!(matches!(back.preamble[0].block, Block::Fence { .. }));
    assert!(matches!(back.preamble[1].block, Block::Fence { .. }));
}
