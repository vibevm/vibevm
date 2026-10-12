//! Whole-lane normal section and preamble coverage at the incumbent link seam.

use std::collections::BTreeMap;

use crate::doctree::NodeKind;

/// Suppress only a previously emitted section of the same authored document
/// whose qualified source text is exactly identical. A later direct seed can
/// therefore keep every uncovered sibling; identical prose from a different
/// source document is never deduplicated by spelling.
pub(super) fn normal_body_once(
    address: &crate::SpecAddress,
    tree: &crate::DocTree,
    emitted: &mut BTreeMap<(String, String), Vec<String>>,
    whole_documents: &mut BTreeMap<String, Vec<String>>,
    preambles: &mut BTreeMap<String, Vec<String>>,
) -> String {
    let document = address.without_pin();
    let document = document.split('#').next().unwrap_or(&document).to_owned();
    let full_body = tree.text(tree.root());
    if address.anchor.is_empty()
        && whole_documents
            .get(&document)
            .is_some_and(|earlier| earlier.contains(&full_body))
    {
        return String::new();
    }
    // A section reached first can make a later whole-document occurrence
    // partial. Its root preamble is still newly emitted and must be recorded
    // independently of full-document coverage for a subsequent direct entry.
    let preamble_lines = if address.anchor.is_empty() {
        tree.children(tree.root())
            .iter()
            .find(|&&child| tree.node(child).kind == NodeKind::Heading)
            .map_or_else(
                || full_body.lines().count(),
                |&child| tree.node(child).span.start,
            )
    } else {
        0
    };
    let preamble = full_body
        .lines()
        .take(preamble_lines)
        .collect::<Vec<_>>()
        .join("\n");
    let repeated_preamble = !preamble.is_empty()
        && preambles
            .get(&document)
            .is_some_and(|earlier| earlier.contains(&preamble));
    let mut drop = Vec::new();
    let mut retain = Vec::new();
    for (section, anchor) in tree.sections() {
        let text = tree.text(section);
        let key = (document.clone(), anchor.to_owned());
        if emitted
            .get(&key)
            .is_some_and(|earlier| earlier.contains(&text))
        {
            drop.push(section);
        } else {
            retain.push((key, text));
        }
    }
    let mut body = tree.text_without(tree.root(), &drop);
    if repeated_preamble {
        body = body
            .lines()
            .skip(preamble_lines)
            .collect::<Vec<_>>()
            .join("\n");
    } else if !preamble.is_empty() {
        preambles
            .entry(document.clone())
            .or_default()
            .push(preamble);
    }
    if address.anchor.is_empty() && drop.is_empty() {
        whole_documents.entry(document).or_default().push(full_body);
    }
    for (key, text) in retain {
        emitted.entry(key).or_default().push(text);
    }
    body
}
