//! The sole finite cohort record under the existing application-work subsystem.
use super::*;
use crate::SettingsRoot;
use std::{fs, path::PathBuf};
pub(crate) use vibe_wire::generated::application::e1::import_work::ImportWork;
const PROTOCOL: &str = "vibe-app-import-work/1";

pub(crate) fn path(settings: &SettingsRoot, id: &str) -> Result<PathBuf, AppError> {
    if !pending::valid_operation(id) {
        return Err(failure(settings, id, "invalid import operation token"));
    }
    Ok(settings.app_work_dir().join("imports").join(id))
}
pub(crate) fn failure(settings: &SettingsRoot, id: &str, reason: &str) -> AppError {
    crate::StepError::ResumeMismatch {
        work: settings
            .app_work_dir()
            .join("imports")
            .display()
            .to_string(),
        reason: format!("import operation `{id}`: {reason}"),
    }
    .into()
}
pub(crate) fn load(settings: &SettingsRoot, id: &str) -> Result<ImportWork, AppError> {
    let directory = path(settings, id)?;
    let bytes = fs::read(directory.join("import.json"))
        .map_err(|_| failure(settings, id, "exact cohort state is unavailable"))?;
    let document: ImportWork = serde_json::from_slice(&bytes)
        .map_err(|_| failure(settings, id, "invalid closed cohort record"))?;
    if document.protocol != PROTOCOL
        || document.target.operation != id
        || document.target.home != pending::actual_home(settings)?
    {
        return Err(failure(
            settings,
            id,
            "cohort belongs to another home, generation or protocol",
        ));
    }
    if settings.is_project() {
        return Err(failure(
            settings,
            id,
            "portable import is a user-home operation",
        ));
    }
    let cohort = &document.cohort;
    if cohort.cursor as usize > cohort.rows.len()
        || cohort.current.as_ref().is_some_and(|t| {
            t.home != document.target.home
                || !pending::valid_operation(&t.operation)
                || cohort
                    .rows
                    .get(cohort.cursor as usize)
                    .is_none_or(|r| r.key != t.key)
        })
        || document.phase == w::ApplicationImportPhase::Complete
            && (!cohort.arrangement_done || cohort.cursor as usize != cohort.rows.len())
    {
        return Err(failure(
            settings,
            id,
            "cohort cursor, current row or terminal state is inconsistent",
        ));
    }
    for result in &cohort.results {
        if result.action == w::ApplicationPendingImportResultAction::Installed
            && (result.row_index.is_none() || result.target.is_none())
        {
            return Err(failure(
                settings,
                id,
                "an Installed cohort result requires its exact rowIndex and publication target",
            ));
        }
        if let Some(row) = result.row_index
            && (row >= cohort.cursor
                || cohort
                    .rows
                    .get(row as usize)
                    .is_none_or(|r| r.key != result.key))
        {
            return Err(failure(
                settings,
                id,
                "result does not identify a completed original row",
            ));
        }
        if let Some(target) = &result.target
            && (result.row_index.is_none()
                || result.action == w::ApplicationPendingImportResultAction::Skipped
                || target.home != document.target.home
                || target.key != result.key
                || !pending::valid_operation(&target.operation))
        {
            return Err(failure(
                settings,
                id,
                "result attempt binding is inconsistent",
            ));
        }
    }
    Ok(document)
}
fn encode(settings: &SettingsRoot, document: &ImportWork) -> Result<Vec<u8>, AppError> {
    let mut bytes = serde_json::to_vec_pretty(document).map_err(|_| {
        failure(
            settings,
            &document.target.operation,
            "cannot encode cohort record",
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}
pub(crate) fn save(settings: &SettingsRoot, document: &ImportWork) -> Result<(), AppError> {
    let directory = path(settings, &document.target.operation)?;
    let bytes = encode(settings, document)?;
    crate::settings::write_atomic(&directory.join("import.json"), &bytes)?;
    Ok(())
}
/// Only the fresh reservation created by this begin call may be discarded, and
/// any published entry is preserved even if publication returned an error.
fn discard_unpublished(directory: &std::path::Path) -> std::io::Result<bool> {
    match fs::symlink_metadata(directory.join("import.json")) {
        Ok(_) => return Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let metadata = match fs::symlink_metadata(directory) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    if !metadata.file_type().is_dir() {
        return Err(std::io::Error::other(
            "unpublished reservation path changed",
        ));
    }
    fs::remove_dir_all(directory)?;
    Ok(true)
}
pub(super) fn begin(
    ctx: &OperationContext<'_>,
    cohort: &PreparedImport,
) -> Result<ImportWork, AppError> {
    if ctx.settings.is_project() {
        return Err(failure(
            ctx.settings,
            "new",
            "portable import is a user-home operation",
        ));
    }
    let target = w::ApplicationImportTarget {
        home: pending::actual_home(ctx.settings)?,
        operation: pending::generation(),
    };
    let directory = path(ctx.settings, &target.operation)?;
    let document = ImportWork {
        protocol: PROTOCOL.into(),
        target,
        phase: w::ApplicationImportPhase::Running,
        context: w::ApplicationImportContext {
            lookup: pending::lookup(ctx)?,
            offline: ctx.offline,
            hosted: ctx.agent.hosted,
            assume_yes: ctx.agent.assume_yes,
            agent: ctx.agent.selected.map(|a| a.id.clone()),
            agent_external: ctx
                .agent
                .selected
                .is_some_and(|a| a.source == crate::AgentSource::External),
        },
        cohort: cohort.state.clone(),
    };
    let bytes = encode(ctx.settings, &document)?;
    fs::create_dir_all(directory.parent().ok_or_else(|| {
        failure(
            ctx.settings,
            &document.target.operation,
            "import namespace has no parent directory",
        )
    })?)
    .map_err(|_| {
        failure(
            ctx.settings,
            &document.target.operation,
            "cannot create import work namespace",
        )
    })?;
    fs::create_dir(&directory).map_err(|_| {
        failure(
            ctx.settings,
            &document.target.operation,
            "cannot reserve import generation",
        )
    })?;
    if let Err(error) = crate::settings::write_atomic(&directory.join("import.json"), &bytes) {
        if let Err(cleanup) = discard_unpublished(&directory)
            && let Some(notice) = ctx.on_notice
        {
            notice(&format!(
                "unpublished import reservation {} could not be removed: {cleanup}",
                directory.display()
            ));
        }
        return Err(error.into());
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn own_unpublished_reservation_is_removed_but_published_record_is_never_deleted() {
        let root = tempfile::tempdir().unwrap();
        let empty = root.path().join("empty");
        fs::create_dir(&empty).unwrap();
        fs::write(empty.join(".import.json.fixture.tmp"), b"partial").unwrap();
        assert!(discard_unpublished(&empty).unwrap());
        assert!(!empty.exists());
        let settings = SettingsRoot::new(&root.path().join("settings"));
        fs::create_dir_all(settings.path()).unwrap();
        let mut document: ImportWork = serde_json::from_slice(include_bytes!(
            "../../../../../formats/corpora/application/e1/import_work.json"
        ))
        .unwrap();
        document.target.home = pending::actual_home(&settings).unwrap();
        document.target.operation = "published-fixture".into();
        if let Some(child) = &mut document.cohort.current {
            child.home = document.target.home.clone();
        }
        save(&settings, &document).unwrap();
        let directory = path(&settings, &document.target.operation).unwrap();
        let before = fs::read(directory.join("import.json")).unwrap();
        assert!(!discard_unpublished(&directory).unwrap());
        assert_eq!(fs::read(directory.join("import.json")).unwrap(), before);
        load(&settings, &document.target.operation).unwrap();
    }
}
