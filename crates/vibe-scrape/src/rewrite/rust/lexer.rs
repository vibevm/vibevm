use super::text::*;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RustTokenKind {
    Ident(String),
    Punct(u8),
}

#[derive(Debug, Clone)]
pub(super) struct RustToken {
    pub(super) kind: RustTokenKind,
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn rust_tokens(bytes: &[u8]) -> Result<Vec<RustToken>, ScrapeError> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            index = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |at| index + at + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            let mut depth = 1usize;
            index += 2;
            while index < bytes.len() && depth > 0 {
                if bytes[index..].starts_with(b"/*") {
                    depth += 1;
                    index += 2;
                } else if bytes[index..].starts_with(b"*/") {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            if depth != 0 {
                return Err(fail("unterminated Rust block comment"));
            }
            continue;
        }
        let raw_prefix = if bytes[index] == b'r' {
            Some(1)
        } else if matches!(bytes[index], b'b' | b'c') && bytes.get(index + 1) == Some(&b'r') {
            Some(2)
        } else {
            None
        };
        if let Some(prefix_len) = raw_prefix {
            let mut cursor = index + prefix_len;
            while bytes.get(cursor) == Some(&b'#') {
                cursor += 1;
            }
            if bytes.get(cursor) == Some(&b'"') {
                let hashes = cursor - index - prefix_len;
                cursor += 1;
                loop {
                    let Some(relative) = bytes[cursor..].iter().position(|byte| *byte == b'"')
                    else {
                        return Err(fail("unterminated Rust raw string"));
                    };
                    cursor += relative + 1;
                    if bytes
                        .get(cursor..cursor + hashes)
                        .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
                    {
                        index = cursor + hashes;
                        break;
                    }
                }
                continue;
            }
        }
        if bytes[index] == b'\''
            && bytes
                .get(index + 1)
                .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphabetic())
        {
            let mut after_name = index + 2;
            while bytes
                .get(after_name)
                .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
            {
                after_name += 1;
            }
            if bytes.get(after_name) != Some(&b'\'') {
                tokens.push(RustToken {
                    kind: RustTokenKind::Punct(b'\''),
                    start: index,
                    end: index + 1,
                });
                index += 1;
                continue;
            }
        }
        if matches!(bytes[index], b'"' | b'\'') {
            let delimiter = bytes[index];
            let mut cursor = index + 1;
            let mut escaped = false;
            while cursor < bytes.len() {
                if escaped {
                    escaped = false;
                } else if bytes[cursor] == b'\\' {
                    escaped = true;
                } else if bytes[cursor] == delimiter {
                    cursor += 1;
                    break;
                }
                cursor += 1;
            }
            if cursor > bytes.len() || bytes.get(cursor.saturating_sub(1)) != Some(&delimiter) {
                // Apostrophes that start lifetimes are punctuation, not chars.
                if delimiter == b'\'' {
                    tokens.push(RustToken {
                        kind: RustTokenKind::Punct(delimiter),
                        start: index,
                        end: index + 1,
                    });
                    index += 1;
                    continue;
                }
                return Err(fail("unterminated Rust string literal"));
            }
            index = cursor;
            continue;
        }
        if bytes[index] == b'_' || bytes[index].is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while bytes
                .get(index)
                .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
            {
                index += 1;
            }
            tokens.push(RustToken {
                kind: RustTokenKind::Ident(
                    std::str::from_utf8(&bytes[start..index])
                        .expect("ASCII identifier")
                        .to_owned(),
                ),
                start,
                end: index,
            });
            continue;
        }
        tokens.push(RustToken {
            kind: RustTokenKind::Punct(bytes[index]),
            start: index,
            end: index + 1,
        });
        index += 1;
    }
    Ok(tokens)
}

pub(super) fn token_ident(token: Option<&RustToken>) -> Option<&str> {
    match token.map(|token| &token.kind) {
        Some(RustTokenKind::Ident(value)) => Some(value),
        _ => None,
    }
}

pub(super) fn token_punct(token: Option<&RustToken>, punct: u8) -> bool {
    matches!(token.map(|token| &token.kind), Some(RustTokenKind::Punct(value)) if *value == punct)
}

pub(super) fn matching_delimiter(tokens: &[RustToken], open_at: usize) -> Option<usize> {
    let open = match tokens.get(open_at)?.kind {
        RustTokenKind::Punct(value @ (b'(' | b'[' | b'{')) => value,
        _ => return None,
    };
    let close = match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => unreachable!(),
    };
    let mut stack = vec![close];
    for (index, token) in tokens.iter().enumerate().skip(open_at + 1) {
        let RustTokenKind::Punct(value) = token.kind else {
            continue;
        };
        match value {
            b'(' => stack.push(b')'),
            b'[' => stack.push(b']'),
            b'{' => stack.push(b'}'),
            value if stack.last() == Some(&value) => {
                stack.pop();
                if stack.is_empty() {
                    return Some(index);
                }
            }
            b')' | b']' | b'}' => return None,
            _ => {}
        }
    }
    None
}

pub(super) fn containing_line(bytes: &[u8], start: usize, end: usize) -> (usize, usize, bool) {
    let line_start = bytes[..start]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |at| at + 1);
    let line_end = bytes[end..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |at| end + at + 1);
    let whole = trim_ascii(&bytes[line_start..start]).is_empty()
        && trim_ascii(&bytes[end..line_end]).is_empty();
    (line_start, line_end, whole)
}
