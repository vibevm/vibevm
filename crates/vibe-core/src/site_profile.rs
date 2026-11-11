//! The administrator's site profile (`PROP-019#site-profile`): one TOML
//! file that configures a whole vibevm installation in a closed network.
//!
//! An administrator writes the profile once — the registries, the mirror
//! and remap rows, the source policy, the artifact drops, the release
//! source, the proxy and the PATH policy — and every developer runs
//! `vibe self apply <file>`. The profile is stored verbatim as its own
//! layer, `<settings-dir>/site-profile.toml`, and never merged into the
//! user's own files: every owner reads it **below** the user's
//! `registry.toml` / `config.toml` and **above** the built-in defaults, so
//! the user still overrides any value, removing the profile removes its
//! whole effect, and re-applying an edited profile replaces it.
//!
//! This module owns the format (a schema-checked, `deny_unknown_fields`
//! reader), its full validation (nothing is stored before every table has
//! been checked), the installed-layer loader every owner calls, and the
//! receipt recording what `apply` installed so `--remove` deletes exactly
//! that and nothing a user wrote.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::GlobalRegistryConfig;
use crate::manifest::{
    EffectiveSourcePolicy, MirrorSection, OverrideSection, RegistrySection, SourcePolicyDecl,
};
use crate::remap::{Layer, LoadedSource, RowDecl};
use crate::trust::TrustLists;
use crate::user_config::{
    ArtifactsConfig, NetConfig, ProxyConfig, SelfConfig, TlsConfig, UserConfig,
};

/// The one schema version this reader accepts (`schema = 1`).
pub const SITE_PROFILE_SCHEMA: u32 = 1;
/// Basename of the stored profile in the settings directory.
pub const SITE_PROFILE_FILE: &str = "site-profile.toml";
/// Basename of the apply receipt in the settings directory.
pub const SITE_RECEIPT_FILE: &str = "site-profile.receipt.toml";
/// The provenance label of every row and value the stored profile supplies.
pub const SITE_PROFILE_SOURCE: &str = "site profile";

/// The `[path] fix` policy: may `vibe self apply` put the shim directory
/// on the durable PATH (`PROP-019#path`)?
///
/// ```
/// use vibe_core::site_profile::PathFixPolicy;
/// let p: PathFixPolicy = serde_json::from_str("\"always\"").unwrap();
/// assert_eq!(p, PathFixPolicy::Always);
/// assert_eq!(PathFixPolicy::default(), PathFixPolicy::Ask);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
pub enum PathFixPolicy {
    /// Ask when a terminal is attached; otherwise leave PATH alone.
    #[default]
    Ask,
    /// Fix without asking, attended or not.
    Always,
    /// Never touch the durable PATH.
    Never,
}

/// `[path]` — the PATH policy of the profile.
///
/// ```
/// use vibe_core::site_profile::{PathFixPolicy, PathSection};
/// let section: PathSection = toml::from_str("fix = 'never'").unwrap();
/// assert_eq!(section.fix, Some(PathFixPolicy::Never));
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathSection {
    /// `fix = "ask" | "always" | "never"`; absent is `ask`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<PathFixPolicy>,
}

