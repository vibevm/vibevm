//! User-level configuration file: `<settings-dir>/config.toml`
//! (canonical `~/.vibe/config.toml`, or under `$VIBE_SETTINGS`).
//!
//! VIBEVM-SPEC §9.5 places this file fourth in the configuration
//! precedence chain (CLI flags > env vars > project `vibe.toml` >
//! user-level config > built-in defaults). The user-config layer
//! carries the `[env]` section — environment-variable defaults for
//! `VIBE_*` / `VIBEVM_*` names only, surfaced by `vibe show config` —
//! `[install]`, the install-
//! behaviour settings of [PROP-011](../../../vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml),
//! `[init]` (`vibe init` prompt defaults), `[net]`, the network
//! posture of [PROP-010](../../../vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml)
//! §2.5, `[llm]`, the user-owned OpenAI-compatible provider
//! endpoint and token-file source, and `[slot.<slot>.extras.<ecosystem>]`,
//! the user's own per-slot ecosystem extras list
//! ([PROP-075](../../../vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml)
//! `#SLOT-EXTRAS`).
//!
//! Path resolution:
//!
//! - `VIBEVM_USER_CONFIG` env-var, when set, points at the file
//!   directly (override; useful for tests + ad-hoc invocations).
//! - Otherwise the canonical `<settings-dir>/config.toml` via the one
//!   `crate::settings` chokepoint.
//!
//! That is the whole list. The single on-disk leg hangs off the one
//! settings dir, so `$VIBE_SETTINGS` relocates the user-config layer
//! together with every credential. The pre-consolidation location
//! (`%APPDATA%\vibe\config.toml` on Windows, else
//! `$HOME/.config/vibe/config.toml`, or wherever the XDG config-home
//! variable redirected it) supplied a second leg until 2026-07-26; that
//! read was removed because `$VIBE_SETTINGS` deliberately did not
//! relocate it, which left a path by which an isolated run still reached
//! the operator's real `config.toml` whenever the isolated settings dir
//! held none — the normal case for a fresh temp home. A config file
//! still sitting there is the operator's to move into `~/.vibe`: vibevm
//! does not read it, does not copy it, and does not touch it. It does
//! say so once ([`left_behind_notice`]), because a config that quietly
//! stopped being read is the failure that rule exists to prevent.
//!
//! The zero-consumer v0 scoping is history: `vibe install` reads
//! `install.slot_integrity` (the PROP-011 §5.2 strategy) and
//! `net.offline`; `vibe-llm`'s effective resolver consumes `[llm]` together
//! with the selected file path for token resolution; `vibe show config`
//! performs the display read. Wiring unrelated values such as
//! `default_cache_root` and `init_tracing` remains follow-up work.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#materialise-diff");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

pub(crate) mod artifacts;
mod init;
mod proxy;
pub use init::InitConfig;
mod location;
pub(crate) use location::config_file_candidates;
use location::warn_once_about_a_left_behind_config;
mod slot;
pub use slot::{SlotConfig, SlotExtras, read_item_lines};
mod template;
pub use template::TemplateConfig;
mod versions;
pub use versions::{VersionSelect, VersionsConfig};
mod tls;
pub use tls::{ExtraCa, TlsConfig, effective_extra_ca};
mod transfer;
pub use artifacts::{ArtifactsConfig, ReleaseSourceKind, SelfConfig, UnknownReleaseSource};
pub use artifacts::{InstallRootError, install_root_at};
pub use proxy::{PX_OWNED_ARGS, PacSource, ProxyConfig, ProxyMode, RevocationCheck};
pub use transfer::TransferConfig;

