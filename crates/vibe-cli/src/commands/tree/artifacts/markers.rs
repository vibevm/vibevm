//! Decode static provenance comments with canonical fences and line framing.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION");

use super::{MARKER_SEP, StaticDecompileError};
use std::collections::BTreeMap;
use vibe_specdoc::decode_generated_xml_comment;

const COMMENT_OPEN: &str = "<!--";
const COMMENT_CLOSE: &str = "-->";
const MARKER_PREFIX: &str = "vibe:static ";

/// A parsed `<!-- vibe:static {origin} — {path} -->` marker line.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile"
)]
pub(super) struct Marker {
    pub origin: String,
    pub source_path: String,
    /// 0-based first and last line indices of the marker within the file.
    pub start_line: usize,
    pub end_line: usize,
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
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
pub(super) fn collect(
    text: &str,
    fenced: &[bool],
) -> std::result::Result<Vec<Marker>, StaticDecompileError> {
    let xml = text
        .lines()
        .enumerate()
        .any(|(index, line)| !fenced[index] && line.starts_with("<?xml "));
    let xml_comments = if xml {
        let mut comments = BTreeMap::new();
        for span in vibe_specdoc::xml_comment_spans(text) {
            let span = span.map_err(|error| {
                let line = text[..error.offset.min(text.len())]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    + 1;
                if error.unclosed_comment {
                    StaticDecompileError::UnterminatedXmlComment { line }
                } else {
                    StaticDecompileError::MalformedEmbed {
                        line,
                        reason: error.to_string(),
                    }
                }
            })?;
            comments.insert(span.start, span);
        }
        Some(comments)
    } else {
        None
    };
    let mut markers = Vec::new();
    let mut cursor = 0;
    let mut cursor_line = 0;
    let mut cursor_line_start = 0;

    while let Some(relative_start) = text[cursor..].find(COMMENT_OPEN) {
        let start = cursor + relative_start;
        let start_line = cursor_line + text[cursor..start].bytes().filter(|b| *b == b'\n').count();
        let line_start = text[cursor..start]
            .rfind('\n')
            .map_or(cursor_line_start, |index| cursor + index + 1);
        if xml_comments
            .as_ref()
            .is_some_and(|comments| comments.get(&start).is_none_or(|span| span.depth != 0))
            || (!xml && fenced[start_line])
        {
            // A literal example cannot consume a real comment after its fence,
            // even when the example itself has no XML closing delimiter.
            cursor = start + COMMENT_OPEN.len();
            cursor_line = start_line;
            cursor_line_start = line_start;
            continue;
        }
        let comment_tail = &text[start..];
        let Some(relative_close) = comment_tail.find(COMMENT_CLOSE) else {
            return Err(StaticDecompileError::UnterminatedXmlComment {
                line: start_line + 1,
            });
        };
        let end = start + relative_close + COMMENT_CLOSE.len();
        let comment = &text[start..end];
        let end_line = start_line + comment.bytes().filter(|b| *b == b'\n').count();

        let candidate = match decode_generated_xml_comment(comment)? {
            Some(payload) if payload.starts_with(MARKER_PREFIX) => {
                // An inline/prose example is not a provenance control. Keep
                // codec validation distinct from recognizing that control.
                if text[line_start..start].trim().is_empty() {
                    Some(parse_generated_marker(&payload, start_line, end_line)?)
                } else {
                    None
                }
            }
            Some(_) => None,
            None => {
                let legacy = comment
                    .strip_prefix("<!-- ")
                    .and_then(|value| value.strip_suffix(" -->"));
                legacy
                    .and_then(|payload| parse_legacy_marker(payload, start_line, end_line))
                    .filter(|_| text[line_start..start].trim().is_empty())
            }
        };
        if let Some(marker) = candidate {
            let suffix = text[end..].split('\n').next().unwrap_or("");
            if !suffix.trim().is_empty() {
                return Err(StaticDecompileError::MalformedGeneratedProvenance {
                    line: end_line + 1,
                    reason: "static provenance markers must occupy separate logical lines with only surrounding whitespace",
                });
            }
            markers.push(marker);
        }
        cursor_line_start = comment
            .rfind('\n')
            .map_or(line_start, |index| start + index + 1);

        cursor_line = end_line;
        cursor = end;
    }
    Ok(markers)
}
