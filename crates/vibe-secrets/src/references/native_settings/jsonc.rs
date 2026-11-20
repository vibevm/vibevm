//! Bounded native JSONC structure/ranges only. No keyring or secret resolution.
use crate::Error;
use std::{collections::BTreeMap, ops::Range};

pub(super) struct Node {
    pub range: Range<usize>,
    pub object: Option<BTreeMap<String, Node>>,
    /// Whether inserting another property needs a leading comma.
    pub comma: bool,
    pub trailing: bool,
}
struct Parser<'a> {
    text: &'a str,
    pos: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn trivia(&mut self) -> Result<(), Error> {
        loop {
            while self
                .text
                .as_bytes()
                .get(self.pos)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.pos += 1;
            }
            if self.text[self.pos..].starts_with("//") {
                self.pos += 2;
                while self.pos < self.text.len() && self.text.as_bytes()[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else if self.text[self.pos..].starts_with("/*") {
                let end = self.text[self.pos + 2..].find("*/").ok_or(Error::Invalid)?;
                self.pos += end + 4;
            } else {
                return Ok(());
            }
        }
    }
    fn string(&mut self) -> Result<String, Error> {
        let start = self.pos;
        if self.text.as_bytes().get(self.pos) != Some(&b'"') {
            return Err(Error::Invalid);
        }
        self.pos += 1;
        loop {
            let c = *self.text.as_bytes().get(self.pos).ok_or(Error::Invalid)?;
            self.pos += 1;
            if c == b'"' {
                break;
            }
            if c == b'\\' {
                self.pos += 1;
                if self.pos > self.text.len() {
                    return Err(Error::Invalid);
                }
            }
        }
        serde_json::from_str(&self.text[start..self.pos]).map_err(|_| Error::Invalid)
    }
    fn value(&mut self, depth: usize) -> Result<Node, Error> {
        self.nodes += 1;
        if depth > 64 || self.nodes > 100_000 {
            return Err(Error::Invalid);
        }
        self.trivia()?;
        let start = self.pos;
        let c = *self.text.as_bytes().get(self.pos).ok_or(Error::Invalid)?;
        let mut object = None;
        let mut comma = false;
        let mut trailing = false;
        if c == b'{' || c == b'[' {
            self.pos += 1;
            let close = if c == b'{' { b'}' } else { b']' };
            let mut keys = BTreeMap::new();
            self.trivia()?;
            if self.text.as_bytes().get(self.pos) != Some(&close) {
                loop {
                    self.trivia()?;
                    let key = if c == b'{' {
                        let key = self.string()?;
                        self.trivia()?;
                        if self.text.as_bytes().get(self.pos) != Some(&b':') {
                            return Err(Error::Invalid);
                        }
                        self.pos += 1;
                        Some(key)
                    } else {
                        None
                    };
                    let value = self.value(depth + 1)?;
                    trailing |= value.trailing;
                    if let Some(key) = key
                        && keys.insert(key, value).is_some()
                    {
                        return Err(Error::Invalid);
                    }
                    self.trivia()?;
                    comma = true;
                    match self.text.as_bytes().get(self.pos) {
                        Some(b',') => {
                            self.pos += 1;
                            self.trivia()?;
                            comma = false;
                            if self.text.as_bytes().get(self.pos) == Some(&close) {
                                trailing = true;
                                break;
                            }
                        }
                        Some(x) if *x == close => break,
                        _ => return Err(Error::Invalid),
                    }
                }
            }
            if self.text.as_bytes().get(self.pos) != Some(&close) {
                return Err(Error::Invalid);
            }
            self.pos += 1;
            if c == b'{' {
                object = Some(keys);
            }
        } else if c == b'"' {
            self.string()?;
        } else {
            while self
                .text
                .as_bytes()
                .get(self.pos)
                .is_some_and(|b| !b.is_ascii_whitespace() && !b",}]/".contains(b))
            {
                self.pos += 1;
            }
            let scalar: serde_json::Value =
                serde_json::from_str(&self.text[start..self.pos]).map_err(|_| Error::Invalid)?;
            if scalar.is_object() || scalar.is_array() || scalar.is_string() {
                return Err(Error::Invalid);
            }
        }
        Ok(Node {
            range: start..self.pos,
            object,
            comma,
            trailing,
        })
    }
}

