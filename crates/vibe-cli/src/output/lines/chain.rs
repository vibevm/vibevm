//! The shapes of an error chain the refusal reader takes apart (PROP-067
//! `#refusal`): parentheses matched by depth, the resolver's `violates REQ`
//! lead and its `fix surface:` clauses, and the cause anyhow repeats.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#refusal");

/// The byte offsets of a pattern outside parentheses and brackets.
pub(super) fn depth0(text: &str, pattern: &str) -> Vec<usize> {
    let mut depth = 0usize;
    let mut positions = Vec::new();
    for (offset, c) in text.char_indices() {
        if depth == 0 && text[offset..].starts_with(pattern) {
            positions.push(offset);
        }
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    positions
}

/// Take the resolver's leading requirement specifications in order.
pub(super) fn strip_req(mut text: &str) -> (Vec<&str>, &str) {
    let mut specs = Vec::new();
    while let Some(rest) = text.strip_prefix("violates REQ ") {
        let Some((spec, remaining)) = rest.split_once(": ") else {
            break;
        };
        specs.push(spec);
        text = remaining;
    }
    (specs, text)
}

/// Remove a cause that anyhow has already included before its separator.
pub(super) fn drop_repeat(text: &str) -> &str {
    for i in depth0(text, ": ") {
        let suffix = strip_req(&text[i + 2..]).1.trim();
        if !suffix.is_empty() && text[..i].contains(suffix) {
            return &text[..i];
        }
    }
    text
}

/// Split clauses only at separators outside parentheses and brackets.
pub(super) fn split0(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    for offset in depth0(text, "; ") {
        pieces.push(&text[start..offset]);
        start = offset + 2;
    }
    pieces.push(&text[start..]);
    pieces
}

/// Find the innermost parenthesis that remains open at the end.
pub(super) fn unmatched_open(text: &str) -> Option<usize> {
    let mut opens = Vec::new();
    for (offset, c) in text.char_indices() {
        match c {
            '(' => opens.push(offset),
            ')' => {
                opens.pop();
            }
            _ => {}
        }
    }
    opens.pop()
}

/// Find the closing parenthesis from a position inside an open pair.
pub(super) fn closing(text: &str, from: usize) -> Option<usize> {
    let mut depth = 1usize;
    for (offset, c) in text[from..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(from + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether a word has the application's snake-case error-code shape.
pub(super) fn is_code(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        && word.contains('_')
}
