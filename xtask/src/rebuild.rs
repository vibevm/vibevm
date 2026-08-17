//! `cargo xtask rebuild` — the proof that a vibe-index catalog is
//! nothing but its journal's projection (PROP-044 §3, phase Ф3.2d of
//! TZ-CHANGE-NATIVE-FORMATS).
//!
//! Phase Ф3 moved the six mutation paths and the server lift-off off
//! reading the catalog; this verb makes that claim mechanical. `--check`
//! folds the data directory's journal back into a catalog, writes it to
//! a scratch directory, and byte-compares the result against the
//! catalog on disk. Any difference — a file under the writer's surface
//! the projection does not produce, a file it produces that the catalog
//! lacks, one byte that differs — means the catalog carries a fact the
//! journal does not describe: a derived artifact holding truth, which
//! [`##FORBID-SECRET-TRUTH`][prop] forbids.
//!
//! The clock never runs here either. Every mutation stamps `write_to`
//! with the same `at` it journals, so the catalog's `generated_at`
//! equals the last journal record's `at` — and the projection takes
//! its stamp from exactly there. When those disagree, the disagreement
//! IS the finding; substituting the on-disk catalog's time would read
//! the derived artifact as an input and hide it.
//!
//! [prop]: ../../spec/common/PROP-044-change-native-formats.md#laws

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_index::index::memory::WriteCtx;
use vibe_index::index::{WRITER_DIRS, WRITER_FILES};
use vibe_index::journal;
use walkdir::WalkDir;

/// The catalog's drift from its journal's projection, in the three
/// ways two file sets can disagree.
#[derive(Debug, Default)]
struct Drift {
    /// The projection writes these; the catalog on disk lacks them.
    missing: Vec<PathBuf>,
    /// The catalog holds these under the writer's surface; the
    /// projection does not produce them.
    extra: Vec<PathBuf>,
    /// Both sides hold the path; the bytes differ.
    changed: Vec<PathBuf>,
}

impl Drift {
    fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty() && self.changed.is_empty()
    }

    fn len(&self) -> usize {
        self.missing.len() + self.extra.len() + self.changed.len()
    }
}

/// What one check established: the rebuilt projection's size (the
/// denominator the clean message reports) and its drift from the
/// catalog on disk.
#[derive(Debug)]
struct CheckOutcome {
    projection_files: usize,
    drift: Drift,
}

/// The engine behind the CLI: fold `data_dir`'s journal into a catalog,
/// write it to `scratch`, and byte-diff the writer's surface against
/// what `data_dir` holds.
///
/// The journal is the ONLY input. The catalog on disk is never read as
/// an input to the projection — not its entries, and not its clock:
/// `at` comes from the last journal record, so a catalog written at a
/// moment the journal does not know surfaces as byte drift instead of
/// being silently reproduced.
fn rebuild_drift(data_dir: &Path, scratch: &Path) -> Result<CheckOutcome> {
    let journal_dir = journal::default_dir(data_dir);
    let records = journal::replay(&journal_dir).with_context(|| {
        format!(
            "rebuild: reading the journal at `{}`",
            journal_dir.display()
        )
    })?;
    let index = journal::project(records).with_context(|| {
        format!(
            "rebuild: folding the journal at `{}`",
            journal_dir.display()
        )
    })?;
    let at = index.generated_at;
    index
        .write_to(scratch, &WriteCtx { at })
        .with_context(|| format!("rebuild: writing the projection to `{}`", scratch.display()))?;
    diff_against_catalog(scratch, data_dir)
}

/// Every file the writer just produced. The scratch directory holds
/// nothing but `write_to`'s output, so walking it whole IS the writer's
/// surface — no parallel list to maintain, nothing a new writer output
/// could fall outside of.
fn projection_file_set(scratch: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    for entry in WalkDir::new(scratch).into_iter() {
        let entry = entry.with_context(|| format!("rebuild: walking `{}`", scratch.display()))?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(scratch)
                .with_context(|| "rebuild: path outside its walk root")?;
            files.insert(rel.to_path_buf());
        }
    }
    Ok(files)
}