pub(super) fn parse(text: &str) -> Result<Node, Error> {
    if text.len() > 4 * 1024 * 1024 {
        return Err(Error::Invalid);
    }
    let mut parser = Parser {
        text,
        pos: 0,
        nodes: 0,
    };
    let root = parser.value(0)?;
    parser.trivia()?;
    if parser.pos != text.len() || root.object.is_none() {
        return Err(Error::Invalid);
    }
    Ok(root)
}

/// Patch one object property, retaining every byte outside its value or insertion.
/// Values are already JSON encoded by the concrete native writer.
pub(super) fn set(text: &str, path: &[&str], encoded: &str) -> Result<String, Error> {
    if path.is_empty() {
        return Err(Error::Invalid);
    }
    serde_json::from_str::<serde_json::Value>(encoded).map_err(|_| Error::Invalid)?;
    let root = parse(text)?;
    fn locate(node: &Node, path: &[&str], encoded: &str) -> Result<(Range<usize>, String), Error> {
        let fields = node.object.as_ref().ok_or(Error::Invalid)?;
        if let Some(child) = fields.get(path[0]) {
            if path.len() == 1 {
                return Ok((child.range.clone(), encoded.into()));
            }
            return locate(child, &path[1..], encoded);
        }
        let mut nested = encoded.to_owned();
        for key in path[1..].iter().rev() {
            nested = format!(
                "{{{}:{nested}}}",
                serde_json::to_string(key).map_err(|_| Error::Invalid)?
            );
        }
        let key = serde_json::to_string(path[0]).map_err(|_| Error::Invalid)?;
        let prefix = if node.comma { "," } else { "" };
        let insertion = format!("{prefix}\n{key}:{nested}\n");
        let position = node.range.end.checked_sub(1).ok_or(Error::Invalid)?;
        Ok((position..position, insertion))
    }
    let (range, replacement) = locate(&root, path, encoded)?;
    let mut output = text.to_owned();
    output.replace_range(range, &replacement);
    parse(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comments_ranges_trailing_comma_and_escaped_keys_are_unambiguous() {
        let text = "{/*keep*/\"security\": {\"auth\": {\"apiKey\": \"old\\\"key\",},}, //keep\n\"other\":[1,{\"x\":true,},],}";
        let root = parse(text).unwrap();
        let fields = root.object.as_ref().unwrap();
        let auth = fields["security"].object.as_ref().unwrap()["auth"]
            .object
            .as_ref()
            .unwrap();
        assert_eq!(&text[auth["apiKey"].range.clone()], "\"old\\\"key\"");
        assert!(!root.comma);
        let patched = set(text, &["security", "auth", "apiKey"], "\"new\"").unwrap();
        assert_eq!(patched, text.replace("\"old\\\"key\"", "\"new\""));
        let added = set(
            &patched,
            &["security", "auth", "baseUrl"],
            "\"https://example.invalid/v1\"",
        )
        .unwrap();
        assert!(
            added.contains("/*keep*/")
                && added.contains("//keep")
                && added.contains("\"other\":[1,{\"x\":true,},]")
        );
        assert!(parse(&added).is_ok());
        let nested = set(
            "{//keep\n}",
            &["security", "auth", "selectedType"],
            "\"openai\"",
        )
        .unwrap();
        assert!(nested.contains("//keep"));
        assert!(parse(&nested).is_ok());
        for bad in [
            "{\"a\":1,\"a\":2}",
            "{\"a\":1,\"\\u0061\":2}",
            "{unquoted:1}",
            "{\"x\": NaN}",
            "{/*unclosed}",
            "{} extra",
            "{\"x\":[1,,2]}",
        ] {
            assert!(parse(bad).is_err());
        }
    }
}
