//! B-206: one `#source` merge, two static targets, one set of labels.
//!
//! The contract and its source share every anchor they declare — the `{#root}`
//! titles and a nested section — and the source overrides one contract fact.
//! The Markdown target always compiled this; the XML target refused it because
//! the merged text defines each shared heading label twice. Both must now
//! define the same qualified labels, the XML one exactly once each.

use std::collections::BTreeSet;

use super::*;
use crate::doctree::{DocTree, NodeKind};

const CONTRACT: &str = "spec://org.acme/review/boot/review#root";
const IMPLEMENTATION: &str = "spec://org.acme/review/source/review";

fn plan(target: ArtifactTarget, generated_path: &str) -> ArtifactPlan {
    ArtifactPlan::static_lane(
        target,
        generated_path,
        "vibevm/vibedeps",
        vec![ArtifactInput::normal("org.acme/review", "boot/review.md", spec(CONTRACT)).unwrap()],
    )
    .unwrap()
}

fn source() -> CountingSource {
    CountingSource::with(&[
        (
            CONTRACT,
            &format!(
                "# Review {{#root}}\n#source {IMPLEMENTATION}\nThe contract.\n\n\
                 ## Rules {{#rules}}\n\n@fact:R1 contract rule\n\n@fact:TONE contract tone\n"
            ),
        ),
        (
            IMPLEMENTATION,
            "# Review {#root}\nThe implementation.\n\n\
             ## Rules {#rules}\n\n@fact:R2 source rule\n\n@fact:TONE source tone\n",
        ),
    ])
}

/// Every label the document defines, in document order, split into heading
/// anchors and fact ids — the shape both targets are compared on.
fn labels(markdown: &str) -> (Vec<String>, Vec<String>) {
    let tree = DocTree::parse(markdown);
    let mut headings = Vec::new();
    let mut facts = Vec::new();
    for (node, label) in tree.anchored() {
        match tree.node(node).kind {
            NodeKind::Heading => headings.push(label.to_string()),
            NodeKind::Fact => facts.push(label.to_string()),
        }
    }
    (headings, facts)
}

/// The one `<spec>` document of a single-contribution STATIC.xml, read back
/// through the pivot and projected to Markdown for [`labels`].
fn xml_document_markdown(artifact: &str) -> String {
    let start = artifact.find("<?xml").expect("an XML document is emitted");
    let end = artifact.rfind("</spec>").expect("the document closes") + "</spec>".len();
    let document = vibe_specdoc::from_xml(&artifact[start..end]).expect("the emitted XML parses");
    vibe_specdoc::to_markdown(&document)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#XML-CONTINUED-HEADING")]
fn a_merge_sharing_anchors_compiles_to_the_same_labels_under_both_targets() {
    let markdown = compile_artifact(
        plan(
            ArtifactTarget::StaticMarkdown,
            "vibevm/vibespecs/boot/STATIC.md",
        ),
        &source(),
    )
    .expect("the Markdown target compiles the merge");
    let xml = compile_artifact(
        plan(
            ArtifactTarget::StaticXml,
            "vibevm/vibespecs/boot/STATIC.xml",
        ),
        &source(),
    )
    .expect("the XML target compiles the same merge");

    let (md_headings, md_facts) = labels(std::str::from_utf8(markdown.bytes()).unwrap());
    let (xml_headings, xml_facts) = labels(&xml_document_markdown(
        std::str::from_utf8(xml.bytes()).unwrap(),
    ));

    // The Markdown target writes the `:add` sum as it is: each shared
    // heading label on the contract's heading and again on the source's.
    assert_eq!(
        md_headings,
        [
            "org-acme--review--root",
            "org-acme--review--rules",
            "org-acme--review--root",
            "org-acme--review--rules",
        ]
    );
    // The XML target defines each of those labels once, on the first heading.
    assert_eq!(
        xml_headings,
        ["org-acme--review--root", "org-acme--review--rules"]
    );
    assert_eq!(
        xml_headings.iter().collect::<BTreeSet<_>>(),
        md_headings.iter().collect::<BTreeSet<_>>(),
        "both targets define the same heading labels"
    );
    // Facts are identical, the override included: the source's TONE replaces
    // the contract's under both targets.
    assert_eq!(
        md_facts,
        [
            "org-acme--review--R1",
            "org-acme--review--R2",
            "org-acme--review--TONE",
        ]
    );
    assert_eq!(xml_facts, md_facts);
    let xml_text = std::str::from_utf8(xml.bytes()).unwrap();
    assert!(xml_text.contains("source tone") && !xml_text.contains("contract tone"));
}
