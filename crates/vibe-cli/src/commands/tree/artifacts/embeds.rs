//! Occurrence attribution within one committed static contribution.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION");

use super::StaticDecompileError;
use crate::commands::tree::model::EmbedSpan;

fn malformed(line: usize, reason: impl Into<String>) -> StaticDecompileError {
    StaticDecompileError::MalformedEmbed {
        line,
        reason: reason.into(),
    }
}

/// Decode only canonical reserved prefixes; other comment genres are text.
fn control(text: &str, line: usize) -> Result<Option<(bool, &str)>, StaticDecompileError> {
    let text = text.trim();
    let (closing, rest) = if let Some(rest) = text.strip_prefix("<!-- /embed:") {
        (true, rest)
    } else if let Some(rest) = text.strip_prefix("<!-- embed:") {
        (false, rest)
    } else {
        return Ok(None);
    };
    let Some(address) = rest.strip_suffix("-->") else {
        return Err(malformed(
            line,
            "reserved control must be a complete standalone comment",
        ));
    };
    if address.contains("-->") || address.contains("<!--") {
        return Err(malformed(
            line,
            "reserved control must be one standalone comment",
        ));
    }
    let address = address.trim();
    if address.is_empty() {
        return Err(malformed(
            line,
            "opening and closing addresses must be nonempty",
        ));
    }
    Ok(Some((closing, address)))
}

/// A stack tracks occurrences, not unique addresses. All completed spans are
/// returned in opening order only after the entire contribution is checked.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
pub(super) fn scan(
    body: &[&str],
    body_offset: usize,
    fenced: &[bool],
) -> Result<Vec<EmbedSpan>, StaticDecompileError> {
    let mut open: Vec<(&str, usize)> = Vec::new();
    let mut spans = Vec::new();
    for (index, text) in body.iter().enumerate() {
        let offset = body_offset + index;
        if fenced[offset] {
            continue;
        }
        let line = offset + 1;
        let Some((closing, address)) = control(text, line)? else {
            continue;
        };
        if !closing {
            open.push((address, line));
            continue;
        }
        let Some(&(expected, start)) = open.last() else {
            return Err(malformed(
                line,
                format!("unexpected closer `{address}` in this contribution"),
            ));
        };
        if address != expected {
            return Err(malformed(
                line,
                format!("closer `{address}` does not match `{expected}` opened at line {start}"),
            ));
        }
        open.pop();
        spans.push(EmbedSpan {
            address: expected.to_owned(),
            start_line: start as u64,
            end_line: line as u64,
        });
    }
    if let Some((address, line)) = open.first() {
        return Err(malformed(
            *line,
            format!("unclosed opener `{address}` before the contribution ended"),
        ));
    }
    spans.sort_by_key(|span| span.start_line);
    Ok(spans)
}
