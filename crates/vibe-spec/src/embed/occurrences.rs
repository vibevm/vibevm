//! Generated occurrences, minted by recursive expansion rather than marker scanning.

use sha2::{Digest, Sha256};

use crate::SpecAddress;

/// One recursive splice in a single expanded document. Identity is its directive
/// ordinal path, independent of the later emitted position or repeated target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbedOccurrence {
    pub(crate) path: Vec<usize>,
    pub(crate) target: SpecAddress,
    /// Identity of the effective input view of this recursion call. A normal
    /// compiler root is the merged node, after resolved directives are removed;
    /// nested calls name the resolved target view. This is not a raw file path.
    pub(crate) source: String,
    /// Zero-based line in that effective input view before recursive splicing,
    /// not an authored-file coordinate or a later emitted output position.
    pub(crate) directive_line: usize,
    /// Half-open line gaps in the associated expanded body.
    pub(crate) start: usize,
    pub(crate) end: usize,
    /// An empty successful source is not a body removed by normal coverage.
    pub(crate) intrinsic_empty: bool,
    pub(crate) suppressed: bool,
    /// Actual containing effective-view region before expansion; never a guessed
    /// adjacent heading. Its bounds are mapped separately from the payload.
    pub(crate) owner: EmbedRegion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EmbedRegionKind {
    Preamble,
    Unit(String),
    Unanchored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbedRegion {
    pub(crate) kind: EmbedRegionKind,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Exact body association and ordered generated provenance for that body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbedTape {
    pub(crate) body_digest: [u8; 32],
    pub(crate) occurrences: Vec<EmbedOccurrence>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum EmbedTapeError {
    #[error("embed annotations do not match their associated body")]
    Body,
    #[error("embed occurrence {index} has an invalid range or identity")]
    Occurrence { index: usize },
    #[error("embed occurrence {index} has a missing or crossed parent")]
    Parent { index: usize },
    #[error("embed body rewrite changed line boundaries without a mapping")]
    LineMap,
}

impl EmbedTape {
    pub(crate) fn meaning_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"vibe-spec/embed-tape/1");
        hash.update(self.body_digest);
        hash.update((self.occurrences.len() as u64).to_le_bytes());
        for row in &self.occurrences {
            hash.update((row.path.len() as u64).to_le_bytes());
            for part in &row.path {
                hash.update((*part as u64).to_le_bytes());
            }
            for value in [row.target.to_string(), row.source.clone()] {
                hash.update((value.len() as u64).to_le_bytes());
                hash.update(value.as_bytes());
            }
            for n in [
                row.directive_line,
                row.start,
                row.end,
                row.owner.start,
                row.owner.end,
            ] {
                hash.update((n as u64).to_le_bytes());
            }
            hash.update([u8::from(row.intrinsic_empty), u8::from(row.suppressed)]);
            match &row.owner.kind {
                EmbedRegionKind::Preamble => hash.update([0]),
                EmbedRegionKind::Unanchored => hash.update([1]),
                EmbedRegionKind::Unit(anchor) => {
                    hash.update([2]);
                    hash.update((anchor.len() as u64).to_le_bytes());
                    hash.update(anchor.as_bytes());
                }
            }
        }
        hash.finalize().into()
    }
    /// Stable generated meaning within a source interval. Recursion identities
    /// and absolute placement are deliberately absent from once equivalence.
    pub(crate) fn coverage_digest(
        &self,
        span: std::ops::Range<usize>,
        include_preamble: bool,
    ) -> [u8; 32] {
        let mut digest = Sha256::new();
        for row in &self.occurrences {
            if row.suppressed {
                continue;
            }
            if !include_preamble
                && row.start == row.end
                && row.owner.start == row.owner.end
                && row.start == span.start
                && matches!(row.owner.kind, EmbedRegionKind::Preamble)
            {
                continue;
            }
            let overlaps = if row.start == row.end {
                (span.start < row.start && row.start <= span.end)
                    || (span.start == 0 && row.start == 0)
            } else {
                row.start < span.end && span.start < row.end
            };
            if !overlaps {
                continue;
            }
            let address = row.target.without_pin();
            digest.update((address.len() as u64).to_le_bytes());
            digest.update(address.as_bytes());
            digest.update((row.start.saturating_sub(span.start) as u64).to_le_bytes());
            digest.update((row.end.min(span.end).saturating_sub(span.start) as u64).to_le_bytes());
            digest.update([u8::from(row.intrinsic_empty)]);
        }
        digest.finalize().into()
    }
    /// Boundary order comes from the typed recursion tree, including coincident
    /// empty and nested splices. It is not recovered from rendered comments.
    pub(crate) fn boundaries(&self) -> Vec<(usize, bool, &EmbedOccurrence)> {
        let mut events = Vec::new();
        let mut stack: Vec<&EmbedOccurrence> = Vec::new();
        for row in &self.occurrences {
            if row.suppressed {
                continue;
            }
            while stack
                .last()
                .is_some_and(|parent| !row.path.starts_with(&parent.path))
            {
                if let Some(parent) = stack.pop() {
                    events.push((parent.end, false, parent));
                }
            }
            events.push((row.start, true, row));
            stack.push(row);
        }
        while let Some(row) = stack.pop() {
            events.push((row.end, false, row));
        }
        events
    }
    pub(crate) fn new(body: &str, occurrences: Vec<EmbedOccurrence>) -> Self {
        Self {
            body_digest: Sha256::digest(body.as_bytes()).into(),
            occurrences,
        }
    }

    pub(crate) fn validate(&self, body: &str) -> Result<(), EmbedTapeError> {
        if self.body_digest != <[u8; 32]>::from(Sha256::digest(body.as_bytes())) {
            return Err(EmbedTapeError::Body);
        }
        let count = body.lines().count();
        for (index, occurrence) in self.occurrences.iter().enumerate() {
            if occurrence.path.is_empty()
                || occurrence.source.is_empty()
                || occurrence.start > occurrence.end
                || occurrence.end > count
                || occurrence.owner.start > occurrence.owner.end
                || occurrence.owner.end > count
                || (occurrence.intrinsic_empty && occurrence.start != occurrence.end)
                || (occurrence.suppressed && occurrence.start != occurrence.end)
                || self.occurrences[..index]
                    .iter()
                    .any(|old| old.path >= occurrence.path)
            {
                return Err(EmbedTapeError::Occurrence { index });
            }
            if occurrence.path.len() > 1 {
                let parent_path = &occurrence.path[..occurrence.path.len() - 1];
                let parent = self.occurrences[..index]
                    .iter()
                    .find(|old| old.path == parent_path);
                if !parent.is_some_and(|p| {
                    (occurrence.suppressed
                        || (p.start <= occurrence.start && occurrence.end <= p.end))
                        && (!p.suppressed || occurrence.suppressed)
                }) {
                    return Err(EmbedTapeError::Parent { index });
                }
            }
            if self.occurrences[..index].iter().any(|old| {
                !old.suppressed
                    && !occurrence.suppressed
                    && !occurrence.path.starts_with(&old.path)
                    && old.end > occurrence.start
            }) {
                return Err(EmbedTapeError::Parent { index });
            }
        }
        Ok(())
    }

    /// Built-in qualify/link rewrites preserve line gaps, but may change bytes.
    pub(crate) fn rewrite_lines(&self, before: &str, after: &str) -> Result<Self, EmbedTapeError> {
        self.validate(before)?;
        if before.lines().count() != after.lines().count() {
            return Err(EmbedTapeError::LineMap);
        }
        Ok(Self::new(after, self.occurrences.clone()))
    }

    /// DocTree's existing canonical join removes the final line terminator.
    /// Map that exact operation, including the last all-layout empty line.
    pub(crate) fn tree_join(&self, before: &str, after: &str) -> Result<Self, EmbedTapeError> {
        self.validate(before)?;
        if before.lines().collect::<Vec<_>>().join("\n") != after {
            return Err(EmbedTapeError::LineMap);
        }
        let count = after.lines().count();
        let mut occurrences = self.occurrences.clone();
        for row in &mut occurrences {
            row.start = row.start.min(count);
            row.end = row.end.min(count);
            row.owner.start = row.owner.start.min(count);
            row.owner.end = row.owner.end.min(count);
        }
        Ok(Self::new(after, occurrences))
    }

    /// Project through an explicit retained-line mask. No address or prose scan
    /// can recreate an occurrence removed by coverage.
    pub(crate) fn retain_lines(
        &self,
        before: &str,
        after: &str,
        retained: &[bool],
        retained_empty: &std::collections::BTreeSet<Vec<usize>>,
    ) -> Result<Self, EmbedTapeError> {
        self.validate(before)?;
        if retained.len() != before.lines().count() {
            return Err(EmbedTapeError::LineMap);
        }
        let expected: String = before
            .split_inclusive('\n')
            .zip(retained)
            .filter(|(_, keep)| **keep)
            .map(|(s, _)| s)
            .collect();
        if expected != after {
            return Err(EmbedTapeError::LineMap);
        }
        let mut gaps = vec![0];
        for keep in retained {
            gaps.push(gaps.last().copied().unwrap_or(0) + usize::from(*keep));
        }
        let mut occurrences: Vec<EmbedOccurrence> = Vec::new();
        for row in &self.occurrences {
            let alive = !row.suppressed
                && if row.start == row.end {
                    retained_empty.contains(&row.path)
                } else {
                    retained[row.start..row.end].iter().any(|keep| *keep)
                };
            let parent_suppressed = row.path.len() > 1
                && occurrences
                    .iter()
                    .any(|p| p.path == row.path[..row.path.len() - 1] && p.suppressed);
            let mut row = row.clone();
            row.start = gaps[row.start];
            row.end = gaps[row.end];
            row.suppressed = !alive || parent_suppressed;
            if row.suppressed {
                row.end = row.start;
            }
            row.owner.start = gaps[row.owner.start];
            row.owner.end = gaps[row.owner.end];
            occurrences.push(row);
        }
        let result = Self::new(after, occurrences);
        result.validate(after)?;
        Ok(result)
    }
}

pub(crate) struct ExpandedBody {
    pub(crate) text: String,
    pub(crate) tape: EmbedTape,
}

impl ExpandedBody {
    /// Historical public string API spelling, driven by the same typed events.
    pub(crate) fn legacy_text(&self) -> String {
        let events = self.tape.boundaries();
        let mut event = 0;
        let mut output = String::new();
        let lines: Vec<_> = self.text.split_inclusive('\n').collect();
        for gap in 0..=lines.len() {
            while let Some((at, opening, row)) = events.get(event) {
                if *at != gap {
                    break;
                }
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                if !opening && row.intrinsic_empty {
                    output.push('\n');
                }
                output.push_str(if *opening {
                    "<!-- embed: "
                } else {
                    "<!-- /embed: "
                });
                output.push_str(&row.target.without_pin());
                output.push_str(" -->\n");
                event += 1;
            }
            if let Some(line) = lines.get(gap) {
                output.push_str(line);
            }
        }
        output
    }
}

pub(crate) fn markdown_body(
    body: &str,
    tape: Option<&EmbedTape>,
    fence: crate::doctree::FenceSnapshot,
) -> String {
    let Some(tape) = tape else {
        return body.to_owned();
    };
    let literal = protect_literals(body, &mut [], fence, false);
    ExpandedBody {
        text: literal,
        tape: tape.clone(),
    }
    .legacy_text()
}

/// Protect projected literal controls before actual XML events are inserted.
/// Positions are checked byte gaps in the supplied plain projection.
pub(crate) fn protect_projection_literals(body: &str, positions: &mut [usize]) -> String {
    protect_literals(body, positions, crate::doctree::FenceSnapshot::Closed, true)
}
fn protect_literals(
    body: &str,
    positions: &mut [usize],
    fence: crate::doctree::FenceSnapshot,
    projection: bool,
) -> String {
    let mut tracker = crate::doctree::FenceTracker::from_snapshot(fence);
    let mut literal = String::new();
    let mut source_offset = 0;
    let mut escaped = Vec::new();
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let fenced = tracker.classify(line.trim_end_matches('\n'));
        if !fenced
            && (["<!-- embed:", "<!-- /embed:", "<!-- vibe:c"]
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
                || (projection
                    && crate::compiler::emit::framing::raw_contribution_comment_prefix(trimmed)))
        {
            let indent = line.len() - trimmed.len();
            escaped.push(source_offset + indent);
            literal.push_str(&line[..indent]);
            literal.push_str("&lt;");
            literal.push_str(&trimmed[1..]);
        } else {
            literal.push_str(line);
        }
        source_offset += line.len();
    }
    for position in positions {
        *position += 3 * escaped.partition_point(|escaped| *escaped < *position);
    }
    literal
}
