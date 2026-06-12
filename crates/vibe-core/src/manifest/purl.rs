//! Package URL (PURL) parser.
//!
//! Spec: <https://github.com/package-url/purl-spec>.
//!
//! Format: `pkg:<type>/<namespace>/<name>@<version>?<qualifiers>#<subpath>`.
//! `<namespace>` and the trailing parts are optional. `<type>` is the
//! package ecosystem (`pypi`, `npm`, `cargo`, `gem`, `maven`, `docker`,
//! `github`, `gitverse`, `vibe`, …). Type names are lowercased on parse.
//!
//! vibevm uses PURLs in two places:
//! - `[package].describes` — the package documents this upstream artefact
//!   at this version (PROP-003 §2.5.6, §M1.9 roadmap).
//! - `[subskill].describes` — same semantics, scoped to one subskill cut.
//!
//! This module deliberately implements only the subset we need: type,
//! optional namespace, name, version. Qualifiers and subpaths parse but
//! are not interpreted. `<version>` may be an exact SemVer string or a
//! [`semver::VersionReq`] requirement (`^0.8`, `>=1, <2`, `*`).

specmark::scope!("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-describes");

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specmark::spec;
use thiserror::Error;

/// A parsed Package URL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Purl {
    /// Lowercased ecosystem type — `npm`, `pypi`, `cargo`, etc.
    pub purl_type: String,
    /// Optional namespace — `pypi/<namespace>/<name>`. May be empty.
    pub namespace: Option<String>,
    /// Package name within the ecosystem.
    pub name: String,
    /// Optional version string. Either an exact version (`0.8.0`) or a
    /// SemVer requirement (`^0.8`, `>=1, <2`, `*`). Stored verbatim;
    /// parse with [`Purl::version_req`] when needed.
    pub version: Option<String>,
}

impl Purl {
    /// Parse a `pkg:` URL into its components.
    pub fn parse(s: &str) -> Result<Self, PurlError> {
        let s = s.trim();
        let body = s
            .strip_prefix("pkg:")
            .ok_or_else(|| PurlError::MissingScheme(s.to_string()))?;

        // Strip trailing #subpath and ?qualifiers — accepted but ignored.
        let body = body.split('#').next().unwrap_or(body);
        let body = body.split('?').next().unwrap_or(body);

        // Split on the LAST '@' — version is everything after. Using
        // rsplit handles namespaces that themselves start with `@` (the
        // npm `@scope/name` pattern produces `pkg:npm/@scope/name@ver`).
        let (path, version) = match body.rsplit_once('@') {
            Some((p, v)) if !v.is_empty() && !p.is_empty() => (p, Some(v.to_string())),
            _ => (body, None),
        };

        // path = type/[namespace/]name. Find first '/'.
        let (purl_type, rest) = path
            .split_once('/')
            .ok_or_else(|| PurlError::Malformed(s.to_string()))?;
        if purl_type.is_empty() {
            return Err(PurlError::Malformed(s.to_string()));
        }

        // rest may be `namespace/name` or just `name`.
        let (namespace, name) = match rest.rsplit_once('/') {
            Some((ns, n)) if !n.is_empty() => (Some(ns.to_string()), n.to_string()),
            _ => (None, rest.to_string()),
        };

        if name.is_empty() {
            return Err(PurlError::Malformed(s.to_string()));
        }

        Ok(Purl {
            purl_type: purl_type.to_lowercase(),
            namespace,
            name,
            version,
        })
    }

    /// If `version` is a SemVer requirement string, parse it. Returns
    /// `None` if there's no version, or `Err(_)` if the string isn't a
    /// valid `VersionReq`.
    pub fn version_req(&self) -> Option<Result<semver::VersionReq, semver::Error>> {
        self.version.as_deref().map(semver::VersionReq::parse)
    }
}

impl fmt::Display for Purl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pkg:{}/", self.purl_type)?;
        if let Some(ns) = &self.namespace {
            write!(f, "{ns}/")?;
        }
        write!(f, "{}", self.name)?;
        if let Some(v) = &self.version {
            write!(f, "@{v}")?;
        }
        Ok(())
    }
}

