//! Shared committed-static decompiler, extracted from the CLI summary reader.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#CAPTURED-GRAPH");
use super::{EmbedSpan, StaticContribution};
use specmark::spec;
use thiserror::Error;
use vibe_specdoc::{XmlCommentCodecError, decode_generated_xml_comment};

const COMMENT_OPEN: &str = "<!--";
const COMMENT_CLOSE: &str = "-->";
const MARKER_PREFIX: &str = "vibe:static ";
/// The `origin — path` separator: space, U+2014 em-dash, space
/// (`crates/vibe-workspace/src/boot_artifacts.rs`).
const MARKER_SEP: &str = " \u{2014} ";

/// Failures specific to decompiling the committed static-lane artifact.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile")]
pub enum StaticDecompileError {
    /// A complete generated comment used an invalid c1 wire spelling.
    #[error(
        "generated XML comment cannot be decoded: {source} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile; \
         fix: regenerate the committed static lane with the current vibe binary)"
    )]
    Codec {
        #[source]
        source: XmlCommentCodecError,
    },
    /// Canonical c1 decoded, but its reserved provenance payload was invalid.
    #[error(
        "malformed generated `vibe:static` provenance at line {line}: {reason}; \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile; \
         fix: regenerate it as `vibe:static <nonempty origin> — <nonempty path>`)"
    )]
    MalformedGeneratedProvenance { line: usize, reason: &'static str },
    /// Any unclosed comment makes the committed lane structurally ambiguous.
    #[error(
        "unterminated XML comment beginning at line {line}; \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile; \
         fix: close it with `-->` or regenerate the committed static lane)"
    )]
    UnterminatedXmlComment { line: usize },
}

impl From<XmlCommentCodecError> for StaticDecompileError {
    fn from(source: XmlCommentCodecError) -> Self {
        Self::Codec { source }
    }
}

/// A parsed `<!-- vibe:static {origin} — {path} -->` marker line.
struct Marker {
    origin: String,
    source_path: String,
    /// 0-based first and last line indices of the marker within the file.
    start_line: usize,
    end_line: usize,
}

fn parse_legacy_marker(payload: &str, start_line: usize, end_line: usize) -> Option<Marker> {
    let inner = payload.strip_prefix(MARKER_PREFIX)?;
    let (origin, path) = inner.split_once(MARKER_SEP)?;
    Some(Marker {
        origin: origin.trim().to_string(),
        source_path: path.trim().to_string(),
        start_line,
        end_line,
    })
}

fn parse_generated_marker(
    payload: &str,
    start_line: usize,
    end_line: usize,
) -> std::result::Result<Marker, StaticDecompileError> {
    let Some(inner) = payload.strip_prefix(MARKER_PREFIX) else {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "the reserved `vibe:static ` prefix is missing",
        });
    };
    let Some((origin, path)) = inner.split_once(MARKER_SEP) else {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "the exact ` space + U+2014 em dash + space ` separator is missing",
        });
    };
    let origin = origin.trim();
    let path = path.trim();
    if origin.is_empty() {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "origin is empty after trimming",
        });
    }
    if path.is_empty() {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "path is empty after trimming",
        });
    }
    Ok(Marker {
        origin: origin.to_string(),
        source_path: path.to_string(),
        start_line,
        end_line,
    })
}

/// Decode generated c1 comments while retaining legacy Markdown markers.
/// Reserved c1 comments are scanned as complete comments rather than lines:
/// the generated preamble and tombstone legitimately span several lines.
fn collect_markers(text: &str) -> std::result::Result<Vec<Marker>, StaticDecompileError> {
    let mut markers = Vec::new();
    let mut cursor = 0;
    let mut cursor_line = 0;

    while let Some(relative_start) = text[cursor..].find(COMMENT_OPEN) {
        let start = cursor + relative_start;
        let start_line = cursor_line + text[cursor..start].bytes().filter(|b| *b == b'\n').count();
        let comment_tail = &text[start..];
        let Some(relative_close) = comment_tail.find(COMMENT_CLOSE) else {
            return Err(StaticDecompileError::UnterminatedXmlComment {
                line: start_line + 1,
            });
        };
        let end = start + relative_close + COMMENT_CLOSE.len();
        let comment = &text[start..end];
        let end_line = start_line + comment.bytes().filter(|b| *b == b'\n').count();

        match decode_generated_xml_comment(comment)? {
            Some(payload) => {
                if payload.starts_with(MARKER_PREFIX) {
                    markers.push(parse_generated_marker(&payload, start_line, end_line)?);
                }
            }
            None => {
                let legacy = comment
                    .strip_prefix("<!-- ")
                    .and_then(|value| value.strip_suffix(" -->"));
                if let Some(marker) =
                    legacy.and_then(|payload| parse_legacy_marker(payload, start_line, end_line))
                {
                    markers.push(marker);
                }
            }
        }

        cursor_line = end_line;
        cursor = end;
    }
    Ok(markers)
}

