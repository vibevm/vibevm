//! The `action://<group>/<name>[?<params>]` address — the behaviour-layer
//! twin of `spec://` (PROP-039 §2). `(group, name)` is the identity; the
//! optional `?<params>` query carries invocation parameters (§5) and is **not**
//! part of identity, `Eq`, or `Hash`.
//!
//! - `<group>` — a dotted namespace, each dot-segment one or more of
//!   `a`–`z`, `0`–`9`, `_`, `-` (e.g. `vibe.tree`, `core`).
//! - `<name>` — a dotted/kebab identifier, each dot-segment kebab-case
//!   (lowercase alphanumeric with internal single hyphens; e.g.
//!   `copy.markdown`, `search.everywhere`).
//!
//! Spec: [PROP-039 §2](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#addressing).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#addressing");

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The URI scheme prefix every action address carries.
const SCHEME: &str = "action://";

/// Parsed `&`-separated `key=value` query pairs lifted off an address.
///
/// Not part of [`ActionAddr`] identity — produced only by
/// [`ActionAddr::parse_uri`] for a caller that wants the invocation
/// parameters embedded in the textual form.
pub type QueryPairs = Vec<(String, String)>;

/// A malformed action address. Parsing never panics — every rejection is this
/// typed error (PROP-039 §2.1, `#address-parse`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-actions/PROP-039#address-grammar")]
pub enum AddrError {
    /// The input does not match the `action://<group>/<name>` grammar.
    #[error(
        "invalid action address `{input}`: {reason} \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#address-grammar; \
          fix: write it as `action://<group>/<name>` with a dotted group and a \
          dotted/kebab name)"
    )]
    Malformed {
        /// The offending input, verbatim.
        input: String,
        /// Why it was rejected.
        reason: String,
    },
}

impl AddrError {
    fn malformed(input: &str, reason: impl Into<String>) -> Self {
        AddrError::Malformed {
            input: input.to_owned(),
            reason: reason.into(),
        }
    }
}

/// An action's address — its stable identity `(group, name)`.
///
/// The textual form is `action://<group>/<name>`; the optional `?<params>`
/// query is parsed separately (see [`ActionAddr::parse_uri`]) and is never
/// stored, so two addresses that differ only by query are equal.
///
/// ```
/// use vibe_actions::ActionAddr;
///
/// let a: ActionAddr = "action://vibe.tree/copy.markdown".parse().unwrap();
/// assert_eq!(a.group(), "vibe.tree");
/// assert_eq!(a.name(), "copy.markdown");
/// // Display round-trips the identity.
/// assert_eq!(a.to_string(), "action://vibe.tree/copy.markdown");
/// assert_eq!("action://vibe.tree/copy.markdown".parse::<ActionAddr>().unwrap(), a);
///
/// // The `?query` is not part of identity.
/// let b: ActionAddr = "action://vibe.tree/copy.markdown?fmt=gfm".parse().unwrap();
/// assert_eq!(a, b);
///
/// // Malformed input is a typed error, never a panic.
/// assert!("vibe.tree/copy".parse::<ActionAddr>().is_err()); // no scheme
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActionAddr {
    group: String,
    name: String,
}

impl ActionAddr {
    /// Construct an address from already-separated parts, validating both
    /// against the grammar (§2.1).
    pub fn new(group: impl Into<String>, name: impl Into<String>) -> Result<Self, AddrError> {
        let group = group.into();
        let name = name.into();
        validate_group(&group).map_err(|reason| AddrError::malformed(&group, reason))?;
        validate_name(&name).map_err(|reason| AddrError::malformed(&name, reason))?;
        Ok(ActionAddr { group, name })
    }

    /// Parse the textual identity form `action://<group>/<name>`, tolerating
    /// (and discarding) a trailing `?<params>` query. The query is *not* part
    /// of identity; use [`ActionAddr::parse_uri`] to recover it.
    pub fn parse(input: &str) -> Result<Self, AddrError> {
        Self::parse_uri(input).map(|(addr, _)| addr)
    }

