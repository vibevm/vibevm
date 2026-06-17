//! VVM domain model: version kinds, the canonical version id, and the
//! on-disk inventory (PROP-019 §2.4).

specmark::scope!("spec://vibevm/common/PROP-019#layout");

use std::fmt;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use specmark::spec;

/// What a version is pinned to (PROP-019 §2.4). The kind namespaces the
/// on-disk layout so a tag `1.2.3` and a branch `1.2.3` never collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(implements = "spec://vibevm/common/PROP-019#layout")]
pub enum Kind {
    Tag,
    Branch,
    Commit,
}

impl Kind {
    /// The lowercase wire/disk token (`tag` / `branch` / `commit`).
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Tag => "tag",
            Kind::Branch => "branch",
            Kind::Commit => "commit",
        }
    }

    /// Parse a kind token (`tag` / `branch` / `commit`).
    fn from_token(s: &str) -> Option<Kind> {
        match s {
            "tag" => Some(Kind::Tag),
            "branch" => Some(Kind::Branch),
            "commit" => Some(Kind::Commit),
            _ => None,
        }
    }
}

/// The canonical identity of an installed version: `<kind>:<id>` (PROP-019
/// §2.4). Rendered with `:` for humans, split into `<kind>/<id>` on disk —
/// the same segment under both `versions/` and `src/` so the two agree.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://vibevm/common/PROP-019#layout")]
pub struct VersionId {
    pub kind: Kind,
    /// The git-side identifier: a tag name, a branch name, or a commit hash.
    pub id: String,
}

impl VersionId {
    pub fn new(kind: Kind, id: impl Into<String>) -> Self {
        VersionId {
            kind,
            id: id.into(),
        }
    }

    /// The on-disk path segment `<kind>/<id>` (PROP-019 §2.4).
    pub fn path_segment(&self) -> PathBuf {
        PathBuf::from(self.kind.as_str()).join(&self.id)
    }
}

impl fmt::Display for VersionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind.as_str(), self.id)
    }
}

/// One installed version's metadata, recorded at install time (PROP-019
/// §2.7) so a moving-branch install stays reproducible after the fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(implements = "spec://vibevm/common/PROP-019#layout")]
pub struct InstallRecord {
    pub kind: Kind,
    pub id: String,
    /// The commit the selector resolved to at install time.
    pub commit: String,
    /// The toolchain that built it (e.g. `rustc 1.93.0`).
    pub toolchain: String,
    /// `debug` or `release`.
    pub profile: String,
    /// RFC3339 install timestamp.
    pub installed_at: String,
}

impl InstallRecord {
    /// The canonical id of this install.
    pub fn version_id(&self) -> VersionId {
        VersionId::new(self.kind, self.id.clone())
    }
}

/// The on-disk inventory at `<root>/vibevm/state.toml` (PROP-019 §2.4).
///
/// The *active* version is deliberately not stored here — it is named by
/// the `VIBEVM_HOME` env var (PROP-019 §2.5), the single source of truth.
/// This file is the inventory of what is installed, nothing more.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(implements = "spec://vibevm/common/PROP-019#layout")]
pub struct State {
    #[serde(default, rename = "install")]
    pub installs: Vec<InstallRecord>,
}

/// The build profile (PROP-019 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(implements = "spec://vibevm/common/PROP-019#build")]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }

    /// Cargo's `target/<subdir>` for this profile.
    pub fn target_subdir(self) -> &'static str {
        self.as_str()
    }

    /// Parse `debug` / `release`.
    pub fn parse(s: &str) -> Result<Profile> {
        match s {
            "debug" => Ok(Profile::Debug),
            "release" => Ok(Profile::Release),
            other => bail!("unknown build profile `{other}` (want debug|release)"),
        }
    }
}

/// The default build profile today (PROP-019 §2.2): `debug`, to be flipped
/// to `release` later — this constant is the single point of that change.
pub const DEFAULT_PROFILE: Profile = Profile::Debug;

/// A parsed user version request, before git resolution (PROP-019 §2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://vibevm/common/PROP-019#selectors")]
pub enum Selector {
    /// The tip of branch `main`.
    Latest,
    /// The highest semantic-version release tag.
    Stable,
    /// An explicit kind+id — a forced `--tag`/`--branch`/`--commit`, or an
    /// unambiguous hex commit / `X.Y.Z` tag.
    Explicit(VersionId),
    /// A bare name, resolved later by precedence commit > branch > tag.
    Ambiguous(String),
}