impl Serialize for Purl {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Purl {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Purl::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#subskill-describes")]
pub enum PurlError {
    #[error(
        "invalid PURL `{0}`: missing `pkg:` scheme \
         (violates spec://vibevm/modules/vibe-resolver/PROP-003#subskill-describes; \
          fix: prefix the URL with `pkg:`)"
    )]
    MissingScheme(String),
    #[error(
        "invalid PURL `{0}`: malformed structure \
         (violates spec://vibevm/modules/vibe-resolver/PROP-003#subskill-describes; \
          fix: write it as `pkg:<type>/<name>[@<version>]`)"
    )]
    Malformed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_unscoped_with_version() {
        let p = Purl::parse("pkg:cargo/sqlx@0.8.0").unwrap();
        assert_eq!(p.purl_type, "cargo");
        assert_eq!(p.namespace, None);
        assert_eq!(p.name, "sqlx");
        assert_eq!(p.version.as_deref(), Some("0.8.0"));
    }

    #[test]
    fn parses_namespaced_npm() {
        let p = Purl::parse("pkg:npm/@my-co/util-formatter@1.0.0").unwrap();
        assert_eq!(p.purl_type, "npm");
        assert_eq!(p.namespace.as_deref(), Some("@my-co"));
        assert_eq!(p.name, "util-formatter");
        assert_eq!(p.version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn parses_without_version() {
        let p = Purl::parse("pkg:pypi/fastapi").unwrap();
        assert_eq!(p.purl_type, "pypi");
        assert_eq!(p.name, "fastapi");
        assert!(p.version.is_none());
    }

    #[test]
    fn parses_semver_requirement_as_version() {
        let p = Purl::parse("pkg:cargo/sqlx@^0.8").unwrap();
        assert_eq!(p.version.as_deref(), Some("^0.8"));
        let req = p.version_req().unwrap().unwrap();
        assert!(req.matches(&semver::Version::new(0, 8, 4)));
        assert!(!req.matches(&semver::Version::new(0, 9, 0)));
    }

    #[test]
    fn type_is_lowercased() {
        let p = Purl::parse("pkg:NPM/foo@1.0.0").unwrap();
        assert_eq!(p.purl_type, "npm");
    }

    #[test]
    fn rejects_missing_scheme() {
        let err = Purl::parse("cargo/sqlx@0.8.0").unwrap_err();
        assert!(matches!(err, PurlError::MissingScheme(_)));
    }

    #[test]
    fn rejects_missing_type() {
        let err = Purl::parse("pkg:/sqlx@0.8.0").unwrap_err();
        assert!(matches!(err, PurlError::Malformed(_)));
    }

    #[test]
    fn rejects_missing_name() {
        // No `/` means no name extracted.
        let err = Purl::parse("pkg:cargo").unwrap_err();
        assert!(matches!(err, PurlError::Malformed(_)));
    }

    #[test]
    fn ignores_subpath_and_qualifiers() {
        let p = Purl::parse("pkg:pypi/fastapi@0.116.0?os=linux#extras").unwrap();
        assert_eq!(p.name, "fastapi");
        assert_eq!(p.version.as_deref(), Some("0.116.0"));
    }

    #[test]
    fn round_trips_via_display() {
        let cases = [
            "pkg:cargo/sqlx@0.8.0",
            "pkg:npm/@my-co/util-formatter@1.0.0",
            "pkg:pypi/fastapi",
        ];
        for c in cases {
            let p = Purl::parse(c).unwrap();
            assert_eq!(p.to_string(), c, "round-trip for {c}");
        }
    }

    #[test]
    fn serde_round_trip() {
        let p = Purl::parse("pkg:cargo/sqlx@0.8.0").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"pkg:cargo/sqlx@0.8.0\"");
        let back: Purl = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn serde_rejects_invalid() {
        let err = serde_json::from_str::<Purl>("\"not-a-purl\"");
        assert!(err.is_err());
    }
}