/// Parsed `<settings-dir>/config.toml` (canonical `~/.vibe/config.toml`).
///
/// ```
/// use vibe_core::user_config::UserConfig;
///
/// // The all-defaults config: no env fallbacks, default install
/// // settings. This is what `load()` returns when no file exists.
/// let cfg = UserConfig::default();
/// assert!(cfg.env.is_empty());
/// assert!(cfg.install.is_default());
/// assert!(cfg.net.is_default());
/// assert!(cfg.llm.is_none());
/// assert!(cfg.proxy.is_default());
/// assert!(cfg.tls.is_default());
/// assert!(cfg.template.is_default());
/// assert!(cfg.versions.is_default());
/// assert!(cfg.slot.is_empty());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserConfig {
    /// Default values for environment variables. Treated as fallbacks
    /// — a real env-var (set in the live environment at vibe
    /// invocation time) wins.
    ///
    /// Only `VIBE_*` / `VIBEVM_*` names are ever promoted into the
    /// process environment: the CLI's startup promotion
    /// (`vibe_cli::promote_user_config_env`, which owns the rule and
    /// the reasoning) ignores every other name and reports it once, by
    /// name. The allowlist is a promotion rule, not a schema rule —
    /// parsing keeps the whole table, so an entry that does nothing is
    /// still visible to whoever reads the file back rather than
    /// vanishing between the disk and the struct.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,

    /// `[install]` — install-behaviour settings (PROP-011).
    #[serde(default, skip_serializing_if = "InstallConfig::is_default")]
    pub install: InstallConfig,

    /// `[init]` — defaults for `vibe init` prompts. `last_author` is
    /// saved after the first interactive `vibe init` (or updated when
    /// the user enters a different author), then reused as the default
    /// for subsequent inits — npm's "license + author remember" pattern.
    #[serde(default, skip_serializing_if = "InitConfig::is_default")]
    pub init: InitConfig,

    /// `[net]` — the network posture (PROP-010 §2.5). The lowest rung
    /// of the offline ladder: the `--offline` flag wins, then the
    /// `VIBE_OFFLINE` env-var, then this key.
    #[serde(default, skip_serializing_if = "NetConfig::is_default")]
    pub net: NetConfig,

    /// `[llm]` — user-owned provider defaults and endpoint. Project
    /// `[llm]` may override provider/model and name an env credential;
    /// it cannot supply an endpoint or token-file path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmConfig>,

    /// `[proxy]` — the one proxy table every HTTP client and child `git` reads (`PROP-066#locator`).
    #[serde(default, skip_serializing_if = "ProxyConfig::is_default")]
    pub proxy: ProxyConfig,

    /// `[tls]` — the certificates trusted besides the operating system's
    /// own (`PROP-068#proxy`). `[proxy] ca_file` is the older spelling of
    /// the same thing and both are read.
    #[serde(default, skip_serializing_if = "TlsConfig::is_default")]
    pub tls: TlsConfig,

    /// `[transfer]` — the download rate limit (`PROP-068#limit`).
    #[serde(default, skip_serializing_if = "TransferConfig::is_default")]
    pub transfer: TransferConfig,
    /// `[artifacts]` — the hand-filled artifact drops (`PROP-066#drops`);
    /// `[self]` below — the release source (`PROP-019#release-sources`).
    #[serde(default, skip_serializing_if = "ArtifactsConfig::is_default")]
    pub artifacts: ArtifactsConfig,
    #[serde(rename = "self")]
    #[serde(default, skip_serializing_if = "SelfConfig::is_default")]
    pub self_source: SelfConfig,
    /// `[template]` — how long a cached template version list is used
    /// before it is refetched (PROP-075 `#TEMPLATE-CACHE`).
    #[serde(default, skip_serializing_if = "TemplateConfig::is_default")]
    pub template: TemplateConfig,
    /// `[versions]` — which version of an application package a
    /// requirement lands on when only part of the domain is installable
    /// here (PROP-075 `#VERSIONS-SELECT`).
    #[serde(default, skip_serializing_if = "VersionsConfig::is_default")]
    pub versions: VersionsConfig,
    /// `[slot.<slot>]` — the user's own per-slot settings, today just
    /// their per-ecosystem extras list (PROP-075 `#SLOT-EXTRAS`). Keyed by
    /// the application slot id; a slot nothing installs is kept, because a
    /// hand-written settings file outlives the packages it mentions and the
    /// consumer reports such a line once rather than refusing the file.
    ///
    /// **This struct is `deny_unknown_fields`**, so the table had to be
    /// declared here before a user could write it at all: without it, a
    /// `[slot.node.extras.npm]` line was a parse error on every command.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slot: BTreeMap<String, SlotConfig>,
}

impl UserConfig {
    /// One `[slot.<slot>.extras.<ecosystem>]` table, or `None` when the
    /// file names neither (PROP-075 `#SLOT-EXTRAS`).
    pub fn extras_of(&self, slot: &str, ecosystem: &str) -> Option<&SlotExtras> {
        self.slot.get(slot)?.extras.get(ecosystem)
    }

