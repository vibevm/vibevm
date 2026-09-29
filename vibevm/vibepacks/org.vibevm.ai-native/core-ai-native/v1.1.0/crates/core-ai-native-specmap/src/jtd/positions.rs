//! Byte-exact line positions for JTD schema units — the `line` and
//! `end_line` a [`CodeItem`](crate::generated::specmap::CodeItem) carries.
//! `serde_json` validates and structures a schema but discards byte offsets,
//! so a second structural pass over the (serde_json-validated) text measures
//! them: the root object's braces for the root unit, and each `definitions`
//! child key plus its value's last byte for a definition unit. Measured, not
//! invented — the only honest source of "where" in a JSON file.
//!
//! Key *content* is decoded by feeding the raw `"…"` literal back through
//! `serde_json` — the same decoder that parsed the whole document — so the
//! names this pass emits match the parsed `definitions` map exactly, with no
//! hand-rolled escape table to drift against.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#addressing-code");

/// One schema unit's measured span: the 1-based line where the unit is
/// defined (the root's opening brace, or a definition's key) and the 1-based
/// line of its far end (the matching close — the root's `}`, or a
/// definition value's last byte).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Span {
    pub line: u32,
    pub end_line: u32,
}

/// Byte offsets of the start of each line (`starts[0] == 0` is line 1), for
/// byte-offset → 1-based-line lookup.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// The 1-based line containing `byte` (a byte offset anywhere on the line).
/// `partition_point` returns the count of line-starts `<= byte`, which is
/// exactly the 1-based line number.
fn line_of(starts: &[usize], byte: usize) -> u32 {
    starts.partition_point(|&s| s <= byte) as u32
}

/// Advance past JSON whitespace (`space | tab | nl | cr`).
fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// The byte span `[start, end)` of a `"…"` string literal at `i` — `start`
/// is the opening quote, `end` is just past the closing quote. Only `\"`
/// needs handling to find the close; the content is decoded elsewhere by
/// `serde_json`, so no escape table lives here.
fn string_span(bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    if bytes.get(i) != Some(&b'"') {
        return None;
    }
    let start = i;
    let mut j = i + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            b'"' => return Some((start, j + 1)),
            _ => j += 1,
        }
    }
    None
}

/// Index just past a JSON value starting at `i` (after leading whitespace).
/// String contents are skipped wholesale so braces/brackets inside them do
/// not count toward nesting.
fn skip_value(bytes: &[u8], i: usize) -> usize {
    let i = skip_ws(bytes, i);
    match bytes.get(i) {
        Some(b'{') => skip_balanced(bytes, i, b'{', b'}'),
        Some(b'[') => skip_balanced(bytes, i, b'[', b']'),
        Some(b'"') => string_span(bytes, i).map_or(i + 1, |(_, e)| e),
        _ => {
            let mut j = i;
            while j < bytes.len()
                && !matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r' | b',' | b'}' | b']')
            {
                j += 1;
            }
            j
        }
    }
}

/// Index just past the `close` matching the `open` at `i`, counting nesting
/// and skipping string contents. Valid JSON in (the caller validated with
/// `serde_json`), so the brace stream is balanced.
fn skip_balanced(bytes: &[u8], i: usize, open: u8, close: u8) -> usize {
    let mut depth: i32 = 0;
    let mut j = i;
    while j < bytes.len() {
        match bytes[j] {
            b'"' => j = string_span(bytes, j).map_or(j + 1, |(_, e)| e),
            c if c == open => {
                depth += 1;
                j += 1;
            }
            c if c == close => {
                depth -= 1;
                j += 1;
                if depth == 0 {
                    return j;
                }
            }
            _ => j += 1,
        }
    }
    j
}

/// Decode a raw `"…"` literal (quotes included) into its `String` content via
/// `serde_json` — the same decoder that parsed the whole document, so a key
/// matches the `definitions` map entry for entry.
fn decode_key(text: &str, span: (usize, usize)) -> Option<String> {
    serde_json::from_str::<String>(&text[span.0..span.1]).ok()
}

