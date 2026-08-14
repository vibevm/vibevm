//! The journal's on-disk store — append-only NDJSON shards under one
//! directory, one shard per calendar month of the record's `at`.
//!
//! Append goes to the END of the shard with an fsync after the write:
//! a journal that rewrites its own past is not a journal, which is why
//! this module does NOT reuse [`atomic_write`](crate::index::persistence::atomic_write)
//! — tmp + rename replaces the whole file, and replacing is what the
//! journal must never do. The atomic-replace trick still guards every
//! whole-file writer under `index/`; there the file is a projection,
//! here it is the truth.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Utc};

use crate::error::{Error, Result};
use crate::journal::record::JournalRecord;

/// The journal's directory. An INPUT, never a constant: the ruling of
/// 2026-08-13 keeps the journal out of the surface handed to clients,
/// and a public deployment points this at a separate repository. The
/// default sits under the data directory's gitignored `state/`, which
/// is the only place measured to be invisible to git, to
/// `repomd.json` and to HTTP alike.
pub fn default_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("state").join("journal")
}

/// The shard file name for `at` — `<YYYY>-<MM>.ndjson`, a pure function
/// of the input, never a clock call.
fn shard_name(at: &DateTime<Utc>) -> String {
    format!("{:04}-{:02}.ndjson", at.year(), at.month())
}

/// Append one record. The shard is chosen from `record.at` — a pure
/// function of the input, never a clock call. The line is written to
/// the end of the shard and fsynced before returning: a reader that
/// sees the append never sees a torn line.
pub fn append(journal_dir: &Path, record: &JournalRecord) -> Result<()> {
    fs::create_dir_all(journal_dir).map_err(|e| io_err(journal_dir, e))?;
    let shard = journal_dir.join(shard_name(&record.at));
    let mut line = serde_json::to_string(record)
        .map_err(|e| Error::Malformed(format!("could not serialise journal record: {e}")))?;
    line.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&shard)
        .map_err(|e| io_err(&shard, e))?;
    file.write_all(line.as_bytes())
        .map_err(|e| io_err(&shard, e))?;
    file.sync_all().map_err(|e| io_err(&shard, e))?;
    Ok(())
}

/// Read every record, in journal order: shards in ascending name order
/// (the zero-padded `<YYYY>-<MM>` names sort chronologically), lines
/// within a shard in file order. A missing journal directory is an
/// empty history, not an error — the journal has not been initialised.
pub fn replay(journal_dir: &Path) -> Result<Vec<JournalRecord>> {
    let mut shards: Vec<PathBuf> = match fs::read_dir(journal_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "ndjson"))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err(journal_dir, e)),
    };
    shards.sort();

    let mut out = Vec::new();
    for shard in shards {
        let text = fs::read_to_string(&shard).map_err(|e| io_err(&shard, e))?;
        for (lineno, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record: JournalRecord = serde_json::from_str(line).map_err(|e| {
                Error::Malformed(format!(
                    "journal {} line {} is malformed: {e}",
                    shard.display(),
                    lineno + 1
                ))
            })?;
            out.push(record);
        }
    }
    Ok(out)
}

fn io_err(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        message: source.to_string(),
    }
}