impl Selector {
    /// Parse a CLI selector plus an optional forced kind
    /// (`--tag`/`--branch`/`--commit`) (PROP-019 §2.3).
    pub fn parse(raw: &str, forced: Option<Kind>) -> Result<Selector> {
        let raw = raw.trim();
        if raw.is_empty() {
            bail!("empty version selector");
        }
        if let Some(kind) = forced {
            return Ok(Selector::Explicit(VersionId::new(kind, raw)));
        }
        // The canonical `<kind>:<id>` form, as `man ls` prints it.
        if let Some((k, rest)) = raw.split_once(':')
            && let Some(kind) = Kind::from_token(k)
            && !rest.is_empty()
        {
            return Ok(Selector::Explicit(VersionId::new(kind, rest)));
        }
        Ok(match raw {
            "latest" => Selector::Latest,
            "stable" => Selector::Stable,
            _ if looks_like_commit(raw) => Selector::Explicit(VersionId::new(Kind::Commit, raw)),
            _ if looks_like_semver_tag(raw) => Selector::Explicit(VersionId::new(Kind::Tag, raw)),
            _ => Selector::Ambiguous(raw.to_string()),
        })
    }
}

/// A hex string of git short/long-hash length — treated as a commit.
fn looks_like_commit(s: &str) -> bool {
    (7..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A `X.Y.Z` (optionally `v`-prefixed) semantic version — treated as a tag.
fn looks_like_semver_tag(s: &str) -> bool {
    semver::Version::parse(s.strip_prefix('v').unwrap_or(s)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#layout", r = 1)]
    fn version_id_renders_and_splits_by_kind() {
        let v = VersionId::new(Kind::Tag, "1.2.3");
        assert_eq!(v.to_string(), "tag:1.2.3");
        assert_eq!(
            v.path_segment(),
            PathBuf::from("tag").join("1.2.3"),
            "on-disk segment is <kind>/<id>"
        );
        // A branch and a tag with the same name never share a path.
        let b = VersionId::new(Kind::Branch, "1.2.3");
        assert_ne!(v.path_segment(), b.path_segment());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#layout", r = 1)]
    fn state_round_trips_through_toml() {
        let state = State {
            installs: vec![InstallRecord {
                kind: Kind::Branch,
                id: "main".into(),
                commit: "abc1234def".into(),
                toolchain: "rustc 1.93.0".into(),
                profile: "debug".into(),
                installed_at: "2026-06-17T00:00:00Z".into(),
            }],
        };
        let text = toml::to_string(&state).unwrap();
        let back: State = toml::from_str(&text).unwrap();
        assert_eq!(state, back);
        assert_eq!(back.installs[0].version_id().to_string(), "branch:main");
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#layout", r = 1)]
    fn empty_state_is_the_default() {
        let back: State = toml::from_str("").unwrap();
        assert_eq!(back, State::default());
        assert!(back.installs.is_empty());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#selectors", r = 1)]
    fn selector_parse_classifies_by_shape() {
        use Selector::*;
        assert_eq!(Selector::parse("latest", None).unwrap(), Latest);
        assert_eq!(Selector::parse("stable", None).unwrap(), Stable);
        assert_eq!(
            Selector::parse("1.2.3", None).unwrap(),
            Explicit(VersionId::new(Kind::Tag, "1.2.3"))
        );
        assert_eq!(
            Selector::parse("v1.2.3", None).unwrap(),
            Explicit(VersionId::new(Kind::Tag, "v1.2.3"))
        );
        assert_eq!(
            Selector::parse("abc1234", None).unwrap(),
            Explicit(VersionId::new(Kind::Commit, "abc1234"))
        );
        assert_eq!(
            Selector::parse("main", None).unwrap(),
            Ambiguous("main".into())
        );
        // A forced kind wins over shape inference.
        assert_eq!(
            Selector::parse("main", Some(Kind::Branch)).unwrap(),
            Explicit(VersionId::new(Kind::Branch, "main"))
        );
        // `feature` is 7 chars but not all-hex → ambiguous, not a commit.
        assert_eq!(
            Selector::parse("feature", None).unwrap(),
            Ambiguous("feature".into())
        );
        // The canonical `<kind>:<id>` form (as `man ls` prints it).
        assert_eq!(
            Selector::parse("tag:1.2.3", None).unwrap(),
            Explicit(VersionId::new(Kind::Tag, "1.2.3"))
        );
        assert_eq!(
            Selector::parse("branch:main", None).unwrap(),
            Explicit(VersionId::new(Kind::Branch, "main"))
        );
        assert!(Selector::parse("   ", None).is_err());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#build", r = 1)]
    fn profile_parses_and_defaults_to_debug() {
        assert_eq!(Profile::parse("debug").unwrap(), Profile::Debug);
        assert_eq!(Profile::parse("release").unwrap(), Profile::Release);
        assert!(Profile::parse("fast").is_err());
        assert_eq!(DEFAULT_PROFILE, Profile::Debug);
        assert_eq!(Profile::Release.target_subdir(), "release");
    }
}
