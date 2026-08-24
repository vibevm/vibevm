//! `conform.toml` — the project's conform policy, lifted out of
//! compile-time constants so the checker runs on *any* project, not
//! only the one it was built in (PROP-024 §2.2; ENGINE-CONFORM §2).
//!
//! The driver (or the `conform` binary) loads this once at startup and
//! constructs the scan + the rule set from it; nothing about the policy
//! is hardcoded in the engine.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#facts");

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// The conform policy for one project: which source roots to scan, which
/// crates the gates apply to, the sanctioned env-reading files, and the
/// budgets. Loaded from a `conform.toml` at the project root; every
/// field defaults, so a minimal file works and an absent file yields a
/// usable default.
///
/// ```
/// let cfg: core_ai_native_conform::Config = toml::from_str(
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
    /// The TypeScript half of the policy (`[typescript]`), consumed by
    /// `typescript-ai-native-conform` (the `ts-tsc` frontend). Absent for
    /// Rust-only projects; the Rust rules never read it.
    pub typescript: TsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            roots: vec!["crates/*".into()],
            exclude_substrings: vec!["/generated/".into()],
            gated_crates: Vec::new(),
            gated_pub_doctest: Vec::new(),
            audit_crates: Vec::new(),
            env_roots: Vec::new(),
            registry_file: None,
            registry_gated_crate: None,
            max_file_lines: 600,
            exempt: Vec::new(),
            typescript: TsConfig::default(),
        }
    }
}

/// The `[typescript]` policy table (GUIDE-AI-NATIVE-TYPESCRIPT §3, §8).
///
/// ```
/// let cfg: core_ai_native_conform::Config = toml::from_str(
///     "[typescript]\nroots = [\"src\"]\ncells_dir = \"src/cells\"\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.typescript.roots, vec!["src".to_string()]);
/// assert_eq!(cfg.typescript.seam, "index");
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TsConfig {
    /// TypeScript source roots (flat walk; `<dir>/*` scans subdirs).
    pub roots: Vec<String>,
    /// A `.ts` file whose repo-relative path contains any of these
    /// substrings is skipped (fixtures, generated output).
    pub exclude_substrings: Vec<String>,
    /// The directory whose immediate subdirectories are cells
    /// (`ts-cell-isolation`); `None` disables the isolation rule.
    pub cells_dir: Option<String>,
    /// The seam module name a sibling cell may be imported through.
    pub seam: String,
    /// Floor steps this project explicitly disables, each with a
    /// recorded reason. The floor PRINTS every disablement every run —
    /// the "a defaulted nothing-gated run announces itself" posture
    /// extended to step disablement; absent tooling without an entry
    /// here is a hard step failure, never a silent skip.
    pub floor_disable: Vec<FloorDisable>,
}

/// One disabled floor step + why (`[[typescript.floor_disable]]`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorDisable {
    /// The step name (`prettier` / `tsc` / `tests` / `eslint` /
    /// `conform` / `specmap` / `test-gate`).
    pub step: String,
    /// Why it is off — never empty.
    pub reason: String,
}

impl Default for TsConfig {
    fn default() -> Self {
        TsConfig {
            roots: vec!["src".into()],
            exclude_substrings: vec!["/fixtures/".into()],
            cells_dir: None,
            seam: "index".into(),
            floor_disable: Vec::new(),
        }
    }
}

/// A crate held outside the gates, with the reason it has not flipped —
/// the checklist the remaining conform-adoption phases drain.
///
/// ```
/// let e = core_ai_native_conform::ExemptEntry {
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

/// Where a [`Config`] came from — a real `conform.toml`, or the built-in
/// default because none exists. The drivers print this so a defaulted
/// (nothing-gated) run can never masquerade as a configured green.
///
/// ```
/// assert_ne!(core_ai_native_conform::ConfigOrigin::Loaded, core_ai_native_conform::ConfigOrigin::Defaulted);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOrigin {
    /// Parsed from the project's `conform.toml`.
    Loaded,
    /// No `conform.toml` at the root — the topology-detected default
    /// (nothing gated, everything advisory) is in force.
    Defaulted,
}

impl Config {
    /// Parse a `conform.toml` from `path`.
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading conform config {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing conform config {}", path.display()))
    }

    /// Load the project's `conform.toml`, or fall back to a usable default
    /// when none exists (the doc-promised behaviour): scan roots detected
    /// from the tree's topology — `crates/*` for a workspace layout, `.`
    /// for a single-crate one — with nothing gated. The origin tells the
    /// caller which case it got.
    pub fn load_or_default(root: &Path) -> Result<(Config, ConfigOrigin)> {
        let path = root.join("conform.toml");
        if path.exists() {
            return Ok((Config::load(&path)?, ConfigOrigin::Loaded));
        }
        let mut cfg = Config::default();
        if !root.join("crates").is_dir() {
            cfg.roots = vec![".".into()];
        }
        Ok((cfg, ConfigOrigin::Defaulted))
    }

