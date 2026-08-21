//! User-level configuration file: `<settings-dir>/config.toml`
//! (canonical `~/.vibe/config.toml`, or under `$VIBE_SETTINGS`).
//!
//! VIBEVM-SPEC §9.5 places this file fourth in the configuration
//! precedence chain (CLI flags > env vars > project `vibe.toml` >
//! user-level config > built-in defaults). The user-config layer
//! carries the `[env]` section — environment-variable defaults for
//! `VIBE_*` / `VIBEVM_*` names only, surfaced by `vibe show config` —
//! `[install]`, the install-
//! behaviour settings of [PROP-011](../../../spec/modules/vibe-workspace/PROP-011-incremental-install.md),
//! `[init]` (`vibe init` prompt defaults), and `[net]`, the network
//! posture of [PROP-010](../../../spec/modules/vibe-registry/PROP-010-local-package-cache.md)
//! §2.5.
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
//! `net.offline` from this layer today, beside `vibe show config`'s
//! display read. Wiring further values (`default_cache_root`,
//! `init_tracing`, future LLM-key paths) remains follow-up work.
//! decision on env-var promotion vs. dedicated config-getters).
//! Until then this module is informational; the operator must
//! `export VIBE_REGISTRY_CACHE=…` for the value to actually apply.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#materialise-diff");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Once;

use serde::{Deserialize, Serialize};
use specmark::spec;

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
}

/// `[install]` section — install-behaviour settings (PROP-011 §5.2).
///
/// ```
/// use vibe_core::user_config::{InstallConfig, SlotIntegrity};
///
/// let c: InstallConfig = toml::from_str(r#"slot_integrity = "verify""#).unwrap();
/// assert_eq!(c.slot_integrity, SlotIntegrity::Verify);
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
}

impl InstallConfig {
    /// `true` for the all-defaults section — lets the serializer skip
    /// `[install]` entirely on a config that never set it.
    pub fn is_default(&self) -> bool {
        *self == InstallConfig::default()
    }
}

/// `[init]` section — defaults persisted from `vibe init` prompts.
///
/// ```
/// use vibe_core::user_config::InitConfig;
///
/// let c = InitConfig::default();
/// assert!(c.last_author.is_none());
/// assert!(c.is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InitConfig {
    /// The author name entered in the last interactive `vibe init`.
    /// Saved on first use, updated when the user enters a different
    /// value. Reused as the default for subsequent inits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_author: Option<String>,
}

impl InitConfig {
    pub fn is_default(&self) -> bool {
        *self == InitConfig::default()
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
        if let Some(custom) = std::env::var_os("VIBEVM_USER_CONFIG") {
            return Some(PathBuf::from(custom));
        }
        let canonical = config_file_candidates().into_iter().next();
        warn_once_about_a_left_behind_config(canonical.as_deref());
        canonical
    }

    /// Read the user-level config from the [`Self::default_path`].
    /// Missing-file is `Ok(UserConfig::default())` — the layer is
    /// optional. Parse errors surface so the operator notices a
    /// malformed file rather than silently ignoring it.
    pub fn load() -> Result<Self, UserConfigError> {
        let Some(path) = Self::default_path() else {
            return Ok(UserConfig::default());
        };
        Self::load_from(&path)
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
            source,
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

    /// Save to an explicit path (tests, ad-hoc).
    pub fn save_to(&self, path: &Path) -> Result<(), UserConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| UserConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let body = toml::to_string_pretty(self).map_err(|source| UserConfigError::Serialize {
            path: path.to_path_buf(),
            source,
        })?;
        std::fs::write(path, body).map_err(|source| UserConfigError::Io {
            path: path.to_path_buf(),
            source,
        })
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
        "`{path}` is malformed: {source} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order; \
          fix: repair the TOML at the reported location)"
    )]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
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
}

