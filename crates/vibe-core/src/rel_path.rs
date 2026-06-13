//! Portable workspace-relative paths.
//!
//! Spec: [PROP-007](../../../spec/modules/vibe-workspace/PROP-007-workspace.md).

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#workspace-section");

use std::fmt;

use serde::{Deserialize, Serialize};

/// A forward-slashed path relative to a workspace's absolute root — a
/// member node's *portable identity* (PROP-007 §2.4). It is what the
/// lockfile records and what selects a member; an absolute path never is.
///
/// The newtype carries one invariant the bare `String` could not: the
/// stored form is always forward-slashed, so a `rel_path` written on
/// Windows and read on Linux names the same member. Construction
/// normalises `\\` to `/`; the type is `serde(transparent)`, so the wire
/// shape is unchanged (it appears as a plain string in TOML/JSON).
///
/// ```
/// use vibe_core::RelPath;
///
/// // Backslashes normalise — the portability invariant.
/// let p = RelPath::new("packages\\flow-wal");
/// assert_eq!(p.as_str(), "packages/flow-wal");
/// assert_eq!(p, "packages/flow-wal"); // compares against &str directly
/// assert_eq!(p.to_string(), "packages/flow-wal");
///
/// // The workspace root is named by ".".
/// assert_eq!(RelPath::root().as_str(), ".");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelPath(String);

impl RelPath {
    /// Wrap a relative path, normalising `\\` to `/` so the stored form
    /// is portable. Trailing slashes are trimmed; the empty string and
    /// `"."` both denote the root. Infallible because every call site is
    /// internal — the value comes from a filesystem walk or a lockfile
    /// this tool wrote.
    pub fn new(path: impl Into<String>) -> Self {
        let normalised = path.into().replace('\\', "/");
        let trimmed = normalised.trim_end_matches('/');
        if trimmed.is_empty() {
            RelPath(".".to_string())
        } else {
            RelPath(trimmed.to_string())
        }
    }

    /// The path that names the workspace root: `"."`.
    pub fn root() -> Self {
        RelPath(".".to_string())
    }

    /// The path as a forward-slashed string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `true` iff this names the workspace root (`"."`).
    pub fn is_root(&self) -> bool {
        self.0 == "."
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for RelPath {
    fn from(s: String) -> Self {
        RelPath::new(s)
    }
}

impl From<&str> for RelPath {
    fn from(s: &str) -> Self {
        RelPath::new(s)
    }
}

impl From<RelPath> for String {
    fn from(p: RelPath) -> String {
        p.0
    }
}

impl AsRef<str> for RelPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Ergonomic comparisons against string slices, so a `RelPath` field can
// be matched against a literal or a `&str` argument without an explicit
// `.as_str()` at every call site.
impl PartialEq<str> for RelPath {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for RelPath {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for RelPath {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<RelPath> for str {
    fn eq(&self, other: &RelPath) -> bool {
        self == other.0
    }
}

impl PartialEq<RelPath> for &str {
    fn eq(&self, other: &RelPath) -> bool {
        *self == other.0
    }
}
