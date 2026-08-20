//! Parsing: one logfmt line -> one [`Rec`].
//!
//! Grammar (the whole contract):
//! - a line is space-separated `key=value` pairs;
//! - keys are `[a-zA-Z_][a-zA-Z0-9_]*`;
//! - a value runs to the next space, or is double-quoted (quotes may
//!   contain spaces; `\"` escapes a quote inside);
//! - duplicate keys: the LAST occurrence wins;
//! - an empty line parses to an empty record;
//! - anything else is an error naming the byte offset.

/// One parsed logfmt record: ordered key/value pairs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Rec {
    pub pairs: Vec<(String, String)>,
}

impl Rec {
    /// The value of `key`, if present (last-wins semantics applied at
    /// parse time).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Parses one line. Errors carry the byte offset of the first bad
/// character and a short cause.
pub fn parse_line(line: &str) -> Result<Rec, String> {
    let mut pairs = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b' ' {
            i += 1;
            continue;
        }
        let key_start = i;
        if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            return Err(format!("offset {i}: a key must start with a letter or _"));
        }
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let key = line[key_start..i].to_string();
        if i >= bytes.len() || bytes[i] != b'=' {
            return Err(format!("offset {i}: expected `=` after key `{key}`"));
        }
        i += 1;
        let value = if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let mut v = String::new();
            loop {
                if i >= bytes.len() {
                    return Err(format!("offset {i}: unterminated quoted value"));
                }
                match bytes[i] {
                    b'"' => {
                        i += 1;
                        break;
                    }
                    b'\\' if i + 1 < bytes.len() && bytes[i + 1] == b'"' => {
                        v.push('"');
                        i += 2;
                    }
                    _ => {
                        v.push(line[i..].chars().next().ok_or("utf8")?);
                        i += line[i..].chars().next().map(char::len_utf8).unwrap_or(1);
                    }
                }
            }
            v
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b' ' {
                i += 1;
            }
            line[start..i].to_string()
        };
        pairs.retain(|(k, _): &(String, String)| k != &key);
        pairs.push((key, value));
    }
    Ok(Rec { pairs })
}
