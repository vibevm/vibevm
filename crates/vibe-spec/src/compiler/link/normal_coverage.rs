//! Source-identity coverage for one artifact's normal contribution stream.
//! Captured variants are compared exactly; unrelated documents never share a key.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#NORMAL-CLOSURE-ONCE"
);

use crate::embed::occurrences::{EmbedRegionKind, EmbedTape};
use crate::{DocTree, SpecAddress, doctree::NodeKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct NormalCoverage {
    documents: BTreeMap<String, DocumentCoverage>,
}

#[derive(Default)]
struct DocumentCoverage {
    whole: BTreeSet<(String, [u8; 32])>,
    preambles: BTreeSet<(String, [u8; 32])>,
    units: BTreeMap<String, BTreeSet<(String, [u8; 32])>>,
}

impl NormalCoverage {
    /// Select new regions from an already qualified and linked captured tree. Span removal
    /// uses this tree's coordinates; identity uses its parsed address and anchors.
    pub(super) fn body(&mut self, address: &SpecAddress, text: &str) -> String {
        self.project(address, text, None).0
    }

    pub(super) fn project(
        &mut self,
        address: &SpecAddress,
        text: &str,
        embeds: Option<&EmbedTape>,
    ) -> (String, Vec<bool>, BTreeSet<Vec<usize>>) {
        let tree = DocTree::parse(text);
        let signature =
            |span, preamble| embeds.map_or([0; 32], |tape| tape.coverage_digest(span, preamble));
        let whole_key = (text.to_owned(), signature(0..text.lines().count(), true));
        let whole = address.anchor.is_empty();
        let mut document = address.clone();
        document.anchor.clear();
        let covered = self.documents.entry(document.without_pin()).or_default();
        if whole && covered.whole.contains(&whole_key) {
            return (
                String::new(),
                vec![false; text.lines().count()],
                BTreeSet::new(),
            );
        }
        let mut omitted = BTreeSet::new();
        let mut omitted_regions = Vec::new();
        let mut omitted_preamble = None;
        let units = tree
            .anchored()
            .map(|(node, anchor)| {
                (
                    tree.node(node).span.clone(),
                    anchor.to_owned(),
                    (
                        tree.text(node),
                        signature(tree.node(node).span.clone(), false),
                    ),
                )
            })
            .collect::<Vec<_>>();
        for (span, anchor, body) in &units {
            if covered
                .units
                .get(anchor)
                .is_some_and(|variants| variants.contains(body))
            {
                omitted.extend(span.clone());
                omitted_regions.push(span.clone());
            }
        }
        if whole {
            let root_span = tree.node(tree.root()).span.clone();
            let first_heading = tree
                .children(tree.root())
                .iter()
                .find(|&&node| tree.node(node).kind == NodeKind::Heading)
                .map_or(root_span.end, |&node| tree.node(node).span.start);
            let preamble = text
                .split('\n')
                .zip(root_span.clone())
                .take_while(|(_, line)| *line < first_heading)
                .map(|(line, _)| line)
                .collect::<Vec<_>>()
                .join("\n");
            let preamble_key = (preamble, signature(root_span.start..first_heading, true));
            if (!preamble_key.0.is_empty()
                || embeds.is_some_and(|t| {
                    t.occurrences.iter().any(|row| {
                        row.owner.start == 0
                            && row.owner.end <= first_heading
                            && matches!(row.owner.kind, EmbedRegionKind::Preamble)
                    })
                }))
                && !covered.preambles.insert(preamble_key)
            {
                omitted.extend(root_span.start..first_heading);
                omitted_preamble = Some(root_span.start..first_heading);
            }
            // Prior identical regions plus the remainder emitted now cover this
            // complete captured variant, including unanchored headings/text.
            covered.whole.insert(whole_key);
        }
        for (_, anchor, body) in units {
            covered.units.entry(anchor).or_default().insert(body);
        }
        let retained_empty = embeds
            .into_iter()
            .flat_map(|t| &t.occurrences)
            .filter(|row| {
                if row.start != row.end {
                    return false;
                }
                let owner = &row.owner;
                let preamble_removed = matches!(owner.kind, EmbedRegionKind::Preamble)
                    && omitted_preamble
                        .as_ref()
                        .is_some_and(|span| span.start <= owner.start && owner.end <= span.end);
                let unit_removed = omitted_regions.iter().any(|span| {
                    span.start <= owner.start
                        && owner.end <= span.end
                        && !(matches!(owner.kind, EmbedRegionKind::Preamble)
                            && owner.start == span.start
                            && owner.start == owner.end)
                });
                !preamble_removed && !unit_removed
            })
            .map(|row| row.path.clone())
            .collect();
        // Union original line spans before removing anything. Removing a fact
        // in the preamble must not shift a later heading into a prefix removal.
        if omitted.is_empty() {
            return (
                text.to_owned(),
                vec![true; text.lines().count()],
                retained_empty,
            );
        }
        let output = text
            .split_inclusive('\n')
            .enumerate()
            .filter(|(line, _)| !omitted.contains(line))
            .map(|(_, text)| text)
            .collect();
        (
            output,
            (0..text.lines().count())
                .map(|line| !omitted.contains(&line))
                .collect(),
            retained_empty,
        )
    }
}

#[cfg(test)]
#[path = "normal_coverage/tests.rs"]
mod tests;
