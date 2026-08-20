//! The parsed-document model: blocks, units, markers, issues.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#placement");

use crate::model::Marker;
use serde::{Deserialize, Serialize};

/// A contiguous run of non-blank lines (outside fences), or one fenced
/// code block. A `Text` block is segmented into countable **facts**
/// (PROP-043 §3.9 fact amendment): plain paragraph, lead lines, list
/// items, table body cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub kind: BlockKind,
    /// 1-based inclusive line range in the source file.
    pub line_start: usize,
    pub line_end: usize,
    /// Block text with inline-code spans blanked (marker scanning input).
    #[serde(skip)]
    pub scan_text: String,
    /// The countable fact units of a Text block (empty for other kinds).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<Fact>,
}

/// One countable unit of the fact grammar (PROP-043 §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub kind: FactKind,
    /// The `##<ID>` fact anchor, when the unit carries one (§3.8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 1-based line the unit starts on.
    pub line: usize,
    /// Byte span of the unit's own text inside the block's `scan_text`.
    #[serde(skip)]
    pub span: (usize, usize),
    /// Set by the marker scan: the unit carries its own marker.
    #[serde(default)]
    pub marked: bool,
    /// The 1-based inclusive line range of an adjacent block this fact's body
    /// extends into, when its anchor names an object type (`@fact/code:<ID>`).
    ///
    /// Without this a claim inside a fenced block belongs to no fact at all:
    /// measured over this corpus, 372 fenced blocks carried zero facts while
    /// every one of 7255 text blocks carried its own. Such a claim can be
    /// neither judged nor made stale, which is how a false line survives in a
    /// document everyone believes is verified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covers: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FactKind {
    /// A whole paragraph (a Text block with no list items or table rows).
    Para,
    /// The lead lines of a block before its first list item / table row.
    Lead,
    /// One list item (any nesting level).
    Item,
    /// One non-empty table body cell.
    Cell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockKind {
    /// Prose — participates in the exhaustiveness requirement.
    Text,
    /// Fenced code — never scanned, never requires a marker.
    Code,
    /// A block that is nothing but HTML comments — exempt.
    Comment,
    /// A block that is exactly one standalone status marker.
    MarkerOnly,
    /// A heading line.
    Heading,
}

/// An anchored (or anchor-less) heading unit: heading → next heading of
/// the same or higher level (the owner-fixed body-span rule, PROP-035 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub heading: String,
    pub level: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    /// sha256 of the unit's text (baseline identity, PROP-043 §7.3).
    pub content_hash: String,
}

/// Validation diagnostics (the `check` surface, PROP-043 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub line: usize,
    pub code: IssueCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueCode {
    /// Attribute value outside the closed vocabulary.
    Vocabulary,
    /// Point marker not self-closed / tag syntax broken.
    Malformed,
    /// Required attribute missing.
    MissingAttr,
    /// Standalone marker between paragraphs (PROP-043 §3.8: forbidden).
    Stranded,
    /// Marker mid-unit (only first/last token positions are legal).
    MidParagraph,
    /// Second status marker on the same node.
    DuplicateStatus,
    /// A `</status>` with no opening tag, or an unclosed wrapper.
    WrapperMismatch,
    /// Unit without a marker under `--exhaustive`.
    Unmarked,
    /// A marked paragraph/list item with no `##<ID>` fact anchor
    /// (the anchored-when-marked law, PROP-043 §3.8).
    MissingAnchor,
    /// A `@fact:<ID>` / legacy `##<ID>` / `{#anchor}` id minted twice in one
    /// document. The duplicate diagnostic names both definition lines.
    DuplicateId,
    /// An anchor naming an object type that does not bind: an unknown type,
    /// or a type with no matching block adjacent to it.
    ///
    /// This is an error rather than a silent skip on purpose. A grammar that
    /// ignores a type it does not implement promises what it cannot do — the
    /// author writes `@fact/image:` today and learns in a year that nothing
    /// ever read it.
    FenceBinding,
    /// A second fact anchor swallowed into the body of another.
    ///
    /// A fact anchor is the first token of its paragraph (`@fact:<ID>`,
    /// `@fact/<type>:<ID>`, or the legacy `##<ID>`). When two anchored facts
    /// sit on neighbouring lines with no blank line between them, Markdown
    /// folds them into a single paragraph: only the first keeps its address,
    /// and the second's anchor becomes body text of the first. Its marker
    /// still parses, its prose reads identically, and the gate stayed silent —
    /// yet no verdict can ever bind to the swallowed anchor again, because it
    /// no longer has an address. The space between the lines was load-bearing,
    /// and its absence cost nine days of orphaned verdicts before this check
    /// existed. Fix: insert a blank line before the swallowed anchor.
    SwallowedAnchor,
}

/// One fully parsed document.
///
/// Two `#[serde(skip)]` fields live inside — `Block::scan_text` and
/// `Fact::span` — and they are the marker scanner's scratch: written by
/// `parse`, read by `parse`, and by nothing downstream. Everything a
/// consumer reads is persisted, which is what lets the cache hand a
/// `ParsedDoc` back instead of re-parsing (PROP-043 §7.1). `PartialEq` is
/// derived so that equality can be *asserted* rather than argued about —
/// see `cache::tests::cached_doc_round_trips_the_parse`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedDoc {
    /// Repo-relative path, `/`-separated.
    pub path: String,
    pub content_hash: String,
    pub blocks: Vec<Block>,
    pub units: Vec<Unit>,
    pub markers: Vec<Marker>,
    pub issues: Vec<Issue>,
    /// `(block index, fact index)` of facts carrying no marker — the
    /// exhaustiveness counter of the fact grammar (PROP-043 §3.9).
    pub unmarked_facts: Vec<(usize, usize)>,
    /// Total countable facts (the exhaustiveness denominator).
    pub fact_count: usize,
}

impl ParsedDoc {
    /// The document-level marker, if any (granularity == Document).
    pub fn document_marker(&self) -> Option<&Marker> {
        self.markers
            .iter()
            .find(|m| m.granularity == crate::model::Granularity::Document)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }
}