/// Exact emitted body ranges from the same marker parser as the graph.
/// Unlike legacy summary sizes, preserves CRLF and a missing final newline.
pub(super) fn body_ranges(text: &str) -> Result<Vec<std::ops::Range<usize>>, StaticDecompileError> {
    let markers = collect_markers(text)?;
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    Ok(markers
        .iter()
        .enumerate()
        .map(|(i, marker)| {
            let start = starts
                .get(marker.end_line + 1)
                .copied()
                .unwrap_or(text.len());
            let end = markers
                .get(i + 1)
                .and_then(|next| starts.get(next.start_line))
                .copied()
                .unwrap_or(text.len());
            start..end
        })
        .collect())
}

/// Decompile static-lane text into its ordered contributions (PROP-036 §2.8).
///
/// Each marker opens a region running to the next marker or EOF; the region
/// yields the source `origin` (`group/name` or a host rel-path) and `path`.
/// A region's `bytes`/`lines` measure its body — the text after the marker
/// line up to (not including) the next marker. Nested
/// `<!-- embed: {addr} -->` … `<!-- /embed: {addr} -->` pairs become
/// [`EmbedSpan`]s with file-relative 1-based line numbers.
pub fn decompile_static(
    text: &str,
) -> std::result::Result<Vec<StaticContribution>, StaticDecompileError> {
    let lines: Vec<&str> = text.lines().collect();
    let markers = collect_markers(text)?;

    let mut out = Vec::with_capacity(markers.len());
    for (order, marker) in markers.iter().enumerate() {
        // Region body: from the line after the marker to the line before the
        // next marker (or EOF).
        let body_start = marker.end_line + 1;
        let body_end = markers
            .get(order + 1)
            .map(|m| m.start_line)
            .unwrap_or(lines.len());
        let body = &lines[body_start.min(lines.len())..body_end.min(lines.len())];
        let bytes: u64 = body.iter().map(|l| l.len() as u64 + 1).sum();
        let embeds = scan_embeds(body, body_start);
        out.push(StaticContribution {
            order: order as u64,
            origin: marker.origin.clone(),
            source_path: marker.source_path.clone(),
            bytes,
            lines: body.len() as u64,
            embeds,
        });
    }
    Ok(out)
}

/// Attribute `<!-- embed: {addr} -->` … `<!-- /embed: {addr} -->` pairs
/// inside a region body. `body_offset` is the 0-based file line index of the
/// region body's first line; emitted spans are 1-based file lines.
fn scan_embeds(body: &[&str], body_offset: usize) -> Vec<EmbedSpan> {
    let mut spans = Vec::new();
    let mut open: Vec<(String, usize)> = Vec::new();
    for (i, line) in body.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("<!-- /embed:") {
            let addr = rest.trim_end_matches("-->").trim().to_string();
            if open
                .last()
                .is_some_and(|(a, _)| addr.is_empty() || *a == addr)
            {
                let (open_addr, start) = open.pop().expect("checked stack");
                spans.push(EmbedSpan {
                    address: if addr.is_empty() { open_addr } else { addr },
                    start_line: (body_offset + start + 1) as u64,
                    end_line: (body_offset + i + 1) as u64,
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("<!-- embed:") {
            let addr = rest.trim_end_matches("-->").trim().to_string();
            open.push((addr, i));
        }
    }
    spans
}

/// Capture the shared graph from these exact caller-supplied artifact bytes.
pub fn capture_static(
    capture_id: String,
    text: &str,
) -> Result<super::SummaryCapture, StaticDecompileError> {
    let rows = decompile_static(text)?;
    Ok(super::from_static_summary(capture_id, &rows))
}
