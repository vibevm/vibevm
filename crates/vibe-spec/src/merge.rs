//! The contract↔source merge (PROP-035 §7.3).
//!
//! A `normal` package splits a spec into a small `contract` and a heavy
//! `source` (§4); `#source` links them. This module performs the section-level
//! merge the link implies, treating anchored sections as the unit (the analogue
//! of methods). For each anchor:
//!
//! - present in the contract only → the contract section, whole;
//! - present in the source only → the source section, whole;
//! - present in both (same `{#tag}`) → merged by the marker the **source**
//!   heading carries after its anchor: `:replace` takes the source text and
//!   drops the contract's; `:add` (the default) is the sum, contract then
//!   source.
//!
//! There is deliberately no access control (`private`/`public`): a section that
//! exists only in the source is still usable (§7.3).

use std::collections::HashSet;

use crate::doctree::DocTree;

/// How a section present in both contract and source is combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMode {
    /// `:add` (the default) — contract text, then source text.
    Add,
    /// `:replace` — source text only; the contract text is dropped.
    Replace,
}

impl MergeMode {
    /// Read the mode from a source heading's trailing marker (`:replace` /
    /// `:add`). Anything else, an absent marker included, is `:add` — the
    /// default that lets the contract text appear without being duplicated.
    pub fn from_trailing(trailing: &str) -> MergeMode {
        if trailing.split_whitespace().any(|t| t == ":replace") {
            MergeMode::Replace
        } else {
            MergeMode::Add
        }
    }
}

/// Where a merged section's text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionOrigin {
    ContractOnly,
    SourceOnly,
    Merged(MergeMode),
}

/// One anchor's resolved text after merging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedSection {
    pub anchor: String,
    pub text: String,
    pub origin: SectionOrigin,
}

/// Merge a contract document with its source, section by section (§7.3).
/// Contract sections come first in document order, then any source-only
/// sections, so the result is deterministic.
pub fn merge_contract_source(contract: &DocTree, source: &DocTree) -> Vec<MergedSection> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (cid, anchor) in contract.anchored() {
        seen.insert(anchor.to_string());
        let section = match source.find_by_anchor(anchor) {
            None => MergedSection {
                anchor: anchor.to_string(),
                text: contract.text(cid),
                origin: SectionOrigin::ContractOnly,
            },
            Some(sid) => {
                let mode = MergeMode::from_trailing(&source.node(sid).trailing);
                let text = match mode {
                    MergeMode::Replace => source.text(sid),
                    MergeMode::Add => format!("{}\n{}", contract.text(cid), source.text(sid)),
                };
                MergedSection {
                    anchor: anchor.to_string(),
                    text,
                    origin: SectionOrigin::Merged(mode),
                }
            }
        };
        out.push(section);
    }

    for (sid, anchor) in source.anchored() {
        if seen.contains(anchor) {
            continue;
        }
        out.push(MergedSection {
            anchor: anchor.to_string(),
            text: source.text(sid),
            origin: SectionOrigin::SourceOnly,
        });
    }

    out
}

