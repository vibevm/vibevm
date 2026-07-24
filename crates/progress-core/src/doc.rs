//! The parsed-document model: blocks, units, markers, issues.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#placement");

use crate::model::Marker;
use serde::{Deserialize, Serialize};

/// A contiguous run of non-blank lines (outside fences), or one fenced
/// code block. The paragraph of PROP-043 §3.8/§3.9 is a `Text` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub kind: BlockKind,
    /// 1-based inclusive line range in the source file.
    pub line_start: usize,
    pub line_end: usize,
    /// Block text with inline-code spans blanked (marker scanning input).
    #[serde(skip)]
    pub scan_text: String,
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
    /// Marker mid-paragraph (only first/last token positions are legal).
    MidParagraph,
    /// Second status marker on the same node.
    DuplicateStatus,
    /// A `</status>` with no opening tag, or an unclosed wrapper.
    WrapperMismatch,
    /// Paragraph without a marker under `--exhaustive`.
    Unmarked,
}

/// One fully parsed document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedDoc {
    /// Repo-relative path, `/`-separated.
    pub path: String,
    pub content_hash: String,
    pub blocks: Vec<Block>,
    pub units: Vec<Unit>,
    pub markers: Vec<Marker>,
    pub issues: Vec<Issue>,
    /// Indices into `blocks` of Text blocks carrying no paragraph-level
    /// marker — the exhaustiveness counter (PROP-043 §3.9).
    pub unmarked_paragraphs: Vec<usize>,
    /// Total Text blocks (the exhaustiveness denominator).
    pub paragraph_count: usize,
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
