//! Package identity — the pkgref grammar `[<kind>:][<group>/]<name>[@<version>]`
//! (PROP-008 §2.4) and its components: [`PackageKind`], [`Group`], [`PackageRef`].
//!
//! Spec: `VIBEVM-SPEC.md` §4.1, §7.1; PROP-008.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#pkgref");

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// One of the four installable package kinds.
///
/// Spec: `VIBEVM-SPEC.md` §4.1. This enum is closed; adding a fifth kind is a
/// spec change, not a code change.
///
/// ```
/// use vibe_core::PackageKind;
///
/// let k: PackageKind = "feat".parse().unwrap();
/// assert_eq!(k, PackageKind::Feat);
/// assert_eq!(k.as_str(), "feat");
/// assert!("widget".parse::<PackageKind>().is_err()); // closed set
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageKind {
    Flow,
    Feat,
    Stack,
    Tool,
}

impl PackageKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            PackageKind::Flow => "flow",
            PackageKind::Feat => "feat",
            PackageKind::Stack => "stack",
            PackageKind::Tool => "tool",
        }
    }

    pub const ALL: [PackageKind; 4] = [
        PackageKind::Flow,
        PackageKind::Feat,
        PackageKind::Stack,
        PackageKind::Tool,
    ];
}

impl fmt::Display for PackageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PackageKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "flow" => Ok(PackageKind::Flow),
            "feat" => Ok(PackageKind::Feat),
            "stack" => Ok(PackageKind::Stack),
            "tool" => Ok(PackageKind::Tool),
            other => Err(Error::BadPackageKind(other.to_owned())),
        }
    }
}

/// A package's **group** — the reverse-FQDN namespace qualifier (PROP-008
/// §2.1).
///
/// Wire form: a dot-separated string of one or more segments, each segment
/// one or more characters from `a`–`z`, `0`–`9`, `_`, `-` — e.g.
/// `org.vibevm`, `com.acme`, `dev.example-team`.
///
/// Reverse-FQDN is the **recommended** convention, but the core does **not**
/// enforce it: whether a group looks like a reversed domain is a matter of
/// style, left to humans and linters — exactly as Maven does not enforce
/// `groupId` shape. Together with `name`, a group forms a package's
/// identity: `name` is unique *within* a group (PROP-008 §2.2), so
/// `(group, name)` identifies a package without `kind`.
///
/// ```
/// use vibe_core::Group;
///
/// let g = Group::parse("org.vibevm").unwrap();
/// assert_eq!(g.as_str(), "org.vibevm");
/// assert!(Group::parse("Org.Vibevm").is_err());  // uppercase rejected
/// assert!(Group::parse("org..vibevm").is_err()); // empty segment rejected
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Group(String);

impl Group {
    /// Parse and validate a group string. Surrounding whitespace is trimmed;
    /// the stored form is the trimmed string.
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(Error::BadGroup {
                input: input.to_owned(),
                reason: "empty input".into(),
            });
        }
        for segment in trimmed.split('.') {
            if segment.is_empty() {
                return Err(Error::BadGroup {
                    input: input.to_owned(),
                    reason: "empty segment — segments are joined by single dots, with no \
                             leading, trailing, or doubled dot"
                        .into(),
                });
            }
            if let Some(bad) = segment
                .chars()
                .find(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
            {
                return Err(Error::BadGroup {
                    input: input.to_owned(),
                    reason: format!(
                        "illegal character `{bad}` in segment `{segment}` — each segment \
                         is one or more of `a`–`z`, `0`–`9`, `_`, `-`"
                    ),
                });
            }
        }
        Ok(Group(trimmed.to_owned()))
    }

    /// The group as a string slice — e.g. `org.vibevm`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Group {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Group::parse(s)
    }
}

impl TryFrom<String> for Group {
    type Error = Error;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        Group::parse(&s)
    }
}

impl From<Group> for String {
    fn from(g: Group) -> String {
        g.0
    }
}

/// A package's **name** — kebab-case, unique within its [`Group`]
/// (PROP-008 §2.2).
///
/// The grammar is the shared kebab-case rule ([`validate_package_name`]):
/// one or more lowercase ASCII alphanumeric segments joined by single
/// hyphens, first and last characters alphanumeric, no doubled hyphens.
/// `serde(transparent)`, so the wire form is the bare string a manifest
/// or lockfile already carries; the validation lives in the constructor
/// and at the [`PackageRef`] parse seam.
///
/// ```
/// use vibe_core::PackageRef;
///
/// let r = PackageRef::parse("org.vibevm/wal").unwrap();
/// assert_eq!(r.name.as_str(), "wal");
/// assert_eq!(r.name, "wal"); // compares against &str directly
///
/// // The name grammar is enforced at the parse seam:
/// assert!(PackageRef::parse("org.vibevm/Not-Kebab").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PackageName(String);

