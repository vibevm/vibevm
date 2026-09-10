//! Read-only proof that the catalog is exactly its journal's projection.
//!
//! The journal is the only input to reconstruction. The projected catalog is
//! written to a scratch directory and byte-compared with the writer-owned
//! surface of the operator's data directory. Nothing is repaired in place.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#persistence");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::index::memory::WriteCtx;
use crate::index::{WRITER_DIRS, WRITER_FILES};
use crate::journal;

/// The catalog's drift from its journal's projection.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProjectionDrift {
    /// The projection writes these paths, but the on-disk catalog lacks them.
    missing: Vec<PathBuf>,
    /// The on-disk catalog carries these paths, but the projection does not.
    extra: Vec<PathBuf>,
    /// Both sides carry the path, but the bytes differ.
    changed: Vec<PathBuf>,
}

impl ProjectionDrift {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty() && self.changed.is_empty()
    }

    pub fn len(&self) -> usize {
        self.missing.len() + self.extra.len() + self.changed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.is_clean()
    }

    pub fn missing(&self) -> &[PathBuf] {
        &self.missing
    }

    pub fn extra(&self) -> &[PathBuf] {
        &self.extra
    }

    pub fn changed(&self) -> &[PathBuf] {
        &self.changed
    }

    /// The stable per-path diagnostics shared by both command surfaces.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for rel in &self.extra {
            lines.push(format!("rebuild: extra   `{}`", rel.display()));
        }
        for rel in &self.missing {
            lines.push(format!("rebuild: missing `{}`", rel.display()));
        }
        for rel in &self.changed {
            lines.push(format!("rebuild: differs `{}`", rel.display()));
        }
        lines
    }
}

/// What one rebuild check established.
#[derive(Debug, PartialEq, Eq)]
pub struct CheckOutcome {
    /// Number of files in the freshly rebuilt projection.
    pub projection_files: usize,
    /// Every byte-level difference from the catalog on disk.
    pub drift: ProjectionDrift,
}

/// Fold `data_dir`'s journal into a scratch catalog and compare it byte for
/// byte with the catalog on disk.
///
/// This function never writes beneath `data_dir`; the only writes go to an
/// automatically removed temporary directory.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#persistence",
    r = 1
)]
pub fn check_catalog(data_dir: &Path) -> Result<CheckOutcome> {
    let scratch = tempfile::Builder::new()
        .prefix("vibe-index-rebuild-")
        .tempdir()
        .map_err(|error| Error::Io {
            path: std::env::temp_dir(),
            message: format!("creating rebuild scratch directory: {error}"),
        })?;
    rebuild_drift(data_dir, scratch.path())
}

/// Run the read-only proof and render the operator-facing verdict.
///
/// Both `vibe-index rebuild --check` and the compatibility
/// `cargo xtask rebuild --check` wrapper call this function.
pub fn run_check(data_dir: &Path) -> Result<()> {
    let outcome = check_catalog(data_dir)?;
    if outcome.drift.is_clean() {
        println!(
            "rebuild --check: the catalog at `{}` is byte-identical to its journal's \
             projection ({} file(s)); no fact lives in the derived artifact \
             (PROP-044 ##FORBID-SECRET-TRUTH).",
            data_dir.display(),
            outcome.projection_files,
        );
        return Ok(());
    }
    for line in outcome.drift.lines() {
        eprintln!("{line}");
    }
    Err(Error::ProjectionDrift {
        data_dir: data_dir.to_path_buf(),
        count: outcome.drift.len(),
    })
}

fn rebuild_drift(data_dir: &Path, scratch: &Path) -> Result<CheckOutcome> {
    let records = journal::replay(&journal::default_dir(data_dir))?;
    let index = journal::project(records)?;
    let at = index.generated_at;
    index.write_to(scratch, &WriteCtx { at })?;
    diff_against_catalog(scratch, data_dir)
}

/// Every file the writer just produced. Since `scratch` began empty, walking
/// it whole discovers future writer output without a parallel list.
fn projection_file_set(scratch: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    for entry in WalkDir::new(scratch) {
        let entry = entry.map_err(|error| walk_error(scratch, error))?;
        if entry.file_type().is_file() {
            files.insert(relative_path(scratch, entry.path())?);
        }
    }
    Ok(files)
}

/// Files under the catalog writer's explicit root-file and directory-tree
/// whitelist. Runtime state, transport metadata and operator prose are outside
/// this set because the projection writer does not produce them.
fn catalog_file_set(data_dir: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    for name in WRITER_FILES {
        if data_dir.join(name).is_file() {
            files.insert(PathBuf::from(name));
        }
    }
    for dir in WRITER_DIRS {
        let root = data_dir.join(dir);
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&root) {
            let entry = entry.map_err(|error| walk_error(&root, error))?;
            if entry.file_type().is_file() {
                files.insert(relative_path(data_dir, entry.path())?);
            }
        }
    }
    Ok(files)
}

fn relative_path(root: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|error| Error::Io {
            path: path.to_path_buf(),
            message: format!(
                "rebuild path escaped its walk root `{}`: {error}",
                root.display()
            ),
        })
}

fn walk_error(root: &Path, error: walkdir::Error) -> Error {
    Error::Io {
        path: error.path().unwrap_or(root).to_path_buf(),
        message: format!("walking rebuild surface: {error}"),
    }
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| Error::Io {
        path: path.to_path_buf(),
        message: format!("reading rebuild comparison input: {error}"),
    })
}

