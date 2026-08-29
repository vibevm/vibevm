//! The one host-owner codec: a project name is arbitrary, its identity is not.
//!
//! An ungrouped `[project]` has no `<group>/<package>` coordinate, so its
//! provider identity is spelled `__host__/<segment>`
//! (SPEC-DEBT-LIFECYCLE §8.3). But `[project].name` is an arbitrary TOML
//! string — it may hold `/`, `#`, `@`, `:`, `\`, spaces, newlines, or any
//! Unicode. Interpolating it raw made two different projects print one key
//! (`odd/# project` + id `x` and `odd` + id `# project#x` both render
//! `__host__/odd/# project#x`), and made the identity unparseable.
//!
//! So `<segment>` is a **reversible** encoding rather than the raw name:
//! RFC 3986 unreserved ASCII (`A-Z a-z 0-9 - . _ ~`) stays verbatim, every
//! other byte becomes an uppercase `%HH` escape. Ordinary names are therefore
//! byte-identical to what they always were (`__host__/demo`), while every
//! legal project name gets exactly one spelling and every spelling decodes
//! back to exactly one name.
//!
//! One codec, every consumer: [`crate::manifest::ExtensionKey::for_host`],
//! the mechanism `ProviderOwner::Host` display and parser, and the
//! `vibe-extension-registry` `HostIdentity` display all route through here, so a
//! state key, an activation `ref` and a provider pin cannot disagree about
//! what a project is called. The name is carried as typed data; this branch
//! is never parsed as a `Group` or a `PackageRef`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION");

use std::fmt;

/// The reserved coordinate head of a project-host identity. It is not a
/// [`crate::Group`] and can never become one — `_` is not an LDH character —
/// so the host spelling cannot collide with a real package coordinate.
pub const HOST_OWNER: &str = "__host__";

const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// An ungrouped project's host-owner identity: the authored name, plus its
/// one canonical reversible spelling.
///
/// ```
/// use vibe_core::HostOwner;
///
/// let plain = HostOwner::new("demo");
/// assert_eq!(plain.to_string(), "__host__/demo");
///
/// let awkward = HostOwner::new("my app");
/// assert_eq!(awkward.to_string(), "__host__/my%20app");
/// assert_eq!(HostOwner::parse(&awkward.to_string()).unwrap(), awkward);
/// assert_eq!(HostOwner::parse(&awkward.to_string()).unwrap().project(), "my app");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HostOwner(String);

impl HostOwner {
    /// Wrap an authored project name exactly as written.
    #[must_use]
    pub fn new(project: impl Into<String>) -> Self {
        Self(project.into())
    }

    /// The authored project name, unchanged.
    #[must_use]
    pub fn project(&self) -> &str {
        &self.0
    }

    /// The canonical `<segment>` half — the percent-encoded name.
    #[must_use]
    pub fn segment(&self) -> String {
        encode_host_segment(&self.0)
    }

    /// Decode one canonical `<segment>` back to its project name.
    pub fn parse_segment(segment: &str) -> Result<Self, HostSegmentError> {
        decode_host_segment(segment).map(Self)
    }

    /// Decode a whole `__host__/<segment>` owner spelling.
    pub fn parse(spelling: &str) -> Result<Self, HostSegmentError> {
        let Some(segment) = spelling
            .strip_prefix(HOST_OWNER)
            .and_then(|rest| rest.strip_prefix('/'))
        else {
            return Err(HostSegmentError::NotAHostOwner);
        };
        Self::parse_segment(segment)
    }
}

impl fmt::Display for HostOwner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{HOST_OWNER}/{}", self.segment())
    }
}

/// Encode one arbitrary project name into its canonical host segment.
///
/// ```
/// use vibe_core::host_owner::encode_host_segment;
///
/// assert_eq!(encode_host_segment("demo"), "demo");
/// assert_eq!(encode_host_segment("a/b#c"), "a%2Fb%23c");
/// // A literal `%20` is itself escaped, so it cannot alias a space.
/// assert_eq!(encode_host_segment("%20"), "%2520");
/// ```
#[must_use]
pub fn encode_host_segment(project: &str) -> String {
    let mut encoded = String::with_capacity(project.len());
    for byte in project.as_bytes() {
        if is_unreserved(*byte) {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[usize::from(byte >> 4)] as char);
            encoded.push(HEX[usize::from(byte & 0x0F)] as char);
        }
    }
    encoded
}