impl PackageName {
    /// Parse and validate a package name against the kebab-case grammar.
    pub fn parse(input: &str) -> Result<Self> {
        validate_package_name(input)?;
        Ok(PackageName(input.to_owned()))
    }

    /// Wrap a string already proven to be a valid package name — one
    /// reconstructed from a `(group, name)` identity that itself came
    /// from a validated [`PackageRef`]. The resolver and registry layers
    /// carry names as bare strings internally, but only ever names that
    /// already passed [`PackageName::parse`] at the input boundary, so
    /// re-validating here would be a check that can never fail. For
    /// untrusted input use [`PackageName::parse`].
    pub fn from_validated(name: String) -> Self {
        PackageName(name)
    }

    /// The name as a string slice — e.g. `wal`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// Read-ergonomics: a validated name reads as the `str` it wraps, so it
// passes anywhere a `&str` argument is expected and answers `str`
// methods directly. Construction stays the guarded seam (`parse` /
// `from_validated`); `Deref` grants no way to forge an invalid name.
impl std::ops::Deref for PackageName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl FromStr for PackageName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        PackageName::parse(s)
    }
}

impl From<PackageName> for String {
    fn from(n: PackageName) -> String {
        n.0
    }
}

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for PackageName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for PackageName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for PackageName {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<PackageName> for String {
    fn eq(&self, other: &PackageName) -> bool {
        self == &other.0
    }
}

impl PartialEq<PackageName> for str {
    fn eq(&self, other: &PackageName) -> bool {
        self == other.0
    }
}

impl PartialEq<PackageName> for &str {
    fn eq(&self, other: &PackageName) -> bool {
        *self == other.0
    }
}

/// What the user wrote after `@` (if anything).
///
/// Spec examples (`VIBEVM-SPEC.md` §7.1):
/// - `flow:wal` → `Latest`.
/// - `flow:wal@0.3.0` → `Req(^0.3.0)` (caret — same Cargo / npm /
///   Poetry default; "compatible release"). Use `=0.3.0` for the
///   strict-equal form.
/// - `flow:wal@=0.3.0` → `Req(=0.3.0)` (exact).
/// - `flow:wal@^0.3` → `Req(^0.3)` (semver caret range).
/// - `flow:wal@~0.3.1` → `Req(~0.3.1)` (tilde range).
/// - `flow:wal@>=0.2, <1.0` → compound constraint (any
///   `semver::VersionReq` syntax).
///
/// ```
/// use vibe_core::VersionSpec;
///
/// // Bare semver is caret — the Cargo / npm / Poetry default.
/// let v = VersionSpec::parse("0.3.0").unwrap();
/// assert!(v.matches(&"0.3.5".parse().unwrap()));
/// assert!(!v.matches(&"0.4.0".parse().unwrap()));
/// // Empty input resolves to the latest stable.
/// assert_eq!(VersionSpec::parse("").unwrap(), VersionSpec::Latest);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSpec {
    /// No version given — resolve to the latest stable.
    Latest,
    /// Explicit version requirement parsed through
    /// [`semver::VersionReq`]. Same parser Cargo uses, so bare
    /// semver like `0.3.0` is treated as caret `^0.3.0`. To pin
    /// strictly equal, write `=0.3.0`.
    Req(semver::VersionReq),
}

impl VersionSpec {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(VersionSpec::Latest);
        }
        let req = semver::VersionReq::parse(trimmed).map_err(|source| Error::BadVersionSpec {
            input: input.to_owned(),
            source,
        })?;
        Ok(VersionSpec::Req(req))
    }

    pub fn matches(&self, version: &semver::Version) -> bool {
        match self {
            VersionSpec::Latest => true,
            VersionSpec::Req(req) => req.matches(version),
        }
    }
}

impl fmt::Display for VersionSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionSpec::Latest => Ok(()),
            VersionSpec::Req(req) => write!(f, "{req}"),
        }
    }
}

