//! VVM domain model: version kinds, the canonical version id, selectors,
//! build profiles, install origin, and the on-disk inventory (PROP-019
//! §2.3–2.4, §2.16).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#layout");

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;
use thiserror::Error;

/// The version-model layer's parse failures (PROP-019 §2.3, §2.2): the two
/// untrusted boundaries where a user-supplied string becomes a typed value.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#selectors")]
pub enum ModelError {
    #[error(
        "empty version selector \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#selectors; \
          fix: pass a selector like `latest`, `stable`, `1.2.3`, or `tag:1.2.3`)"
    )]
    EmptySelector,

    #[error(
        "invalid exact instance selector `{0}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#selectors; \
          fix: use a copyable selector such as `tag:1.2.3#4`, with a positive terminal instance number)"
    )]
    InvalidInstanceSelector(String),

    #[error(
        "unknown build profile `{0}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#build; \
          fix: pass `debug` or `release`)"
    )]
    UnknownProfile(String),
}

/// What a version is pinned to (PROP-019 §2.4). The kind namespaces the
/// on-disk layout so a tag `1.2.3` and a branch `1.2.3` never collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
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

    /// Parse a kind token.
    fn from_token(s: &str) -> Option<Kind> {
        match s {
            "tag" => Some(Kind::Tag),
            "branch" => Some(Kind::Branch),
            "commit" => Some(Kind::Commit),
            _ => None,
        }
    }
}

/// The canonical identity of a version: `<kind>:<id>` (PROP-019 §2.4).
/// Rendered with `:` for humans, split into `<kind>/<id>` on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
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

/// The immutable identity of one locally installed payload. The version id
/// may move as its remote tag/branch is republished; the terminal `#N` never
/// does and is globally unique within a store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceId {
    pub version: VersionId,
    pub instance: u64,
}

impl InstanceId {
    pub fn new(version: VersionId, instance: u64) -> Self {
        InstanceId { version, instance }
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.version, self.instance)
    }
}

/// The build profile (PROP-019 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#build")]
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

    pub fn parse(s: &str) -> Result<Profile, ModelError> {
        match s {
            "debug" => Ok(Profile::Debug),
            "release" => Ok(Profile::Release),
            other => Err(ModelError::UnknownProfile(other.to_string())),
        }
    }
}

/// The default build profile today (PROP-019 §2.2): `debug`, flipped to
/// `release` later — this constant is the single point of that change.
pub const DEFAULT_PROFILE: Profile = Profile::Debug;

/// A parsed user version request, before git resolution (PROP-019 §2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#selectors")]
pub enum Selector {
    /// The tip of branch `main`.
    Latest,
    /// The highest semantic-version release tag.
    Stable,
    /// An explicit kind+id (forced `--tag/--branch/--commit`, the canonical
    /// `<kind>:<id>` form, an unambiguous hex commit, or `X.Y.Z`).
    Explicit(VersionId),
    /// One exact immutable local payload, rendered as `<kind>:<id>#N`.
    Exact(InstanceId),
    /// A bare name, resolved later by precedence branch > tag > commit.
    Ambiguous(String),
}

impl Selector {
    /// Parse a CLI selector plus an optional forced kind (PROP-019 §2.3).
    pub fn parse(raw: &str, forced: Option<Kind>) -> Result<Selector, ModelError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(ModelError::EmptySelector);
        }
        let full = raw;
        let (raw, instance) = split_instance_suffix(raw)?;
        let selector = Self::parse_version(raw, forced);
        if let Some(instance) = instance {
            return match selector {
                Selector::Explicit(version) => {
                    Ok(Selector::Exact(InstanceId::new(version, instance)))
                }
                _ => Err(ModelError::InvalidInstanceSelector(full.to_string())),
            };
        }
        Ok(selector)
    }

    fn parse_version(raw: &str, forced: Option<Kind>) -> Selector {
        if let Some(kind) = forced {
            return Selector::Explicit(VersionId::new(kind, raw));
        }
        // The canonical `<kind>:<id>` form, as `man ls` prints it.
        if let Some((k, rest)) = raw.split_once(':')
            && let Some(kind) = Kind::from_token(k)
            && !rest.is_empty()
        {
            return Selector::Explicit(VersionId::new(kind, rest));
        }
        match raw {
            "latest" => Selector::Latest,
            "stable" => Selector::Stable,
            _ if looks_like_commit(raw) => Selector::Explicit(VersionId::new(Kind::Commit, raw)),
            _ if looks_like_semver_tag(raw) => Selector::Explicit(VersionId::new(Kind::Tag, raw)),
            _ => Selector::Ambiguous(raw.to_string()),
        }
    }
}

