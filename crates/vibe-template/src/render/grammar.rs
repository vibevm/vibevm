//! The variable grammar of PROP-075 `#TEMPLATE-VARIABLE-GRAMMAR`, as a
//! scanner over one string value.
//!
//! `{{` and `}}` are the literal braces; a `{` that opens no known
//! variable is a refusal, never a silent pass-through. One level of
//! nesting is allowed and only inside the argument of `sha256:` / `size:`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-VARIABLE-GRAMMAR");

/// One piece of a scanned string: text as written, or a variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    Literal(String),
    Var(Var),
}

/// The closed variable set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Var {
    Version,
    VersionRaw,
    Major,
    Minor,
    Patch,
    Prerelease,
    Platform,
    Arch,
    /// A dotted path into the version record.
    Meta(String),
    /// The digest of the file its argument names, inner variables first.
    Sha256(Vec<Piece>),
    /// The byte size of the file its argument names.
    Size(Vec<Piece>),
}

impl Var {
    /// The spelling a refusal quotes.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Version => "version",
            Self::VersionRaw => "version_raw",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::Prerelease => "prerelease",
            Self::Platform => "platform",
            Self::Arch => "arch",
            Self::Meta(_) => "meta",
            Self::Sha256(_) => "sha256",
            Self::Size(_) => "size",
        }
    }
}

/// Scan one string value. `nested` is set while scanning the argument of a
/// digest variable, where a second digest variable is refused — that is the
/// "one level of nesting" rule.
pub(crate) fn scan(text: &str, nested: bool) -> Result<Vec<Piece>, String> {
    let bytes = text.as_bytes();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut literal = String::new();
    let mut segment = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'{' if bytes.get(index + 1) == Some(&b'{') => {
                literal.push_str(&text[segment..index]);
                literal.push('{');
                index += 2;
                segment = index;
            }
            b'}' if bytes.get(index + 1) == Some(&b'}') => {
                literal.push_str(&text[segment..index]);
                literal.push('}');
                index += 2;
                segment = index;
            }
            b'{' => {
                literal.push_str(&text[segment..index]);
                let close = matching_brace(text, index)?;
                let inner = &text[index + 1..close];
                if !literal.is_empty() {
                    pieces.push(Piece::Literal(std::mem::take(&mut literal)));
                }
                pieces.push(Piece::Var(parse_var(inner, nested)?));
                index = close + 1;
                segment = index;
            }
            b'}' => {
                return Err(format!(
                    "a `}}` that closes no variable, in `{text}` — write `}}}}` for a literal \
                     closing brace"
                ));
            }
            _ => index += 1,
        }
    }
    literal.push_str(&text[segment..]);
    if !literal.is_empty() {
        pieces.push(Piece::Literal(literal));
    }
    Ok(pieces)
}

/// The index of the `}` that closes the `{` at `open`, counting depth so a
/// nested digest argument closes correctly.
///
/// `{{` / `}}` are escapes of the **literal** text only: inside a variable
/// a brace is structural, which is what lets `{sha256:t-{version}}` close
/// both braces in the right order. A file name carrying a literal brace is
/// therefore not expressible, and no real package needs one.
fn matching_brace(text: &str, open: usize) -> Result<usize, String> {
    let mut depth = 1usize;
    for (index, byte) in text.bytes().enumerate().skip(open + 1) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err(format!(
        "a `{{` that opens no variable, in `{text}` — it is never closed; write `{{{{` for a \
         literal opening brace"
    ))
}

fn parse_var(inner: &str, nested: bool) -> Result<Var, String> {
    for (prefix, build) in [
        ("sha256:", Var::Sha256 as fn(Vec<Piece>) -> Var),
        ("size:", Var::Size as fn(Vec<Piece>) -> Var),
    ] {
        if let Some(argument) = inner.strip_prefix(prefix) {
            if nested {
                return Err(format!(
                    "`{{{inner}}}` nests a digest variable inside another one — one level of \
                     nesting is allowed and only inside the argument of `sha256:` or `size:`"
                ));
            }
            if argument.is_empty() {
                return Err(format!("`{{{inner}}}` names no file"));
            }
            return Ok(build(scan(argument, true)?));
        }
    }
    match inner {
        "version" => Ok(Var::Version),
        "version_raw" => Ok(Var::VersionRaw),
        "major" => Ok(Var::Major),
        "minor" => Ok(Var::Minor),
        "patch" => Ok(Var::Patch),
        "prerelease" => Ok(Var::Prerelease),
        "platform" => Ok(Var::Platform),
        "arch" => Ok(Var::Arch),
        _ => match inner.strip_prefix("meta.") {
            Some(path) if is_dotted_path(path) => Ok(Var::Meta(path.to_string())),
            _ => Err(format!(
                "`{{{inner}}}` is not a known variable — the set is version, version_raw, major, \
                 minor, patch, prerelease, platform, arch, meta.PATH, sha256:FILE and size:FILE; \
                 write `{{{{` for a literal opening brace"
            )),
        },
    }
}

/// A `{meta.PATH}` path: dot-separated names, nothing structural. A path
/// carrying a brace is not a nesting — nesting is allowed only inside the
/// argument of `sha256:` / `size:`.
fn is_dotted_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('.')
        && !path.ends_with('.')
        && !path.contains("..")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

#[cfg(test)]
#[path = "grammar/tests.rs"]
mod tests;