/// A parsed, validated site profile (`schema = 1`).
///
/// ```
/// use vibe_core::site_profile::SiteProfile;
///
/// let profile = SiteProfile::parse(
///     "schema = 1\nname = 'corp'\n\
///      [[registry]]\nname = 'corp'\ntype = 'localdir'\nurl = 'file:///srv/vibe-packages'\n\
///      [source_policy]\nexclude_hosting = ['github', 'gitverse']\n\
///      [artifacts]\nvibe = '/srv/vibe-releases'\npackages = '/srv/vibe-artifacts'\n\
///      [self]\nsource = 'LocalDir'\n",
///     "corp.toml",
/// )
/// .unwrap();
/// assert_eq!(profile.name.as_deref(), Some("corp"));
/// assert_eq!(profile.registry_layer().registries[0].name, "corp");
/// // A typo is refused with the key and the file named.
/// let error = SiteProfile::parse("schema = 1\n[artifact]\nvibe = '/x'\n", "corp.toml").unwrap_err();
/// assert!(error.to_string().contains("corp.toml"));
/// assert!(error.to_string().contains("PROP-019#site-profile"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
pub struct SiteProfile {
    /// The format version; must be [`SITE_PROFILE_SCHEMA`].
    pub schema: u32,
    /// A human name for the profile, shown by `vibe self apply --show`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `[[registry]]` — the site's registries, below the user's own.
    #[serde(default, rename = "registry", skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistrySection>,
    /// `[[mirror]]` — mirrors, after the user's.
    #[serde(default, rename = "mirror", skip_serializing_if = "Vec::is_empty")]
    pub mirrors: Vec<MirrorSection>,
    /// `[[override]]` — source pins, the user's winning by `pkgref`.
    #[serde(default, rename = "override", skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<OverrideSection>,
    /// `[source_policy]` — `prefer_hosting` / `exclude_hosting`.
    #[serde(default, skip_serializing_if = "SourcePolicyDecl::is_empty")]
    pub source_policy: SourcePolicyDecl,
    /// `[[remap]]` — PROP-066 rows, below the user's `settings.toml` rows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remap: Vec<RowDecl>,
    /// `[artifacts]` — the release and package drops.
    #[serde(default, skip_serializing_if = "ArtifactsConfig::is_default")]
    pub artifacts: ArtifactsConfig,
    /// `[self]` — the release source.
    #[serde(rename = "self")]
    #[serde(default, skip_serializing_if = "SelfConfig::is_default")]
    pub self_source: SelfConfig,
    /// `[proxy]` — the proxy every client reads.
    #[serde(default, skip_serializing_if = "ProxyConfig::is_default")]
    pub proxy: ProxyConfig,
    /// `[tls]` — the certificates trusted besides the operating system's
    /// own verifier (`PROP-068#proxy`), below the user's own `[tls]`.
    #[serde(default, skip_serializing_if = "TlsConfig::is_default")]
    pub tls: TlsConfig,
    /// `[net]` — `offline = true` makes the installation offline by default.
    #[serde(default, skip_serializing_if = "NetConfig::is_default")]
    pub net: NetConfig,
    /// `[path]` — the PATH policy of `vibe self apply`.
    #[serde(default, skip_serializing_if = "is_default_path")]
    pub path: PathSection,
    /// `[trust]` — the installation's trusted and untrusted package groups
    /// (`crate::trust`, PROP-065 `#ordinary-trust`). The site layer of the
    /// one trust mechanism: `trusted_groups` adds to the base list vibe
    /// ships, and `untrusted_groups` takes trust away — including the
    /// base's, which is how an administrator refuses a group vibe trusts
    /// out of the box for a whole installation.
    #[serde(default, skip_serializing_if = "TrustLists::is_empty")]
    pub trust: TrustLists,
}

fn is_default_path(section: &PathSection) -> bool {
    *section == PathSection::default()
}

