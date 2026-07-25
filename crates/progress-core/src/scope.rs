//! Observed-tree scoping: `progress.toml` include globs plus the always-on
//! default excludes (PROP-043 §4).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#config");

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The always-applied exclusions — even under explicit includes.
pub const DEFAULT_EXCLUDES: [&str; 8] = [
    "vibedeps",
    ".vibe",
    "refs",
    "fixtures",
    "campaigns",
    "target",
    "node_modules",
    "vendor",
];

pub const DEFAULT_INCLUDES: [&str; 2] = ["spec/**/*.md", "packages/**/*.md"];

/// The `[progress]` table — knobs that are not about which files are
/// observed. Absent in most projects, and absent means "the defaults".
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProgressSection {
    /// Explicit home for the parse-payload sidecar (DRIFT-016 §4.2):
    /// absolute, or relative to the project root. Absent ⇒ the per-user
    /// default under the settings home. This is the escape hatch for a
    /// project that wants the store somewhere it can see, and for a test
    /// that must not write a real per-user directory.
    #[serde(default)]
    pub cache_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScopeConfig {
    #[serde(default)]
    pub schema: Option<u32>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub progress: ProgressSection,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        ScopeConfig {
            schema: Some(1),
            include: DEFAULT_INCLUDES.iter().map(|s| s.to_string()).collect(),
            progress: ProgressSection::default(),
        }
    }
}

/// Load `progress.toml` at `root`, falling back to defaults when absent.
pub fn load_config(root: &Path) -> Result<ScopeConfig> {
    let path = root.join("progress.toml");
    if !path.exists() {
        return Ok(ScopeConfig::default());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut cfg: ScopeConfig =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    if cfg.include.is_empty() {
        cfg.include = DEFAULT_INCLUDES.iter().map(|s| s.to_string()).collect();
    }
    Ok(cfg)
}

/// Enumerate the observed files under `root`, sorted, `/`-separated
/// repo-relative paths.
pub fn observed_files(root: &Path, cfg: &ScopeConfig) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for pat in &cfg.include {
        let full = root.join(pat).to_string_lossy().replace('\\', "/");
        for entry in glob::glob(&full).with_context(|| format!("bad glob `{pat}`"))? {
            let path = entry?;
            if !path.is_file() {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            if is_excluded(&rel) {
                continue;
            }
            out.push(rel);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn is_excluded(rel: &Path) -> bool {
    rel.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        DEFAULT_EXCLUDES.iter().any(|e| s == *e)
    })
}

/// Normalize a relative path to the `/`-separated report form.
pub fn rel_str(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_excludes_hold_under_explicit_includes() {
        assert!(is_excluded(Path::new("vibedeps/x/spec/a.md")));
        assert!(is_excluded(Path::new("packages/g/n/vibedeps/a.md")));
        assert!(is_excluded(Path::new("campaigns/p/run/RESUME.md")));
        assert!(is_excluded(Path::new("a/vendor/b.md")));
        assert!(!is_excluded(Path::new("spec/modules/x/PROP-001.md")));
    }
}
