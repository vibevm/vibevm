//! Deliberately small block-mapping YAML subset for native settings edits.
//! Every line is admitted; opaque collections, aliases and implicit merges refuse.
use crate::Error;
use std::{collections::BTreeMap, ops::Range};

struct Field {
    value: Option<Range<usize>>,
    decoded: Option<String>,
    end: usize,
    indent: usize,
}
struct Document {
    fields: BTreeMap<Vec<String>, Field>,
}

fn scalar(raw: &str) -> Result<String, Error> {
    if raw.starts_with('"') {
        return serde_json::from_str::<String>(raw).map_err(|_| Error::Invalid);
    }
    if raw.starts_with('\'') {
        if !raw.ends_with('\'') || raw.len() < 2 {
            return Err(Error::Invalid);
        }
        let inner = &raw[1..raw.len() - 1];
        let mut chars = inner.chars().peekable();
        let mut result = String::new();
        while let Some(c) = chars.next() {
            if c == '\'' && chars.next() != Some('\'') {
                return Err(Error::Invalid);
            }
            result.push(c);
        }
        return Ok(result);
    }
    if raw.is_empty()
        || raw
            .chars()
            .any(|c| c.is_control() || "{}[]&*!|>@`\"'".contains(c))
        || raw.contains(": ")
        || raw.starts_with(['-', '?', ':', '%'])
    {
        return Err(Error::Invalid);
    }
    // YAML's many implicit scalar types are not string literals. Preserve them
    // outside edited fields, but never interpret them as a selected string.
    Ok(raw.into())
}

fn parse(text: &str) -> Result<Document, Error> {
    if text.len() > 4 * 1024 * 1024 || text.contains('\r') || text.contains('\t') {
        return Err(Error::Invalid);
    }
    let mut fields: BTreeMap<Vec<String>, Field> = BTreeMap::new();
    let mut parents: Vec<(usize, String)> = Vec::new();
    let mut previous_mapping = false;
    let mut previous_indent = 0;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let indent = body.bytes().take_while(|b| *b == b' ').count();
        let content = &body[indent..];
        if content.is_empty() || content.starts_with('#') {
            offset += line.len();
            continue;
        }
        if fields.len() >= 100_000 || indent > 128 {
            return Err(Error::Invalid);
        }
        // Keys are a closed spelling: quoted/complex keys and all directives
        // are refused, rather than guessed using indentation alone.
        let colon = content.find(':').ok_or(Error::Invalid)?;
        let key = &content[..colon];
        if key.is_empty()
            || !key
                .as_bytes()
                .first()
                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
            || matches!(
                key.to_ascii_lowercase().as_str(),
                "true" | "false" | "null" | "yes" | "no" | "on" | "off"
            )
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err(Error::Invalid);
        }
        if colon + 1 < content.len() && !content.as_bytes()[colon + 1].is_ascii_whitespace() {
            return Err(Error::Invalid);
        }
        if indent > previous_indent && (!previous_mapping || parents.is_empty()) {
            return Err(Error::Invalid);
        }
        while parents.last().is_some_and(|(depth, _)| *depth >= indent) {
            parents.pop();
        }
        if indent != 0 && parents.is_empty() {
            return Err(Error::Invalid);
        }
        // Fixed two-space block levels make dedents unambiguous. Never infer
        // a sibling from an indentation width that YAML would reject.
        if indent != parents.last().map_or(0, |(depth, _)| depth + 2) {
            return Err(Error::Invalid);
        }
        for (path, field) in &mut fields {
            if field.value.is_none() && field.indent >= indent && field.end == usize::MAX {
                let _ = path;
                field.end = offset;
            }
        }
        let rest = &content[colon + 1..];
        let leading = rest.len() - rest.trim_start_matches(' ').len();
        let raw = &rest[leading..];
        let mut quote = None;
        let mut escaped = false;
        let mut end = raw.len();
        for (i, c) in raw.char_indices() {
            if quote == Some('"') {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    quote = None;
                }
            } else if quote == Some('\'') {
                if c == '\'' {
                    quote = None;
                }
            } else if c == '"' || c == '\'' {
                quote = Some(c);
            } else if c == '#' && (i == 0 || raw.as_bytes()[i - 1] == b' ') {
                end = i;
                break;
            }
        }
        if quote.is_some() {
            return Err(Error::Invalid);
        }
        let value = raw[..end].trim_end_matches(' ');
        let mut path: Vec<String> = parents.iter().map(|(_, key)| key.clone()).collect();
        path.push(key.into());
        let is_mapping = value.is_empty();
        let field = if is_mapping {
            Field {
                value: None,
                decoded: None,
                end: usize::MAX,
                indent,
            }
        } else {
            let start = offset + indent + colon + 1 + leading;
            Field {
                value: Some(start..start + value.len()),
                decoded: Some(scalar(value)?),
                end: offset + line.len(),
                indent,
            }
        };
        if fields.insert(path, field).is_some() {
            return Err(Error::Invalid);
        }
        if is_mapping {
            parents.push((indent, key.into()));
        }
        previous_mapping = is_mapping;
        previous_indent = indent;
        offset += line.len();
    }
    for field in fields.values_mut() {
        if field.end == usize::MAX {
            field.end = text.len();
        }
    }
    Ok(Document { fields })
}

