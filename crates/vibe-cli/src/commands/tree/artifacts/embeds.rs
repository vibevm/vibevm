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
    let mut events = Vec::new();
    let xml = body.join("\n");
    if xml.trim_start().starts_with("<?xml ") {
        for span in vibe_specdoc::xml_comment_spans(&xml) {
            let span = span.map_err(|error| {
                malformed(
                    body_offset
                        + xml[..error.offset.min(xml.len())]
                            .bytes()
                            .filter(|byte| *byte == b'\n')
                            .count()
                        + 1,
                    error.to_string(),
                )
            })?;
            let raw = &xml[span.start..span.end];
            let line = body_offset
                + xml[..span.start]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                + 1;
            let end_line = line + raw.bytes().filter(|byte| *byte == b'\n').count();
            let payload = vibe_specdoc::decode_generated_xml_comment(raw)
                .map_err(|error| malformed(line, error.to_string()))?;
            if let Some(payload) = payload {
                let (closing, address) = if let Some(address) = payload.strip_prefix("/embed:") {
                    (true, address)
                } else if let Some(address) = payload.strip_prefix("embed:") {
                    (false, address)
                } else {
                    continue;
                };
                let address = address.trim();
                if address.is_empty() {
                    return Err(malformed(line, "generated embed address is empty"));
                }
                events.push((line, end_line, closing, address.to_owned()));
            } else {
                // Legacy raw framing remains standalone; XML literal text and
                // CDATA never enter this actual-comment branch.
                let prefix = xml[..span.start].rsplit('\n').next().unwrap_or("");
                let suffix = xml[span.end..].split('\n').next().unwrap_or("");
                if prefix.trim().is_empty()
                    && suffix.trim().is_empty()
                    && let Some((closing, address)) = control(raw, line)?
                {
                    events.push((line, end_line, closing, address.to_owned()));
                }
            }
        }
    } else {
        for (index, text) in body.iter().enumerate() {
            let offset = body_offset + index;
            if fenced[offset] {
                continue;
            }
            let line = offset + 1;
            if let Some((closing, address)) = control(text, line)? {
                events.push((line, line, closing, address.to_owned()));
            }
        }
    }
    // Stable order preserves several actual comments on one paragraph line.
    events.sort_by_key(|event| event.0);
    let mut open: Vec<(String, usize, usize)> = Vec::new();
    let mut spans = Vec::new();
    let mut ordinal = 0;
    for (line, end_line, closing, address) in events {
        if !closing {
            open.push((address, line, ordinal));
            ordinal += 1;
            continue;
        }
        let Some((expected, start, order)) = open.last() else {
            return Err(malformed(
                line,
                format!("unexpected closer `{address}` in this contribution"),
            ));
        };
        if &address != expected {
            return Err(malformed(
                line,
                format!("closer `{address}` does not match `{expected}` opened at line {start}"),
            ));
        }
        let span = (
            *order,
            EmbedSpan {
                address: expected.clone(),
                start_line: *start as u64,
                end_line: end_line as u64,
            },
        );
        open.pop();
        spans.push(span);
    }
    if let Some((address, line, _)) = open.first() {
        return Err(malformed(
            *line,
            format!("unclosed opener `{address}` before the contribution ended"),
        ));
    }
    spans.sort_by_key(|(order, _)| *order);
    Ok(spans.into_iter().map(|(_, span)| span).collect())
}
