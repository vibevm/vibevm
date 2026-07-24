//! The merged-view uniqueness gate (PROP-035 §7.3, fact-inheritance clause 3).
//!
//! After the `#source` merge produces an effective document, the compiler
//! re-runs the anchor-uniqueness check over it — heading and fact ids, one
//! namespace. A duplicate that the per-fact override did **not** cancel (a
//! non-override collision across sections, or a fact colliding with a heading)
//! is a **build error**, never a warning: per-file cleanliness of the merge's
//! inputs does not exempt the merged output.
//!
//! The check runs on the *effective* document, so two independently-clean files
//! that collide only once folded together are caught here rather than shipping a
//! `spec://…#<id>` that resolves ambiguously.
//!
//! **Why a fact must be involved.** An `:add` merge concatenates the contract
//! and source versions of a section *verbatim* (§7.3), so the merged text
//! legitimately repeats that one section's own heading anchor — the same
//! section, not a collision. The gate therefore flags a repeat only when at
//! least one side is a **fact leaf**: that is exactly the clause-3 set (a
//! cross-section fact collision, or a fact-vs-heading collision). A pure
//! heading-vs-heading repeat is either that accepted `:add` artifact or a
//! per-file duplicate the input files already own — never introduced here.

use std::collections::HashMap;
use std::fmt;

use crate::doctree::{DocTree, NodeId, NodeKind};

/// A surviving id collision in a merged document: the id and both occurrences,
/// each located by its enclosing section and 1-based line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateId {
    pub id: String,
    pub first_section: String,
    pub first_line: usize,
    pub second_section: String,
    pub second_line: usize,
}

impl fmt::Display for DuplicateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "id `{}` is declared twice in the merged document \
             (heading and fact ids share one namespace): \
             first in section `{}` at line {}, again in section `{}` at line {}",
            self.id, self.first_section, self.first_line, self.second_section, self.second_line
        )
    }
}

/// Collect every id (heading + fact) of `tree` in document order and return the
/// **first** surviving duplicate, if any. `None` means the merged view holds one
/// id per anchor — the build may proceed.
pub fn first_duplicate(tree: &DocTree) -> Option<DuplicateId> {
    let mut seen: HashMap<&str, NodeId> = HashMap::new();
    for (nid, anchor) in tree.anchored() {
        let Some(&prev) = seen.get(anchor) else {
            seen.insert(anchor, nid);
            continue;
        };
        // A repeat is a build error only when a fact is on either side (§7.3
        // clause 3). A pure heading-vs-heading repeat is the accepted `:add`
        // concatenation artifact — skip it, keeping the first occurrence.
        if tree.node(prev).kind == NodeKind::Fact || tree.node(nid).kind == NodeKind::Fact {
            return Some(DuplicateId {
                id: anchor.to_string(),
                first_section: section_label(tree, prev),
                first_line: tree.node(prev).span.start + 1,
                second_section: section_label(tree, nid),
                second_line: tree.node(nid).span.start + 1,
            });
        }
    }
    None
}

/// Where an occurrence sits: a heading is its own section (its anchor); a fact
/// climbs its parent chain to the nearest anchored heading. Falls back to a
/// synthetic label when no anchored ancestor exists.
fn section_label(tree: &DocTree, node: NodeId) -> String {
    let n = tree.node(node);
    if n.kind == NodeKind::Heading {
        return n.id.clone().unwrap_or_else(|| "(document)".to_string());
    }
    let mut cur = n.parent;
    while let Some(p) = cur {
        let pn = tree.node(p);
        if pn.kind == NodeKind::Heading
            && let Some(a) = &pn.id
        {
            return a.clone();
        }
        cur = pn.parent;
    }
    "(preamble)".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_merged_document_has_no_duplicate() {
        let t = DocTree::parse("# A {#a}\n##fact-one x\n## B {#b}\n##fact-two y\n");
        assert!(first_duplicate(&t).is_none());
    }

    #[test]
    fn a_fact_repeated_across_sections_is_caught() {
        // The same fact id survives in two different sections (no override,
        // because they are not the same merged `:add` section).
        let t = DocTree::parse("# A {#a}\n##shared here\n## B {#b}\n##shared there\n");
        let dup = first_duplicate(&t).expect("collision");
        assert_eq!(dup.id, "shared");
        assert_eq!(dup.first_section, "a");
        assert_eq!(dup.second_section, "b");
        assert!(dup.first_line < dup.second_line);
    }

    #[test]
    fn a_fact_colliding_with_a_heading_is_caught() {
        let t = DocTree::parse("# A {#a}\nbody\n## Zed {#zed}\ntext\n# C {#c}\n##zed clash\n");
        let dup = first_duplicate(&t).expect("collision");
        assert_eq!(dup.id, "zed");
        // The heading occurrence is reported first (document order).
        assert_eq!(dup.first_section, "zed");
        assert_eq!(dup.second_section, "c");
    }

    #[test]
    fn a_repeated_section_heading_is_not_flagged() {
        // The `:add` merge concatenates both versions of a section verbatim, so
        // its own heading anchor legitimately repeats — a pure heading-vs-heading
        // repeat is not a collision and must not fail the build.
        let t = DocTree::parse("# API {#root}\ncontract side\n# Impl {#root}\nsource side\n");
        assert!(first_duplicate(&t).is_none());
    }

    #[test]
    fn display_names_the_id_and_both_lines() {
        let t = DocTree::parse("# A {#a}\n##shared here\n## B {#b}\n##shared there\n");
        let msg = first_duplicate(&t).unwrap().to_string();
        assert!(msg.contains("`shared`"), "{msg}");
        assert!(msg.contains("line"), "{msg}");
    }
}
