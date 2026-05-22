//! Capability references.
//!
//! A **capability** is an abstract interface a package can publish in its
//! [`provides`][prov] section and another package can consume in its
//! [`requires.capabilities`][req] section. A capability is identified by a
//! `<namespace>:<name>` tuple (both kebab-case), optionally constrained by a
//! semver range.
//!
//! Wire form — always a single string:
//!
//! | Form                              | Meaning                                       |
//! |-----------------------------------|-----------------------------------------------|
//! | `db:any`                          | any version of `db:any`                       |
//! | `ui:landing-page@0.3.0`           | exactly `0.3.0`                               |
//! | `ui:landing-page@^0.3`            | semver range `^0.3`                           |
//! | `auth:oauth-callback@>=1.0, <2.0` | compound semver constraint                    |
//!
//! Namespace and name validation reuses the kebab-case rule in
//! [`super::package_ref`]'s `validate_package_name`.
//!
//! Spec: `VIBEVM-SPEC.md` §7.3, [PROP-002 §2.9](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#capability).
//!
//! [prov]: crate::manifest::Provides
//! [req]: crate::manifest::Requires

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::package_ref::{VersionSpec, validate_package_name};

/// A capability identifier plus an optional version constraint.
///
/// Semantics:
///
/// - In `[provides].capabilities`, the version — if present — is an exact
///   version that this package publishes the capability at.
/// - In `[requires].capabilities` / `[[requires_any]]`, the version is the
///   constraint the consumer accepts from any provider.
///
/// The type does not distinguish the two roles at the Rust level — callers do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CapabilityRef {
    pub namespace: String,
    pub name: String,
    pub version: VersionSpec,
}

impl CapabilityRef {
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        version: VersionSpec,
    ) -> Result<Self> {
        let namespace = namespace.into();
        let name = name.into();
        validate_package_name(&namespace).map_err(|_| Error::BadCapabilityRef {
            input: format!("{namespace}:{name}"),
            reason: "namespace is not a valid kebab-case identifier".into(),
        })?;
        validate_package_name(&name).map_err(|_| Error::BadCapabilityRef {
            input: format!("{namespace}:{name}"),
            reason: "name is not a valid kebab-case identifier".into(),
        })?;
        Ok(Self {
            namespace,
            name,
            version,
        })
    }

    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(Error::BadCapabilityRef {
                input: input.to_owned(),
                reason: "empty input".into(),
            });
        }

        let (prefix, version_part) = match trimmed.split_once('@') {
            Some((p, v)) => (p, Some(v)),
            None => (trimmed, None),
        };

        let (ns, name) = prefix
            .split_once(':')
            .ok_or_else(|| Error::BadCapabilityRef {
                input: input.to_owned(),
                reason: "expected `<namespace>:<name>[@<constraint>]` — missing `:`".into(),
            })?;

        validate_package_name(ns).map_err(|_| Error::BadCapabilityRef {
            input: input.to_owned(),
            reason: "invalid namespace — must be kebab-case".into(),
        })?;
        validate_package_name(name).map_err(|_| Error::BadCapabilityRef {
            input: input.to_owned(),
            reason: "invalid name — must be kebab-case".into(),
        })?;

        let version = match version_part {
            None => VersionSpec::Latest,
            Some(v) => VersionSpec::parse(v)?,
        };

        Ok(Self {
            namespace: ns.to_owned(),
            name: name.to_owned(),
            version,
        })
    }

    /// Just the `<namespace>:<name>` portion, without version. Useful as a
    /// match key between a `provides` entry and a `requires` entry.
    pub fn qualified(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }
}

impl fmt::Display for CapabilityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.name)?;
        match &self.version {
            VersionSpec::Latest => Ok(()),
            VersionSpec::Req(req) => write!(f, "@{req}"),
        }
    }
}

impl FromStr for CapabilityRef {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        CapabilityRef::parse(s)
    }
}

impl TryFrom<String> for CapabilityRef {
    type Error = Error;
    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        CapabilityRef::parse(&s)
    }
}

impl From<CapabilityRef> for String {
    fn from(r: CapabilityRef) -> String {
        r.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare() {
        let c = CapabilityRef::parse("db:any").unwrap();
        assert_eq!(c.namespace, "db");
        assert_eq!(c.name, "any");
        assert_eq!(c.version, VersionSpec::Latest);
        assert_eq!(c.to_string(), "db:any");
    }

    #[test]
    fn parse_with_bare_semver_is_caret_per_cargo() {
        // Same Cargo / npm / Poetry default the package_ref parser
        // uses: bare semver `0.3.0` is shorthand for `^0.3.0`.
        let c = CapabilityRef::parse("ui:landing-page@0.3.0").unwrap();
        assert!(c.version.matches(&semver::Version::parse("0.3.0").unwrap()));
        // pre-1.0 caret accepts patch bumps within the same minor…
        assert!(c.version.matches(&semver::Version::parse("0.3.5").unwrap()));
        // …but not the next minor.
        assert!(!c.version.matches(&semver::Version::parse("0.4.0").unwrap()));
    }

    #[test]
    fn parse_with_eq_version_is_exact() {
        let c = CapabilityRef::parse("ui:landing-page@=0.3.0").unwrap();
        assert!(c.version.matches(&semver::Version::parse("0.3.0").unwrap()));
        assert!(!c.version.matches(&semver::Version::parse("0.3.1").unwrap()));
    }

    #[test]
    fn parse_with_range() {
        let c = CapabilityRef::parse("db:any@>=1.0").unwrap();
        let v = semver::Version::parse("1.5.0").unwrap();
        assert!(c.version.matches(&v));
        let v2 = semver::Version::parse("0.9.0").unwrap();
        assert!(!c.version.matches(&v2));
    }

    #[test]
    fn qualified_strips_version() {
        let c = CapabilityRef::parse("ui:landing-page@^0.3").unwrap();
        assert_eq!(c.qualified(), "ui:landing-page");
    }

    #[test]
    fn rejects_missing_colon() {
        let err = CapabilityRef::parse("db.any").unwrap_err();
        assert!(matches!(err, Error::BadCapabilityRef { .. }));
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            CapabilityRef::parse("").unwrap_err(),
            Error::BadCapabilityRef { .. }
        ));
    }

    #[test]
    fn rejects_bad_namespace() {
        let err = CapabilityRef::parse("DB:any").unwrap_err();
        assert!(matches!(err, Error::BadCapabilityRef { .. }));
    }

    #[test]
    fn rejects_bad_name() {
        let err = CapabilityRef::parse("db:An-Y").unwrap_err();
        assert!(matches!(err, Error::BadCapabilityRef { .. }));
    }

    #[test]
    fn display_roundtrip_latest() {
        let c = CapabilityRef::parse("auth:oauth-callback").unwrap();
        assert_eq!(c.to_string(), "auth:oauth-callback");
    }

    #[test]
    fn display_roundtrip_exact() {
        let c = CapabilityRef::parse("ui:landing-page@0.3.0").unwrap();
        let back = c.to_string();
        let c2 = CapabilityRef::parse(&back).unwrap();
        assert_eq!(c, c2);
    }

    #[test]
    fn serde_roundtrip_via_string() {
        let c = CapabilityRef::parse("ui:landing-page@^0.3").unwrap();
        let json = serde_json::to_string(&c).unwrap();
        // Wire form is a bare JSON string.
        assert_eq!(json, r#""ui:landing-page@^0.3""#);
        let back: CapabilityRef = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
