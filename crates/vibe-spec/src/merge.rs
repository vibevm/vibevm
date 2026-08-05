//! The contract↔source merge (PROP-035 §7.3).
//!
//! A `normal` package splits a spec into a small `contract` and a heavy
//! `source` (§4); `#source` links them. This module performs the section-level
//! merge the link implies, treating anchored sections as the unit (the analogue
//! of methods). Two entry points share the logic: `merge_contract_source`, the
//! finer per-section view against **one** source; and `fold_sources`, the
//! document-level reconstruction the pipeline wants, against an **ordered list**
//! of sources. `fold_source(c, s)` is the single-source degenerate case,
//! `fold_sources(c, &[s])`.
//!
//! For a top-level contract section at anchor `a`, let `S(a)` be the
//! subsequence of sources whose top level carries that anchor, in slice order:
//!
//! - `S(a)` empty → the contract section, whole;
//! - **any** member of `S(a)` carries `:replace` → the contract text is dropped
//!   and the members are emitted in order — `:replace` from one source throws
//!   away the *contract* text only; the sources still add together;
//! - otherwise (`:add` for every member) → the contract text minus each fact
//!   **any** member redeclares (the override set is the *union* over `S(a)`),
//!   then the members in order.
//!
//! Top-level sections present only in a source are appended after, across all
//! sources in slice order, and are **not** deduplicated between sources: two
//! sources declaring the same source-only anchor both appear. A declaration is
//! idempotent, a definition is not, so that IS an error — but this module is
//! not where it is caught, and the post-merge anchor-uniqueness check is not
//! either: [`crate::gate::first_duplicate`] deliberately tolerates a repeated
//! heading (indistinguishable from the legitimate `:add` concatenation), and by
//! the time it runs nothing records which source brought what. The catcher sits
//! upstream in [`crate::pipeline::fold`], which still holds each source's tree
//! separately.
//!
//! **Per-fact override** (§7.3, fact-inheritance clause 2). Within an `:add`
//! merge, a fact redeclaring a contract fact's `##<ID>` overrides it: the
//! contract fact's span is dropped and the redeclaring source's stays in place.
//! Against a list of sources the dropped set is the union of every member's
//! redeclarations (last-wins, contract→sources order). Facts on one side only,
//! and all non-fact text, are carried unchanged; `:replace` supersedes the
//! whole contract side, facts included.
//!
//! There is deliberately no access control (`private`/`public`): a section that
//! exists only in a source is still usable (§7.3).

use std::collections::HashSet;