/// Why a site profile, its stored copy or its receipt could not be used.
/// Every message names the file, the key and the fix.
///
/// ```
/// use vibe_core::site_profile::SiteProfileError;
/// let e = SiteProfileError::Invalid {
///     origin: "corp.toml".into(),
///     key: "artifacts.vibe".into(),
///     reason: "the path is relative".into(),
///     fix: "give an absolute directory".into(),
/// };
/// let text = e.to_string();
/// assert!(text.contains("corp.toml") && text.contains("artifacts.vibe"));
/// assert!(text.contains("PROP-019#site-profile") && text.contains("fix:"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
pub enum SiteProfileError {
    #[error(
        "could not read `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
          fix: check the path and its permissions)"
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "`{origin}` is not a valid site profile: {diagnostic} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
          fix: repair the TOML at the reported location — the tables are \
          schema, name, [[registry]], [[mirror]], [[override]], [[remap]], \
          [source_policy], [artifacts], [self], [proxy], [net], [path], [trust])"
    )]
    Parse {
        origin: String,
        #[source]
        diagnostic: Box<crate::SafeTomlParseError>,
    },
    #[error(
        "`{origin}` declares `schema = {found}`, this vibe reads schema {SITE_PROFILE_SCHEMA} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
          fix: set `schema = {SITE_PROFILE_SCHEMA}`, or update vibe to read the newer profile)"
    )]
    Schema { origin: String, found: u32 },
    #[error(
        "`{origin}`: `{key}` is invalid: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; fix: {fix})"
    )]
    Invalid {
        origin: String,
        key: String,
        reason: String,
        fix: String,
    },
    #[error(
        "`{origin}`: a `[[remap]]` row is invalid: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
          fix: correct the row as `vibe remap` documents it)"
    )]
    Remap {
        origin: String,
        #[source]
        source: Box<crate::remap::RemapParseError>,
    },
    #[error(
        "the receipt `{path}` is malformed: {diagnostic} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
          fix: run `vibe self apply --remove` and apply the profile again)"
    )]
    Receipt {
        path: PathBuf,
        #[source]
        diagnostic: Box<crate::SafeTomlParseError>,
    },
    #[error(
        "could not serialise `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
          fix: report this — the in-memory value is malformed)"
    )]
    Serialize {
        path: PathBuf,
        #[source]
        source: toml::ser::Error,
    },
}

impl SiteProfile {
    /// Parse and fully validate profile text; `origin` (a path or URL)
    /// labels every error. Nothing is accepted half-valid.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
    pub fn parse(text: &str, origin: &str) -> Result<Self, SiteProfileError> {
        let profile: SiteProfile =
            toml::from_str(text).map_err(|source| SiteProfileError::Parse {
                origin: origin.to_owned(),
                diagnostic: Box::new(crate::SafeTomlParseError::new(source, text)),
            })?;
        if profile.schema != SITE_PROFILE_SCHEMA {
            return Err(SiteProfileError::Schema {
                origin: origin.to_owned(),
                found: profile.schema,
            });
        }
        profile.validate(origin)?;
        Ok(profile)
    }

