//! Safe snapshot and transaction-bound stage preparation.

use std::io::Write;
use std::path::Path;

use tempfile::Builder;

use crate::WorkspaceError;
use crate::safe_file;

use super::journal::{
    Journal, JournalEntry, JournalMode, RecordedState, STAGE_PREFIX, StageRole, file_name,
    stage_name, validate_journal, validate_owned_stage_name,
};
use super::{
    ArtifactWrite, FaultInjector, JOURNAL_SCHEMA, WritePoint, absent_state, aggregate_error,
    bytes_digest, io_error, journal_path, read_regular_optional, remove_owned_stage,
};

pub(super) fn prepare_journal(
    write: ArtifactWrite<'_>,
    parent: &Path,
    faults: &impl FaultInjector,
) -> Result<Journal, WorkspaceError> {
    let index_before_bytes = read_regular_optional(write.index_path)?;
    let selected_before_bytes = read_regular_optional(write.static_path)?;
    let stale_before_bytes = read_regular_optional(write.stale_path)?;
    let mut assets = Vec::new();

    let result = (|| {
        faults.check(WritePoint::IndexWrite, write.index_path)?;
        let (transaction, index_after) = stage_initial_index(parent, write.index_bytes)?;
        assets.push(index_after.staged.clone().ok_or_else(|| {
            io_error(
                write.index_path,
                "staged INDEX payload has no owned stage name",
            )
        })?);
        let index_before = stage_optional(
            parent,
            &transaction,
            StageRole::IndexBefore,
            index_before_bytes.as_deref(),
            &mut assets,
        )?;
        let selected_before = stage_optional(
            parent,
            &transaction,
            StageRole::SelectedBefore,
            selected_before_bytes.as_deref(),
            &mut assets,
        )?;
        let stale_before = stage_optional(
            parent,
            &transaction,
            StageRole::StaleBefore,
            stale_before_bytes.as_deref(),
            &mut assets,
        )?;
        let selected_after = match write.static_bytes {
            Some(bytes) => {
                faults.check(WritePoint::StaticWrite, write.static_path)?;
                stage_optional(
                    parent,
                    &transaction,
                    StageRole::SelectedAfter,
                    Some(bytes),
                    &mut assets,
                )?
            }
            None => absent_state(),
        };
        let journal = Journal {
            schema: JOURNAL_SCHEMA,
            transaction,
            mode: JournalMode::Commit,
            index: JournalEntry {
                target: file_name(write.index_path)?,
                before: index_before,
                after: index_after,
            },
            selected: JournalEntry {
                target: file_name(write.static_path)?,
                before: selected_before,
                after: selected_after,
            },
            stale: JournalEntry {
                target: file_name(write.stale_path)?,
                before: stale_before,
                after: absent_state(),
            },
        };
        validate_journal(&journal, &journal_path(parent), JournalMode::Commit)?;
        Ok(journal)
    })();

    match result {
        Ok(journal) => Ok(journal),
        Err(primary) => {
            let mut cleanup = Vec::new();
            for name in assets {
                if let Err(error) = remove_owned_stage(parent, &name, None) {
                    cleanup.push(error);
                }
            }
            if cleanup.is_empty() {
                Err(primary)
            } else {
                Err(aggregate_error(
                    parent,
                    primary,
                    cleanup,
                    "prepare-stage cleanup",
                ))
            }
        }
    }
}

fn stage_initial_index(
    parent: &Path,
    bytes: &[u8],
) -> Result<(String, RecordedState), WorkspaceError> {
    let mut temp = Builder::new()
        .prefix(STAGE_PREFIX)
        .suffix("-index-after.stage")
        .tempfile_in(parent)
        .map_err(|error| io_error(parent, error))?;
    temp.write_all(bytes)
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|error| io_error(temp.path(), error))?;
    let path = temp.path().to_path_buf();
    let name = file_name(&path)?;
    let transaction = name
        .strip_prefix(STAGE_PREFIX)
        .and_then(|value| value.strip_suffix("-index-after.stage"))
        .ok_or_else(|| io_error(&path, "random INDEX stage has no transaction identity"))?
        .to_string();
    if name != stage_name(&transaction, StageRole::IndexAfter) {
        return Err(io_error(&path, "random INDEX stage violates owned naming"));
    }
    temp.keep().map_err(|error| io_error(&path, error.error))?;
    verify_stage(&path, bytes)?;
    Ok((
        transaction,
        RecordedState {
            present: true,
            digest: Some(bytes_digest(bytes)),
            staged: Some(name),
        },
    ))
}

fn stage_optional(
    parent: &Path,
    transaction: &str,
    role: StageRole,
    bytes: Option<&[u8]>,
    assets: &mut Vec<String>,
) -> Result<RecordedState, WorkspaceError> {
    let Some(bytes) = bytes else {
        return Ok(absent_state());
    };
    let name = stage_name(transaction, role);
    let path = parent.join(&name);
    validate_owned_stage_name(&name, Some(transaction), &path)?;
    let mut file =
        safe_file::create_new_read_write(&path).map_err(|error| io_error(&path, error))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| io_error(&path, error))?;
    let identity = safe_file::identity(&file).map_err(|error| io_error(&path, error))?;
    safe_file::assert_path_identity(&path, identity).map_err(|error| io_error(&path, error))?;
    assets.push(name.clone());
    Ok(RecordedState {
        present: true,
        digest: Some(bytes_digest(bytes)),
        staged: Some(name),
    })
}

fn verify_stage(path: &Path, expected: &[u8]) -> Result<(), WorkspaceError> {
    let actual = read_regular_optional(path)?
        .ok_or_else(|| io_error(path, "newly staged payload disappeared"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(io_error(path, "newly staged payload changed before intent"))
    }
}
