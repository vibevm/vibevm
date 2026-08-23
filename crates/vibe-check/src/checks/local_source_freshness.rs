//! Check — every locked package whose `source_kind = "local"` still matches
//! its on-disk source tree; a source that changed (or vanished) since
//! install warns to `vibe install --assume-yes`.
//!
//! `vibedeps/` holds the materialised copies `vibe install` writes. Nothing
//! reconciles them with the source: a `packages/` edit leaves the copy
//! silently stale, and the drift surfaces only by accident. This cell gives
//! the project the missing signal — recompute the source's content hash with
//! the same `vibe_registry::compute_content_hash` install used and compare it
//! against the `content_hash` recorded at install time. A `Warning` (never an
//! `Error`): a changed source does not by itself require a reinstall, so a
//! red panel would only train operators to ignore it. The signal is the point.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{LockedPackage, Lockfile, SourceKind};
use vibe_registry::compute_content_hash;

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::LocalSourceFreshness`] cell.
#[cell(seam = "Check", variant = "local-source-freshness")]
pub struct LocalSourceFreshnessCheck;

impl Check for LocalSourceFreshnessCheck {
    fn id(&self) -> CheckId {
        CheckId::LocalSourceFreshness
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let lockfile_path = project_root.join(Lockfile::FILENAME);
        if !lockfile_path.exists() {
            // Not a project with installed dependencies — nothing to
            // reconcile, and `ManifestValidity` owns lockfile well-formedness.
            return;
        }
        let lockfile = match Lockfile::read(&lockfile_path) {
            Ok(l) => l,
            Err(_) => {
                // Surfaced by the manifest-validity cell; don't double-report.
                return;
            }
        };

        for pkg in &lockfile.packages {
            // Only a project-local source lives on this machine — a registry,
            // git, override, or path-source entry has no on-disk tree here to
            // hash against.
            if pkg.source_kind != Some(SourceKind::Local) {
                continue;
            }
            let Some(src) = file_url_to_path(pkg.source_url.as_str()) else {
                // A `local` entry whose `source_url` is not a `file://` path
                // can't be located; leave it to other checks rather than guess.
                continue;
            };
            if !src.is_dir() {
                report.warn(
                    CheckId::LocalSourceFreshness,
                    Some(PathBuf::from(Lockfile::FILENAME)),
                    None,
                    format!(
                        "package {}: its local source `{}` is no longer on disk — the \
                         installed copy in `vibedeps/` cannot be reconciled; run \
                         `vibe install --assume-yes`",
                        coordinate(pkg),
                        pkg.source_url.as_str(),
                    ),
                );
                continue;
            }
            let fresh = match compute_content_hash(&src) {
                Ok(h) => h,
                Err(e) => {
                    report.warn(
                        CheckId::LocalSourceFreshness,
                        Some(PathBuf::from(Lockfile::FILENAME)),
                        None,
                        format!(
                            "package {}: could not recompute its local source's content hash \
                             (`{}`): {e} — freshness unverified",
                            coordinate(pkg),
                            pkg.source_url.as_str(),
                        ),
                    );
                    continue;
                }
            };
            if fresh != pkg.content_hash.as_str() {
                report.warn(
                    CheckId::LocalSourceFreshness,
                    Some(PathBuf::from(Lockfile::FILENAME)),
                    None,
                    format!(
                        "package {}: its local source changed since install (recorded \
                         content_hash {}, source now {fresh}) — run \
                         `vibe install --assume-yes` to refresh the installed copy",
                        coordinate(pkg),
                        pkg.content_hash.as_str(),
                    ),
                );
            }
        }
    }
}

/// `group/name@version` — the human-facing coordinate for a finding.
fn coordinate(pkg: &LockedPackage) -> String {
    format!(
        "{group}/{name}@{version}",
        group = pkg.group,
        name = pkg.name,
        version = pkg.version,
    )
}