    /// The gated-or-exempt tree invariant: every crate on disk under this
    /// policy's roots is classified exactly once — gated or
    /// exempt-with-a-reason, never both and never neither — and every
    /// listed name matches a real crate directory. A silent exemption
    /// reads as a bug, a phantom entry as a typo; this turns the
    /// exemption *table* into an enforced *invariant* for every consumer
    /// of the engine (it began life as a dev-repo-only test).
    pub fn validate_against_tree(&self, root: &Path) -> Result<()> {
        use std::collections::BTreeSet;

        let gated: BTreeSet<&str> = self.gated_crates.iter().map(|s| s.as_str()).collect();
        let exempt: BTreeSet<&str> = self.exempt.iter().map(|e| e.crate_name.as_str()).collect();
        if gated.len() != self.gated_crates.len() {
            anyhow::bail!("conform.toml: `gated_crates` carries a duplicate crate name");
        }
        if exempt.len() != self.exempt.len() {
            anyhow::bail!("conform.toml: `[[exempt]]` carries a duplicate crate name");
        }
        let both: Vec<&str> = gated.intersection(&exempt).copied().collect();
        if !both.is_empty() {
            anyhow::bail!("conform.toml: crates both gated and exempt: {both:?}");
        }
        for e in &self.exempt {
            if e.reason.trim().is_empty() {
                anyhow::bail!(
                    "conform.toml: `{}` is exempt without a recorded reason — the one \
                     thing the exemption table exists to forbid",
                    e.crate_name
                );
            }
        }

        // Expand the roots the way the scanner does: a `<dir>/*` glob names
        // each crate-shaped subdir; a literal root names itself — resolved
        // against the project root through the scanner's own derivation
        // (`crate_dir_name`), so `.` names the project directory (the bare
        // single-crate layout) instead of nothing.
        let mut on_disk: BTreeSet<String> = BTreeSet::new();
        let mut literals: BTreeSet<String> = BTreeSet::new();
        for entry in &self.roots {
            if let Some(parent) = entry.strip_suffix("/*") {
                if let Ok(rd) = std::fs::read_dir(root.join(parent)) {
                    for e in rd.filter_map(std::result::Result::ok) {
                        if e.path().is_dir() && e.path().join("Cargo.toml").exists() {
                            on_disk.insert(e.file_name().to_string_lossy().into_owned());
                        }
                    }
                }
            } else if let Some(name) = crate::store::crate_dir_name(&root.join(entry)) {
                literals.insert(name);
            }
        }
        for c in &on_disk {
            if !gated.contains(c.as_str()) && !exempt.contains(c.as_str()) {
                anyhow::bail!(
                    "conform.toml: crate `{c}` is neither gated nor exempt — classify it"
                );
            }
        }
        for c in gated.union(&exempt) {
            if !on_disk.contains(*c) && !literals.contains(*c) {
                anyhow::bail!(
                    "conform.toml: `{c}` is listed but no crate directory matches it — typo?"
                );
            }
        }
        Ok(())
    }

