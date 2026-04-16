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
/// - `flow:wal@0.3.0` → `Req(=0.3.0)` (exact).
/// - `flow:wal@^0.3` → `Req(^0.3)` (semver range).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSpec {
    /// No version given — resolve to the latest stable.
    Latest,
    /// Explicit version requirement. Bare semver (e.g. `0.3.0`) is treated as
    /// exact (`=0.3.0`) to match the spec's "installs exactly that version"
    /// wording; anything starting with `^`, `~`, `>=`, `<`, `*` or a comma-
    /// separated list is parsed as a full `semver::VersionReq`.
    Req(semver::VersionReq),
}

impl VersionSpec {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(VersionSpec::Latest);
        }

        if let Ok(version) = semver::Version::parse(trimmed) {
            let req_src = format!("={version}");
            let req = semver::VersionReq::parse(&req_src).map_err(|source| {
                Error::BadVersionSpec {
                    input: input.to_owned(),
                    source,
                }
            })?;
            return Ok(VersionSpec::Req(req));
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Kebab-case: one or more lowercase ASCII alphanumeric segments joined by
/// single hyphens. First and last characters must be alphanumeric; consecutive
/// hyphens are rejected.
fn validate_package_name(name: &str) -> Result<()> {
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
    fn parse_exact_version() {
        let r = PackageRef::parse("flow:wal@0.3.0").unwrap();
        assert_eq!(r.kind, PackageKind::Flow);
        assert_eq!(r.name, "wal");
        let v = semver::Version::parse("0.3.0").unwrap();
        assert!(r.version.matches(&v));
        let v2 = semver::Version::parse("0.3.1").unwrap();
        assert!(!r.version.matches(&v2), "`0.3.0` must not match 0.3.1");
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
