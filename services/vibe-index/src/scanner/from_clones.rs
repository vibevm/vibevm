//! Walk a local directory of org clones, fold each `(repo, tag)` into
//! a [`VersionEntry`].
//!
//! For every subdirectory of `org_dir` that has a `.git` (regular
//! clone), list the tags, filter to `v<semver>`, and for each tag:
//! materialise the working tree to a clean temp dir, parse the
//! manifest, walk subskills, compute `content_hash`, assemble the
//! entry. Skipped repos / tags / manifests surface as warnings on
//! `tracing::warn!` but do not abort the scan — the operator gets a
//! best-effort index even with one bad package in the mix.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use semver::Version;

use crate::content_hash::compute_content_hash;
use crate::error::{Error, Result};
use crate::index::checkpoint::{Checkpoint, RepoSnapshot};
use crate::scanner::git_cli;
use crate::scanner::manifest as mfst;
use crate::types::{NamingConvention, PackageKind, VersionEntry};

#[derive(Debug, Clone)]
pub struct FromClonesOptions {
    pub registry: String,
    pub registry_url: String,
    pub naming: NamingConvention,
    pub generator: String,
    /// Indexed-at timestamp stamped on every entry produced in this
    /// scan. Single shared timestamp for determinism within a run.
    pub indexed_at: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct ScanReport {
    pub entries: Vec<VersionEntry>,
    pub skipped: Vec<SkipNote>,
    /// Snapshot of every walked repo's HEAD + tag list. Persisted by
    /// the reindex driver as `<data-dir>/state/checkpoint.json` so
    /// the next `--incremental` run can skip unchanged repos.
    pub snapshots: BTreeMap<String, RepoSnapshot>,
}

#[derive(Debug, Clone)]
pub struct SkipNote {
    pub repo: String,
    pub tag: Option<String>,
    pub reason: String,
}

pub fn scan_org_dir(org_dir: &Path, opts: &FromClonesOptions) -> Result<ScanReport> {
    scan_org_dir_with_filter(org_dir, opts, None)
}

/// Walk `org_dir` and produce a [`ScanReport`]. When `prior` is
/// `Some`, repos whose HEAD commit AND tag list match the recorded
/// snapshot are skipped — the reindex driver carries forward their
/// existing index entries unchanged. PROP-005 §2.8 incremental.
pub fn scan_org_dir_with_filter(
    org_dir: &Path,
    opts: &FromClonesOptions,
    prior: Option<&Checkpoint>,
) -> Result<ScanReport> {
    if !org_dir.is_dir() {
        return Err(Error::InvalidInput(format!(
            "org-dir `{}` is not a directory",
            org_dir.display()
        )));
    }
    let mut entries = Vec::new();
    let mut skipped = Vec::new();
    let mut snapshots: BTreeMap<String, RepoSnapshot> = BTreeMap::new();

    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(org_dir)
        .map_err(|e| Error::Io {
            path: org_dir.to_path_buf(),
            message: e.to_string(),
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    subdirs.sort();

    for repo in subdirs {
        let repo_name = repo
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !git_cli::is_git_dir(&repo) {
            skipped.push(SkipNote {
                repo: repo_name,
                tag: None,
                reason: "not a git working tree (no .git found)".into(),
            });
            continue;
        }
        let tags = match git_cli::list_tags(&repo) {
            Ok(t) => t,
            Err(e) => {
                skipped.push(SkipNote {
                    repo: repo_name,
                    tag: None,
                    reason: format!("could not list tags: {e}"),
                });
                continue;
            }
        };
        let head = git_cli::head_commit(&repo);

        let mut sorted_tags = tags.clone();
        sorted_tags.sort();
        let snapshot = RepoSnapshot {
            head_commit: head.clone(),
            tags: sorted_tags.clone(),
        };
        snapshots.insert(repo_name.clone(), snapshot.clone());

        let prior_snap = prior.and_then(|p| p.repos.get(&repo_name));
        if let Some(prev) = prior_snap
            && prev == &snapshot
        {
            // Unchanged — caller copies entries from the previous
            // index. Skip note is informational rather than a warning.
            skipped.push(SkipNote {
                repo: repo_name,
                tag: None,
                reason: "unchanged since last checkpoint (incremental skip)".into(),
            });
            continue;
        }

        for tag in tags {
            let Some(version) = parse_v_tag(&tag) else {
                skipped.push(SkipNote {
                    repo: repo_name.clone(),
                    tag: Some(tag.clone()),
                    reason: "tag is not a `v<semver>` form".into(),
                });
                continue;
            };
            match build_entry(&repo, &repo_name, &tag, version, opts) {
                Ok(entry) => entries.push(entry),
                Err(e) => skipped.push(SkipNote {
                    repo: repo_name.clone(),
                    tag: Some(tag),
                    reason: e.to_string(),
                }),
            }
        }
    }

    Ok(ScanReport {
        entries,
        skipped,
        snapshots,
    })
}

fn build_entry(
    repo: &Path,
    repo_name: &str,
    tag: &str,
    version: Version,
    opts: &FromClonesOptions,
) -> Result<VersionEntry> {
    let workspace = tempfile::tempdir().map_err(|e| Error::Io {
        path: repo.to_path_buf(),
        message: format!("could not create scratch dir: {e}"),
    })?;
    let snapshot = workspace.path().join("snapshot");
    git_cli::materialise_at_ref(repo, tag, &snapshot)?;

    let manifest_bytes =
        std::fs::read(snapshot.join("vibe.toml")).map_err(|e| Error::Io {
            path: snapshot.join("vibe.toml"),
            message: e.to_string(),
        })?;
    let raw = mfst::parse_manifest(&manifest_bytes)?;

    let content_hash = compute_content_hash(&snapshot)?;
    let resolved_commit = git_cli::resolve_commit(repo, tag).ok();
    let files_count = count_files(&snapshot)? as u32;

    let kind = raw.package.kind;
    let name = raw.package.name.clone();

    if name != raw.package.name {
        // Manifest carries a name; we trust it (not the dir name).
    }

    let _ = repo_name; // dir name kept for diagnostics; not part of the entry.

    let subskills = mfst::collect_subskills(&snapshot)?;

    let entry = VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        name,
        version: version.clone(),
        content_hash,
        source_url: source_url_for(&opts.registry_url, opts.naming, kind, &raw.package.name),
        source_ref: tag.to_string(),
        resolved_commit,
        registry: opts.registry.clone(),
        license: raw.package.license.clone(),
        authors: raw.package.authors.clone(),
        description: raw.package.description.clone(),
        homepage: raw.package.homepage.clone(),
        keywords: raw.package.keywords.clone(),
        describes: raw.package.describes.clone(),
        compatibility: mfst::compatibility_from_raw(&raw.compatibility),
        provides: mfst::provides_from_raw(&raw.provides),
        requires: mfst::requires_from_raw(&raw.requires),
        requires_any: mfst::requires_any_from_raw(&raw.requires_any),
        obsoletes: mfst::obsoletes_from_raw(&raw.obsoletes),
        conflicts: mfst::conflicts_from_raw(&raw.conflicts),
        features: mfst::features_from_raw(&raw.features)?,
        subskills,
        i18n: mfst::i18n_from_raw(&raw.i18n),
        boot_snippet: mfst::boot_snippet_from_raw(&raw.boot_snippet),
        files_count,
        indexed_at: opts.indexed_at,
        indexed_by: opts.generator.clone(),
    };
    Ok(entry)
}

pub fn parse_v_tag(tag: &str) -> Option<Version> {
    let stripped = tag.strip_prefix('v')?;
    Version::parse(stripped).ok()
}

fn source_url_for(
    registry_url: &str,
    naming: NamingConvention,
    kind: PackageKind,
    name: &str,
) -> String {
    let repo = naming.repo_name(kind, name);
    let trimmed = registry_url.trim_end_matches('/');
    format!("{trimmed}/{repo}.git")
}

fn count_files(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PackageKind;

    #[test]
    fn parse_v_tag_accepts_simple_form() {
        assert_eq!(parse_v_tag("v0.1.0").unwrap().to_string(), "0.1.0");
        assert_eq!(parse_v_tag("v1.0.0-rc.1").unwrap().to_string(), "1.0.0-rc.1");
        assert!(parse_v_tag("0.1.0").is_none());
        assert!(parse_v_tag("v-not-semver").is_none());
        assert!(parse_v_tag("vibe").is_none());
    }

    #[test]
    fn source_url_uses_naming_convention() {
        assert_eq!(
            source_url_for(
                "https://github.com/vibespecs",
                NamingConvention::KindName,
                PackageKind::Flow,
                "wal"
            ),
            "https://github.com/vibespecs/flow-wal.git"
        );
        assert_eq!(
            source_url_for(
                "https://gitverse.ru/vibespecs/",
                NamingConvention::Name,
                PackageKind::Flow,
                "wal"
            ),
            "https://gitverse.ru/vibespecs/wal.git"
        );
    }
}
