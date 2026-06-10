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
    pub name: String,
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
        let name = name.into();
        validate_package_name(&name)?;
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
        validate_package_name(name_str)?;

        let version = match version_part {
            None => VersionSpec::Latest,
            Some(v) => VersionSpec::parse(v)?,
        };

        Ok(PackageRef {
            kind,
            group,
            name: name_str.to_owned(),
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
            None => self.name.clone(),
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
        f.write_str(&self.name)?;
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
    fn group_accepts_valid() {
        for g in [
            "org.vibevm",
            "com.acme",
            "a",
            "x.y.z",
            "dev.example-team",
            "org.vibevm_internal",
            "h2o.mol",
        ] {
            Group::parse(g).unwrap_or_else(|_| panic!("should accept `{g}`"));
        }
    }

    #[test]
    fn group_rejects_invalid() {
        for g in [
            "",
            "   ",
            ".org",
            "org.",
            "org..vibevm",
            "Org.Vibevm",
            "org vibevm",
            "org/vibevm",
            "org.vibevm!",
            "org:vibevm",
        ] {
            assert!(Group::parse(g).is_err(), "should reject `{g}`");
        }
    }

    #[test]
    fn group_display_roundtrips() {
        let g = Group::parse("org.vibevm").unwrap();
        assert_eq!(g.to_string(), "org.vibevm");
        assert_eq!(g.as_str(), "org.vibevm");
        let back: Group = g.to_string().parse().unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn group_trims_whitespace() {
        let g = Group::parse("  org.vibevm  ").unwrap();
        assert_eq!(g.as_str(), "org.vibevm");
    }

    #[test]
    fn group_serde_via_string() {
        let g = Group::parse("com.acme").unwrap();
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(json, r#""com.acme""#);
        let back: Group = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn parse_short_bare() {
        let r = PackageRef::parse("wal").unwrap();
        assert_eq!(r.kind, None);
        assert_eq!(r.group, None);
        assert_eq!(r.name, "wal");
        assert_eq!(r.version, VersionSpec::Latest);
        assert_eq!(r.to_string(), "wal");
        assert!(!r.is_qualified());
    }

    #[test]
    fn parse_short_with_kind() {
        let r = PackageRef::parse("flow:wal").unwrap();
        assert_eq!(r.kind, Some(PackageKind::Flow));
        assert_eq!(r.group, None);
        assert_eq!(r.name, "wal");
        assert_eq!(r.to_string(), "flow:wal");
    }

    #[test]
    fn parse_qualified() {
        let r = PackageRef::parse("org.vibevm/wal").unwrap();
        assert_eq!(r.kind, None);
        assert_eq!(r.group.as_ref().unwrap().as_str(), "org.vibevm");
        assert_eq!(r.name, "wal");
        assert!(r.is_qualified());
        assert_eq!(r.to_string(), "org.vibevm/wal");
    }

    #[test]
    fn parse_qualified_with_kind() {
        let r = PackageRef::parse("flow:org.vibevm/wal").unwrap();
        assert_eq!(r.kind, Some(PackageKind::Flow));
        assert_eq!(r.group.as_ref().unwrap().as_str(), "org.vibevm");
        assert_eq!(r.name, "wal");
        assert_eq!(r.to_string(), "flow:org.vibevm/wal");
    }

    #[test]
    fn parse_bare_semver_is_caret_per_cargo() {
        // Cargo / npm / Poetry semantics: a bare semver like `0.3.0` is
        // shorthand for `^0.3.0` (caret — compatible release). To pin
        // strictly equal, write `=0.3.0`. Holds across every pkgref form.
        for s in [
            "wal@0.3.0",
            "flow:wal@0.3.0",
            "org.vibevm/wal@0.3.0",
            "flow:org.vibevm/wal@0.3.0",
        ] {
            let r = PackageRef::parse(s).unwrap();
            assert_eq!(r.name, "wal");
            assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
            assert!(
                r.version.matches(&semver::Version::parse("0.3.5").unwrap()),
                "{s}: 0.3.0 caret must accept 0.3.5"
            );
            assert!(
                !r.version.matches(&semver::Version::parse("0.4.0").unwrap()),
                "{s}: 0.3.0 caret must reject 0.4.0"
            );
        }
    }

    #[test]
    fn parse_eq_version_is_exact() {
        let r = PackageRef::parse("org.vibevm/wal@=0.3.0").unwrap();
        assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
        assert!(
            !r.version.matches(&semver::Version::parse("0.3.1").unwrap()),
            "=0.3.0 must reject 0.3.1"
        );
    }

    #[test]
    fn parse_range_and_tilde_versions() {
        let caret = PackageRef::parse("org.vibevm/wal@^0.3").unwrap();
        assert!(
            caret
                .version
                .matches(&semver::Version::parse("0.3.5").unwrap())
        );
        assert!(
            !caret
                .version
                .matches(&semver::Version::parse("0.4.0").unwrap())
        );
        let tilde = PackageRef::parse("org.vibevm/wal@~0.3.1").unwrap();
        assert!(
            tilde
                .version
                .matches(&semver::Version::parse("0.3.5").unwrap())
        );
        assert!(
            !tilde
                .version
                .matches(&semver::Version::parse("0.4.0").unwrap())
        );
    }

    #[test]
    fn parse_all_kinds_in_prefix() {
        for kind in PackageKind::ALL {
            let r = PackageRef::parse(&format!("{kind}:org.vibevm/thing")).unwrap();
            assert_eq!(r.kind, Some(kind));
        }
    }

    #[test]
    fn parse_rejects_bad_kind() {
        assert!(matches!(
            PackageRef::parse("widget:wal").unwrap_err(),
            Error::BadPackageKind(_)
        ));
        assert!(matches!(
            PackageRef::parse("widget:org.vibevm/wal").unwrap_err(),
            Error::BadPackageKind(_)
        ));
    }

    #[test]
    fn parse_rejects_bad_group() {
        // Uppercase in the group segment — `Group::parse` rejects it.
        assert!(matches!(
            PackageRef::parse("Org.Vibevm/wal").unwrap_err(),
            Error::BadGroup { .. }
        ));
    }

    #[test]
    fn parse_rejects_bad_name() {
        // Empty name after the group separator.
        assert!(PackageRef::parse("org.vibevm/").is_err());
        // Empty name after the kind prefix.
        assert!(matches!(
            PackageRef::parse("flow:").unwrap_err(),
            Error::BadPackageName(_)
        ));
        // A dot in the name — no `:`/`/`, so the whole token is the name,
        // and kebab-case forbids the dot.
        assert!(matches!(
            PackageRef::parse("flow.wal").unwrap_err(),
            Error::BadPackageName(_)
        ));
    }

    #[test]
    fn display_round_trips_every_form() {
        for s in [
            "wal",
            "flow:wal",
            "org.vibevm/wal",
            "flow:org.vibevm/wal",
            "org.vibevm/wal@^0.3",
            "flow:org.vibevm/wal@=0.3.0",
        ] {
            let r = PackageRef::parse(s).unwrap();
            let r2 = PackageRef::parse(&r.to_string()).unwrap();
            assert_eq!(r, r2, "round-trip failed for `{s}`");
        }
    }

    #[test]
    fn qualified_name_is_the_identity_string() {
        // kind and version drop; `<group>/<name>` is the identity.
        let q = PackageRef::parse("flow:org.vibevm/wal@0.1.0").unwrap();
        assert_eq!(q.qualified_name(), "org.vibevm/wal");
        // No group yet — the bare name is the best identity available.
        let short = PackageRef::parse("wal@0.1.0").unwrap();
        assert_eq!(short.qualified_name(), "wal");
    }

    #[test]
    fn empty_input_rejected() {
        assert!(PackageRef::parse("").is_err());
        assert!(PackageRef::parse("   ").is_err());
    }

    #[test]
    fn serde_round_trips_via_string() {
        let r = PackageRef::parse("flow:org.vibevm/wal@^0.3").unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#""flow:org.vibevm/wal@^0.3""#);
        let back: PackageRef = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}
