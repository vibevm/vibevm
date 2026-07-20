//! The canonical per-user vibevm settings directory and the typed paths
//! that hang off it.
//!
//! Before this module the settings home was resolved eight different ways
//! across the workspace — `dirs::home_dir()` in some crates, a hand-rolled
//! `HOME → USERPROFILE` walk in others — each joining its own `".vibe"` /
//! `".vibevm"` string literal. This module is the single authority:
//!
//! - `$VIBE_SETTINGS`, when set, is the settings directory, verbatim. It
//!   lets a second application that contends for `~/.vibe` push vibevm's
//!   settings elsewhere without touching config.
//! - Otherwise the settings directory is `<home>/.vibe` — the canonical
//!   location.
//! - The pre-consolidation `<home>/.vibevm` survives only as a
//!   backward-compatible *read* fallback ([`legacy_settings_dir`]) so a
//!   machine (or teammate) mid-migration keeps working; nothing is ever
//!   written there.
//!
//! Home is resolved one way for the whole workspace (`HOME`, then
//! `USERPROFILE` on Windows), so every settings path agrees. VIBEVM-SPEC
//! §9.5 places user-level configuration in the precedence chain; this
//! module fixes *where* that configuration lives.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order");

use std::path::PathBuf;

/// Environment variable that overrides the settings directory wholesale.
/// When set (non-empty), its value is used verbatim as the settings dir.
pub const SETTINGS_DIR_ENV: &str = "VIBE_SETTINGS";

/// Canonical settings-dir name under the user's home.
const CANONICAL_DIR: &str = ".vibe";

/// Pre-consolidation settings-dir name — a read-only migration fallback.
const LEGACY_DIR: &str = ".vibevm";

/// The canonical per-user settings directory.
///
/// Precedence: `$VIBE_SETTINGS` (verbatim) → `<home>/.vibe`. Returns
/// `None` only when no override is set and no home directory can be
/// determined.
pub fn settings_dir() -> Option<PathBuf> {
    settings_dir_from(env_override(), home_dir())
}

/// Pure core of [`settings_dir`] — resolve from an explicit override and
/// home, without touching the environment. The override wins verbatim
/// (an empty override is ignored); otherwise `<home>/.vibe`.
///
/// ```
/// use vibe_core::settings::settings_dir_from;
/// use std::path::PathBuf;
///
/// // The override is taken verbatim, even over a present home.
/// let d = settings_dir_from(Some(PathBuf::from("/opt/vibe")), Some(PathBuf::from("/home/u")));
/// assert_eq!(d, Some(PathBuf::from("/opt/vibe")));
///
/// // Without an override, it is `<home>/.vibe`.
/// let d = settings_dir_from(None, Some(PathBuf::from("/home/u"))).unwrap();
/// assert!(d.ends_with(".vibe"));
///
/// // No override and no home is unresolvable.
/// assert_eq!(settings_dir_from(None, None), None);
/// ```
pub fn settings_dir_from(override_dir: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(o) = override_dir.filter(|p| !p.as_os_str().is_empty()) {
        return Some(o);
    }
    Some(home?.join(CANONICAL_DIR))
}

/// The pre-consolidation settings directory `<home>/.vibevm`.
///
/// A backward-compatible **read** fallback only — publish-token and
/// user-config readers consult it when the canonical `~/.vibe` file is
/// absent, so a not-yet-migrated machine keeps working. Nothing is
/// written here, and `$VIBE_SETTINGS` deliberately does *not* affect it:
/// the legacy location was always `~/.vibevm`.
pub fn legacy_settings_dir() -> Option<PathBuf> {
    Some(home_dir()?.join(LEGACY_DIR))
}

/// Machine-global registry config: `<settings-dir>/registry.toml`.
pub fn registry_config_path() -> Option<PathBuf> {
    Some(settings_dir()?.join("registry.toml"))
}

/// User-level config: `<settings-dir>/config.toml` (the consolidated home
/// of the former XDG `~/.config/vibe/config.toml`).
pub fn user_config_path() -> Option<PathBuf> {
    Some(settings_dir()?.join("config.toml"))
}

/// PROP-040 L1 preferences: `<settings-dir>/settings.toml`.
pub fn settings_toml_path() -> Option<PathBuf> {
    Some(settings_dir()?.join("settings.toml"))
}

/// Registry-clone cache root: `<settings-dir>/registries`.
///
/// Callers still let `VIBE_REGISTRY_CACHE` override this specific path;
/// the chokepoint only fixes the default.
pub fn registries_cache_dir() -> Option<PathBuf> {
    Some(settings_dir()?.join("registries"))
}

/// Search-index cache root: `<settings-dir>/search-cache`.
pub fn search_cache_dir() -> Option<PathBuf> {
    Some(settings_dir()?.join("search-cache"))
}

/// vibeterm/aiui discovery dir: `<settings-dir>/aiui`.
pub fn aiui_dir() -> Option<PathBuf> {
    Some(settings_dir()?.join("aiui"))
}

/// Read the `$VIBE_SETTINGS` override, treating an empty value as unset.
fn env_override() -> Option<PathBuf> {
    std::env::var_os(SETTINGS_DIR_ENV)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Best-effort home directory: `HOME`, then `USERPROFILE` on Windows (for
/// native shells that set only the latter). One resolver for the whole
/// workspace so every settings path agrees; avoids the `dirs` crate for a
/// single lookup, matching [`crate::user_config`].
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
    use specmark::verifies;

    #[test]
    #[verifies("spec://vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order")]
    fn override_wins_verbatim_over_home() {
        // `$VIBE_SETTINGS` is an escape hatch: it must land exactly where
        // pointed, ignoring the home-derived default entirely.
        let d = settings_dir_from(
            Some(PathBuf::from("/opt/x")),
            Some(PathBuf::from("/home/u")),
        );
        assert_eq!(d, Some(PathBuf::from("/opt/x")));
    }

    #[test]
    fn empty_override_falls_back_to_home() {
        // An empty env value is "unset", not "the empty path".
        let d = settings_dir_from(Some(PathBuf::new()), Some(PathBuf::from("/home/u"))).unwrap();
        assert!(d.ends_with(".vibe"));
        assert!(d.starts_with("/home/u"));
    }

    #[test]
    fn unresolvable_without_override_or_home() {
        assert_eq!(settings_dir_from(None, None), None);
    }

    #[test]
    fn legacy_dir_is_dot_vibevm_and_never_the_canonical() {
        // The migration fallback points at the historical `~/.vibevm`,
        // distinct from the canonical `.vibe`.
        let legacy = legacy_settings_dir().expect("home dir present in test env");
        assert!(legacy.ends_with(".vibevm"));
    }

    #[test]
    fn accessors_hang_the_right_filenames_off_the_dir() {
        // The suffix is asserted (not the `.vibe` segment) so the test is
        // robust to a dev running with `$VIBE_SETTINGS` set.
        assert!(registry_config_path().unwrap().ends_with("registry.toml"));
        assert!(user_config_path().unwrap().ends_with("config.toml"));
        assert!(settings_toml_path().unwrap().ends_with("settings.toml"));
        assert!(registries_cache_dir().unwrap().ends_with("registries"));
        assert!(search_cache_dir().unwrap().ends_with("search-cache"));
        assert!(aiui_dir().unwrap().ends_with("aiui"));
    }
}