pub(super) fn validate(text: &str) -> Result<(), Error> {
    parse(text).map(|_| ())
}
pub(super) fn contains(text: &str, path: &[&str]) -> Result<bool, Error> {
    let path: Vec<_> = path.iter().map(|s| s.to_string()).collect();
    Ok(parse(text)?.fields.contains_key(&path))
}
pub(super) fn set(text: &str, path: &[&str], desired: &str) -> Result<String, Error> {
    if path.is_empty() {
        return Err(Error::Invalid);
    }
    let doc = parse(text)?;
    let key: Vec<_> = path.iter().map(|s| s.to_string()).collect();
    let encoded = serde_json::to_string(desired).map_err(|_| Error::Invalid)?;
    if let Some(field) = doc.fields.get(&key) {
        if field.decoded.as_deref() == Some(desired) {
            let raw = field
                .value
                .as_ref()
                .map(|range| &text[range.clone()])
                .ok_or(Error::Invalid)?;
            // Unquoted YAML booleans/null/numbers are not string scalars.
            // Conservatively quote every nonalphabetic plain spelling rather
            // than claim a semantic no-op for an implicit YAML type.
            let string_scalar = raw.starts_with(['\"', '\''])
                || (!matches!(
                    raw.to_ascii_lowercase().as_str(),
                    "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~"
                ) && raw.as_bytes().first().is_some_and(u8::is_ascii_alphabetic));
            if string_scalar {
                return Ok(text.into());
            }
        }
        let range = field.value.clone().ok_or(Error::Invalid)?;
        let mut result = text.to_owned();
        result.replace_range(range, &encoded);
        parse(&result)?;
        return Ok(result);
    }
    let mut parent = path.len() - 1;
    while parent > 0 && !doc.fields.contains_key(&key[..parent]) {
        parent -= 1;
    }
    let (position, indent) = if parent == 0 {
        (text.len(), 0)
    } else {
        let field = &doc.fields[&key[..parent]];
        if field.value.is_some() {
            return Err(Error::Invalid);
        }
        (field.end, field.indent + 2)
    };
    let mut inserted = String::new();
    if position > 0 && text.as_bytes()[position - 1] != b'\n' {
        inserted.push('\n');
    }
    for (i, component) in path[parent..].iter().enumerate() {
        inserted.push_str(&" ".repeat(indent + i * 2));
        inserted.push_str(component);
        inserted.push(':');
        if parent + i == path.len() - 1 {
            inserted.push(' ');
            inserted.push_str(&encoded);
        }
        inserted.push('\n');
    }
    let mut result = text.to_owned();
    result.insert_str(position, &inserted);
    parse(&result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admitted_mapping_preserves_comments_and_refuses_ambiguous_yaml() {
        let text = "# keep\nmodel:\n  provider: 'old' # inline\n  default: old\nneighbor:\n  enabled: true\n";
        let result = set(text, &["model", "provider"], "custom:vibevm").unwrap();
        assert_eq!(result, text.replace("'old'", "\"custom:vibevm\""));
        let result = set(&result, &["providers", "vibevm", "api_key"], "synthetic").unwrap();
        assert!(result.contains("neighbor:\n  enabled: true\n"));
        assert_eq!(
            set(&result, &["providers", "vibevm", "api_key"], "synthetic").unwrap(),
            result
        );
        for scalar in ["true", "false", "null", "yes", "12", "2026-11-21", ".nan"] {
            let input = format!("model:\n  default: {scalar}\n");
            let quoted = set(&input, &["model", "default"], scalar).unwrap();
            assert!(quoted.contains(&format!("default: \"{scalar}\"")));
            assert_eq!(set(&quoted, &["model", "default"], scalar).unwrap(), quoted);
        }
        for bad in [
            "a: 1\na: 2\n",
            "true: one\nTrue: two\n",
            "1: one\n01: two\n",
            "a: &x secret\n",
            "a: *x\n",
            "---\na: 1\n",
            "a:\n  <<: *x\n",
            "a: |\n  opaque\n",
            "a:\n  - value\n",
            "a: {key: value}\n",
            "a: value\n  nested: impossible\n",
        ] {
            assert!(validate(bad).is_err());
        }
    }
}