    /// Parse the full URI, returning the identity address **and** the parsed
    /// query pairs (§2.1, §5). The query never enters the address.
    pub fn parse_uri(input: &str) -> Result<(Self, QueryPairs), AddrError> {
        let trimmed = input.trim();
        let rest = trimmed
            .strip_prefix(SCHEME)
            .ok_or_else(|| AddrError::malformed(input, "missing `action://` scheme"))?;

        let (group, name_and_query) = rest
            .split_once('/')
            .ok_or_else(|| AddrError::malformed(input, "missing `/` between group and name"))?;

        let (name, query) = match name_and_query.split_once('?') {
            Some((name, query)) => (name, Some(query)),
            None => (name_and_query, None),
        };

        validate_group(group).map_err(|reason| AddrError::malformed(input, reason))?;
        validate_name(name).map_err(|reason| AddrError::malformed(input, reason))?;

        let pairs = query.map(parse_query).unwrap_or_default();
        Ok((
            ActionAddr {
                group: group.to_owned(),
                name: name.to_owned(),
            },
            pairs,
        ))
    }

    /// The dotted namespace half, e.g. `vibe.tree`.
    pub fn group(&self) -> &str {
        &self.group
    }

    /// The dotted/kebab name half, e.g. `copy.markdown`.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ActionAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{SCHEME}{}/{}", self.group, self.name)
    }
}

impl FromStr for ActionAddr {
    type Err = AddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ActionAddr::parse(s)
    }
}

impl TryFrom<String> for ActionAddr {
    type Error = AddrError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        ActionAddr::parse(&s)
    }
}

impl From<ActionAddr> for String {
    fn from(a: ActionAddr) -> String {
        a.to_string()
    }
}

/// Split a raw `key=value&key=value` query into pairs. A token with no `=` is
/// kept as a key with an empty value; empty tokens are dropped. Lenient by
/// design — the schema-typed validation happens in [`crate::params`], not here.
fn parse_query(query: &str) -> QueryPairs {
    query
        .split('&')
        .filter(|token| !token.is_empty())
        .map(|token| match token.split_once('=') {
            Some((k, v)) => (k.to_owned(), v.to_owned()),
            None => (token.to_owned(), String::new()),
        })
        .collect()
}

/// A group is one or more dot-joined segments, each a non-empty run of
/// `a`–`z`, `0`–`9`, `_`, `-`.
fn validate_group(group: &str) -> Result<(), String> {
    if group.is_empty() {
        return Err("empty group".to_owned());
    }
    for segment in group.split('.') {
        if segment.is_empty() {
            return Err("empty group segment — segments are joined by single dots".to_owned());
        }
        if let Some(bad) = segment
            .chars()
            .find(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
        {
            return Err(format!(
                "illegal character `{bad}` in group segment `{segment}` — each is one \
                 or more of `a`–`z`, `0`–`9`, `_`, `-`"
            ));
        }
    }
    Ok(())
}

/// A name is one or more dot-joined segments, each kebab-case: a non-empty run
/// of lowercase alphanumerics with internal single hyphens (first and last
/// characters alphanumeric, no doubled hyphen).
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("empty name".to_owned());
    }
    for segment in name.split('.') {
        validate_kebab_segment(segment)?;
    }
    Ok(())
}

