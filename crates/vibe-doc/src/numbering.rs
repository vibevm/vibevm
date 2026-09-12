//! Block numbers (PROP-057 `##PIPE-NUMBERING`, `##READER-NUMBERED-BLOCKS`).
//!
//! Every flow block of a page gets an ordinal and the id `pNN` **at build
//! time**, in Rust, never by a script in the reader's browser. The number
//! then appears in the island, in the `.md` and `.xml` projections and in
//! `llms-full.txt`, so a human quoting `p12` and an agent quoting `p12`
//! quote the same place.
//!
//! ## Three decisions worth their reasons
//!
//! **Numbering happens before `when` filtering.** A page that varies by
//! platform or by agent would otherwise number differently in each build,
//! and `p12` in a bug report would mean one thing on Windows and another
//! on Linux. Counting the whole text instead leaves gaps in what any one
//! build shows, and the gaps are accepted: a number that means one thing
//! everywhere is worth more than a dense sequence.
//!
//! **The number is not state.** [`number_blocks`] is a pure function of
//! the document and [`Numbering`] is a value beside it, never a field of
//! the pivot's IR. Every law of the pivot is an equality of `SpecDoc`
//! values, so a number stored inside one would enter `PartialEq` and
//! quietly change what «the same document» means. The pivot's own
//! `to_xml` and `to_markdown` do not change and never see a `Numbering`.
//!
//! **`pNN` is positional, and that is accepted.** Insert a paragraph and
//! every number below it moves, so an old `#p12` may land on a neighbour
//! — exactly as a link to a file line does after an edit. Nothing tries
//! to remember where it used to point. Stable addressing is the named
//! anchor, which is immutable and renamed only by a tombstone
//! (`##INV-ANCHORS-IMMUTABLE`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-NUMBERING");

use std::collections::BTreeMap;

use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc};

use crate::content::Content;

/// The section id whose blocks carry no numbers.
///
/// Footnotes are apparatus, not flow: they are generated from references
/// in the text above them, their count changes whenever a reference is
/// added, and numbering them would make every `pNN` on the page move for
/// a reason no reader can see.
pub const UNNUMBERED_SECTION: &str = "footnotes";

/// A block's deterministic address in a document: the path of sections
/// from the root, then the block's index in its owner's list.
///
/// Two numbers and nothing else, because the IR holds nothing else: a
/// block carries no identity of its own (a paragraph is equal to another
/// paragraph with the same text), so a number has to come from position.
///
/// The natural order of a `BlockPath` is document order. It holds because
/// the dialect forbids a parent's block after a nested section — «`<p>`
/// cannot follow a nested `<section>`» — so a section's own blocks are
/// always read before its subsections' blocks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BlockPath {
    /// `[]` is the preamble; `[0]` the first section; `[0, 2]` its third
    /// subsection.
    pub section: Vec<u16>,
    /// The block's index inside `SpecDoc::preamble` or `Section::blocks`.
    pub block: u16,
}

impl BlockPath {
    /// The address of the `n`-th block of the section at `section`.
    pub fn new(section: Vec<u16>, block: u16) -> BlockPath {
        BlockPath { section, block }
    }
}

/// A finished numbering: address to number, and back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Numbering {
    by_path: BTreeMap<BlockPath, u32>,
    paths: Vec<BlockPath>,
}

impl Numbering {
    /// No numbers at all — the neutral element for a caller that wants a
    /// projection without them.
    pub fn none() -> Numbering {
        Numbering::default()
    }

    /// The number of the block at `path`, when it has one.
    pub fn get(&self, path: &BlockPath) -> Option<u32> {
        self.by_path.get(path).copied()
    }

