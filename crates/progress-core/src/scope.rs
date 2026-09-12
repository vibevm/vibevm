//! Observed-tree scoping: `facts.toml` include globs, the always-on
//! default excludes — by directory and by file name — and the project's own
//! enumerated `exclude` globs (PROP-043 §6, the facts home).
//!
//! Beside them, on a different axis and never touching the enumeration:
//! `[judging] exempt` ([`JudgingExemption`]), which says of an observed
//! file that it is never judged (PROP-057 `##OBS-NOT-JUDGED`).
//!
//! This cell is the configuration: the four default tables and the
//! `facts.toml` shape they are overridden from. The two things that read
//! that configuration have their own cells beside it — [`enumerate`]
//! walks the tree, [`judging`] answers the exemption question — and
//! their public names are re-exported here, so `scope::observed_files`
//! and `scope::JudgingExemption` are the paths they have always been.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#config");

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

mod enumerate;
mod judging;

pub use enumerate::{ExcludeReport, observed_files, observed_files_reported, rel_str};
pub use judging::JudgingExemption;

#[cfg(test)]
mod tests;

/// The always-applied exclusions — even under explicit includes. Matched
/// against **path components**, so each entry names a directory.
///
/// Layout note (PROP-052): `vibedeps` is matched as a *component name*,
/// and the component keeps its name under the new layout too (only its
/// parent changes: root `vibedeps/` → `vibevm/vibedeps/`), so this entry
/// excludes the dependency slots under either shape. progress-core is
/// standalone by law (PROP-043 §2 — no vibe-* dependencies), so it cannot
/// route through `vibe_core::layout` (`crates/vibe-core/src/layout.rs`,
/// the one home of the root names, PROP-052 L2); this name is the
/// sanctioned duplication and is verified against that module by R6's
/// grep panel.
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

/// The always-applied exclusions matched against the **file name** alone —
/// the same footing as [`DEFAULT_EXCLUDES`] (applied even under an explicit
/// include), but naming a file wherever it sits rather than a directory.
///
/// A licence is verbatim third-party text: the observing project neither
/// authored it nor is the source of truth for it, and it is replaced
/// wholesale from upstream — so a marker written into one claims a contract
/// over words the project does not own, which is exactly why `refs` is a
/// `DEFAULT_EXCLUDES` entry. The rule is project-neutral (PROP-043 §5):
/// every project has licence files, and in no project are they its
/// contracts.
pub const DEFAULT_EXCLUDE_FILES: [&str; 1] = ["LICENSE.md"];

/// The default include globs — both spec serialisations, side by side
/// (PROP-045 ##LOADER-LAW): a project may hold a document as Markdown or
/// as dialect XML, and the corpus observes whichever form each document
/// took. One logical document in BOTH forms is not a scope question — the
/// consumer's pair-collision check rejects it loudly; the globs themselves
/// stay blind so an explicit project config inherits the same pairing.
///
/// Layout note (PROP-052): these globs spell the ROOT names of the live
/// directory layout (`vibevm/vibespecs/`, `vibevm/vibepacks/` since the
/// R4 flip). progress-core is standalone by law (PROP-043 §2 — no vibe-*
/// dependencies), so it cannot route through `vibe_core::layout`
/// (`crates/vibe-core/src/layout.rs`, the one home of the root names,
/// PROP-052 L2); these four entries are the sanctioned duplication,
/// flipped BY HAND with RELAYOUT-PLAN R4.
pub const DEFAULT_INCLUDES: [&str; 4] = [
    "vibevm/vibespecs/**/*.md",
    "vibevm/vibespecs/**/*.xml",
    "vibevm/vibepacks/**/*.md",
    "vibevm/vibepacks/**/*.xml",
];

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

/// The `[judging]` table — which observed documents are never *judged*
/// (PROP-057 `##OBS-NOT-JUDGED`). Absent in most projects, and absent
/// means "everything observed is also judged".
#[derive(Debug, Clone, Default, Deserialize)]
pub struct JudgingSection {
    /// Globs matched against the `/`-separated repo-relative path of an
    /// observed file: the file stays in the corpus and keeps its statuses,
    /// and it enters no judging debt.
    ///
    /// This is the one axis `exclude` cannot express. A documentation
    /// package is authored, current and worth observing — `vibe facts`
    /// must see its pages — but its genre is non-normative, so a verdict
    /// on one of its paragraphs asserts nothing. Excluding it would hide
    /// the pages; judging it would mint addresses on prose that nothing
    /// resolves against. `exempt` is the third answer: observed, never
    /// judged.
    ///
    /// Absent ⇒ empty ⇒ the behaviour of a config that never had the key.
    #[serde(default)]
    pub exempt: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScopeConfig {
    #[serde(default)]
    pub schema: Option<u32>,
    #[serde(default)]
    pub include: Vec<String>,
    /// Project-specific exclusions: globs matched against the
    /// `/`-separated repo-relative path, applied **after** the include
    /// globs and after the two default-exclusion rules.
    ///
    /// §4 is include-style by design so that nothing is observed by
    /// accident, and an *enumerated* exclude list serves that purpose
    /// exactly as well as an enumerated include list — both are explicit
    /// and both are reviewable. What it must not become is a wildcard
    /// escape hatch, which is why a pattern that matches nothing is
    /// reported rather than tolerated ([`ExcludeReport::stale`]) and the
    /// files it removes are counted ([`ExcludeReport::dropped`]).
    ///
    /// Absent ⇒ empty ⇒ the behaviour of a config that never had the key.
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub judging: JudgingSection,
    #[serde(default)]
    pub progress: ProgressSection,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        ScopeConfig {
            schema: Some(1),
            include: DEFAULT_INCLUDES.iter().map(|s| s.to_string()).collect(),
            exclude: Vec::new(),
            judging: JudgingSection::default(),
            progress: ProgressSection::default(),
        }
    }
}

impl ScopeConfig {
    /// The `[judging] exempt` globs of this config, compiled once.
    pub fn judging_exemption(&self) -> Result<JudgingExemption> {
        JudgingExemption::compile(&self.judging.exempt)
    }
}

/// Load `facts.toml` at `root`, falling back to defaults when absent.
/// `progress.toml` is read as a silent legacy spelling for the transition
/// (PROP-043 ##CONFIG-FILE — the observed tree is a facts-layer concern);
/// when both exist the facts spelling wins.
pub fn load_config(root: &Path) -> Result<ScopeConfig> {
    let mut path = root.join("facts.toml");
    if !path.exists() {
        path = root.join("progress.toml");
    }
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
