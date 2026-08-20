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

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use semver::Version;
use vibe_core::Group;

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
    /// Org-image cache outcome, surfaced for visibility (Р5):
    /// `Some(true)` = served from a fresh cache (304 hit);
    /// `Some(false)` = re-enumerated; `None` = caching not in use
    /// (`--from-clones`, or `--from-github --no-cache-org`). Only the
    /// `from-github` scanner sets this (Р6).
    pub org_cache_hit: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SkipNote {
    pub repo: String,
    pub tag: Option<String>,
    pub reason: String,
}

/// Record a skip in the report AND surface it as a `tracing::warn!` —
/// the docblock's promise: a skipped repo / tag / manifest is visible
/// to the operator the moment it is skipped, not only when they go
/// looking for the report. The report stays the record (it feeds the
/// reindex summary); the warn is the surface.
fn note_skip(skipped: &mut Vec<SkipNote>, note: SkipNote) {
    match &note.tag {
        Some(tag) => tracing::warn!("skipped {}@{tag}: {}", note.repo, note.reason),
        None => tracing::warn!("skipped {}: {}", note.repo, note.reason),
    }
    skipped.push(note);
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
            note_skip(
                &mut skipped,
                SkipNote {
                    repo: repo_name,
                    tag: None,
                    reason: "not a git working tree (no .git found)".into(),
                },
            );
            continue;
        }
        let tags = match git_cli::list_tags(&repo) {
            Ok(t) => t,
            Err(e) => {
                note_skip(
                    &mut skipped,
                    SkipNote {
                        repo: repo_name,
                        tag: None,
                        reason: format!("could not list tags: {e}"),
                    },
                );
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
            // index. Informational, deliberately NOT a warn (so it
            // stays a plain push, not `note_skip`): an unchanged repo
            // is the healthy path of every incremental run, and
            // warning on it would bury the four defect-kind skips the
            // docblock promises warnings for. Nothing was lost here —
            // the entries carry forward from the previous index.
            skipped.push(SkipNote {
                repo: repo_name,
                tag: None,
                reason: "unchanged since last checkpoint (incremental skip)".into(),
            });
            continue;
        }

        for tag in tags {
            let Some(version) = parse_v_tag(&tag) else {
                note_skip(
                    &mut skipped,
                    SkipNote {
                        repo: repo_name.clone(),
                        tag: Some(tag.clone()),
                        reason: "tag is not a `v<semver>` form".into(),
                    },
                );
                continue;
            };
            match build_entry(&repo, &repo_name, &tag, version, opts) {
                Ok(entry) => entries.push(entry),
                Err(e) => note_skip(
                    &mut skipped,
                    SkipNote {
                        repo: repo_name.clone(),
                        tag: Some(tag),
                        reason: e.to_string(),
                    },
                ),
            }
        }
    }

    Ok(ScanReport {
        entries,
        skipped,
        snapshots,
        org_cache_hit: None,
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

    let manifest_bytes = std::fs::read(snapshot.join("vibe.toml")).map_err(|e| Error::Io {
        path: snapshot.join("vibe.toml"),
        message: e.to_string(),
    })?;
    let manifest = mfst::parse_manifest(&manifest_bytes)?;
    let pkg = mfst::require_package(&manifest)?;

    let content_hash = compute_content_hash(&snapshot)?;
    let resolved_commit = git_cli::resolve_commit(repo, tag).ok();
    let files_count = count_files(&snapshot)? as u32;

    let kind = mfst::package_kind(pkg.kind);
    let _ = repo_name; // dir name kept for diagnostics; not part of the entry.

    let subskills = mfst::collect_subskills(&snapshot)?;

    let source_url = source_url_for(
        &opts.registry_url,
        &opts.naming,
        &kind,
        &pkg.group,
        &pkg.name,
    );

    let entry = VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        group: pkg.group.clone(),
        name: pkg.name.clone(),
        version: version.clone(),
        content_hash,
        source_url,
        source_ref: tag.to_string(),
        resolved_commit,
        registry: opts.registry.clone(),
        workspace_origin: mfst::workspace_origin_from(&manifest.origin),
        license: pkg.license.clone(),
        authors: pkg.authors.clone(),
        description: pkg.description.clone(),
        homepage: pkg.homepage.clone(),
        keywords: pkg.keywords.clone(),
        describes: pkg.describes.as_ref().map(|p| p.to_string()),
        compatibility: mfst::compatibility_from(&manifest.compatibility),
        provides: mfst::provides_from(&manifest.provides),
        requires: mfst::requires_from(&manifest.requires),
        requires_any: mfst::requires_any_from(&manifest.requires_any),
        obsoletes: mfst::obsoletes_from(&manifest.obsoletes),
        conflicts: mfst::conflicts_from(&manifest.conflicts),
        features: mfst::features_from(&manifest.features),
        subskills,
        i18n: mfst::i18n_from(&manifest.i18n),
        boot_snippet: mfst::boot_snippet_from(&manifest.boot_snippet),
        files_count,
        must_understand: Vec::new(),
        yanked: false,
        frozen: pkg.frozen,
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
    naming: &NamingConvention,
    kind: &PackageKind,
    group: &Group,
    name: &str,
) -> String {
    let repo = naming.repo_name(kind, group, name);
    let trimmed = registry_url.trim_end_matches('/');
    format!("{trimmed}/{repo}.git")
}

fn count_files(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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

    /// A capture sink for `tracing_subscriber::fmt` — the warn lines
    /// the scanner emits land in a buffer the test reads back, via a
    /// scoped `with_default` subscriber (no global state, no races
    /// with the crate's other tests). `make_writer` hands out clones
    /// sharing one Arc, so the builder gets an owned writer that
    /// satisfies `for<'writer> MakeWriter<'writer>`.
    #[derive(Default, Clone)]
    struct WarnCapture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl WarnCapture {
        fn contents(&self) -> Vec<u8> {
            self.0.lock().unwrap().clone()
        }
    }

    impl std::io::Write for WarnCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for WarnCapture {
        type Writer = WarnCapture;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// Build a one-commit-one-tag repo under `parent`. `manifest` is
    /// written when `Some` — `None` builds a repo whose tagged tree
    /// carries no `vibe.toml` (the manifest-skip shape).
    fn make_tagged_repo(parent: &Path, name: &str, tag: &str, manifest: Option<&str>) {
        let repo = parent.join(name);
        std::fs::create_dir_all(&repo).unwrap();
        let git = |args: &[&str]| {
            let s = std::process::Command::new("git")
                .args(["-C", repo.to_str().unwrap()])
                .args(args)
                .status()
                .unwrap();
            assert!(s.success(), "git {args:?} failed");
        };
        git(&["init", "--quiet", "-b", "main"]);
        git(&["config", "user.email", "test@test.invalid"]);
        git(&["config", "user.name", "Test"]);
        if let Some(body) = manifest {
            std::fs::write(repo.join("vibe.toml"), body).unwrap();
        }
        std::fs::write(repo.join("README.md"), format!("# {tag}\n")).unwrap();
        git(&["add", "."]);
        git(&["commit", "--quiet", "-m", tag]);
        git(&["tag", tag]);
    }

    /// The docblock's promise, made checkable: every defect-kind skip
    /// (repo / tag / manifest) surfaces as a `tracing::warn!` naming
    /// what was skipped and why — not only as a row in the report.
    #[test]
    fn skips_surface_as_tracing_warns() {
        if std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let parent = tempfile::tempdir().unwrap();

        // Repo skip: a plain subdirectory with no `.git`.
        std::fs::create_dir_all(parent.path().join("not-a-repo")).unwrap();
        // Tag skip: a real repo whose only tag is not `v<semver>`.
        make_tagged_repo(parent.path(), "odd-tags", "release-1", None);
        // Manifest skip: a real repo tagged `v<semver>` with no
        // `vibe.toml` at the tag.
        make_tagged_repo(parent.path(), "no-manifest", "v0.2.0", None);

        let capture = WarnCapture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .with_target(false)
            .with_writer(capture.clone())
            .finish();
        let report = tracing::subscriber::with_default(subscriber, || {
            scan_org_dir(
                parent.path(),
                &FromClonesOptions {
                    registry: "vibespecs".into(),
                    registry_url: "https://example.invalid/vibespecs".into(),
                    naming: NamingConvention::Fqdn,
                    generator: "test".into(),
                    indexed_at: Utc::now(),
                },
            )
            .unwrap()
        });

        let out = String::from_utf8(capture.contents()).unwrap();
        assert!(
            out.contains("skipped not-a-repo"),
            "the repo skip must warn; captured:\n{out}"
        );
        assert!(
            out.contains("skipped odd-tags@release-1"),
            "the tag skip must warn; captured:\n{out}"
        );
        assert!(
            out.contains("skipped no-manifest@v0.2.0"),
            "the manifest skip must warn; captured:\n{out}"
        );
        // The warn is the surface, the report the record — both carry
        // the note.
        assert!(report.skipped.iter().any(|s| s.repo == "not-a-repo"));
        assert!(report.skipped.iter().any(|s| s.repo == "odd-tags"));
        assert!(report.skipped.iter().any(|s| s.repo == "no-manifest"));
    }

    #[test]
    fn parse_v_tag_accepts_simple_form() {
        assert_eq!(parse_v_tag("v0.1.0").unwrap().to_string(), "0.1.0");
        assert_eq!(
            parse_v_tag("v1.0.0-rc.1").unwrap().to_string(),
            "1.0.0-rc.1"
        );
        assert!(parse_v_tag("0.1.0").is_none());
        assert!(parse_v_tag("v-not-semver").is_none());
        assert!(parse_v_tag("vibe").is_none());
    }

    /// PROP-044 §2a — the manifest's `frozen` reaches the catalog entry
    /// through the org-scanner projection path (the second of the two
    /// disjoint birth paths; `cli::add` covers the first). Skipped when
    /// git is unavailable.
    #[test]
    fn scan_projects_manifest_frozen_into_the_entry() {
        if std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let parent = tempfile::tempdir().unwrap();
        let repo = parent.path().join("flow-wal");
        std::fs::create_dir_all(&repo).unwrap();
        let git = |args: &[&str]| {
            let s = std::process::Command::new("git")
                .args(["-C", repo.to_str().unwrap()])
                .args(args)
                .status()
                .unwrap();
            assert!(s.success(), "git {args:?} failed");
        };
        git(&["init", "--quiet", "-b", "main"]);
        git(&["config", "user.email", "test@test.invalid"]);
        git(&["config", "user.name", "Test"]);
        std::fs::write(
            repo.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\nfrozen = true\n",
        )
        .unwrap();
        git(&["add", "."]);
        git(&["commit", "--quiet", "-m", "initial"]);
        git(&["tag", "v0.1.0"]);

        let report = scan_org_dir(
            parent.path(),
            &FromClonesOptions {
                registry: "vibespecs".into(),
                registry_url: "https://example.invalid/vibespecs".into(),
                naming: NamingConvention::Fqdn,
                generator: "test".into(),
                indexed_at: Utc::now(),
            },
        )
        .unwrap();
        assert_eq!(report.entries.len(), 1);
        assert!(
            report.entries[0].frozen,
            "manifest `frozen = true` must reach the catalog entry via the scanner"
        );
    }

    #[test]
    fn source_url_uses_naming_convention() {
        let org = Group::parse("org.vibevm").unwrap();
        assert_eq!(
            source_url_for(
                "https://github.com/vibespecs",
                &NamingConvention::Fqdn,
                &PackageKind::Flow,
                &org,
                "wal"
            ),
            "https://github.com/vibespecs/org.vibevm.wal.git"
        );
        assert_eq!(
            source_url_for(
                "https://github.com/vibespecs",
                &NamingConvention::KindName,
                &PackageKind::Flow,
                &org,
                "wal"
            ),
            "https://github.com/vibespecs/flow-wal.git"
        );
        assert_eq!(
            source_url_for(
                "https://gitverse.ru/vibespecs/",
                &NamingConvention::Name,
                &PackageKind::Flow,
                &org,
                "wal"
            ),
            "https://gitverse.ru/vibespecs/wal.git"
        );
    }
}
