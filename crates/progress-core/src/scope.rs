//! Observed-tree scoping: `progress.toml` include globs, the always-on
//! default excludes — by directory and by file name — and the project's own
//! enumerated `exclude` globs (PROP-043 §4).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#config");

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The always-applied exclusions — even under explicit includes. Matched
/// against **path components**, so each entry names a directory.
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
    pub progress: ProgressSection,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        ScopeConfig {
            schema: Some(1),
            include: DEFAULT_INCLUDES.iter().map(|s| s.to_string()).collect(),
            exclude: Vec::new(),
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

    /// A package slot as the campaign meets it: a licence, a derived index,
    /// the authored cards beside it, and a doc whose name merely starts
    /// like the licence.
    fn package_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let slot = dir.path().join("packages/x/v0.1.0");
        std::fs::create_dir_all(slot.join("spec/cards")).expect("mkdir");
        for rel in [
            "LICENSE.md",
            "spec/LICENSE-NOTES.md",
            "spec/cards/INDEX.md",
            "spec/cards/scaffold-a.md",
            "spec/cards/scaffold-b.md",
        ] {
            std::fs::write(slot.join(rel), "# T {#t}\n").expect("write");
        }
        dir
    }

    fn scan(dir: &tempfile::TempDir, exclude: &[&str]) -> (Vec<String>, ExcludeReport) {
        let cfg = ScopeConfig {
            include: vec!["packages/**/*.md".into()],
            exclude: exclude.iter().map(|s| s.to_string()).collect(),
            ..ScopeConfig::default()
        };
        let (files, report) = observed_files_reported(dir.path(), &cfg).expect("enumerate");
        (files.iter().map(|f| rel_str(f)).collect(), report)
    }

    #[test]
    fn the_licence_leaves_by_name_and_the_notes_beside_it_stay() {
        let dir = package_tree();
        let (files, report) = scan(&dir, &[]);
        assert!(!files.contains(&"packages/x/v0.1.0/LICENSE.md".to_string()));
        assert!(files.contains(&"packages/x/v0.1.0/spec/LICENSE-NOTES.md".to_string()));
        // The name rule is not a config exclusion, so it reports nothing.
        assert_eq!(report.dropped, 0);
        assert!(report.stale.is_empty());
    }

    #[test]
    fn a_config_exclude_drops_the_derived_index_and_keeps_the_cards() {
        let dir = package_tree();
        let (files, report) = scan(&dir, &["packages/**/spec/cards/INDEX.md"]);
        assert!(!files.contains(&"packages/x/v0.1.0/spec/cards/INDEX.md".to_string()));
        assert!(files.contains(&"packages/x/v0.1.0/spec/cards/scaffold-a.md".to_string()));
        assert!(files.contains(&"packages/x/v0.1.0/spec/cards/scaffold-b.md".to_string()));
        assert_eq!(report.dropped, 1);
        assert!(report.stale.is_empty());
    }

    #[test]
    fn an_exclude_matching_nothing_names_itself_and_removes_nothing() {
        let dir = package_tree();
        let (files, report) = scan(&dir, &["packages/**/spec/cards/RETIRED.md"]);
        assert_eq!(files.len(), 4);
        assert_eq!(report.dropped, 0);
        assert_eq!(report.stale, vec!["packages/**/spec/cards/RETIRED.md"]);
    }

    #[test]
    fn no_exclude_key_changes_nothing_and_says_nothing() {
        let dir = package_tree();
        let (files, report) = scan(&dir, &[]);
        let cfg = ScopeConfig {
            include: vec!["packages/**/*.md".into()],
            ..ScopeConfig::default()
        };
        let plain = observed_files(dir.path(), &cfg).expect("enumerate");
        assert_eq!(files.len(), plain.len());
        assert_eq!(report.dropped, 0);
        assert!(report.stale.is_empty());
    }

    #[test]
    fn an_invalid_exclude_glob_is_an_error_naming_the_pattern() {
        let dir = package_tree();
        let cfg = ScopeConfig {
            include: vec!["packages/**/*.md".into()],
            exclude: vec!["packages/a**/x.md".into()],
            ..ScopeConfig::default()
        };
        let err = observed_files_reported(dir.path(), &cfg).expect_err("invalid glob");
        assert!(
            format!("{err:#}").contains("packages/a**/x.md"),
            "error must name the pattern: {err:#}"
        );
    }

    #[test]
    fn an_absent_exclude_key_parses_to_an_empty_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("progress.toml"),
            "schema = 1\ninclude = [\"spec/**/*.md\"]\n",
        )
        .expect("write");
        let cfg = load_config(dir.path()).expect("load");
        assert!(cfg.exclude.is_empty());

        std::fs::write(
            dir.path().join("progress.toml"),
            "schema = 1\ninclude = [\"spec/**/*.md\"]\nexclude = [\"spec/gen/**/*.md\"]\n",
        )
        .expect("write");
        let cfg = load_config(dir.path()).expect("load");
        assert_eq!(cfg.exclude, vec!["spec/gen/**/*.md"]);
    }
}