fn diff_against_catalog(rebuilt: &Path, on_disk: &Path) -> Result<CheckOutcome> {
    let projection = projection_file_set(rebuilt)?;
    let projection_files = projection.len();
    let catalog = catalog_file_set(on_disk)?;
    let mut drift = ProjectionDrift::default();
    for rel in projection.union(&catalog) {
        match (projection.contains(rel), catalog.contains(rel)) {
            (true, false) => drift.missing.push(rel.clone()),
            (false, true) => drift.extra.push(rel.clone()),
            (true, true) => {
                if read_bytes(&rebuilt.join(rel))? != read_bytes(&on_disk.join(rel))? {
                    drift.changed.push(rel.clone());
                }
            }
            (false, false) => unreachable!("union yields members of at least one set"),
        }
    }
    Ok(CheckOutcome {
        projection_files,
        drift,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::journal::append;
    use crate::journal::record::{Event, JournalRecord};
    use crate::types::{Group, NamingConvention, PackageKind, VersionEntry};
    use chrono::{DateTime, Utc};

    fn at(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339)
            .expect("fixture timestamps parse")
            .with_timezone(&Utc)
    }

    fn org() -> Group {
        Group::parse("org.vibevm").expect("fixture group parses")
    }

    fn fixture(tmp: &Path) -> PathBuf {
        let data_dir = tmp.join("data");
        let t1 = at("2026-08-01T12:00:00Z");
        let t2 = at("2026-08-02T12:00:00Z");
        let records = vec![
            JournalRecord {
                at: t1,
                actor: "vibe-index 1.0.0".into(),
                event: Event::Initialised {
                    registry: "vibespecs".into(),
                    registry_url: "https://example.invalid/vibespecs".into(),
                    naming: NamingConvention::Fqdn,
                },
            },
            JournalRecord {
                at: t2,
                actor: "vibe-index 1.0.0".into(),
                event: Event::Published {
                    entry: Box::new(VersionEntry::minimal(
                        PackageKind::Flow,
                        org(),
                        "wal",
                        "1.0.0".parse().expect("fixture version parses"),
                        t2,
                    )),
                },
            },
        ];
        for record in &records {
            append(&journal::default_dir(&data_dir), record).expect("journal append");
        }
        let index = journal::project(records).expect("fixture journal folds");
        index
            .write_to(
                &data_dir,
                &WriteCtx {
                    at: index.generated_at,
                },
            )
            .expect("fixture catalog writes");
        data_dir
    }

    #[test]
    fn a_projected_catalog_passes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        let outcome = check_catalog(&data_dir).expect("check runs");
        assert!(outcome.drift.is_clean(), "{:?}", outcome.drift);
        assert!(
            outcome.projection_files >= 4,
            "the writer's surface is at least its four per-fixture files, got {}",
            outcome.projection_files
        );
    }

    #[test]
    fn an_extra_by_name_file_is_named() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        fs::write(data_dir.join("by-name/ghost.json"), "{}\n").expect("plant extra");
        let outcome = check_catalog(&data_dir).expect("check runs");
        assert_eq!(outcome.drift.extra(), [PathBuf::from("by-name/ghost.json")]);
        assert!(outcome.drift.missing().is_empty());
        assert!(outcome.drift.changed().is_empty());
        assert!(outcome.drift.lines()[0].contains("extra"));
    }

    #[test]
    fn a_flipped_byte_in_primary_is_named() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        let path = data_dir.join("primary.jsonl");
        let mut bytes = fs::read(&path).expect("read primary");
        let last = bytes.len() - 1;
        bytes[last] = if bytes[last] == b'0' { b'1' } else { b'0' };
        fs::write(&path, bytes).expect("flip byte");
        let outcome = check_catalog(&data_dir).expect("check runs");
        assert_eq!(outcome.drift.changed(), [PathBuf::from("primary.jsonl")]);
        assert!(outcome.drift.missing().is_empty());
        assert!(outcome.drift.extra().is_empty());
    }

    #[test]
    fn a_missing_writer_file_is_named() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        fs::remove_file(data_dir.join("primary.jsonl.gz")).expect("remove projected file");
        let outcome = check_catalog(&data_dir).expect("check runs");
        assert_eq!(outcome.drift.missing(), [PathBuf::from("primary.jsonl.gz")]);
        assert!(outcome.drift.extra().is_empty());
        assert!(outcome.drift.changed().is_empty());
    }

    #[test]
    fn state_dir_files_do_not_count() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        fs::write(data_dir.join("state/server.lock"), "pid=42\n").expect("plant state");
        let outcome = check_catalog(&data_dir).expect("check runs");
        assert!(outcome.drift.is_clean(), "{:?}", outcome.drift);
    }

    #[test]
    fn an_empty_or_absent_journal_refuses_cleanly() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let bare = tmp.path().join("bare");
        fs::create_dir_all(&bare).expect("mkdir");
        let message = check_catalog(&bare).expect_err("must refuse").to_string();
        assert!(message.contains("journal"), "{message}");
        assert!(message.contains("init"), "{message}");

        let hollow = tmp.path().join("hollow");
        fs::create_dir_all(journal::default_dir(&hollow)).expect("mkdir");
        let message = check_catalog(&hollow).expect_err("must refuse").to_string();
        assert!(message.contains("journal"), "{message}");
    }
}