use crate::doctree::{DocTree, NodeId, NodeKind};

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

    for (cid, anchor) in contract.sections() {
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
                    MergeMode::Add => {
                        let dropped = overridden_facts(contract, cid, &[(source, sid)]);
                        format!(
                            "{}\n{}",
                            contract.text_without(cid, &dropped),
                            source.text(sid)
                        )
                    }
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

    // Source-only heading sections, appended in order; source-only facts ride
    // inside their section's span, never as their own merge unit.
    for (sid, anchor) in source.sections() {
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

/// The contract facts (in `cid`'s subtree) whose id **any** of the matched
/// source sections redeclares — the spans dropped under per-fact override
/// (PROP-035 §7.3, clause 2), generalised from one source to the sequence
/// `S(a)`: the override set is the *union* over the members. One id, one unit:
/// redeclaration is the override. A single-member slice reproduces the original
/// one-source behaviour exactly.
fn overridden_facts(
    contract: &DocTree,
    cid: NodeId,
    members: &[(&DocTree, NodeId)],
) -> Vec<NodeId> {
    let source_ids: HashSet<&str> = members
        .iter()
        .flat_map(|&(s, sid)| s.facts_under(sid))
        .map(|(_, a)| a)
        .collect();
    contract
        .facts_under(cid)
        .into_iter()
        .filter(|(_, a)| source_ids.contains(a))
        .map(|(id, _)| id)
        .collect()
}

/// Fold an ordered list of `sources` into `contract` at the **top level**,
/// producing one document (PROP-035 §7.3, §8 phase 3). Each top-level contract
/// section is merged against the subsequence of sources carrying its anchor,
/// and top-level source-only sections are appended after, across all sources in
/// slice order. See the module-level docs for the section law. Nested sections
/// merge as part of their top-level ancestor's subtree text; the clean case is a
/// flat contract, which is the norm (§4). `merge_contract_source` is the finer,
/// per-section view; this is the document-level reconstruction the pipeline
/// wants.
pub fn fold_sources(contract: &DocTree, sources: &[&DocTree]) -> String {
    let mut out = String::new();

    for &child in contract.children(contract.root()) {
        // Top-level fact leaves (preamble facts) ride with the preamble, which
        // the fold does not re-emit; only heading sections are folded.
        if contract.node(child).kind != NodeKind::Heading {
            continue;
        }

        // S(a): the sources whose top level carries this section's anchor, in
        // slice order. Empty when the section has no anchor or no source matches.
        let members: Vec<(&DocTree, NodeId)> = match contract.node(child).id.as_deref() {
            Some(anchor) => sources
                .iter()
                .copied()
                .filter_map(|s| s.find_by_anchor(anchor).map(|sid| (s, sid)))
                .collect(),
            None => Vec::new(),
        };

        // The pieces that make up this section's text, in order. They are joined
        // by a single '\n' and followed by one trailing '\n' — the layout the
        // single-source fold always used, so the one-member case matches it byte
        // for byte (see `fold_source_equals_fold_sources_singleton`).
        let pieces: Vec<String> = if members.is_empty() {
            vec![contract.text(child)]
        } else {
            let any_replace = members.iter().any(|&(s, sid)| {
                MergeMode::from_trailing(&s.node(sid).trailing) == MergeMode::Replace
            });
            if any_replace {
                // `:replace` from any member drops the contract text only; the
                // members still add together, in slice order.
                members.iter().map(|&(s, sid)| s.text(sid)).collect()
            } else {
                // `:add` for every member: contract text minus the union of
                // facts any member redeclares, then the members in slice order.
                let dropped = overridden_facts(contract, child, &members);
                let mut p = Vec::with_capacity(members.len() + 1);
                p.push(contract.text_without(child, &dropped));
                p.extend(members.iter().map(|&(s, sid)| s.text(sid)));
                p
            }
        };

        out.push_str(&pieces.join("\n"));
        out.push('\n');
    }

    // Top-level source-only sections, across all sources in slice order. No
    // deduplication between sources: two sources declaring the same source-only
    // anchor both appear. That is an error (a declaration is idempotent, a
    // definition is not), and it is rejected BEFORE this runs, by
    // `pipeline::fold::first_source_section_collision` — the only layer that
    // still knows which source brought which section. The post-merge gate
    // cannot do it: it tolerates a heading-only repeat on purpose.
    for source in sources {
        for &schild in source.children(source.root()) {
            if source.node(schild).kind == NodeKind::Heading
                && let Some(anchor) = source.node(schild).id.as_deref()
                && contract.find_by_anchor(anchor).is_none()
            {
                out.push_str(&source.text(schild));
                out.push('\n');
            }
        }
    }

    out
}

/// Fold `source` into `contract` at the **top level** — the single-source
/// degenerate case of [`fold_sources`]: `fold_sources(contract, &[source])`.
/// See the module-level docs for the section law.
pub fn fold_source(contract: &DocTree, source: &DocTree) -> String {
    fold_sources(contract, &[source])
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

    fn count(hay: &str, needle: &str) -> usize {
        hay.matches(needle).count()
    }

    #[test]
    fn fold_add_overrides_a_redeclared_fact() {
        // Source's `##fact-a` overrides the contract's; `##fact-b` (contract
        // only) survives; the id appears exactly once in the merged output.
        let contract =
            DocTree::parse("# API {#root}\n- ##fact-a contract version\n- ##fact-b keep me\n");
        let source = DocTree::parse("# Impl {#root}\n- ##fact-a source version\n");
        let folded = fold_source(&contract, &source);
        assert!(folded.contains("source version"), "{folded}");
        assert!(!folded.contains("contract version"), "{folded}");
        assert!(folded.contains("##fact-b"), "{folded}");
        assert_eq!(
            count(&folded, "##fact-a"),
            1,
            "one surviving fact-a:\n{folded}"
        );
    }

    #[test]
    fn fold_add_keeps_both_when_no_redeclaration() {
        let contract = DocTree::parse("# API {#root}\n- ##fact-a contract\n");
        let source = DocTree::parse("# Impl {#root}\n- ##fact-b source\n");
        let folded = fold_source(&contract, &source);
        assert!(folded.contains("##fact-a"), "{folded}");
        assert!(folded.contains("##fact-b"), "{folded}");
    }

    #[test]
    fn fold_replace_drops_all_contract_facts() {
        let contract = DocTree::parse("# API {#root}\n- ##fact-a contract\n");
        let source = DocTree::parse("# Impl {#root} :replace\n- ##fact-z source\n");
        let folded = fold_source(&contract, &source);
        assert!(
            !folded.contains("##fact-a"),
            "contract facts survive:\n{folded}"
        );
        assert!(folded.contains("##fact-z"), "{folded}");
    }

    #[test]
    fn merge_contract_source_add_overrides_a_fact() {
        let contract =
            DocTree::parse("# API {#root}\n- ##fact-a contract version\n- ##fact-b keep\n");
        let source = DocTree::parse("# Impl {#root}\n- ##fact-a source version\n");
        let merged = merge_contract_source(&contract, &source);
        let s = find(&merged, "root");
        assert_eq!(s.origin, SectionOrigin::Merged(MergeMode::Add));
        assert!(s.text.contains("source version"), "{}", s.text);
        assert!(!s.text.contains("contract version"), "{}", s.text);
        assert_eq!(count(&s.text, "##fact-a"), 1, "one fact-a:\n{}", s.text);
    }

    #[test]
    fn merge_contract_source_ignores_facts_as_units() {
        // A source-only fact does not surface as its own MergedSection — it
        // rides inside its section's span (here, the source-only section).
        let contract = DocTree::parse("# A {#a}\nx\n");
        let source = DocTree::parse("# B {#b}\n- ##loose-fact y\n");
        let merged = merge_contract_source(&contract, &source);
        assert!(
            merged.iter().all(|s| s.anchor != "loose-fact"),
            "{merged:?}"
        );
        // The fact still travels with its section's text.
        assert!(find(&merged, "b").text.contains("##loose-fact"));
    }

    #[test]
    fn fold_sources_add_two_in_slice_order() {
        // Two sources both carry #both under :add; output is contract, then s1,
        // then s2 — verified by index order, not just presence.
        let contract = DocTree::parse("# Both {#both}\ncontract part\n");
        let s1 = DocTree::parse("# S1 {#both}\nsource one\n");
        let s2 = DocTree::parse("# S2 {#both}\nsource two\n");
        let folded = fold_sources(&contract, &[&s1, &s2]);
        let c = folded.find("contract part").unwrap();
        let one = folded.find("source one").unwrap();
        let two = folded.find("source two").unwrap();
        assert!(c < one && one < two, "contract → s1 → s2:\n{folded}");
    }

    #[test]
    fn fold_sources_replace_on_second_keeps_both_sources() {
        // :replace from s2 drops the CONTRACT text only; both sources survive,
        // in order s1 → s2 (the sources still add together).
        let contract = DocTree::parse("# Both {#both}\ncontract part\n");
        let s1 = DocTree::parse("# S1 {#both}\nsource one\n");
        let s2 = DocTree::parse("# S2 {#both} :replace\nsource two\n");
        let folded = fold_sources(&contract, &[&s1, &s2]);
        assert!(
            !folded.contains("contract part"),
            "contract text survived:\n{folded}"
        );
        let one = folded.find("source one").unwrap();
        let two = folded.find("source two").unwrap();
        assert!(one < two, "s1 before s2:\n{folded}");
    }

    #[test]
    fn fold_sources_add_union_of_overridden_facts() {
        // s1 redeclares ##fact-a, s2 redeclares ##fact-b; the dropped set is the
        // union — both contract versions vanish, both source versions survive,
        // and each id appears exactly once.
        let contract =
            DocTree::parse("# API {#root}\n- ##fact-a contract-a\n- ##fact-b contract-b\n");
        let s1 = DocTree::parse("# I1 {#root}\n- ##fact-a src-a\n");
        let s2 = DocTree::parse("# I2 {#root}\n- ##fact-b src-b\n");
        let folded = fold_sources(&contract, &[&s1, &s2]);
        assert!(
            !folded.contains("contract-a"),
            "contract-a survived:\n{folded}"
        );
        assert!(
            !folded.contains("contract-b"),
            "contract-b survived:\n{folded}"
        );
        assert!(folded.contains("src-a"), "{folded}");
        assert!(folded.contains("src-b"), "{folded}");
        assert_eq!(count(&folded, "##fact-a"), 1, "one fact-a:\n{folded}");
        assert_eq!(count(&folded, "##fact-b"), 1, "one fact-b:\n{folded}");
    }

    #[test]
    fn fold_sources_source_only_no_dedup_between_sources() {
        // Source-only sections append in slice order; the SAME source-only
        // anchor declared by both sources appears TWICE (no dedup — the
        // post-merge uniqueness check trips on the duplicate, by design).
        let contract = DocTree::parse("# A {#a}\ncontract-a\n");
        let s1 = DocTree::parse("# Extra {#extra}\nfrom s1\n");
        let s2 = DocTree::parse("# Extra {#extra}\nfrom s2\n");
        let folded = fold_sources(&contract, &[&s1, &s2]);
        assert_eq!(count(&folded, "from s1"), 1, "{folded}");
        assert_eq!(count(&folded, "from s2"), 1, "{folded}");
        assert_eq!(
            count(&folded, "# Extra"),
            2,
            "two declarations, no dedup:\n{folded}"
        );
        let i1 = folded.find("from s1").unwrap();
        let i2 = folded.find("from s2").unwrap();
        assert!(i1 < i2, "s1's section before s2's:\n{folded}");
    }

    #[test]
    fn fold_source_equals_fold_sources_singleton() {
        // РТ-1 proof: the single-source fold is the byte-for-byte degenerate
        // case of the list fold. One input exercising every branch — a matched
        // :add section with a fact override, a contract-only section, and a
        // source-only section.
        let contract = DocTree::parse(
            "# API {#root}\n- ##fact-a contract version\n# Keep {#keep}\ncontract-keep\n",
        );
        let source = DocTree::parse(
            "# Impl {#root}\n- ##fact-a source version\n# Extra {#extra}\nsource-extra\n",
        );
        assert_eq!(
            fold_source(&contract, &source),
            fold_sources(&contract, &[&source]),
            "single-source fold must equal the singleton list fold, byte for byte"
        );
    }

    #[test]
    fn fold_sources_empty_slice_emits_contract_whole() {
        // No sources → no panic, every contract section emitted whole.
        let contract = DocTree::parse("# A {#a}\nbody-a\n# B {#b}\nbody-b\n");
        let folded = fold_sources(&contract, &[]);
        assert!(folded.contains("body-a"), "{folded}");
        assert!(folded.contains("body-b"), "{folded}");
    }
}
