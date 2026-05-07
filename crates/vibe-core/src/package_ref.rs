//! Package identity: `<kind>:<name>[@<version>]`.
//!
//! Spec: `VIBEVM-SPEC.md` §4.1, §7.1.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// One of the four installable package kinds.
///
/// Spec: `VIBEVM-SPEC.md` §4.1. This enum is closed; adding a fifth kind is a
/// spec change, not a code change.
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
/// Parsed from strings like `flow:wal`, `flow:wal@0.3.0`, or `flow:wal@^0.3`.
///
/// Serde support goes via the string wire form: on the wire a `PackageRef`
/// is always a single string, parsed through [`PackageRef::parse`]. This
/// lets it appear inline in TOML arrays (e.g. `requires.packages =
/// ["flow:wal@^0.1"]`) and makes the schema self-documenting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackageRef {
    pub kind: PackageKind,
    pub name: String,
    pub version: VersionSpec,
}

impl PackageRef {
    pub fn new(kind: PackageKind, name: impl Into<String>, version: VersionSpec) -> Result<Self> {
        let name = name.into();
        validate_package_name(&name)?;
        Ok(PackageRef {
            kind,
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

        let (kind_str, name_str) =
            prefix
                .split_once(':')
                .ok_or_else(|| Error::BadPackageRef {
                    input: input.to_owned(),
                    reason: "expected `<kind>:<name>[@<version>]` — missing `:`".into(),
                })?;

        let kind = PackageKind::from_str(kind_str)?;
        validate_package_name(name_str)?;

        let version = match version_part {
            None => VersionSpec::Latest,
            Some(v) => VersionSpec::parse(v)?,
        };

        Ok(PackageRef {
            kind,
            name: name_str.to_owned(),
            version,
        })
    }

    /// Just the `<kind>:<name>` portion, no version. Useful as a key.
    pub fn qualified_name(&self) -> String {
        format!("{}:{}", self.kind, self.name)
    }
}

impl fmt::Display for PackageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.name)?;
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
mod tests {
    use super::*;

    #[test]
    fn kind_roundtrip() {
        for kind in PackageKind::ALL {
            let s = kind.to_string();
            let parsed: PackageKind = s.parse().unwrap();
            assert_eq!(kind, parsed);
        }
    }

    #[test]
    fn kind_rejects_unknown() {
        let err = "widget".parse::<PackageKind>().unwrap_err();
        assert!(matches!(err, Error::BadPackageKind(_)));
    }

    #[test]
    fn name_accepts_valid_kebab() {
        for name in ["wal", "welcome-page", "auth-email", "x", "a-b-c", "h2o"] {
            validate_package_name(name).unwrap_or_else(|_| panic!("should accept `{name}`"));
        }
    }

    #[test]
    fn name_rejects_invalid() {
        for name in [
            "",
            "-leading",
            "trailing-",
            "Double--hyphen",
            "Upper",
            "with_underscore",
            "with space",
            "unicode-😊",
        ] {
            assert!(
                validate_package_name(name).is_err(),
                "should reject `{name}`"
            );
        }
    }

    #[test]
    fn parse_bare() {
        let r = PackageRef::parse("flow:wal").unwrap();
        assert_eq!(r.kind, PackageKind::Flow);
        assert_eq!(r.name, "wal");
        assert_eq!(r.version, VersionSpec::Latest);
        assert_eq!(r.to_string(), "flow:wal");
    }

    #[test]
    fn parse_bare_semver_is_caret_per_cargo() {
        // Cargo / npm / Poetry semantics: a bare semver like `0.3.0`
        // is shorthand for `^0.3.0` (caret — compatible release).
        // To pin strictly equal, write `=0.3.0`. This matches what
        // every mainstream package manager does and avoids the
        // surprising "I wrote 0.3.0, why won't 0.3.1 install?" footgun.
        let r = PackageRef::parse("flow:wal@0.3.0").unwrap();
        assert_eq!(r.kind, PackageKind::Flow);
        assert_eq!(r.name, "wal");
        // `0.3.0` matches itself.
        assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
        // Caret behaviour for pre-1.0: matches the same minor, no farther.
        assert!(
            r.version.matches(&semver::Version::parse("0.3.5").unwrap()),
            "0.3.0 caret must accept 0.3.5"
        );
        assert!(
            !r.version.matches(&semver::Version::parse("0.4.0").unwrap()),
            "0.3.0 caret must reject 0.4.0 (different pre-1.0 minor)"
        );
    }

    #[test]
    fn parse_eq_version_is_exact() {
        // `=0.3.0` is the explicit exact form. Same Cargo notation.
        let r = PackageRef::parse("flow:wal@=0.3.0").unwrap();
        assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
        assert!(
            !r.version.matches(&semver::Version::parse("0.3.1").unwrap()),
            "=0.3.0 must reject 0.3.1"
        );
    }

    #[test]
    fn parse_range_version() {
        let r = PackageRef::parse("flow:wal@^0.3").unwrap();
        let v = semver::Version::parse("0.3.5").unwrap();
        assert!(r.version.matches(&v));
        let v2 = semver::Version::parse("0.4.0").unwrap();
        assert!(!r.version.matches(&v2));
    }

    #[test]
    fn parse_tilde_version() {
        // Tilde: `~0.3.1` → `>=0.3.1, <0.4.0`. Same Cargo / npm.
        let r = PackageRef::parse("flow:wal@~0.3.1").unwrap();
        assert!(r.version.matches(&semver::Version::parse("0.3.1").unwrap()));
        assert!(r.version.matches(&semver::Version::parse("0.3.5").unwrap()));
        assert!(!r.version.matches(&semver::Version::parse("0.4.0").unwrap()));
    }

    #[test]
    fn parse_all_kinds() {
        for kind in PackageKind::ALL {
            let s = format!("{kind}:thing");
            let r = PackageRef::parse(&s).unwrap();
            assert_eq!(r.kind, kind);
        }
    }

    #[test]
    fn parse_rejects_missing_colon() {
        let err = PackageRef::parse("flow.wal").unwrap_err();
        assert!(matches!(err, Error::BadPackageRef { .. }));
    }

    #[test]
    fn parse_rejects_bad_kind() {
        let err = PackageRef::parse("widget:thing").unwrap_err();
        assert!(matches!(err, Error::BadPackageKind(_)));
    }

    #[test]
    fn parse_rejects_empty_name() {
        let err = PackageRef::parse("flow:").unwrap_err();
        assert!(matches!(err, Error::BadPackageName(_)));
    }

    #[test]
    fn display_roundtrips_latest() {
        let r = PackageRef::parse("flow:wal").unwrap();
        assert_eq!(r.to_string(), "flow:wal");
    }

    #[test]
    fn display_roundtrips_exact() {
        let r = PackageRef::parse("flow:wal@0.3.0").unwrap();
        let back = r.to_string();
        let r2 = PackageRef::parse(&back).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn qualified_name_strips_version() {
        let r = PackageRef::parse("stack:rust-cli@0.1.0").unwrap();
        assert_eq!(r.qualified_name(), "stack:rust-cli");
    }

    #[test]
    fn empty_input_rejected() {
        assert!(PackageRef::parse("").is_err());
        assert!(PackageRef::parse("   ").is_err());
    }
}