    fn validate(&self, origin: &str) -> Result<(), SiteProfileError> {
        let invalid = |key: String, reason: &str, fix: &str| SiteProfileError::Invalid {
            origin: origin.to_owned(),
            key,
            reason: reason.to_owned(),
            fix: fix.to_owned(),
        };
        if self
            .name
            .as_deref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(invalid(
                "name".into(),
                "it is empty",
                "give the profile a name or drop the key",
            ));
        }
        let mut names = BTreeSet::new();
        for (index, registry) in self.registries.iter().enumerate() {
            let key = format!("registry[{index}]");
            if registry.name.trim().is_empty() {
                return Err(invalid(
                    format!("{key}.name"),
                    "it is empty",
                    "name every registry",
                ));
            }
            if !names.insert(registry.name.as_str()) {
                return Err(invalid(
                    format!("{key}.name"),
                    &format!("`{}` is declared twice", registry.name),
                    "give each `[[registry]]` a distinct name",
                ));
            }
            if registry.url.trim().is_empty() {
                return Err(invalid(
                    format!("{key}.url"),
                    "it is empty",
                    "give the registry's url",
                ));
            }
            registry.checked_type().map_err(|error| {
                invalid(
                    format!("{key}.type"),
                    &error.to_string(),
                    "match the type to the url, or drop `type` to infer it",
                )
            })?;
        }
        for (index, mirror) in self.mirrors.iter().enumerate() {
            if mirror.of.trim().is_empty() || mirror.url.trim().is_empty() {
                return Err(invalid(
                    format!("mirror[{index}]"),
                    "`of` and `url` must both be set",
                    "name the registry (`of`, or `*`) and the mirror url",
                ));
            }
        }
        for (index, row) in self.overrides.iter().enumerate() {
            if row.pkgref.trim().is_empty() || row.source_url.trim().is_empty() {
                return Err(invalid(
                    format!("override[{index}]"),
                    "`pkgref` and `source_url` must both be set",
                    "name the package and its source",
                ));
            }
        }
        let empty = SourcePolicyDecl::default();
        EffectiveSourcePolicy::merge(&self.source_policy, &empty, &empty).map_err(|error| {
            invalid(
                "source_policy".into(),
                &error.to_string(),
                "use `github`, `gitverse` or `host:<name>`, each once",
            )
        })?;
        self.remap_rows(origin)?;
        home_paths::check(self).map_err(|(key, reason, fix)| invalid(key.into(), &reason, fix))?;
        for (key, value) in [
            ("self.url", self.self_source.url.as_deref()),
            ("self.api", self.self_source.api.as_deref()),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                return Err(invalid(
                    key.into(),
                    "it is empty",
                    "give a value or drop the key",
                ));
            }
        }
        if let Some((key, reason, fix)) = self.proxy.problem() {
            return Err(invalid(key.into(), &reason, fix));
        }
        if let Some((key, reason, fix)) = self.tls.problem() {
            return Err(invalid(key.into(), &reason, fix));
        }
        for (key, written) in [
            ("trust.trusted_groups", &self.trust.trusted_groups),
            ("trust.untrusted_groups", &self.trust.untrusted_groups),
        ] {
            for (index, entry) in written.iter().enumerate() {
                crate::trust::GroupPattern::parse(entry).map_err(|error| {
                    invalid(
                        format!("{key}[{index}]"),
                        &error.to_string(),
                        "name a group (`org.example`) or a subtree (`org.example.*`)",
                    )
                })?;
            }
        }
        Ok(())
    }

    /// The profile's `[[remap]]` rows as one local-layer source labelled
    /// [`SITE_PROFILE_SOURCE`] — what the composition root places beneath
    /// the user's own `settings.toml` rows.
    pub fn remap_rows(&self, origin: &str) -> Result<LoadedSource, SiteProfileError> {
        crate::remap::rows_from_decls(
            self.remap.clone(),
            Layer::Local,
            format!("{SITE_PROFILE_SOURCE} {origin}"),
        )
        .map_err(|source| SiteProfileError::Remap {
            origin: origin.to_owned(),
            source: Box::new(source),
        })
    }

    /// The registry layer (`PROP-002#registry-layers`, scope `site`): the
    /// tables a `registry.toml` would carry.
    pub fn registry_layer(&self) -> GlobalRegistryConfig {
        GlobalRegistryConfig {
            registries: self.registries.clone(),
            mirrors: self.mirrors.clone(),
            overrides: self.overrides.clone(),
            index: Default::default(),
            source_policy: self.source_policy.clone(),
        }
    }

    /// The user-config tables the profile supplies, as a `config.toml`
    /// would carry them.
    pub fn user_config_layer(&self) -> UserConfig {
        UserConfig {
            artifacts: self.artifacts.clone(),
            self_source: self.self_source.clone(),
            proxy: self.proxy.clone(),
            tls: self.tls.clone(),
            net: self.net.clone(),
            ..UserConfig::default()
        }
    }

    /// Whether the profile decides where `vibe self` reads releases from.
    pub fn declares_release_source(&self) -> bool {
        !self.self_source.release_is_default() || self.artifacts.vibe.is_some()
    }

    /// The `[path] fix` policy, `ask` when absent.
    pub fn path_fix(&self) -> PathFixPolicy {
        self.path.fix.unwrap_or_default()
    }
}

/// The stored, applied profile and where it lives.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join(vibe_core::site_profile::SITE_PROFILE_FILE);
/// assert!(vibe_core::site_profile::installed_at(&path).unwrap().is_none());
/// std::fs::write(&path, "schema = 1\nname = 'corp'\n").unwrap();
/// let site = vibe_core::site_profile::installed_at(&path).unwrap().unwrap();
/// assert_eq!(site.profile.name.as_deref(), Some("corp"));
/// assert_eq!(site.path, path);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledSiteProfile {
    pub path: PathBuf,
    pub profile: SiteProfile,
}

