//! User-level configuration file: `~/.config/vibe/config.toml`.
//!
//! VIBEVM-SPEC §9.5 places this file fourth in the configuration
//! precedence chain (CLI flags > env vars > project `vibe.toml` >
//! user-level config > built-in defaults). The user-config layer
//! carries two sections: `[env]` — environment-variable defaults,
//! surfaced by `vibe show config` — and `[install]`, the install-
//! behaviour settings of [PROP-011](../../../spec/modules/vibe-workspace/PROP-011-incremental-install.md).
//!
//! Path resolution:
//!
//! - `VIBEVM_USER_CONFIG` env-var, when set, points at the file
//!   directly (override; useful for tests + ad-hoc invocations).
//! - Otherwise: `$XDG_CONFIG_HOME/vibe/config.toml` if `XDG_CONFIG_HOME`
//!   is set; else `$HOME/.config/vibe/config.toml` on Unix and
//!   `%APPDATA%\vibe\config.toml` on Windows.
//!
//! v0 deliberately scopes "what runtime consumers do with this layer"
//! to ZERO — only `vibe show config` reads it today. Wiring user-
//! config values into `default_cache_root` / `init_tracing` / future
//! LLM-key paths is a follow-up slice (it requires a workspace-wide
//! decision on env-var promotion vs. dedicated config-getters).
//! Until then this module is informational; the operator must
//! `export VIBE_REGISTRY_CACHE=…` for the value to actually apply.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-011#materialise-diff");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

/// Parsed `~/.config/vibe/config.toml`.
///
/// ```
/// use vibe_core::user_config::UserConfig;
///
/// // The all-defaults config: no env fallbacks, default install
/// // settings. This is what `load()` returns when no file exists.
/// let cfg = UserConfig::default();
/// assert!(cfg.env.is_empty());
/// assert!(cfg.install.is_default());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserConfig {
    /// Default values for environment variables. Treated as fallbacks
    /// — a real env-var (set in the live environment at vibe
    /// invocation time) wins.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,

    /// `[install]` — install-behaviour settings (PROP-011).
    #[serde(default, skip_serializing_if = "InstallConfig::is_default")]
    pub install: InstallConfig,
}

/// `[install]` section — install-behaviour settings (PROP-011 §5.2).
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

/// `[install].slot_integrity` — the materialisation slot-skip strategy
/// (PROP-011 §2.3 / §5.2). Chosen once in the user config; it persists
/// across runs.
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
    /// Returns `None` on platforms where no home / config directory
    /// can be determined.
    pub fn default_path() -> Option<PathBuf> {
        if let Some(custom) = std::env::var_os("VIBEVM_USER_CONFIG") {
            return Some(PathBuf::from(custom));
        }
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|s| !s.is_empty()) {
            return Some(PathBuf::from(xdg).join("vibe").join("config.toml"));
        }
        let home = home_dir()?;
        if cfg!(windows) {
            // Windows precedence: %APPDATA% wins over ~/.config (which
            // is not the canonical Windows shape) when no XDG_CONFIG_HOME
            // is set.
            if let Some(appdata) = std::env::var_os("APPDATA").filter(|s| !s.is_empty()) {
                return Some(PathBuf::from(appdata).join("vibe").join("config.toml"));
            }
        }
        Some(home.join(".config").join("vibe").join("config.toml"))
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
}

#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order")]
pub enum UserConfigError {
    #[error(
        "could not read `{path}`: {source} \
         (violates spec://vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order; \
          fix: check the file's permissions, or remove it to fall back to defaults)"
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "`{path}` is malformed: {source} \
         (violates spec://vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order; \
          fix: repair the TOML at the reported location)"
    )]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
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
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_is_empty_env() {
        let cfg = UserConfig::default();
        assert!(cfg.env.is_empty());
    }

    #[test]
    fn load_from_missing_file_is_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = UserConfig::load_from(&path).unwrap();
        assert_eq!(cfg, UserConfig::default());
    }

    #[test]
    fn load_from_parses_env_block() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"[env]
VIBE_REGISTRY_CACHE = "/custom/cache"
VIBE_LOG = "vibe_registry=debug"
"#,
        )
        .unwrap();
        let cfg = UserConfig::load_from(&path).unwrap();
        assert_eq!(
            cfg.env.get("VIBE_REGISTRY_CACHE").map(String::as_str),
            Some("/custom/cache")
        );
        assert_eq!(
            cfg.env.get("VIBE_LOG").map(String::as_str),
            Some("vibe_registry=debug")
        );
    }

    #[test]
    fn load_from_rejects_unknown_top_level_section() {
        // `deny_unknown_fields` keeps the schema strict so a typo
        // surfaces instead of a silent no-op.
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"[envv]
VIBE_REGISTRY_CACHE = "/typo"
"#,
        )
        .unwrap();
        let err = UserConfig::load_from(&path).unwrap_err();
        assert!(matches!(err, UserConfigError::Parse { .. }));
    }

    #[test]
    fn load_from_malformed_toml_errors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is = not = toml").unwrap();
        let err = UserConfig::load_from(&path).unwrap_err();
        assert!(matches!(err, UserConfigError::Parse { .. }));
    }

    // --- PROP-011 §5.2 — the `[install]` section --------------------------

    #[test]
    fn slot_integrity_defaults_to_trust_presence() {
        let cfg = UserConfig::default();
        assert_eq!(cfg.install.slot_integrity, SlotIntegrity::TrustPresence);
        assert!(cfg.install.is_default());
    }

    #[test]
    fn load_from_parses_install_slot_integrity() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[install]\nslot_integrity = \"verify\"\n").unwrap();
        let cfg = UserConfig::load_from(&path).unwrap();
        assert_eq!(cfg.install.slot_integrity, SlotIntegrity::Verify);
        assert!(!cfg.install.is_default());
    }

    #[test]
    fn install_section_round_trips() {
        let cfg = UserConfig {
            install: InstallConfig {
                slot_integrity: SlotIntegrity::Verify,
            },
            ..Default::default()
        };
        let rendered = toml::to_string_pretty(&cfg).unwrap();
        assert!(
            rendered.contains("slot_integrity = \"verify\""),
            "{rendered}"
        );
        let back: UserConfig = toml::from_str(&rendered).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn load_from_rejects_an_unknown_install_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[install]\nbogus = true\n").unwrap();
        assert!(matches!(
            UserConfig::load_from(&path).unwrap_err(),
            UserConfigError::Parse { .. }
        ));
    }
}