    /// The `[slot.<slot>.extras.<ecosystem>]` table to edit, created empty
    /// when the file holds neither — what `vibe app extras add|drop|import`
    /// writes into before [`Self::save`].
    pub fn extras_mut(&mut self, slot: &str, ecosystem: &str) -> &mut SlotExtras {
        self.slot
            .entry(slot.to_owned())
            .or_default()
            .extras
            .entry(ecosystem.to_owned())
            .or_default()
    }
}

/// `[install]` section — install-behaviour settings (PROP-011 §5.2).
///
/// ```
/// use vibe_core::manifest::SpecFormat;
/// use vibe_core::user_config::{InstallConfig, SlotIntegrity};
///
/// let c: InstallConfig = toml::from_str(
///     "slot_integrity = \"verify\"\nspec_format = \"xml\"",
/// ).unwrap();
/// assert_eq!(c.slot_integrity, SlotIntegrity::Verify);
/// assert_eq!(c.spec_format, Some(SpecFormat::Xml));
/// assert!(InstallConfig::default().is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstallConfig {
    /// How `vibe install` treats a `vibedeps/` slot that already exists
    /// for the resolved version (PROP-011 §2.3). Default:
    /// [`SlotIntegrity::TrustPresence`].
    #[serde(default)]
    pub slot_integrity: SlotIntegrity,
    /// Operator default for projects whose `[project]` table does not pin a
    /// reproducible materialisation representation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_format: Option<crate::manifest::SpecFormat>,
}

impl InstallConfig {
    /// `true` for the all-defaults section — lets the serializer skip
    /// `[install]` entirely on a config that never set it.
    pub fn is_default(&self) -> bool {
        *self == InstallConfig::default()
    }
}

/// `[net]` section — the network posture (PROP-010 §2.5).
///
/// ```
/// use vibe_core::user_config::NetConfig;
///
/// let c: NetConfig = toml::from_str("offline = true").unwrap();
/// assert!(c.offline);
/// assert!(NetConfig::default().is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetConfig {
    /// Resolve every registry-touching invocation offline by default:
    /// resolution and fetch must be satisfiable from local sources,
    /// and a package not available locally is a hard error (PROP-010
    /// §2.5). The lowest rung of the ladder — `--offline` and
    /// `VIBE_OFFLINE` each win over this key. Default `false`: online
    /// remains the default and is unchanged.
    #[serde(default)]
    pub offline: bool,
}

impl NetConfig {
    /// `true` for the all-defaults section — lets the serializer skip
    /// `[net]` entirely on a config that never set it.
    pub fn is_default(&self) -> bool {
        *self == NetConfig::default()
    }
}

/// `[llm]` section in the user config.
///
/// ```
/// use vibe_core::user_config::LlmConfig;
///
/// let c: LlmConfig = toml::from_str(r#"
///     provider = "openai-compatible"
///     model = "example-chat"
///     endpoint = "https://example.invalid/v1/chat/completions"
///     token_file = "provider.token"
/// "#).unwrap();
/// assert_eq!(c.provider, "openai-compatible");
/// assert_eq!(c.token_file.unwrap().to_string_lossy(), "provider.token");
/// ```
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct LlmConfig {
    /// Exact provider adapter id. R7 accepts `openai-compatible` only.
    pub provider: String,
    /// User-level model fallback, overridden by project
    /// `[llm].default_model` when that section exists.
    pub model: String,
    /// Full Chat Completions endpoint URL. There is deliberately no
    /// project-level counterpart.
    pub endpoint: String,
    /// Optional credential file. Relative paths resolve beside the selected
    /// user-config file; absolute paths remain absolute; `~` has no magic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_file: Option<PathBuf>,
}

impl std::fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("endpoint", &"[REDACTED ENDPOINT]")
            .field("token_file", &self.token_file)
            .finish()
    }
}

