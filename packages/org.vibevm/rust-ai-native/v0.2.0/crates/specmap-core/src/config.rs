//! `specmap.toml` — the project's traceability policy, lifted out of
//! hardcoded paths so the specmap engine runs on *any* project, not only
//! the one it was built in (PROP-014; the same productisation conform made
//! in its Ф3, mirrored here in the Traceability Relocation Plan Phase 2).
//!
//! The driver (or the `specmap-rust` binary) loads this once at startup and
//! constructs the scan + the orphan ratchet from it; nothing about which
//! roots to walk or which crates are exempt is hardcoded in the engine.
//! An absent `specmap.toml` yields the default policy and turns the orphan
//! ratchet off — the pre-config behaviour.

specmark::scope!("spec://vibevm/discipline/PROP-014#index");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// The specmap policy for one project: which code roots carry taggable
/// source, which markdown trees hold spec units, and which crates are exempt
/// from the orphan ratchet. Loaded from a `specmap.toml` at the project root;
/// every field defaults, so a minimal file works and an absent file yields a
/// usable default.
///
/// ```
/// let cfg: specmap_core::config::Config = toml::from_str(
///     "scan_roots = [\"crates/*\"]\nexempt = [\"vibe-wire\"]\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.scan_roots, vec!["crates/*".to_string()]);
/// // Unset fields fall back to the defaults.
/// assert_eq!(cfg.spec_roots, vec!["spec".to_string()]);
/// assert_eq!(cfg.root_spec_docs, vec!["VIBEVM-SPEC.md".to_string()]);
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Code roots to scan for `#[spec]`/`#[verifies]`/`scope!` tags. A
    /// `<dir>/*` entry scans each subdirectory of `<dir>` as one crate; any
    /// other entry is a literal crate dir.
    pub scan_roots: Vec<String>,
    /// Markdown trees walked for anchored spec units (`<root>/**/*.md`).
    pub spec_roots: Vec<String>,
    /// Individual root-level spec documents scanned in addition to
    /// [`spec_roots`](Config::spec_roots) — the owner-frozen implementation
    /// spec lives at the repo root, outside any `spec/` tree (DBT-0019).
    pub root_spec_docs: Vec<String>,
    /// Crates exempt from the orphan ratchet (PLAYBOOK `#phase2`). A crate
    /// **not** listed is gated: its `pub` items must carry an own edge or a
    /// `scope!`-inherited module edge. Empty = every crate gated.
    pub exempt: Vec<String>,
    /// Orphans allowed to stand, each carrying its debt id (the "dispositioned
    /// into debt.json" arm of the Phase 2 acceptance).
    pub dispositioned: Vec<Disposition>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scan_roots: vec!["crates/*".into(), "xtask".into()],
            spec_roots: vec!["spec".into()],
            root_spec_docs: vec!["VIBEVM-SPEC.md".into()],
            exempt: Vec::new(),
            dispositioned: Vec::new(),
        }
    }
}

/// One orphan held outside the gate, with the debt id that records why it is
/// allowed to stand.
///
/// ```
/// let d = specmap_core::config::Disposition {
///     symbol: "vibe_cli::commands::mcp::serve".into(),
///     debt: "DBT-0020".into(),
/// };
/// assert_eq!(d.debt, "DBT-0020");
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct Disposition {
    /// The module-qualified symbol the disposition covers.
    pub symbol: String,
    /// The debt id it is filed under.
    pub debt: String,
}

impl Config {
    /// Repo-relative location of the policy file.
    pub const REL_PATH: &'static str = "specmap.toml";

    /// Load `specmap.toml` from `root`. `Ok(None)` when the file is absent —
    /// the caller defaults the scan and turns the ratchet off.
    pub fn load(root: &Path) -> Result<Option<Config>> {
        let path = root.join(Self::REL_PATH);
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading specmap config {}", path.display()))?;
        let cfg = toml::from_str(&text)
            .with_context(|| format!("parsing specmap config {}", path.display()))?;
        Ok(Some(cfg))
    }

    /// Resolve [`scan_roots`](Config::scan_roots) to concrete crate
    /// directories under `root`, deterministically (sorted). A `<dir>/*`
    /// entry expands to each existing subdirectory of `<dir>`; any other
    /// entry is taken literally. The sort makes the downstream index
    /// order — and therefore `specmap.json` — stable across platforms.
    pub fn scan_dirs(&self, root: &Path) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        for entry in &self.scan_roots {
            if let Some(parent) = entry.strip_suffix("/*") {
                if let Ok(rd) = std::fs::read_dir(root.join(parent)) {
                    for e in rd.filter_map(std::result::Result::ok) {
                        if e.path().is_dir() {
                            dirs.push(e.path());
                        }
                    }
                }
            } else {
                dirs.push(root.join(entry));
            }
        }
        dirs.sort();
        dirs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_file_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Config::load(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn defaults_match_the_historical_hardcode() {
        let cfg = Config::default();
        assert_eq!(cfg.scan_roots, ["crates/*", "xtask"]);
        assert_eq!(cfg.spec_roots, ["spec"]);
        assert_eq!(cfg.root_spec_docs, ["VIBEVM-SPEC.md"]);
    }

    #[test]
    fn glob_expands_subdirs_and_literals_pass_through() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("crates/a")).unwrap();
        std::fs::create_dir_all(root.join("crates/b")).unwrap();
        std::fs::create_dir_all(root.join("xtask")).unwrap();
        let cfg = Config::default();
        let dirs = cfg.scan_dirs(root);
        // crates/a, crates/b (glob) + xtask (literal), sorted.
        assert_eq!(
            dirs,
            vec![
                root.join("crates/a"),
                root.join("crates/b"),
                root.join("xtask"),
            ]
        );
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = toml::from_str::<Config>("bogus = 1\n");
        assert!(err.is_err(), "deny_unknown_fields must reject typos");
    }
}