/// Measure the root object's span and every `definitions.<name>` span in
/// `text`. `text` MUST be valid JSON whose root is an object (the caller
/// validates with `serde_json` first), so the walker needs no error
/// recovery. Returns `(root_span, definition_spans)` with the definitions in
/// source order.
pub(crate) fn schema_spans(text: &str) -> (Option<Span>, Vec<(String, Span)>) {
    let bytes = text.as_bytes();
    let starts = line_starts(text);
    let mut defs = Vec::new();

    let mut i = skip_ws(bytes, 0);
    if bytes.get(i) != Some(&b'{') {
        // Non-object root — the caller emits `schema-not-object`; there is
        // nothing here to measure.
        return (None, defs);
    }
    let root_open = i;
    let root_close = skip_balanced(bytes, i, b'{', b'}').saturating_sub(1);
    let root_span = Span {
        line: line_of(&starts, root_open),
        end_line: line_of(&starts, root_close),
    };

    // Walk the top-level object's members to locate `definitions`; any other
    // key is skipped whole. `definitions` is a JTD top-level key, so only a
    // depth-1 match descends.
    i = skip_ws(bytes, root_open + 1);
    while i < bytes.len() && bytes.get(i) != Some(&b'}') {
        let Some(key_span) = string_span(bytes, i) else {
            break;
        };
        i = skip_ws(bytes, key_span.1);
        if bytes.get(i) != Some(&b':') {
            break;
        }
        let val_start = skip_ws(bytes, i + 1);
        if decode_key(text, key_span).as_deref() == Some("definitions")
            && bytes.get(val_start) == Some(&b'{')
        {
            defs = walk_definitions(bytes, val_start, text, &starts);
        }
        i = skip_ws(bytes, skip_value(bytes, val_start));
        if bytes.get(i) == Some(&b',') {
            i = skip_ws(bytes, i + 1);
        }
    }

    (Some(root_span), defs)
}

/// Measure every member of a valid top-level JSON object. Each span starts at
/// the member key and ends at the final byte of its value. Shared vocabulary
/// projections use these positions so their units point at the authored
/// vocabulary member rather than inventing a line in the thin schema.
pub(crate) fn top_level_member_spans(text: &str) -> Vec<(String, Span)> {
    let bytes = text.as_bytes();
    let starts = line_starts(text);
    let mut out = Vec::new();
    let mut i = skip_ws(bytes, 0);
    if bytes.get(i) != Some(&b'{') {
        return out;
    }
    i = skip_ws(bytes, i + 1);
    while i < bytes.len() && bytes.get(i) != Some(&b'}') {
        let Some(key_span) = string_span(bytes, i) else {
            break;
        };
        let Some(name) = decode_key(text, key_span) else {
            break;
        };
        let key_byte = key_span.0;
        i = skip_ws(bytes, key_span.1);
        if bytes.get(i) != Some(&b':') {
            break;
        }
        let value_start = skip_ws(bytes, i + 1);
        let value_end = skip_value(bytes, value_start);
        out.push((
            name,
            Span {
                line: line_of(&starts, key_byte),
                end_line: line_of(&starts, value_end.saturating_sub(1)),
            },
        ));
        i = skip_ws(bytes, value_end);
        if bytes.get(i) == Some(&b',') {
            i = skip_ws(bytes, i + 1);
        }
    }
    out
}

/// Walk one `definitions` object (opening brace at `obj_open`), recording
/// each child key (decoded name), the key's opening-quote byte offset, and
/// its value's last byte — the span a `schema-def` unit occupies. The value
/// is usually an object (the definition schema); the "last byte" is then its
/// closing `}`, but a non-object value is handled uniformly by taking the
/// byte before [`skip_value`]'s end.
fn walk_definitions(
    bytes: &[u8],
    obj_open: usize,
    text: &str,
    starts: &[usize],
) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    let mut i = skip_ws(bytes, obj_open + 1);
    while i < bytes.len() && bytes.get(i) != Some(&b'}') {
        let Some(key_span) = string_span(bytes, i) else {
            break;
        };
        let key_byte = key_span.0;
        let Some(name) = decode_key(text, key_span) else {
            break;
        };
        i = skip_ws(bytes, key_span.1);
        if bytes.get(i) != Some(&b':') {
            break;
        }
        let val_start = skip_ws(bytes, i + 1);
        let val_end = skip_value(bytes, val_start);
        let close_byte = val_end.saturating_sub(1);
        out.push((
            name,
            Span {
                line: line_of(starts, key_byte),
                end_line: line_of(starts, close_byte),
            },
        ));
        i = skip_ws(bytes, val_end);
        if bytes.get(i) == Some(&b',') {
            i = skip_ws(bytes, i + 1);
        }
    }
    out
}
