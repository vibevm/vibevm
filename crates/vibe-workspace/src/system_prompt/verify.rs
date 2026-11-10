//! Compose-time verification, and the import-inertness report.
//!
//! Two different things live here because they are two different verdicts on
//! the same bytes, and conflating them would be the mistake.
//!
//! **Verification refuses.** `#SYS-NO-RECURSION` closes the loop in both
//! directions: agent mode never delivers `SYSTEM.md`, and the prefix's
//! entrypoint segment replaces the managed block with fixed text carrying no
//! `@` import line and no router — so even a mistaken import of the prefix
//! cannot recurse. The composer proves its half rather than asserting it:
//! the whole composed prefix carries no exactly-standalone `<vibevm>` /
//! `</vibevm>` line and no lane marker line (which together are the
//! mechanical form of "no router"), and the composer's **own generated
//! text** — the frame lines and the transport contract — carries no
//! `@`-initial line.
//!
//! The `@` half is scoped to the generated text on purpose, and the scope is
//! the honest one. The compiled lane of this repository carries 52
//! line-initial `@spec://` tokens, and `#SYS-LANE-VERBATIM` forbids
//! rewriting them; `#SYS-TEMPLATE-LITERAL` forbids treating a character in
//! the user's own template as a directive. Refusing them would make the
//! feature unavailable to the project that specified it, and editing them
//! would break the one differential proof the campaign has. So the composer
//! refuses what it wrote itself and **reports** what it carries.
//!
//! **Reporting does not refuse.** `#SYS-IMPORT-INERTNESS` is the reason the
//! `import` delivery tier stays refused in version 1: Gemini CLI and Qwen
//! Code process `@path` recursively inside imported content, so an imported
//! lane could make a harness try to resolve a `spec://` address as a file
//! path. The test is per harness and per flavour, and it is a token census —
//! count and locations, with each site classified by whether it sits inside
//! a code fence, since a fenced token is inert. `vibe prompt check` prints
//! it; the decision to refuse `import` is P5's, made from this evidence.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-NO-RECURSION");

use std::ops::Range;

use crate::WorkspaceError;
use crate::boot_artifacts::{BLOCK_CLOSE, BLOCK_OPEN};

use super::compose_error;
use super::markers::parse_marker_line;

/// One `@`-initial line in a composed artifact — a line a harness that
/// processes imports might act on.
///
/// A site is a **token**, not a line, and that is the faithful unit: a
/// harness that resolves `@path` references scans the text for them and does
/// not require one at the start of a line. Counting lines would have
/// understated this repository's lane by sixteen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSite {
    /// Which segment's body carries it.
    pub segment: String,
    /// The 1-based line number inside that segment's body.
    pub line: usize,
    /// The 0-based byte offset of the `@` inside that line.
    pub column: usize,
    /// `true` when the line sits inside a fenced code block, where a harness
    /// that respects fences leaves it alone. Only an **unfenced** token can
    /// keep an import flavour from being chosen.
    pub fenced: bool,
    /// `true` when the token starts the line, which is the shape a
    /// line-oriented import directive takes.
    pub line_initial: bool,
    /// The token a harness would try to resolve, `@` included.
    pub token: String,
}

/// The import-inertness census of one composed artifact.
///
/// ```
/// use vibe_workspace::system_prompt::ImportInertness;
///
/// let census = ImportInertness::of("project.lane", "see @spec://a/b#c for it\n");
/// assert_eq!(census.len(), 1);
/// assert_eq!(census.unfenced(), 1);
/// assert!(!census.is_inert(), "an unfenced @ token keeps `import` refused");
/// assert_eq!(census.sites[0].token, "@spec://a/b#c");
/// assert!(!census.sites[0].line_initial, "mid-line counts too");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportInertness {
    /// Every site found, in artifact order.
    pub sites: Vec<ImportSite>,
}

/// The characters a reference token may carry after its `@`. Deliberately
/// the union of what a filesystem path and a `spec://` address need, so a
/// census does not have to know which kind it found.
fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"._/:#+-~".contains(&byte)
}

impl ImportInertness {
    /// Census one segment body.
    pub fn of(segment: &str, body: &str) -> Self {
        let mut census = Self::default();
        census.add(segment, body);
        census
    }