/// The applied profile of this settings directory, if any
/// (`<settings-dir>/site-profile.toml`). A missing file is `Ok(None)`; a
/// stored file that no longer validates (edited by hand) is an error, so it
/// is noticed rather than silently ignored.
pub fn installed() -> Result<Option<InstalledSiteProfile>, SiteProfileError> {
    match crate::settings::site_profile_path() {
        Some(path) => installed_at(&path),
        None => Ok(None),
    }
}

/// [`installed`] for an explicit path.
pub fn installed_at(path: &Path) -> Result<Option<InstalledSiteProfile>, SiteProfileError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(SiteProfileError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let profile = SiteProfile::parse(&text, &path.display().to_string())?;
    Ok(Some(InstalledSiteProfile {
        path: path.to_path_buf(),
        profile,
    }))
}

/// What the PATH step of the last apply did, recorded for `--show`.
///
/// ```
/// use vibe_core::site_profile::ReceiptPath;
/// let path: ReceiptPath =
///     toml::from_str("shim_dir = '/home/u/.vibe/opt/bin'\noutcome = 'applied'").unwrap();
/// assert_eq!(path.outcome, "applied");
/// assert_eq!(path.target, None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptPath {
    /// The shim directory the step was about.
    pub shim_dir: String,
    /// `already_on_path`, `applied` or `skipped`.
    pub outcome: String,
    /// Where the durable PATH lives (`~/.bashrc`, `HKCU\Environment\Path`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// `<settings-dir>/site-profile.receipt.toml`: what `vibe self apply`
/// installed. `owns` lists the settings-directory files apply wrote —
/// exactly what `--remove` deletes; nothing a user wrote is ever listed.
///
/// ```
/// use vibe_core::site_profile::SiteReceipt;
/// let r: SiteReceipt = toml::from_str(
///     "schema = 1\nsource = '/srv/corp.toml'\nsha256 = 'ab'\napplied_at = 't'\n\
///      vibe_version = '1.0.0'\nowns = ['site-profile.toml']\n",
/// ).unwrap();
/// assert_eq!(r.owns, ["site-profile.toml"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
pub struct SiteReceipt {
    pub schema: u32,
    /// The path or URL the profile was applied from.
    pub source: String,
    /// SHA-256 of the stored profile bytes.
    pub sha256: String,
    /// RFC 3339 time of the apply.
    pub applied_at: String,
    /// The vibe version that applied it.
    pub vibe_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Settings-directory basenames apply wrote and `--remove` deletes.
    pub owns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<ReceiptPath>,
}

impl SiteReceipt {
    /// Read a receipt; a missing file is `Ok(None)`.
    pub fn load_from(path: &Path) -> Result<Option<Self>, SiteProfileError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(SiteProfileError::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        toml::from_str(&text)
            .map(Some)
            .map_err(|source| SiteProfileError::Receipt {
                path: path.to_path_buf(),
                diagnostic: Box::new(crate::SafeTomlParseError::new(source, &text)),
            })
    }

    /// The receipt as it would be written over `existing`
    /// ([`crate::toml_preserve::render_preserving`], so a comment someone
    /// added to the receipt survives a re-apply).
    pub fn render(&self, existing: Option<&str>, path: &Path) -> Result<String, SiteProfileError> {
        crate::toml_preserve::render_preserving(existing, self).map_err(|source| {
            SiteProfileError::Serialize {
                path: path.to_path_buf(),
                source,
            }
        })
    }
}

#[path = "site_profile/home_paths.rs"]
mod home_paths;
#[path = "site_profile/windows_paths.rs"]
mod windows_paths;
pub use windows_paths::{is_absolute_anywhere, is_windows_absolute, starts_at_home};

#[cfg(test)]
#[path = "site_profile/tests.rs"]
mod tests;
