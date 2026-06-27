//! `conform.toml` — the project's conform policy, lifted out of
//! compile-time constants so the checker runs on *any* project, not
//! only the one it was built in (PROP-024 §2.2; ENGINE-CONFORM §2).
//!
//! The driver (or the `conform` binary) loads this once at startup and
//! constructs the scan + the rule set from it; nothing about the policy
//! is hardcoded in the engine.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#facts");

/// The conform policy for one project: which source roots to scan, which
/// crates the gates apply to, the sanctioned env-reading files, and the
/// budgets. Loaded from a `conform.toml` at the project root; every
/// field defaults, so a minimal file works and an absent file yields a
/// usable default.
///
/// ```
/// let cfg: conform_core::Config = toml::from_str(
///     "roots = [\"crates/*\"]\n\
///      gated_crates = [\"app\"]\n\
///      registry_file = \"crates/app/src/registry.rs\"\n\
///      registry_gated_crate = \"app\"\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.max_file_lines, 600);
/// assert_eq!(cfg.gated_crates, vec!["app".to_string()]);
/// assert_eq!(cfg.registry_gated_crate.as_deref(), Some("app"));
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Source roots to scan. A `<dir>/*` entry scans each subdirectory
    /// of `<dir>` as one crate; any other entry is a literal crate dir.
    pub roots: Vec<String>,
    /// A source file whose repo-relative path contains any of these
    /// substrings is skipped (generated code, vendored trees).
    pub exclude_substrings: Vec<String>,
    /// Crates the Class-F/G gates apply to.
    pub gated_crates: Vec<String>,
    /// Crates whose whole public *type* surface is gated for doctests
    /// (the wider `pub-doctest` lens).
    pub gated_pub_doctest: Vec<String>,
    /// Designated audit crates — exempt wholesale from the unsafe and
    /// ambient-env gates (they own the unsafety behind a safe API).
    pub audit_crates: Vec<String>,
    /// Repo-relative files where reading the ambient environment is
    /// sanctioned (the composition / config-resolution roots).
    pub env_roots: Vec<String>,
    /// The one legal cell-construction site (R-001 flag-sites). `None`
    /// disables R-001 — a project without the cell idiom omits it.
    pub registry_file: Option<String>,
    /// The crate R-001 gates; meaningful only with `registry_file`.
    pub registry_gated_crate: Option<String>,
    /// The per-file line budget (`file-length`).
    pub max_file_lines: u32,
    /// Crates deliberately *outside* `gated_crates`, each paired with the
    /// reason it has not (yet) flipped — a silent exemption reads as a
    /// bug, a recorded one as a decision.
    pub exempt: Vec<ExemptEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            roots: vec!["crates/*".into(), "xtask".into()],
            exclude_substrings: vec!["/generated/".into()],
            gated_crates: Vec::new(),
            gated_pub_doctest: Vec::new(),
            audit_crates: Vec::new(),
            env_roots: Vec::new(),
            registry_file: None,
            registry_gated_crate: None,
            max_file_lines: 600,
            exempt: Vec::new(),
        }
    }
}

/// A crate held outside the gates, with the reason it has not flipped —
/// the checklist the remaining conform-adoption phases drain.
///
/// ```
/// let e = conform_core::ExemptEntry {
///     crate_name: "vibe-graph".into(),
///     reason: "M0 stub, no code yet".into(),
/// };
/// assert_eq!(e.crate_name, "vibe-graph");
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct ExemptEntry {
    /// The crate name (TOML key `crate`).
    #[serde(rename = "crate")]
    pub crate_name: String,
    /// Why it is exempt — never empty.
    pub reason: String,
}

impl Config {
    /// Parse a `conform.toml` from `path`.
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading conform config {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing conform config {}", path.display()))
    }
}
