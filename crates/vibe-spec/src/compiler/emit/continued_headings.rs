//! The XML target's reading of a heading label that a lane repeats.
//!
//! An `:add` merge (PROP-035 §7.3) is the contract section followed by the
//! source section at the same `{#tag}`, so the merged text carries that tag on
//! two headings — the two `{#root}` titles of a contract and its source
//! included. PROP-035 ##FACT-MERGED-UNIQUENESS calls that repeat the
//! concatenation's own artefact, not a collision, and the Markdown target
//! emits it as written: both headings carry one qualified label. XML gives an
//! id to one element only, so the XML target keeps the label on the first
//! heading and emits every later heading with the same label as an unanchored
//! continuation. The first occurrence is the one [`DocTree`] already indexes
//! and the one a Markdown reader's `(#label)` link lands on, so both targets
//! define the same labels and resolve them to the same place.
//!
//! Only heading-against-heading repeats are continued. A fact sharing a label
//! with anything is a real collision the merge gate already refused, and one
//! that reached this far is left for the pivot to refuse again.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#source");

use std::borrow::Cow;
use std::collections::HashSet;

use crate::doctree::{DocTree, NodeKind};

/// `markdown` with the `{#label}` of every heading after the first that
/// carries the same label removed; borrowed back unchanged when no heading
/// label repeats, which is every lane without a shared merge anchor.
pub(super) fn continue_repeated_headings(markdown: &str) -> Cow<'_, str> {
    let tree = DocTree::parse(markdown);
    if tree.duplicate_anchors().is_empty() {
        return Cow::Borrowed(markdown);
    }
    let facts: HashSet<&str> = tree
        .anchored()
        .filter(|(node, _)| tree.node(*node).kind == NodeKind::Fact)
        .map(|(_, label)| label)
        .collect();
    let mut seen = HashSet::new();
    let mut continued = HashSet::new();
    for (node, label) in tree.sections() {
        if !seen.insert(label) && !facts.contains(label) {
            continued.insert(tree.node(node).heading_line);
        }
    }
    if continued.is_empty() {
        return Cow::Borrowed(markdown);
    }
    let mut output = String::with_capacity(markdown.len());
    for (index, line) in markdown.split_inclusive('\n').enumerate() {
        if continued.contains(&index) {
            output.push_str(&without_anchor(line));
        } else {
            output.push_str(line);
        }
    }
    Cow::Owned(output)
}

/// One heading line without its first `{#…}` token and the space before it —
/// the same token [`DocTree`]'s heading parser takes as the anchor.
fn without_anchor(line: &str) -> String {
    let Some(open) = line.find("{#") else {
        return line.to_string();
    };
    let Some(close) = line[open..].find('}').map(|offset| open + offset + 1) else {
        return line.to_string();
    };
    format!("{}{}", line[..open].trim_end(), &line[close..])
}

#[cfg(test)]
mod tests {
    use super::continue_repeated_headings;

    #[test]
    fn a_repeated_heading_label_stays_on_the_first_heading_only() {
        let merged = "# Review {#root}\nC\n## Rules {#rules} :add\n@fact:R1 c\n\
                      # Review {#root}\nS\n## Rules {#rules} :add\n@fact:R2 s\n";
        assert_eq!(
            continue_repeated_headings(merged),
            "# Review {#root}\nC\n## Rules {#rules} :add\n@fact:R1 c\n\
             # Review\nS\n## Rules :add\n@fact:R2 s\n"
        );
    }

    #[test]
    fn a_lane_without_a_repeat_is_returned_untouched() {
        let lane = "# A {#a}\n```\n# A {#a}\n```\n";
        assert!(matches!(
            continue_repeated_headings(lane),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn a_fact_collision_is_left_for_the_pivot_to_refuse() {
        let lane = "# A {#x}\n\n@fact:x body\n\n## B {#x}\n";
        assert_eq!(continue_repeated_headings(lane), lane);
    }
}