fn split_instance_suffix(raw: &str) -> Result<(&str, Option<u64>), ModelError> {
    let Some((version, suffix)) = raw.rsplit_once('#') else {
        return Ok((raw, None));
    };
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok((raw, None));
    }
    let instance = suffix
        .parse::<u64>()
        .ok()
        .filter(|instance| *instance > 0)
        .ok_or_else(|| ModelError::InvalidInstanceSelector(raw.to_string()))?;
    if version.is_empty() {
        return Err(ModelError::InvalidInstanceSelector(raw.to_string()));
    }
    Ok((version, Some(instance)))
}

fn looks_like_commit(s: &str) -> bool {
    (7..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn looks_like_semver_tag(s: &str) -> bool {
    semver::Version::parse(s.strip_prefix('v').unwrap_or(s)).is_ok()
}

/// Where an installed instance's distribution came from (PROP-019 §2.16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#provenance")]
pub enum Origin {
    /// A VVM-owned clone under `src/<kind>/<id>` (VVM updates/drops it).
    Managed,
    /// A committer's own checkout, referenced by `source_path`; never touched.
    External,
    /// A prebuilt artifact (far-backlog).
    Binary,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Managed => "managed",
            Origin::External => "external",
            Origin::Binary => "binary",
        }
    }
}

/// One installed instance and its metadata (PROP-019 §2.4, §2.7, §2.16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
pub struct InstallRecord {
    pub kind: Kind,
    pub id: String,
    /// The monotonic instance number (PROP-019 §2.4, §9.4).
    pub instance: u64,
    /// The commit the selector resolved to at install time.
    pub commit: String,
    /// The toolchain that built it (e.g. `rustc 1.93.0`).
    pub toolchain: String,
    /// `debug` or `release`.
    pub profile: Profile,
    /// RFC3339 install timestamp.
    pub installed_at: String,
    /// Where the source came from (PROP-019 §2.16).
    pub origin: Origin,
    /// Canonical absolute path of the source — only for `external` origins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Integrity digest of a locally imported prebuilt payload. This is
    /// populated only for `binary` origins and is not a signature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_sha256: Option<String>,
}

impl InstallRecord {
    /// The canonical id of this install (ignoring the instance number).
    pub fn version_id(&self) -> VersionId {
        VersionId::new(self.kind, self.id.clone())
    }

    /// The copyable selector for this exact immutable local payload.
    pub fn selector(&self) -> InstanceId {
        InstanceId::new(self.version_id(), self.instance)
    }

    /// A stable human provenance label even for old records that predate a
    /// source path or for binary imports whose original file may be gone.
    pub fn source_label(&self) -> &str {
        match self.origin {
            Origin::External => self.source_path.as_deref().unwrap_or("external:unknown"),
            Origin::Managed => "managed-git",
            Origin::Binary => "local-import",
        }
    }

    /// One compact provenance-rich line used by human `ls`/`current` output;
    /// its first token is always directly copyable back into selector verbs.
    pub fn human_identity(&self) -> String {
        format!(
            "{}  origin={} source={:?} commit={} sha256={} profile={}",
            self.selector(),
            self.origin.as_str(),
            self.source_label(),
            self.commit,
            self.payload_sha256.as_deref().unwrap_or("-"),
            self.profile.as_str(),
        )
    }
}