    /// Add one segment body's sites to the census.
    pub fn add(&mut self, segment: &str, body: &str) {
        let mut fenced = false;
        for (index, line) in body.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                fenced = !fenced;
                continue;
            }
            let bytes = line.as_bytes();
            for (column, _) in line.char_indices().filter(|(_, c)| *c == '@') {
                // A token's `@` opens it: a preceding token byte makes this
                // something else — an e-mail-shaped word or a continuation —
                // and a non-token byte after it makes the `@` bare prose.
                if column > 0 && is_token_byte(bytes[column - 1]) {
                    continue;
                }
                let tail: String = line[column + 1..]
                    .bytes()
                    .take_while(|byte| is_token_byte(*byte))
                    .map(char::from)
                    .collect();
                if tail.is_empty() {
                    continue;
                }
                self.sites.push(ImportSite {
                    segment: segment.to_owned(),
                    line: index + 1,
                    column,
                    fenced,
                    line_initial: line[..column].trim().is_empty(),
                    token: format!("@{tail}"),
                });
            }
        }
    }

    /// How many tokens there are in all.
    pub fn len(&self) -> usize {
        self.sites.len()
    }

    /// `true` when there is no site at all.
    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// How many tokens sit outside a code fence — the ones that decide
    /// whether an import flavour may be chosen at all.
    pub fn unfenced(&self) -> usize {
        self.sites.iter().filter(|site| !site.fenced).count()
    }

    /// How many tokens start their line, the shape a line-oriented import
    /// directive takes.
    pub fn line_initial(&self) -> usize {
        self.sites.iter().filter(|site| site.line_initial).count()
    }

    /// `true` when the artifact is import-inert: no reference token outside
    /// a code fence anywhere in it.
    pub fn is_inert(&self) -> bool {
        self.unfenced() == 0
    }

    /// The first `count` sites, for a report that names a few locations
    /// rather than three hundred.
    pub fn first(&self, count: usize) -> &[ImportSite] {
        &self.sites[..self.sites.len().min(count)]
    }
}

/// Refuse a marker-shaped line inside a segment's own bytes.
///
/// A marker-shaped line in a body would make the agent lane's presence,
/// truncation and duplication rules read a segment boundary that is not
/// one — so it is a refusal naming the segment and the line, in both
/// artifacts (`#SYS-LANE-MARKERS`).
pub fn refuse_marker_shaped_lines(segment: &str, body: &str) -> Result<(), WorkspaceError> {
    for (index, line) in body.lines().enumerate() {
        if parse_marker_line(line).is_some() {
            return Err(compose_error(
                "segment body",
                format!(
                    "segment `{segment}` carries a lane-marker-shaped line at its line {}, \
                     which would be read as a segment boundary",
                    index + 1
                ),
            ));
        }
    }
    Ok(())
}

/// Prove the composed prefix cannot recurse (`#SYS-NO-RECURSION`).
///
/// `generated` names the byte ranges the composer itself wrote — the frame
/// lines and the transport contract. Over the whole text: no standalone
/// `<vibevm>` / `</vibevm>` line and no lane marker line, which together are
/// the mechanical form of "the managed block was replaced and no router
/// survived". Over the generated ranges only: no `@`-initial line, so the
/// one piece of prose the composer authored can never be an import
/// directive.
pub fn verify_no_recursion(text: &str, generated: &[Range<usize>]) -> Result<(), WorkspaceError> {
    refuse_managed_block_markers(text)?;
    let mut offset = 0usize;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let line_no = index + 1;
        let trimmed = line.strip_suffix('\n').unwrap_or(line).trim();
        if parse_marker_line(line).is_some() {
            return Err(recursion_defect(
                line_no,
                "a lane segment marker line reached the prefix — markers belong to the \
                 agent lane alone",
            ));
        }
        if trimmed.starts_with('@')
            && generated
                .iter()
                .any(|range| range.start <= offset && offset + line.len() <= range.end)
        {
            return Err(recursion_defect(
                line_no,
                "the composer's own generated text carries an `@` import line",
            ));
        }
        offset += line.len();
    }
    Ok(())
}

/// Refuse an exactly-standalone `<vibevm>` / `</vibevm>` line.
///
/// This is the mechanical form of "no router" in both artifacts: the router
/// of `#SYS-NO-RECURSION` lives inside that managed block, the prefix
/// replaces the block with fixed text, and the agent lane never carries one
/// at all. `locate_block` matches exactly a standalone marker line, so the
/// same test the writer uses is the test the composer applies.
pub fn refuse_managed_block_markers(text: &str) -> Result<(), WorkspaceError> {
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let trimmed = line.strip_suffix('\n').unwrap_or(line).trim();
        if trimmed == BLOCK_OPEN || trimmed == BLOCK_CLOSE {
            return Err(recursion_defect(
                index + 1,
                &format!(
                    "a `{trimmed}` managed-block marker survived composition — a composed \
                     artifact carries no managed block, so no router can enter it"
                ),
            ));
        }
    }
    Ok(())
}

fn recursion_defect(line_no: usize, reason: &str) -> WorkspaceError {
    compose_error("prefix bytes", format!("line {line_no}: {reason}"))
}

#[cfg(test)]
#[path = "verify/tests.rs"]
mod tests;