fn validate_kebab_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err("empty name segment — segments are joined by single dots".to_owned());
    }
    let bytes = segment.as_bytes();
    let mut prev_hyphen = false;
    for (idx, b) in bytes.iter().enumerate() {
        let is_edge = idx == 0 || idx == bytes.len() - 1;
        match b {
            b'a'..=b'z' | b'0'..=b'9' => prev_hyphen = false,
            b'-' if !is_edge && !prev_hyphen => prev_hyphen = true,
            b'-' => {
                return Err(format!(
                    "misplaced hyphen in name segment `{segment}` — hyphens must be \
                     internal and single"
                ));
            }
            other => {
                return Err(format!(
                    "illegal character `{}` in name segment `{segment}` — each is \
                     kebab-case (`a`–`z`, `0`–`9`, internal single hyphens)",
                    *other as char
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_round_trips_identity() {
        let a = ActionAddr::parse("action://vibe.tree/copy.markdown").unwrap();
        assert_eq!(a.group(), "vibe.tree");
        assert_eq!(a.name(), "copy.markdown");
        let printed = a.to_string();
        assert_eq!(printed, "action://vibe.tree/copy.markdown");
        assert_eq!(ActionAddr::parse(&printed).unwrap(), a);
    }

    #[test]
    fn parses_core_group_and_dotted_name() {
        let a = ActionAddr::parse("action://core/search.everywhere").unwrap();
        assert_eq!(a.group(), "core");
        assert_eq!(a.name(), "search.everywhere");
    }

    #[test]
    fn query_is_not_part_of_identity() {
        let bare = ActionAddr::parse("action://vibe.tree/sort").unwrap();
        let (with_query, pairs) =
            ActionAddr::parse_uri("action://vibe.tree/sort?by=name&dir=asc").unwrap();
        assert_eq!(bare, with_query);
        assert_eq!(bare.to_string(), "action://vibe.tree/sort"); // Display omits query
        assert_eq!(
            pairs,
            vec![
                ("by".to_owned(), "name".to_owned()),
                ("dir".to_owned(), "asc".to_owned()),
            ]
        );
    }

    #[test]
    fn identity_hashes_ignore_query() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ActionAddr::parse("action://vibe.tree/sort?by=name").unwrap());
        assert!(set.contains(&ActionAddr::parse("action://vibe.tree/sort?by=size").unwrap()));
    }

    #[test]
    fn rejects_missing_scheme() {
        let err = ActionAddr::parse("vibe.tree/copy.markdown").unwrap_err();
        assert!(matches!(err, AddrError::Malformed { .. }));
    }

    #[test]
    fn rejects_missing_name_separator() {
        assert!(matches!(
            ActionAddr::parse("action://vibe.tree").unwrap_err(),
            AddrError::Malformed { .. }
        ));
    }

    #[test]
    fn rejects_empty_group_and_name() {
        assert!(ActionAddr::parse("action:///copy.markdown").is_err());
        assert!(ActionAddr::parse("action://vibe.tree/").is_err());
    }

    #[test]
    fn rejects_uppercase_and_bad_chars() {
        assert!(ActionAddr::parse("action://Vibe.Tree/copy").is_err());
        assert!(ActionAddr::parse("action://vibe.tree/Copy.Markdown").is_err());
        assert!(ActionAddr::parse("action://vibe.tree/copy markdown").is_err());
    }

    #[test]
    fn rejects_doubled_and_edge_hyphens_in_name() {
        assert!(ActionAddr::parse("action://vibe.tree/copy--markdown").is_err());
        assert!(ActionAddr::parse("action://vibe.tree/-copy").is_err());
        assert!(ActionAddr::parse("action://vibe.tree/copy-").is_err());
    }

    #[test]
    fn rejects_empty_dotted_segments() {
        assert!(ActionAddr::parse("action://vibe..tree/copy").is_err());
        assert!(ActionAddr::parse("action://vibe.tree/copy..markdown").is_err());
    }

    #[test]
    fn new_validates_parts() {
        assert!(ActionAddr::new("vibe.tree", "copy.markdown").is_ok());
        assert!(ActionAddr::new("Vibe", "copy").is_err());
        assert!(ActionAddr::new("vibe.tree", "Copy").is_err());
    }

    #[test]
    fn serde_round_trip_via_string() {
        let a = ActionAddr::parse("action://vibe.tree/copy.markdown").unwrap();
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, r#""action://vibe.tree/copy.markdown""#);
        let back: ActionAddr = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