/// A reference to an installable package.
///
/// The pkgref grammar (PROP-008 §2.4):
///
/// ```text
/// pkgref := [ <kind> ":" ] [ <group> "/" ] <name> [ "@" <version> ]
/// ```
///
/// - `org.vibevm/wal` — qualified; the form written into manifests.
/// - `flow:org.vibevm/wal` — qualified, with a `kind` prefix (validated
///   against the resolved manifest, never used to disambiguate — `(group,
///   name)` is already unique).
/// - `wal` — short; CLI-only sugar, resolved to the qualified form at the
///   input boundary via the index (PROP-008 §2.6).
/// - `flow:wal` — short, with a `kind` prefix.
///
/// `group` is `None` only for a short ref still awaiting resolution; once
/// resolved — and always inside a manifest — it is `Some`. `kind` is `None`
/// whenever the optional prefix was omitted.
///
/// Serde goes via the string wire form, so a `PackageRef` appears inline in
/// TOML (e.g. as a `[requires.packages]` key) and the schema is
/// self-documenting.
///
/// ```
/// use vibe_core::PackageRef;
///
/// let r = PackageRef::parse("flow:org.vibevm/wal@^0.3").unwrap();
/// assert_eq!(r.qualified_name(), "org.vibevm/wal");
/// assert!(r.is_qualified());
/// // Display round-trips the canonical wire form.
/// assert_eq!(r.to_string(), "flow:org.vibevm/wal@^0.3");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackageRef {
    /// Optional `kind` prefix. Present only when the pkgref was written with
    /// one; it is validated against the resolved manifest (a `KindMismatch`)
    /// but never disambiguates — `(group, name)` is already unique
    /// (PROP-008 §2.3 / §2.4).
    pub kind: Option<PackageKind>,
    /// Reverse-FQDN group. `None` only for an unresolved short CLI ref; a
    /// manifest pkgref is always qualified (PROP-008 §2.6).
    pub group: Option<Group>,
    pub name: PackageName,
    pub version: VersionSpec,
}

impl PackageRef {
    /// Construct a `PackageRef` from already-typed parts. `name` is
    /// re-validated as kebab-case.
    pub fn new(
        kind: Option<PackageKind>,
        group: Option<Group>,
        name: impl Into<String>,
        version: VersionSpec,
    ) -> Result<Self> {
        let name = PackageName::parse(&name.into())?;
        Ok(PackageRef {
            kind,
            group,
            name,
            version,
        })
    }

    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(Error::BadPackageRef {
                input: input.to_owned(),
                reason: "empty input".into(),
            });
        }

        let (prefix, version_part) = match trimmed.split_once('@') {
            Some((p, v)) => (p, Some(v)),
            None => (trimmed, None),
        };

        // `[ <kind> ":" ] [ <group> "/" ] <name>` — the `kind` prefix is
        // delimited by `:`, the `group` by `/`. Both are optional.
        let (kind, after_kind) = match prefix.split_once(':') {
            Some((k, rest)) => (Some(PackageKind::from_str(k)?), rest),
            None => (None, prefix),
        };
        let (group, name_str) = match after_kind.split_once('/') {
            Some((g, n)) => (Some(Group::parse(g)?), n),
            None => (None, after_kind),
        };
        let name = PackageName::parse(name_str)?;

        let version = match version_part {
            None => VersionSpec::Latest,
            Some(v) => VersionSpec::parse(v)?,
        };

        Ok(PackageRef {
            kind,
            group,
            name,
            version,
        })
    }

    /// The version-stripped identity string. For a qualified ref this is
    /// `<group>/<name>` — the `(group, name)` identity (PROP-008 §2.2); for
    /// an unresolved short ref (no group) it is the bare `<name>`. The
    /// `kind` prefix is never part of it — `kind` is metadata, not identity.
    /// Useful as a map key once refs are qualified.
    pub fn qualified_name(&self) -> String {
        match &self.group {
            Some(group) => format!("{group}/{}", self.name),
            None => self.name.to_string(),
        }
    }

    /// `true` when this ref carries a `group` — fully qualified, no index
    /// resolution needed.
    pub fn is_qualified(&self) -> bool {
        self.group.is_some()
    }
}

impl fmt::Display for PackageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(kind) = self.kind {
            write!(f, "{kind}:")?;
        }
        if let Some(group) = &self.group {
            write!(f, "{group}/")?;
        }
        f.write_str(self.name.as_str())?;
        match &self.version {
            VersionSpec::Latest => Ok(()),
            VersionSpec::Req(req) => write!(f, "@{req}"),
        }
    }
}

impl FromStr for PackageRef {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        PackageRef::parse(s)
    }
}

impl TryFrom<String> for PackageRef {
    type Error = Error;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        PackageRef::parse(&s)
    }
}

impl From<PackageRef> for String {
    fn from(r: PackageRef) -> String {
        r.to_string()
    }
}

/// Kebab-case: one or more lowercase ASCII alphanumeric segments joined by
/// single hyphens. First and last characters must be alphanumeric; consecutive
/// hyphens are rejected.
///
/// Used to validate both package names (`<kind>:<name>`) and capability
/// segments (`<namespace>:<name>` in [`crate::capability_ref::CapabilityRef`]).
pub(crate) fn validate_package_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::BadPackageName(name.to_owned()));
    }
    let bytes = name.as_bytes();
    let mut prev_was_hyphen = false;
    for (idx, b) in bytes.iter().enumerate() {
        let is_edge = idx == 0 || idx == bytes.len() - 1;
        match b {
            b'a'..=b'z' | b'0'..=b'9' => prev_was_hyphen = false,
            b'-' if !is_edge && !prev_was_hyphen => prev_was_hyphen = true,
            _ => return Err(Error::BadPackageName(name.to_owned())),
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "package_ref/tests.rs"]
mod tests;