/// The files of `data_dir`'s catalog that the comparison owns:
/// everything under the writer's surface ([`WRITER_FILES`] +
/// [`WRITER_DIRS`]), nothing else.
///
/// The exclusion is stated positively — "compare the set the writer
/// writes" — not as a blacklist. A blacklist must enumerate the world
/// (`state/`, `.git/`, `README.md`, `.gitignore`, …) and rots the day
/// the data directory grows a file nobody listed; a whitelist says what
/// the catalog IS. `state/` — the server lock, the scanner checkpoint,
/// the journal itself — is the host's runtime state, not a projection
/// of any fact; a file the writer never writes is not the catalog
/// either. Both stay outside the comparison for the one reason that
/// holds tomorrow too: the writer does not produce them.
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
        for entry in WalkDir::new(&root).into_iter() {
            let entry = entry.with_context(|| format!("rebuild: walking `{}`", root.display()))?;
            if entry.file_type().is_file() {
                let rel = entry
                    .path()
                    .strip_prefix(data_dir)
                    .with_context(|| "rebuild: path outside its walk root")?;
                files.insert(rel.to_path_buf());
            }
        }
    }
    Ok(files)
}

/// Byte-diff the rebuilt projection (`rebuilt`) against the catalog on
/// disk (`on_disk`), in the shape of [`Drift`].
fn diff_against_catalog(rebuilt: &Path, on_disk: &Path) -> Result<CheckOutcome> {
    let projection = projection_file_set(rebuilt)?;
    let projection_files = projection.len();
    let catalog = catalog_file_set(on_disk)?;
    let mut drift = Drift::default();
    for rel in projection.union(&catalog) {
        match (projection.contains(rel), catalog.contains(rel)) {
            (true, false) => drift.missing.push(rel.clone()),
            (false, true) => drift.extra.push(rel.clone()),
            (true, true) => {
                let a = fs::read(rebuilt.join(rel))
                    .with_context(|| format!("rebuild: reading projection `{}`", rel.display()))?;
                let b = fs::read(on_disk.join(rel))
                    .with_context(|| format!("rebuild: reading catalog `{}`", rel.display()))?;
                if a != b {
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

/// The per-path lines the failure report prints — a function, so the
/// tests assert against the very lines the operator reads.
fn drift_lines(drift: &Drift) -> Vec<String> {
    let mut lines = Vec::new();
    for rel in &drift.extra {
        lines.push(format!("rebuild: extra   `{}`", rel.display()));
    }
    for rel in &drift.missing {
        lines.push(format!("rebuild: missing `{}`", rel.display()));
    }
    for rel in &drift.changed {
        lines.push(format!("rebuild: differs `{}`", rel.display()));
    }
    lines
}

pub(crate) fn run_rebuild(check: bool, data_dir: &Path) -> Result<()> {
    if !check {
        bail!(
            "rebuild: only `--check` exists — repairing the catalog from its journal in \
             place is a separate decision; this verb ships the proof only"
        );
    }
    let scratch = tempfile::tempdir().context("rebuild: creating the scratch directory")?;
    let outcome = rebuild_drift(data_dir, scratch.path())?;
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
    for line in drift_lines(&outcome.drift) {
        eprintln!("{line}");
    }
    bail!(
        "rebuild --check: {} drift item(s) against the journal under `{}`. A catalog \
         that differs from its journal's projection carries a fact the journal does \
         not describe — a derived artifact holding truth, which PROP-044 \
         `##FORBID-SECRET-TRUTH` forbids. Fix: regenerate the catalog FROM the \
         journal (every vibe-index mutation reprojects it wholesale); never edit \
         the journal to match the catalog — that would launder the secret truth \
         into the truth layer.",
        outcome.drift.len(),
        data_dir.display(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, Utc};
    use vibe_index::journal::append;
    use vibe_index::journal::record::{Event, JournalRecord};
    use vibe_index::types::{Group, NamingConvention, PackageKind, VersionEntry};

    fn at(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339)
            .expect("fixture timestamps parse")
            .with_timezone(&Utc)
    }

    fn org() -> Group {
        Group::parse("org.vibevm").expect("fixture group parses")
    }

    /// A data directory whose catalog is the honest projection of its
    /// journal: two records (`Initialised` + `Published`) appended by
    /// hand, the catalog written by the very projection the check
    /// performs. That is the contract under test — every mutation
    /// stamps `write_to` with the same `at` it journals, so catalog and
    /// projection agree byte-for-byte by construction, and the drift
    /// tests below then break exactly one thing.
    fn fixture(tmp: &Path) -> PathBuf {
        let data_dir = tmp.join("data");
        let t1 = at("2026-08-01T12:00:00Z");
        let t2 = at("2026-08-02T12:00:00Z");
        let records = vec![
            JournalRecord {
                at: t1,
                actor: "vibe-index 0.1.0-dev".into(),
                event: Event::Initialised {
                    registry: "vibespecs".into(),
                    registry_url: "https://example.invalid/vibespecs".into(),
                    naming: NamingConvention::Fqdn,
                },
            },
            JournalRecord {
                at: t2,
                actor: "vibe-index 0.1.0-dev".into(),
                event: Event::Published {
                    entry: Box::new(VersionEntry::minimal(
                        PackageKind::Flow,
                        org(),
                        "wal",
                        "0.1.0".parse().expect("fixture version parses"),
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

    /// §3.1 — a catalog the projection reproduces passes, and the
    /// outcome carries a sane denominator.
    #[test]
    fn a_projected_catalog_passes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        let scratch = tempfile::tempdir().expect("tempdir");
        let outcome = rebuild_drift(&data_dir, scratch.path()).expect("check runs");
        assert!(outcome.drift.is_clean(), "{:?}", outcome.drift);
        // repomd.json + primary.jsonl + primary.jsonl.gz + by-name/wal.json
        assert!(
            outcome.projection_files >= 4,
            "the writer's surface is at least its four per-fixture files, got {}",
            outcome.projection_files
        );
    }

    /// §3.2 — a file under `by-name/` the projection does not produce
    /// is named as extra drift, and nothing else moves.
    #[test]
    fn an_extra_by_name_file_is_named() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        fs::write(data_dir.join("by-name/ghost.json"), "{}\n").expect("plant extra");
        let scratch = tempfile::tempdir().expect("tempdir");
        let outcome = rebuild_drift(&data_dir, scratch.path()).expect("check runs");
        let lines = drift_lines(&outcome.drift);
        let joined = lines.join("\n");
        assert!(
            joined.contains("extra") && joined.contains("ghost.json"),
            "the extra path must be named, got: {joined}"
        );
        assert!(outcome.drift.missing.is_empty(), "{:?}", outcome.drift);
        assert!(outcome.drift.changed.is_empty(), "{:?}", outcome.drift);
    }

    /// §3.3 — one flipped byte in `primary.jsonl` is named as changed
    /// drift. Same length, so sizes and the untouched `repomd.json`
    /// stay comparable and exactly this one path moves.
    #[test]
    fn a_flipped_byte_in_primary_is_named() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        let path = data_dir.join("primary.jsonl");
        let mut bytes = fs::read(&path).expect("read primary");
        let last = bytes.len() - 1;
        bytes[last] = if bytes[last] == b'0' { b'1' } else { b'0' };
        fs::write(&path, bytes).expect("flip byte");
        let scratch = tempfile::tempdir().expect("tempdir");
        let outcome = rebuild_drift(&data_dir, scratch.path()).expect("check runs");
        let lines = drift_lines(&outcome.drift);
        let joined = lines.join("\n");
        assert!(
            joined.contains("differs") && joined.contains("primary.jsonl"),
            "the changed path must be named, got: {joined}"
        );
        assert!(outcome.drift.missing.is_empty(), "{:?}", outcome.drift);
        assert!(outcome.drift.extra.is_empty(), "{:?}", outcome.drift);
    }

    /// §3.4 — `state/` is the host's runtime state, not a projection of
    /// any journal fact: a file under it is not drift. The red proof
    /// runs this test against a whole-directory comparison and quotes
    /// the fall.
    #[test]
    fn state_dir_files_do_not_count() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = fixture(tmp.path());
        fs::write(data_dir.join("state/server.lock"), "pid=42\n").expect("plant state");
        let scratch = tempfile::tempdir().expect("tempdir");
        let outcome = rebuild_drift(&data_dir, scratch.path()).expect("check runs");
        assert!(outcome.drift.is_clean(), "{:?}", outcome.drift);
    }

    /// §3.5 — a journal that is absent or empty refuses with a message
    /// that names the emptiness and the `init` recipe, never a panic.
    #[test]
    fn an_empty_or_absent_journal_refuses_cleanly() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let scratch = tempfile::tempdir().expect("tempdir");

        // No journal at all.
        let bare = root.join("bare");
        fs::create_dir_all(&bare).expect("mkdir");
        let err = rebuild_drift(&bare, scratch.path()).expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("journal is empty"),
            "the refusal must name the emptiness: {msg}"
        );
        assert!(
            msg.contains("init"),
            "the refusal must carry the `init` recipe: {msg}"
        );

        // A journal directory that exists but holds no shard.
        let hollow = root.join("hollow");
        fs::create_dir_all(journal::default_dir(&hollow)).expect("mkdir");
        let err = rebuild_drift(&hollow, scratch.path()).expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("journal is empty"),
            "the refusal must name the emptiness: {msg}"
        );
    }
}