/// `[install].slot_integrity` — the materialisation slot-skip strategy
/// (PROP-011 §2.3 / §5.2). Chosen once in the user config; it persists
/// across runs.
///
/// ```
/// use vibe_core::user_config::SlotIntegrity;
///
/// // The default trusts a slot present for the resolved version (PROP-011 §2.3);
/// // the wire form `slot_integrity = "verify"` is shown on `InstallConfig`.
/// assert_eq!(SlotIntegrity::default(), SlotIntegrity::TrustPresence);
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SlotIntegrity {
    /// A `vibedeps/` slot present for the resolved version is trusted —
    /// `vibe install` skips re-copying it. Versions are immutable, so a
    /// slot for the exact version is correct content; this is the
    /// **default**, and the win PROP-011 §2.3 ships. A hand-corrupted
    /// slot is repaired with `vibe reinstall --force`.
    #[default]
    TrustPresence,
    /// A present slot is re-materialised regardless — its content is
    /// re-copied from source on every install, so a hand-edited or
    /// corrupted slot is silently overwritten. Trades the §2.3 speed-up
    /// for a per-install correctness guarantee.
    Verify,
}

impl UserConfig {
    /// Path the loader would consult, given the current environment.
    /// Returns `None` on platforms where no home / settings directory
    /// can be determined.
    ///
    /// Two legs, and no third: the `VIBEVM_USER_CONFIG` override — an
    /// explicit file, not a home — then the one on-disk candidate in
    /// [`config_file_candidates`]. Whether that candidate exists does not
    /// change the answer; a missing file is `UserConfig::default()`, and
    /// the reported path is where [`UserConfig::save`] would write.
    pub fn default_path() -> Option<PathBuf> {
        if let Some(custom) = location::override_path() {
            return Some(custom);
        }
        let canonical = config_file_candidates().into_iter().next();
        warn_once_about_a_left_behind_config(canonical.as_deref());
        canonical
    }

    /// The **effective** user-level configuration: the file at
    /// [`Self::default_path`] layered over the applied site profile
    /// (`PROP-019#site-profile`, [`Self::over_site`]). Missing files are
    /// defaults — both layers are optional. Parse errors surface so the
    /// operator notices a malformed file rather than silently ignoring it.
    ///
    /// A command that edits and saves the user's file reads it with
    /// [`Self::load_own`], so no site value is ever written into it.
    pub fn load() -> Result<Self, UserConfigError> {
        let own = Self::load_own()?;
        match crate::site_profile::installed().map_err(|e| UserConfigError::Site(Box::new(e)))? {
            Some(site) => Ok(own.over_site(&site.profile)),
            None => Ok(own),
        }
    }

    /// The user's own file at [`Self::default_path`], without the site
    /// layer — what an editor loads before [`Self::save`].
    pub fn load_own() -> Result<Self, UserConfigError> {
        let Some(path) = Self::default_path() else {
            return Ok(UserConfig::default());
        };
        Self::load_from(&path)
    }

    /// This (user) configuration over a site profile
    /// (`PROP-019#site-profile`). The release decision — `[self]` together
    /// with `[artifacts] vibe` — comes whole from the higher layer that
    /// declares either, so a user's own drop is never outranked by the
    /// site's `[self]`; `[artifacts] packages` and each `[proxy]` key fall
    /// back to the site's value; `[net] offline` is on when either says so.
    ///
    /// ```
    /// use vibe_core::site_profile::SiteProfile;
    /// use vibe_core::user_config::{ReleaseSourceKind, UserConfig};
    ///
    /// let site = SiteProfile::parse(
    ///     "schema = 1\n[self]\nsource = 'nexus'\nurl = 'https://nexus.corp.example/vibe'\n\
    ///      [artifacts]\npackages = '/srv/drop'\n[proxy]\nhttps = 'http://proxy:3128'\n",
    ///     "corp.toml",
    /// )
    /// .unwrap();
    /// let effective = UserConfig::default().over_site(&site);
    /// assert_eq!(effective.self_source.source, Some(ReleaseSourceKind::Nexus));
    /// assert_eq!(effective.proxy.https.as_deref(), Some("http://proxy:3128"));
    ///
    /// // A user's own release drop keeps the release decision.
    /// let mine: UserConfig = toml::from_str("[artifacts]\nvibe = '/home/me/drop'\n").unwrap();
    /// let effective = mine.over_site(&site);
    /// assert_eq!(effective.self_source.source, None);
    /// assert_eq!(effective.artifacts.packages.unwrap().to_str(), Some("/srv/drop"));
    /// ```
    pub fn over_site(mut self, site: &crate::site_profile::SiteProfile) -> UserConfig {
        // The release decision is one pair, taken whole from the higher
        // declaring layer; `[self] install_root` answers a different
        // question and layers on its own, so a user who picked a release
        // source still inherits the site's install root.
        if self.self_source.release_is_default() && self.artifacts.vibe.is_none() {
            let install_root = self.self_source.install_root.take();
            self.self_source = site.self_source.clone();
            self.self_source.install_root = install_root;
            self.artifacts.vibe = site.artifacts.vibe.clone();
        }
        if self.self_source.install_root.is_none() {
            self.self_source.install_root = site.self_source.install_root.clone();
        }
        if self.artifacts.packages.is_none() {
            self.artifacts.packages = site.artifacts.packages.clone();
        }
        self.proxy = std::mem::take(&mut self.proxy).over(&site.proxy);
        self.tls = std::mem::take(&mut self.tls).over(&site.tls);
        self.net.offline |= site.net.offline;
        // `[versions] select` is one value and falls back by absence, so
        // a site can set a closed network's posture and a developer can
        // still override it per machine (PROP-075 `#VERSIONS-SELECT`).
        // Not an `|=` like `offline`: the three policies are not ordered
        // by strictness in a way a merge could pick from, and a site that
        // wants `drop-only` enforced says so by being the only declaring
        // layer.
        if self.versions.select.is_none() {
            self.versions.select = site.versions.select;
        }
        self
    }