/// Decode a `file://` URL to a filesystem path, mirroring
/// `vibe_workspace::freshness::source` (the project's canonical decoder):
/// `file:///C:/x` → `C:/x` (drop the leading slash before a Windows drive
/// letter); `file:///home/x` → `/home/x` (already an absolute Unix path).
/// Any non-`file://` URL returns `None` — it is not a local source path.
fn file_url_to_path(source_url: &str) -> Option<PathBuf> {
    let rest = source_url.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let path_str = if bytes.len() >= 3
        && bytes[0] == b'/'
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
    {
        &rest[1..]
    } else {
        rest
    };
    Some(PathBuf::from(path_str))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Severity, check_project};

    use super::file_url_to_path;

    /// Build the `file://` URL for a local source directory the way
    /// `vibe install` records it: forward slashes, with a leading slash
    /// before a Windows drive letter (`file:///C:/...`).
    fn file_url_of(path: &Path) -> String {
        let s = path.to_string_lossy().replace('\\', "/");
        format!("file:///{s}")
    }

    fn write_local_lock(project: &Path, source_url: &str, content_hash: &str) {
        let lockfile = format!(
            "[meta]\ngenerated_by = \"vibe-test\"\ngenerated_at = \"2026-08-05T00:00:00Z\"\n\
             schema_version = 6\n\
             \n[[package]]\n\
             kind = \"flow\"\n\
             group = \"org.vibevm\"\n\
             name = \"wal\"\n\
             version = \"0.1.0\"\n\
             source_url = \"{source_url}\"\n\
             content_hash = \"{content_hash}\"\n\
             files_written = []\n\
             source_kind = \"local\"\n"
        );
        fs::write(project.join("vibe.lock"), lockfile).unwrap();
    }

    #[test]
    fn matching_local_source_emits_no_finding() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let src = project.path().join("packages/flow-wal");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("vibe.toml"), "[package]\nname = \"wal\"\n").unwrap();
        // Seed the recorded hash with the source's real hash → no drift.
        let recorded = vibe_registry::compute_content_hash(&src).unwrap();
        write_local_lock(project.path(), &file_url_of(&src), &recorded);

        let report = check_project(project.path(), &opts());
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.check == CheckId::LocalSourceFreshness),
            "expected no local-source-freshness finding; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn drifted_local_source_warns_once() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let src = project.path().join("packages/flow-wal");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("vibe.toml"), "[package]\nname = \"wal\"\n").unwrap();
        // A content_hash the source does NOT produce — drift.
        write_local_lock(project.path(), &file_url_of(&src), "sha256:deadbeef");

        let report = check_project(project.path(), &opts());
        let hits: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::LocalSourceFreshness)
            .collect();
        assert_eq!(hits.len(), 1, "got: {:?}", report.findings);
        assert_eq!(hits[0].severity, Severity::Warning);
        assert!(hits[0].message.contains("org.vibevm/wal@0.1.0"));
        assert!(hits[0].message.contains("vibe install --assume-yes"));
    }

    #[test]
    fn missing_local_source_dir_warns() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // Point at a directory that does not exist on disk.
        let gone = project.path().join("packages/flow-wal");
        write_local_lock(project.path(), &file_url_of(&gone), "sha256:deadbeef");

        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::LocalSourceFreshness
                    && f.severity == Severity::Warning
                    && f.message.contains("no longer on disk")),
            "got: {:?}",
            report.findings
        );
    }

    #[test]
    fn non_local_source_kind_is_skipped() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let src = project.path().join("packages/flow-wal");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("vibe.toml"), "[package]\nname = \"wal\"\n").unwrap();
        // An `embedded` entry pointing at a source whose hash differs from the
        // recorded one — must NOT be flagged: only `local` is reconciled here.
        let lockfile = format!(
            "[meta]\ngenerated_by = \"vibe-test\"\ngenerated_at = \"2026-08-05T00:00:00Z\"\n\
             schema_version = 6\n\
             \n[[package]]\n\
             kind = \"flow\"\n\
             group = \"org.vibevm\"\n\
             name = \"wal\"\n\
             version = \"0.1.0\"\n\
             source_url = \"{src}\"\n\
             content_hash = \"sha256:deadbeef\"\n\
             files_written = []\n\
             source_kind = \"embedded\"\n",
            src = file_url_of(&src),
        );
        fs::write(project.path().join("vibe.lock"), lockfile).unwrap();

        let report = check_project(project.path(), &opts());
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.check == CheckId::LocalSourceFreshness),
            "a non-local source_kind must be skipped; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn file_url_decoder_round_trips_drive_and_unix() {
        // Windows drive-letter URL drops the leading slash.
        assert_eq!(
            file_url_to_path("file:///C:/Users/x").unwrap(),
            PathBuf::from("C:/Users/x")
        );
        // A Unix absolute URL is kept as-is.
        assert_eq!(
            file_url_to_path("file:///home/me/x").unwrap(),
            PathBuf::from("/home/me/x")
        );
        // Non-file:// URLs are not local sources.
        assert_eq!(file_url_to_path("https://example/x"), None);
    }
}
