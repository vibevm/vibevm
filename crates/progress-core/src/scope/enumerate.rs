//! The observed-file enumeration (PROP-043 §6).
//!
//! One fixed order, written once so no caller can reorder it: expand the
//! include globs, drop [`super::DEFAULT_EXCLUDES`] by path component,
//! drop [`super::DEFAULT_EXCLUDE_FILES`] by file name, then drop the
//! project's own `exclude` globs — the only one of the four a project
//! chose, and so the only one that is reported.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#config");

use super::{DEFAULT_EXCLUDE_FILES, DEFAULT_EXCLUDES, ScopeConfig};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// What the config-side `exclude` globs did to one enumeration.
///
/// Only the config side: [`DEFAULT_EXCLUDES`] and [`DEFAULT_EXCLUDE_FILES`]
/// are structural — they hold in every project and under every include, so
/// there is no per-project decision to report about them. `exclude` is a
/// choice this project made, and a file leaving the corpus by a choice must
/// never be invisible.
#[derive(Debug, Clone, Default)]
pub struct ExcludeReport {
    /// Distinct observed files the config `exclude` globs removed.
    pub dropped: usize,
    /// The `exclude` patterns that matched no observed file — a stale
    /// exclusion protects nothing, and a scope rots by accumulating them.
    pub stale: Vec<String>,
}

/// Enumerate the observed files under `root`, sorted, `/`-separated
/// repo-relative paths.
pub fn observed_files(root: &Path, cfg: &ScopeConfig) -> Result<Vec<PathBuf>> {
    Ok(observed_files_reported(root, cfg)?.0)
}

/// [`observed_files`], plus what the config `exclude` cost — for callers
/// that report the corpus rather than only consume it (PROP-043 §4).
///
/// The order is fixed: expand the include globs → drop [`DEFAULT_EXCLUDES`]
/// by path component → drop [`DEFAULT_EXCLUDE_FILES`] by file name → drop
/// the config `exclude` by glob.
pub fn observed_files_reported(
    root: &Path,
    cfg: &ScopeConfig,
) -> Result<(Vec<PathBuf>, ExcludeReport)> {
    let mut out = Vec::new();
    for pat in &cfg.include {
        let full = root.join(pat).to_string_lossy().replace('\\', "/");
        for entry in glob::glob(&full).with_context(|| format!("bad glob `{pat}`"))? {
            let path = entry?;
            if !path.is_file() {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            if is_excluded(&rel) || is_excluded_file(&rel) {
                continue;
            }
            out.push(rel);
        }
    }
    out.sort();
    out.dedup();
    let report = apply_config_excludes(&mut out, &cfg.exclude)?;
    Ok((out, report))
}

fn is_excluded(rel: &Path) -> bool {
    rel.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        DEFAULT_EXCLUDES.iter().any(|e| s == *e)
    })
}

/// The file-name half of the always-on rule, deliberately separate from
/// [`is_excluded`]: a component match would also drop a *directory* named
/// `LICENSE.md`, and would drop nothing a caller could name as a file.
fn is_excluded_file(rel: &Path) -> bool {
    rel.file_name().is_some_and(|n| {
        let s = n.to_string_lossy();
        DEFAULT_EXCLUDE_FILES.iter().any(|e| s == *e)
    })
}

/// Remove from `files` every path a config `exclude` glob matches, and say
/// what that cost.
///
/// `files` is already deduplicated, so `dropped` counts files rather than
/// (file, include-glob) pairs — two includes reaching the same derived
/// index is one exclusion, not two. Every pattern is tested against every
/// path rather than short-circuiting on the first hit, because "matched
/// nothing" is a per-pattern fact and a pattern that only ever overlaps
/// another is still doing work.
///
/// A pattern that is not a valid glob is an error naming the pattern —
/// never a panic, and never a silent skip that would leave the corpus
/// wider than the config says.
fn apply_config_excludes(files: &mut Vec<PathBuf>, patterns: &[String]) -> Result<ExcludeReport> {
    if patterns.is_empty() {
        return Ok(ExcludeReport::default());
    }
    let compiled = patterns
        .iter()
        .map(|p| {
            glob::Pattern::new(p)
                .with_context(|| format!("bad exclude glob `{p}`"))
                .map(|c| (p.clone(), c))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut hits = vec![0usize; compiled.len()];
    let before = files.len();
    files.retain(|rel| {
        let path = rel_str(rel);
        let mut keep = true;
        for (i, (_, pattern)) in compiled.iter().enumerate() {
            if pattern.matches(&path) {
                hits[i] += 1;
                keep = false;
            }
        }
        keep
    });
    Ok(ExcludeReport {
        dropped: before - files.len(),
        stale: compiled
            .iter()
            .zip(&hits)
            .filter(|(_, n)| **n == 0)
            .map(|((p, _), _)| p.clone())
            .collect(),
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

    #[test]
    fn default_exclude_files_match_the_name_and_not_a_prefix_of_it() {
        assert!(is_excluded_file(Path::new("packages/x/v0.1.0/LICENSE.md")));
        assert!(is_excluded_file(Path::new("LICENSE.md")));
        assert!(!is_excluded_file(Path::new(
            "packages/x/v0.1.0/spec/LICENSE-NOTES.md"
        )));
        assert!(!is_excluded_file(Path::new("spec/modules/x/PROP-001.md")));
        // The component rule is untouched: it never knew this name.
        assert!(!is_excluded(Path::new("packages/x/v0.1.0/LICENSE.md")));
    }
}