/// The ordered on-disk user-config candidates: exactly one,
/// `<settings-dir>/config.toml` in the canonical settings dir (`~/.vibe`,
/// or `$VIBE_SETTINGS`).
///
/// This is the entire disk surface of the user-config layer and its single
/// authority. [`UserConfig::default_path`] resolves its non-override answer
/// from here, and `settings`'s
/// `every_accessor_is_rooted_in_the_one_settings_dir` walks the same list —
/// so the two cannot drift, and a candidate added here that is not rooted
/// in the settings dir goes red in a gate rather than surviving until a
/// campaign goes looking for it.
///
/// It stays a list rather than collapsing into a bare path because a second
/// entry is the exact defect removed on 2026-07-26. One directory means
/// `$VIBE_SETTINGS` relocates every candidate at once, so an isolated run
/// cannot reach the operator's real config — the property a second,
/// separately-rooted home silently took away.
pub(crate) fn config_file_candidates() -> Vec<PathBuf> {
    crate::settings::user_config_path().into_iter().collect()
}

/// Guards [`warn_once_about_a_left_behind_config`]. `default_path` runs on
/// every invocation and more than once in some (`vibe show config` reports
/// the path and then loads it), so an unguarded diagnostic would repeat
/// per call. Once per process, whatever the call pattern.
static LEFT_BEHIND_NOTICE: Once = Once::new();

/// Emit [`left_behind_notice`] on stderr, at most once per process.
fn warn_once_about_a_left_behind_config(canonical: Option<&Path>) {
    LEFT_BEHIND_NOTICE.call_once(|| {
        if let Some(line) = left_behind_notice(canonical, former_config_path().as_deref()) {
            eprintln!("{line}");
        }
    });
}

/// The one-line notice for a config left at the pre-consolidation
/// location. `Some(line)` only when no canonical config exists *and* a file
/// does sit at the old one — i.e. exactly when removing that read stopped a
/// file that used to be read from being read. Switching silently is the
/// failure this prevents; it is the one thing the 2026-07-26 change added
/// rather than removed, so it is one line and no more.
///
/// Pure, and it takes both paths instead of resolving them, so the
/// message's shape is assertable without an environment.
///
/// What it does **not** do is the point. It does not read the file, does
/// not copy it, does not move it, and prints no byte of its contents: a
/// `[env]` table is a plausible place for someone to have parked a
/// credential, so the rule `vibe_cli::promote_user_config_env` follows for
/// refused names holds here too — once, by path, never the contents.
fn left_behind_notice(canonical: Option<&Path>, former: Option<&Path>) -> Option<String> {
    let (canonical, former) = (canonical?, former?);
    if canonical.exists() || !former.is_file() {
        return None;
    }
    Some(format!(
        "vibe: warning: `{}` is no longer read; move it to `{}` for it to take effect",
        former.display(),
        canonical.display(),
    ))
}

/// The pre-consolidation user-config location: `%APPDATA%\vibe\config.toml`
/// on Windows, else `<home>/.config/vibe/config.toml`.
///
/// **Not a resolution leg.** Nothing here reads this path, nothing writes
/// it, and [`UserConfig::default_path`] never returns it. It exists so
/// [`left_behind_notice`] can name a file that used to be read, and for no
/// other purpose — keeping it out of [`config_file_candidates`] is the
/// whole change.
///
/// It deliberately does not consult the XDG config-home variable, which the
/// removed leg checked first. A probe steered by a redirect variable would
/// re-create the removed shape one step weaker: a run isolated by
/// `$VIBE_SETTINGS` would still resolve a path from an ambient variable the
/// harness does not control. Home is read the one way the settings dir
/// reads it, and nothing redirects it. The cost, named rather than hidden:
/// an operator who had pointed the XDG config home somewhere non-default
/// gets no notice — only the switch to `<settings-dir>/config.toml`, which
/// is correct either way.
fn former_config_path() -> Option<PathBuf> {
    // Windows precedence: `%APPDATA%` wins over `~/.config`, which is not
    // the canonical Windows shape.
    if cfg!(windows)
        && let Some(appdata) = std::env::var_os("APPDATA").filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(appdata).join("vibe").join("config.toml"));
    }
    Some(home_dir()?.join(".config").join("vibe").join("config.toml"))
}

/// Best-effort home-directory detection. Reads `HOME` on Unix and
/// `USERPROFILE` on Windows (or `HOME` as a fallback for Git Bash
/// / WSL shells that set both). Avoids pulling in the `dirs` crate
/// for one lookup.
fn home_dir() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(h));
    }
    if cfg!(windows)
        && let Some(p) = std::env::var_os("USERPROFILE").filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(p));
    }
    None
}

#[cfg(test)]
#[path = "user_config/tests.rs"]
mod tests;