    /// Like [`Self::load`] but reads from an explicit path. Used by
    /// the entry-point loader and by tests.
    pub fn load_from(path: &Path) -> Result<Self, UserConfigError> {
        if !path.exists() {
            return Ok(UserConfig::default());
        }
        let body = std::fs::read_to_string(path).map_err(|source| UserConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let cfg: UserConfig = toml::from_str(&body).map_err(|source| UserConfigError::Parse {
            path: path.to_path_buf(),
            diagnostic: Box::new(crate::SafeTomlParseError::new(source, &body)),
        })?;
        Ok(cfg)
    }

    /// Save the user config to its canonical path, creating the parent
    /// directory if needed. Used by `vibe init` to persist `last_author`.
    pub fn save(&self) -> Result<(), UserConfigError> {
        let Some(path) = crate::settings::user_config_path() else {
            return Ok(()); // no settings dir — the layer is optional
        };
        self.save_to(&path)
    }

    /// Save to an explicit path, keeping an existing file's comments and
    /// unchanged bytes ([`crate::toml_preserve::render_preserving`]).
    pub fn save_to(&self, path: &Path) -> Result<(), UserConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| UserConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let io = |source| UserConfigError::Io {
            path: path.to_path_buf(),
            source,
        };
        let existing = crate::global_registry::read_existing(path).map_err(io)?;
        let body = crate::toml_preserve::render_preserving(existing.as_deref(), self).map_err(
            |source| UserConfigError::Serialize {
                path: path.to_path_buf(),
                source,
            },
        )?;
        std::fs::write(path, body).map_err(io)
    }
}

/// Why loading the user config failed — an I/O error reading the file, or
/// a TOML parse error. Missing-file is *not* an error (the layer is
/// optional); each variant's `Display` cites the governing REQ.
///
/// ```
/// use vibe_core::user_config::UserConfigError;
///
/// let e = UserConfigError::Io {
///     path: "/etc/vibe/config.toml".into(),
///     source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
/// };
/// assert!(e.to_string().contains("could not read"));
/// assert!(e.to_string().contains("configuration-sources-in-precedence-order"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order"
)]
pub enum UserConfigError {
    #[error(
        "could not read `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order; \
          fix: check the file's permissions, or remove it to fall back to defaults)"
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "`{path}` is malformed: {diagnostic} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order; \
          fix: repair the TOML at the reported location)"
    )]
    Parse {
        path: PathBuf,
        #[source]
        diagnostic: Box<crate::SafeTomlParseError>,
    },
    #[error(
        "could not serialise the user config for `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order)"
    )]
    Serialize {
        path: PathBuf,
        #[source]
        source: toml::ser::Error,
    },
    /// The applied site profile (`PROP-019#site-profile`) is unreadable.
    #[error(transparent)]
    Site(Box<crate::site_profile::SiteProfileError>),
}

#[cfg(test)]
#[path = "user_config/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "user_config/slot_tests.rs"]
mod slot_tests;