    /// Gated crates the scan attributed NO sources to — each names a gate
    /// that would pass by vacuity (nothing scanned means nothing findable),
    /// the silent failure mode of a mis-shaped `roots` list. The drivers
    /// print every entry as a warning on check and freeze, the same
    /// announce-yourself posture as [`ConfigOrigin`]; an empty return means
    /// every gated crate contributed at least one scanned file.
    ///
    /// ```
    /// use core_ai_native_conform::{Config, SourceFacts};
    ///
    /// let cfg: Config = toml::from_str("gated_crates = [\"app\"]").unwrap();
    /// let nothing: Vec<SourceFacts> = Vec::new();
    /// assert_eq!(cfg.vacuously_gated(&nothing), vec!["app".to_string()]);
    ///
    /// let scanned = vec![SourceFacts {
    ///     file: "crates/app/src/lib.rs".into(),
    ///     crate_name: "app".into(),
    ///     facts: vec![],
    /// }];
    /// assert!(cfg.vacuously_gated(&scanned).is_empty());
    /// ```
    pub fn vacuously_gated(&self, facts: &[crate::facts::SourceFacts]) -> Vec<String> {
        use std::collections::BTreeSet;
        let scanned: BTreeSet<&str> = facts.iter().map(|f| f.crate_name.as_str()).collect();
        self.gated_crates
            .iter()
            .filter(|c| !scanned.contains(c.as_str()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crate_dir(root: &Path, name: &str) {
        let d = root.join("crates").join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("Cargo.toml"), "[package]\n").unwrap();
    }

    #[test]
    fn load_or_default_detects_topology() {
        let tmp = tempfile::tempdir().unwrap();
        // Single-crate layout → scan the root itself.
        let (cfg, origin) = Config::load_or_default(tmp.path()).unwrap();
        assert_eq!(origin, ConfigOrigin::Defaulted);
        assert_eq!(cfg.roots, ["."]);
        // Workspace layout → scan crates/*.
        std::fs::create_dir_all(tmp.path().join("crates")).unwrap();
        let (cfg, _) = Config::load_or_default(tmp.path()).unwrap();
        assert_eq!(cfg.roots, ["crates/*"]);
        // A real file wins and reports Loaded.
        std::fs::write(tmp.path().join("conform.toml"), "max_file_lines = 500\n").unwrap();
        let (cfg, origin) = Config::load_or_default(tmp.path()).unwrap();
        assert_eq!(origin, ConfigOrigin::Loaded);
        assert_eq!(cfg.max_file_lines, 500);
    }

    #[test]
    fn tree_invariant_catches_each_violation_class() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        crate_dir(root, "app");
        crate_dir(root, "helper");

        // Unclassified on-disk crate.
        let cfg: Config = toml::from_str("gated_crates = [\"app\"]\n").unwrap();
        let err = cfg.validate_against_tree(root).unwrap_err().to_string();
        assert!(
            err.contains("`helper` is neither gated nor exempt"),
            "{err}"
        );

        // Phantom listed crate.
        let cfg: Config =
            toml::from_str("gated_crates = [\"app\", \"helper\", \"ghost\"]\n").unwrap();
        let err = cfg.validate_against_tree(root).unwrap_err().to_string();
        assert!(err.contains("`ghost` is listed"), "{err}");

        // Both gated and exempt.
        let cfg: Config = toml::from_str(
            "gated_crates = [\"app\", \"helper\"]\n\
             [[exempt]]\ncrate = \"app\"\nreason = \"x\"\n",
        )
        .unwrap();
        let err = cfg.validate_against_tree(root).unwrap_err().to_string();
        assert!(err.contains("both gated and exempt"), "{err}");

        // Empty reason.
        let cfg: Config = toml::from_str(
            "gated_crates = [\"app\"]\n[[exempt]]\ncrate = \"helper\"\nreason = \"  \"\n",
        )
        .unwrap();
        let err = cfg.validate_against_tree(root).unwrap_err().to_string();
        assert!(err.contains("without a recorded reason"), "{err}");

        // A literal root (tooling crate outside crates/) satisfies the
        // listed-name check without a crates/ directory.
        std::fs::create_dir_all(root.join("tooling")).unwrap();
        let cfg: Config = toml::from_str(
            "roots = [\"crates/*\", \"tooling\"]\n\
             gated_crates = [\"app\", \"helper\"]\n\
             [[exempt]]\ncrate = \"tooling\"\nreason = \"dev tooling\"\n",
        )
        .unwrap();
        cfg.validate_against_tree(root).unwrap();
    }

    /// A bare single-crate layout (`roots = ["."]`) gates or exempts the
    /// crate under the name the scanner attributes its files to — the
    /// project directory's basename. `Path::new(".").file_name()` is
    /// `None`, so deriving the name from the raw entry (the pre-fix
    /// behaviour) left the crate unnameable: listing it as gated OR
    /// exempt failed the invariant as a phantom entry.
    #[test]
    fn dot_root_names_the_project_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Cargo.toml"), "[package]\n").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let name = root.file_name().unwrap().to_string_lossy().into_owned();

        let gated: Config =
            toml::from_str(&format!("roots = [\".\"]\ngated_crates = [\"{name}\"]\n")).unwrap();
        gated.validate_against_tree(root).unwrap();

        let exempt: Config = toml::from_str(&format!(
            "roots = [\".\"]\n[[exempt]]\ncrate = \"{name}\"\nreason = \"pre-adoption\"\n"
        ))
        .unwrap();
        exempt.validate_against_tree(root).unwrap();

        // The phantom check survives the fix: a name matching neither the
        // directory nor any glob-expanded crate still refuses.
        let ghost: Config =
            toml::from_str("roots = [\".\"]\ngated_crates = [\"ghost\"]\n").unwrap();
        let err = ghost.validate_against_tree(root).unwrap_err().to_string();
        assert!(err.contains("`ghost` is listed"), "{err}");
    }
}