/// Fold `source` into `contract` at the **top level**, producing one document
/// (PROP-035 §7.3, §8 phase 3). Each top-level contract section is emitted
/// merged with its same-anchor source section — `:add` (contract then source)
/// or `:replace` (source only) — unmatched contract sections unchanged, and
/// top-level source-only sections appended after. Nested sections merge as part
/// of their top-level ancestor's subtree text; the clean case is a flat
/// contract, which is the norm (§4). `merge_contract_source` is the finer,
/// per-section view; this is the document-level reconstruction the pipeline
/// wants.
pub fn fold_source(contract: &DocTree, source: &DocTree) -> String {
    let mut out = String::new();

    for &child in contract.children(contract.root()) {
        match contract
            .node(child)
            .id
            .as_deref()
            .and_then(|a| source.find_by_anchor(a))
        {
            Some(sid) => match MergeMode::from_trailing(&source.node(sid).trailing) {
                MergeMode::Replace => out.push_str(&source.text(sid)),
                MergeMode::Add => {
                    out.push_str(&contract.text(child));
                    out.push('\n');
                    out.push_str(&source.text(sid));
                }
            },
            None => out.push_str(&contract.text(child)),
        }
        out.push('\n');
    }

    for &schild in source.children(source.root()) {
        if let Some(anchor) = source.node(schild).id.as_deref()
            && contract.find_by_anchor(anchor).is_none()
        {
            out.push_str(&source.text(schild));
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(sections: &'a [MergedSection], anchor: &str) -> &'a MergedSection {
        sections.iter().find(|s| s.anchor == anchor).unwrap()
    }

    #[test]
    fn contract_only_section() {
        let contract = DocTree::parse("# C {#only-c}\ncontract body\n");
        let source = DocTree::parse("# other {#other}\nx\n");
        let merged = merge_contract_source(&contract, &source);
        let s = find(&merged, "only-c");
        assert_eq!(s.origin, SectionOrigin::ContractOnly);
        assert!(s.text.contains("contract body"));
    }

    #[test]
    fn source_only_section() {
        let contract = DocTree::parse("# C {#c}\nx\n");
        let source = DocTree::parse("# S {#only-s}\nsource body\n");
        let merged = merge_contract_source(&contract, &source);
        let s = find(&merged, "only-s");
        assert_eq!(s.origin, SectionOrigin::SourceOnly);
        assert!(s.text.contains("source body"));
    }

    #[test]
    fn add_is_the_default_merge() {
        let contract = DocTree::parse("# Both {#both}\ncontract part\n");
        let source = DocTree::parse("# Both {#both}\nsource part\n");
        let merged = merge_contract_source(&contract, &source);
        let s = find(&merged, "both");
        assert_eq!(s.origin, SectionOrigin::Merged(MergeMode::Add));
        // The sum: contract first, then source.
        let ci = s.text.find("contract part").unwrap();
        let si = s.text.find("source part").unwrap();
        assert!(ci < si, "contract text must precede source text");
    }

    #[test]
    fn replace_drops_the_contract_text() {
        let contract = DocTree::parse("# Both {#both}\ncontract part\n");
        let source = DocTree::parse("# Both {#both} :replace\nsource part\n");
        let merged = merge_contract_source(&contract, &source);
        let s = find(&merged, "both");
        assert_eq!(s.origin, SectionOrigin::Merged(MergeMode::Replace));
        assert!(s.text.contains("source part"));
        assert!(!s.text.contains("contract part"));
    }

    #[test]
    fn contract_sections_come_first_in_order() {
        let contract = DocTree::parse("# A {#a}\n1\n# B {#b}\n2\n");
        let source = DocTree::parse("# C {#c}\n3\n");
        let merged = merge_contract_source(&contract, &source);
        let anchors: Vec<&str> = merged.iter().map(|s| s.anchor.as_str()).collect();
        assert_eq!(anchors, ["a", "b", "c"]);
    }

    #[test]
    fn fold_merges_matched_and_keeps_unmatched() {
        let contract = DocTree::parse("# A {#a}\ncontract-a\n# B {#b}\ncontract-b\n");
        let source = DocTree::parse("# A {#a}\nsource-a\n");
        let folded = fold_source(&contract, &source);
        // A is :add-merged (contract then source); B is contract-only.
        let ca = folded.find("contract-a").unwrap();
        let sa = folded.find("source-a").unwrap();
        assert!(ca < sa, "contract before source:\n{folded}");
        assert!(folded.contains("contract-b"));
    }

    #[test]
    fn fold_replace_drops_the_contract_side() {
        let contract = DocTree::parse("# A {#a}\ncontract-a\n");
        let source = DocTree::parse("# A {#a} :replace\nsource-a\n");
        let folded = fold_source(&contract, &source);
        assert!(folded.contains("source-a"));
        assert!(!folded.contains("contract-a"));
    }

    #[test]
    fn fold_appends_source_only_sections() {
        let contract = DocTree::parse("# A {#a}\ncontract-a\n");
        let source = DocTree::parse("# A {#a}\nsource-a\n# Extra {#extra}\nsource-extra\n");
        let folded = fold_source(&contract, &source);
        assert!(folded.contains("source-extra"), "{folded}");
    }
}