/// The on-disk inventory at `<root>/vibevm/state.toml` (PROP-019 §2.4).
///
/// The *active* instance is named by the separate `current` file (PROP-019
/// §2.5), not stored here. This file is the inventory plus the monotonic
/// instance counter (§9.4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
pub struct State {
    /// The next instance number to allocate (0 means "start at 1").
    #[serde(default)]
    pub next_instance: u64,
    #[serde(default, rename = "install")]
    pub installs: Vec<InstallRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#layout", r = 1)]
    fn version_id_renders_and_splits_by_kind() {
        let v = VersionId::new(Kind::Tag, "1.2.3");
        assert_eq!(v.to_string(), "tag:1.2.3");
        assert_eq!(v.path_segment(), PathBuf::from("tag").join("1.2.3"));
        assert_ne!(
            v.path_segment(),
            VersionId::new(Kind::Branch, "1.2.3").path_segment()
        );
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#layout", r = 1)]
    fn state_round_trips_through_toml() {
        let state = State {
            next_instance: 4,
            installs: vec![InstallRecord {
                kind: Kind::Branch,
                id: "main".into(),
                instance: 3,
                commit: "abc1234def".into(),
                toolchain: "rustc 1.93.0".into(),
                profile: Profile::Debug,
                installed_at: "2026-06-17T00:00:00Z".into(),
                origin: Origin::External,
                source_path: Some("C:/src/vibevm".into()),
                payload_sha256: None,
            }],
        };
        let text = toml::to_string(&state).unwrap();
        let back: State = toml::from_str(&text).unwrap();
        assert_eq!(state, back);
        assert_eq!(back.installs[0].version_id().to_string(), "branch:main");
        assert_eq!(back.next_instance, 4);
        assert_eq!(back.installs[0].origin, Origin::External);
    }

    #[test]
    fn legacy_state_without_optional_provenance_fields_remains_readable() {
        let legacy = r#"
next_instance = 0

[[install]]
kind = "tag"
id = "1.0.0"
instance = 3
commit = "abc1234"
toolchain = "prebuilt"
profile = "release"
installed_at = "2026-01-01T00:00:00Z"
origin = "binary"
"#;
        let state: State = toml::from_str(legacy).unwrap();
        assert_eq!(state.installs[0].selector().to_string(), "tag:1.0.0#3");
        assert!(state.installs[0].payload_sha256.is_none());
        assert!(state.installs[0].source_path.is_none());
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#selectors", r = 1)]
    fn selector_parse_classifies_by_shape() {
        use Selector::*;
        assert_eq!(Selector::parse("latest", None).unwrap(), Latest);
        assert_eq!(Selector::parse("stable", None).unwrap(), Stable);
        assert_eq!(
            Selector::parse("1.2.3", None).unwrap(),
            Explicit(VersionId::new(Kind::Tag, "1.2.3"))
        );
        assert_eq!(
            Selector::parse("abc1234", None).unwrap(),
            Explicit(VersionId::new(Kind::Commit, "abc1234"))
        );
        assert_eq!(
            Selector::parse("main", None).unwrap(),
            Ambiguous("main".into())
        );
        assert_eq!(
            Selector::parse("main", Some(Kind::Branch)).unwrap(),
            Explicit(VersionId::new(Kind::Branch, "main"))
        );
        assert_eq!(
            Selector::parse("tag:1.2.3", None).unwrap(),
            Explicit(VersionId::new(Kind::Tag, "1.2.3"))
        );
        assert_eq!(
            Selector::parse("tag:1.2.3#7", None).unwrap(),
            Exact(InstanceId::new(VersionId::new(Kind::Tag, "1.2.3"), 7))
        );
        assert_eq!(
            Selector::parse("1.2.3#7", None).unwrap(),
            Exact(InstanceId::new(VersionId::new(Kind::Tag, "1.2.3"), 7))
        );
        assert!(Selector::parse("latest#7", None).is_err());
        assert!(Selector::parse("tag:1.2.3#0", None).is_err());
        assert!(Selector::parse("   ", None).is_err());
    }

    #[test]
    fn instance_selector_is_terminal_copyable_identity() {
        let record = InstallRecord {
            kind: Kind::Tag,
            id: "1.2.3".into(),
            instance: 9,
            commit: "abc".into(),
            toolchain: "prebuilt".into(),
            profile: Profile::Release,
            installed_at: "now".into(),
            origin: Origin::Binary,
            source_path: None,
            payload_sha256: Some("feedface".into()),
        };
        assert_eq!(record.selector().to_string(), "tag:1.2.3#9");
        assert_eq!(record.source_label(), "local-import");
        let identity = record.human_identity();
        assert!(identity.starts_with("tag:1.2.3#9 "));
        assert!(identity.contains("origin=binary"));
        assert!(identity.contains("source=\"local-import\""));
        assert!(identity.contains("commit=abc"));
        assert!(identity.contains("sha256=feedface"));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#build", r = 1)]
    fn profile_parses_and_defaults_to_debug() {
        assert_eq!(Profile::parse("release").unwrap(), Profile::Release);
        assert!(Profile::parse("fast").is_err());
        assert_eq!(DEFAULT_PROFILE, Profile::Debug);
        assert_eq!(Profile::Release.target_subdir(), "release");
    }
}
