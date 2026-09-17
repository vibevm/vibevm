//! Bounded, terminal-safe redaction for external progress text.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#VERBOSE");

pub(super) const MAX_MESSAGE_CHARS: usize = 480;

/// Reduce external text to one bounded terminal-safe line, then redact the
/// credential-bearing shapes progress diagnostics commonly receive.
pub(super) fn sanitize(input: &str) -> String {
    let flattened: String = input
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let urls = flattened
        .split_whitespace()
        .map(redact_url)
        .collect::<Vec<_>>()
        .join(" ");
    let redacted = redact_assignments(urls);
    let mut bounded: String = redacted.chars().take(MAX_MESSAGE_CHARS).collect();
    if redacted.chars().count() > MAX_MESSAGE_CHARS {
        bounded.push('…');
    }
    bounded
}

/// The one redaction boundary for child-process lines that may later appear
/// in progress, terminal errors, or JSON error envelopes.
pub(crate) fn sanitize_progress_text(input: &str) -> String {
    sanitize(input)
}

fn redact_url(token: &str) -> String {
    let Some(scheme_end) = token.find("://") else {
        return token.to_string();
    };
    let authority_start = scheme_end + 3;
    let authority_end = token[authority_start..]
        .find(['/', '?', '#'])
        .map(|index| authority_start + index)
        .unwrap_or(token.len());
    let authority = &token[authority_start..authority_end];
    let safe_authority = authority
        .rfind('@')
        .map(|at| format!("[redacted]@{}", &authority[at + 1..]))
        .unwrap_or_else(|| authority.to_string());
    let mut rendered = format!("{}{}", &token[..authority_start], safe_authority);
    let tail = &token[authority_end..];
    if let Some(query) = tail.find('?') {
        rendered.push_str(&tail[..=query]);
        rendered.push_str("[redacted]");
    } else if let Some(fragment) = tail.find('#') {
        rendered.push_str(&tail[..=fragment]);
        rendered.push_str("[redacted]");
    } else {
        rendered.push_str(tail);
    }
    rendered
}

fn redact_assignments(mut text: String) -> String {
    for key in [
        "authorization",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "token",
        "secret",
    ] {
        let mut search_from = 0;
        loop {
            let lower = text.to_ascii_lowercase();
            let Some(relative) = lower[search_from..].find(key) else {
                break;
            };
            let start = search_from + relative;
            let before_ok = start == 0 || !lower.as_bytes()[start - 1].is_ascii_alphanumeric();
            let mut after_key = start + key.len();
            if after_key < text.len() && matches!(text.as_bytes()[after_key], b'\'' | b'"') {
                after_key += 1;
            }
            let mut separator = after_key;
            while separator < text.len() && text.as_bytes()[separator].is_ascii_whitespace() {
                separator += 1;
            }
            let cli_value = text.as_bytes().get(start.saturating_sub(2)..start) == Some(b"--")
                && separator > after_key;
            let assignment =
                separator < text.len() && matches!(text.as_bytes()[separator], b'=' | b':');
            if !before_ok || (!assignment && !cli_value) {
                search_from = start + key.len();
                continue;
            }
            let mut value_start = if assignment { separator + 1 } else { separator };
            while value_start < text.len() && text.as_bytes()[value_start].is_ascii_whitespace() {
                value_start += 1;
            }
            let quote = text
                .as_bytes()
                .get(value_start)
                .copied()
                .filter(|byte| matches!(byte, b'\'' | b'"'));
            if quote.is_some() {
                value_start += 1;
            }
            let value_end = if key == "authorization" {
                text.len()
            } else if let Some(quote) = quote {
                text[value_start..]
                    .bytes()
                    .position(|byte| byte == quote)
                    .map(|offset| value_start + offset)
                    .unwrap_or(text.len())
            } else {
                text[value_start..]
                    .find(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '}'))
                    .map(|offset| value_start + offset)
                    .unwrap_or(text.len())
            };
            text.replace_range(value_start..value_end, "[redacted]");
            search_from = value_start + "[redacted]".len();
        }
    }
    text
}
