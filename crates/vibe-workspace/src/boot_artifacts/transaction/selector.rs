//! Atomic durable replacement for managed boot selector co-tenants.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use tempfile::{NamedTempFile, TempPath};

use crate::WorkspaceError;
use crate::safe_file;

use super::journal::{StageRole, stage_name, validate_owned_stage_name};
use super::{io_error, read_regular_optional, sweep_orphan_stages, sync_directory};

pub(in crate::boot_artifacts) fn replace_selector(
    path: &Path,
    bytes: &[u8],
    transaction: &str,
) -> Result<bool, WorkspaceError> {
    let existing = read_regular_optional(path)?;
    if existing.as_deref() == Some(bytes) {
        return Ok(false);
    }
    let parent = path
        .parent()
        .ok_or_else(|| io_error(path, "managed selector has no parent"))?;
    let current = BTreeSet::from([transaction.to_string()]);
    sweep_orphan_stages(parent, &current)?;
    let role = match path.file_name().and_then(|name| name.to_str()) {
        Some("CLAUDE.md") => StageRole::RedirectClaude,
        Some("AGENTS.md") => StageRole::RedirectAgents,
        Some("GEMINI.md") => StageRole::RedirectGemini,
        _ => return Err(io_error(path, "managed selector has an unknown role")),
    };
    let name = stage_name(transaction, role);
    let staged = parent.join(&name);
    validate_owned_stage_name(&name, Some(transaction), &staged)?;
    let mut file =
        safe_file::create_new_read_write(&staged).map_err(|error| io_error(&staged, error))?;
    if let Ok(existing_file) = safe_file::open_existing_read(path) {
        let permissions = existing_file
            .metadata()
            .map_err(|error| io_error(path, error))?
            .permissions();
        file.set_permissions(permissions)
            .map_err(|error| io_error(&staged, error))?;
    }
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| io_error(&staged, error))?;
    let temp_path =
        TempPath::try_from_path(staged.clone()).map_err(|error| io_error(&staged, error))?;
    let temp = NamedTempFile::from_parts(file, temp_path);
    let persisted = match temp.persist(path) {
        Ok(file) => file,
        Err(error) => {
            let primary = error.error.to_string();
            let keep = error.file.keep().err().map(|error| error.error.to_string());
            return Err(io_error(
                path,
                match keep {
                    Some(keep) => format!(
                        "atomic selector replace failed: {primary}; preserving stage failed: {keep}"
                    ),
                    None => format!(
                        "atomic selector replace failed: {primary}; stage retained at `{}`",
                        staged.display()
                    ),
                },
            ));
        }
    };
    persisted
        .sync_all()
        .map_err(|error| io_error(path, error))?;
    drop(persisted);
    let actual = read_regular_optional(path)?
        .ok_or_else(|| io_error(path, "managed selector disappeared after replace"))?;
    if actual != bytes {
        return Err(io_error(path, "managed selector changed after replace"));
    }
    sync_directory(parent)?;
    Ok(true)
}