/// Decode one canonical host segment. Anything that is not exactly what
/// [`encode_host_segment`] would have produced is refused, so the mapping
/// stays one-to-one in both directions.
///
/// ```
/// use vibe_core::host_owner::decode_host_segment;
///
/// assert_eq!(decode_host_segment("my%20app").unwrap(), "my app");
/// assert!(decode_host_segment("my app").is_err());   // unescaped
/// assert!(decode_host_segment("my%2fapp").is_err()); // lowercase escape
/// assert!(decode_host_segment("%2D").is_err());      // `-` is unreserved
/// ```
pub fn decode_host_segment(segment: &str) -> Result<String, HostSegmentError> {
    let bytes = segment.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            let Some(pair) = bytes.get(index + 1..index + 3) else {
                return Err(HostSegmentError::TruncatedEscape);
            };
            if pair.iter().any(|digit| digit.is_ascii_lowercase()) {
                return Err(HostSegmentError::LowercaseEscape);
            }
            let (Some(high), Some(low)) = (hex_value(pair[0]), hex_value(pair[1])) else {
                return Err(HostSegmentError::MalformedEscape);
            };
            decoded.push(high << 4 | low);
            index += 3;
        } else if is_unreserved(byte) {
            decoded.push(byte);
            index += 1;
        } else {
            return Err(HostSegmentError::UnescapedByte);
        }
    }
    let project = String::from_utf8(decoded).map_err(|_| HostSegmentError::InvalidUtf8)?;
    // The round trip is the law: a segment that re-encodes to something else
    // was a second spelling of the same name, and two spellings would let two
    // raw project names print one key.
    if encode_host_segment(&project) != segment {
        return Err(HostSegmentError::NonCanonical);
    }
    Ok(project)
}

/// Why a host segment is not the canonical spelling of any project name.
///
/// ```
/// use vibe_core::{HostOwner, HostSegmentError};
/// use vibe_core::host_owner::decode_host_segment;
///
/// assert_eq!(decode_host_segment("a b"), Err(HostSegmentError::UnescapedByte));
/// assert_eq!(HostOwner::parse("demo"), Err(HostSegmentError::NotAHostOwner));
/// assert!(HostSegmentError::LowercaseEscape.reason().contains("%HH"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSegmentError {
    /// The spelling does not start with `__host__/`.
    NotAHostOwner,
    /// A `%` with fewer than two following characters.
    TruncatedEscape,
    /// A `%` escape whose hex digits are not uppercase.
    LowercaseEscape,
    /// A `%` escape whose two characters are not hex digits.
    MalformedEscape,
    /// A byte outside the unreserved set appears without an escape.
    UnescapedByte,
    /// The decoded bytes are not UTF-8.
    InvalidUtf8,
    /// A second, non-canonical spelling of a name (e.g. `%2D` for `-`).
    NonCanonical,
}

impl HostSegmentError {
    /// The clause a diagnostic appends.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotAHostOwner => "it does not start with `__host__/`",
            Self::TruncatedEscape => "a `%` escape is cut short",
            Self::LowercaseEscape => "a `%` escape uses lowercase hex; the canonical form is `%HH`",
            Self::MalformedEscape => "a `%` is not followed by two hex digits",
            Self::UnescapedByte => {
                "a byte outside `A-Z a-z 0-9 - . _ ~` appears unescaped; write it as `%HH`"
            }
            Self::InvalidUtf8 => "the escapes decode to bytes that are not UTF-8",
            Self::NonCanonical => {
                "it is a second spelling of the same name; only the shortest canonical encoding is a host identity"
            }
        }
    }
}

impl fmt::Display for HostSegmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

impl std::error::Error for HostSegmentError {}

const fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[path = "host_owner/tests.rs"]
mod tests;