    /// The label a projection prints — `p07`.
    ///
    /// Two digits with a leading zero, because a column of `p7` and `p12`
    /// does not line up and a reader's eye is the whole reason the number
    /// is in the margin. Past ninety-nine the number simply grows.
    ///
    /// ```
    /// use vibe_doc::numbering::{BlockPath, number_blocks};
    ///
    /// let doc = vibe_specdoc::from_xml_with(
    ///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
    ///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
    ///        <title id=\"root\">T</title>\n<p>one</p>\n<p>two</p>\n\
    ///      </spec>\n",
    ///     vibe_specdoc::Vocabulary::Doc,
    /// )
    /// .unwrap();
    ///
    /// let numbering = number_blocks(&doc);
    /// assert_eq!(numbering.label(&BlockPath::new(vec![], 0)).as_deref(), Some("p01"));
    /// assert_eq!(numbering.label(&BlockPath::new(vec![], 1)).as_deref(), Some("p02"));
    /// assert_eq!(numbering.len(), 2);
    /// ```
    pub fn label(&self, path: &BlockPath) -> Option<String> {
        self.get(path).map(Numbering::spell)
    }

    /// The label for a number — the one place the spelling lives.
    pub fn spell(n: u32) -> String {
        format!("p{}", Numbering::digits(n))
    }

    /// The digits a reader sees in the margin, without the `p`.
    pub fn digits(n: u32) -> String {
        if n < 10 {
            format!("0{n}")
        } else {
            n.to_string()
        }
    }

    /// The block a number names — what «see p12» resolves to, and what a
    /// translation checks itself against.
    pub fn path_of(&self, n: u32) -> Option<&BlockPath> {
        self.paths.get(n.checked_sub(1)? as usize)
    }

    /// How many blocks are numbered.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    fn assign(&mut self, path: BlockPath) {
        self.paths.push(path.clone());
        self.by_path.insert(path, self.paths.len() as u32);
    }
}

/// Number every flow block of a page, in document order.
///
/// Called after `derived` blocks are expanded and **before** `when`
/// filtering and the backends, exactly once per page. Expansion replaces
/// one block with one block, so the two orders agree — a law
/// [`expand_derived`] is tested against rather than assumed.
///
/// What is numbered: every entry of a block list. What is not: list
/// items, table cells, the children of a `prompt` and section headings —
/// none of them is a block, and a heading has an address of its own, its
/// named anchor. Nor the `footnotes` section ([`UNNUMBERED_SECTION`]).
pub fn number_blocks(doc: &SpecDoc) -> Numbering {
    let mut out = Numbering::default();
    walk_blocks(&doc.preamble, &[], &mut out);
    for (i, section) in doc.sections.iter().enumerate() {
        walk_section(section, &[i as u16], &mut out);
    }
    out
}

fn walk_section(section: &Section, path: &[u16], out: &mut Numbering) {
    if section.id.as_deref() == Some(UNNUMBERED_SECTION) {
        return;
    }
    walk_blocks(&section.blocks, path, out);
    for (i, sub) in section.sections.iter().enumerate() {
        let mut child = path.to_vec();
        child.push(i as u16);
        walk_section(sub, &child, out);
    }
}

fn walk_blocks(blocks: &[BlockNode], path: &[u16], out: &mut Numbering) {
    for (i, _) in blocks.iter().enumerate() {
        out.assign(BlockPath::new(path.to_vec(), i as u16));
    }
}

/// Replace every `derived` block with the fence its generator built.
///
/// One block in, one block out — which is what makes the numbering the
/// same before and after, and what lets `##PIPE-NUMBERING`'s order
/// («expand, then number, then render») cost nothing to obey.
///
/// A block this build could not generate is left as it is, so the
/// backends can mark it unresolved rather than render an empty fence that
/// looks like a command with no output.
pub fn expand_derived(doc: &SpecDoc, content: &Content) -> SpecDoc {
    let mut out = doc.clone();
    expand_blocks(&mut out.preamble, content);
    for section in &mut out.sections {
        expand_section(section, content);
    }
    out
}

fn expand_section(section: &mut Section, content: &Content) {
    expand_blocks(&mut section.blocks, content);
    for sub in &mut section.sections {
        expand_section(sub, content);
    }
}

fn expand_blocks(blocks: &mut [BlockNode], content: &Content) {
    for node in blocks.iter_mut() {
        let Block::Derived { kind, reference } = &node.block else {
            continue;
        };
        let Some(text) = content.derived_text(*kind, reference) else {
            continue;
        };
        node.block = Block::Fence {
            lang: Some("text".to_owned()),
            fact: None,
            text: text.to_owned(),
        };
    }
}

#[cfg(test)]
mod tests;
