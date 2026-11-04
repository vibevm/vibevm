//! Inspect and explicitly abandon finite import cohorts within application work.
use super::*;
use crate::operations::{import, pending};
use vibe_wire::generated::shared as w;

pub(super) fn imports(
    settings: &SettingsRoot,
    id: Option<&str>,
) -> Result<Vec<wire::ImportEntry>, AppError> {
    let ids = if let Some(id) = id {
        vec![id.to_owned()]
    } else {
        directories(&settings.app_work_dir().join("imports"))
            .into_iter()
            .map(|p| file_name(&p))
            .collect()
    };
    let mut rows = Vec::new();
    for id in ids {
        let document = import::load(settings, &id)?;
        let path = import::store::path(settings, &id)?;
        rows.push(wire::ImportEntry {
            target: document.target,
            path: path.display().to_string(),
            phase: document.phase,
            cursor: document.cohort.cursor,
            total: u32::try_from(document.cohort.rows.len())
                .map_err(|_| import::store::failure(settings, &id, "too many retained rows"))?,
            changed: changed(&path),
            bytes: tree_bytes(&path),
            current: document.cohort.current,
        });
    }
    Ok(rows)
}
/// Inspect one exact cohort without consulting or inventing a package coordinate.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn work_import(settings: &SettingsRoot, id: &str) -> Result<wire::ApplicationWork, AppError> {
    Ok(wire::ApplicationWork {
        protocol: WORK_PROTOCOL.into(),
        home: settings.app_work_dir().display().to_string(),
        entries: vec![],
        imports: imports(settings, Some(id))?,
        removed: vec![],
        freed: 0,
    })
}
/// Abandon this cohort and only its exact owned attempts, after whole-set preflight.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn clean_import(settings: &SettingsRoot, id: &str) -> Result<wire::ApplicationWork, AppError> {
    let held = settings.lock()?;
    let lock = &held;
    let mut listing = work_import(settings, id)?;
    let document = import::load(settings, id)?;
    let mut visited = std::collections::BTreeSet::new();
    if let Some(target) = &document.cohort.current {
        collect(
            settings,
            target,
            &pending::import_reference(&document.target, document.cohort.cursor),
            &mut visited,
            &mut listing.entries,
        )?;
    }
    for result in &document.cohort.results {
        if let (Some(target), Some(row)) = (&result.target, result.row_index) {
            collect(
                settings,
                target,
                &pending::import_reference(&document.target, row),
                &mut visited,
                &mut listing.entries,
            )?;
        }
    }
    cleanup(lock, settings, &mut listing, false)?;
    Ok(listing)
}
fn collect(
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
    parent: &w::ApplicationContinuationTarget,
    visited: &mut std::collections::BTreeSet<(String, String)>,
    entries: &mut Vec<wire::Entry>,
) -> Result<(), AppError> {
    if !visited.insert((target.key.clone(), target.operation.clone())) {
        return Ok(());
    }
    let package = pending::coordinate(&target.key)?;
    let path = pending::work(settings, &package);
    let Some(state) = StepsFile::load(&path)? else {
        return Ok(());
    };
    let Some(op) = &state.operation else {
        return Err(pending::failure(
            settings,
            &package,
            "selected cohort attempt was replaced by unverifiable legacy work".into(),
        ));
    };
    if op.target != *target {
        return Ok(());
    } // A later generation is outside this deletion set.
    if op.parent.as_ref() != Some(parent) {
        return Err(pending::failure(
            settings,
            &package,
            "selected cohort no longer owns this exact attempt".into(),
        ));
    }
    pending::load(settings, &package, Some(&target.operation))?;
    for child in op.children.iter().filter_map(|c| c.target.as_ref()) {
        collect(
            settings,
            child,
            &pending::application_reference(target),
            visited,
            entries,
        )?;
    }
    entries.push(entry(settings, &path, &file_name(&path), false)?);
    Ok(())
}

/// Ordinary package work and cohort cleanup share this single deletion owner.
pub(super) fn cleanup(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    listing: &mut wire::ApplicationWork,
    all: bool,
) -> Result<(), AppError> {
    let selected = listing
        .entries
        .iter()
        .map(|e| PathBuf::from(&e.path))
        .chain(listing.imports.iter().map(|e| PathBuf::from(&e.path)))
        .map(|p| {
            p.canonicalize()
                .map_err(|source| io("resolving", &p, source))
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let mut targets = Vec::new();
    // Preflight every owner before changing index references or any directory.
    for entry in &listing.entries {
        let path = Path::new(&entry.path);
        let loaded=match StepsFile::load(path){Ok(state)=>state,Err(_) if all=>None,Err(_)=>return Err(StepError::ResumeMismatch{work:entry.path.clone(),reason:"unreadable selected package work cannot prove its surviving ownership; inspect it or explicitly abandon the complete same-home cohort with `app work --clean --all`".into()}.into())};
        if let Some(state) = loaded
            && let Some(op) = &state.operation
        {
            let actual = path
                .canonicalize()
                .map_err(|source| io("resolving", path, source))?;
            if op.target.home != pending::actual_home(settings)?
                || op.target.key != state.package.key()
                || pending::work(settings, &state.package)
                    .canonicalize()
                    .ok()
                    .as_ref()
                    != Some(&actual)
            {
                return Err(StepError::ResumeMismatch {
                    work: entry.path.clone(),
                    reason: "selected work has a foreign or mismatched application target".into(),
                }
                .into());
            }
            if let Some(parent) = &op.parent {
                let owner = pending::owner_path(settings, parent)?;
                let absent = fs::symlink_metadata(&owner)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
                let selected_parent = owner
                    .canonicalize()
                    .ok()
                    .is_some_and(|p| selected.contains(&p));
                if !selected_parent
                    && !(all && absent)
                    && pending::retained_for(settings, &state)?.is_some()
                {
                    return Err(StepError::ResumeMismatch{work:entry.path.clone(),reason:format!("work is retained for `{}`; finish that owner or explicitly clean its whole cohort",pending::continuation_command(parent))}.into());
                }
            }
            targets.push(op.target.clone());
        }
    }
    for cohort in &listing.imports {
        let document = import::load(settings, &cohort.target.operation)?;
        if document.target != cohort.target {
            return Err(import::store::failure(
                settings,
                &cohort.target.operation,
                "selected cohort target changed",
            ));
        }
        targets.extend(document.cohort.current);
        targets.extend(document.cohort.results.into_iter().filter_map(|r| r.target));
    }
    // Removing work deliberately abandons only matching pending references. Real
    // installation/step evidence remains; future fresh repair is not blocked by
    // an index reference to a directory this command just removed.
    let mut index = crate::Index::load(settings)?;
    if all {
        let home = pending::actual_home(settings)?;
        for record in index.installations.values() {
            if let Some(target) = &record.pending {
                let local = pending::work(settings, &record.package);
                if local
                    .canonicalize()
                    .ok()
                    .is_some_and(|p| selected.contains(&p))
                {
                    if target.home != home
                        || target.key != record.package.key()
                        || !pending::valid_operation(&target.operation)
                    {
                        return Err(StepError::ResumeMismatch{work:local.display().to_string(),reason:"selected installed pending reference has a foreign or invalid target".into()}.into());
                    }
                    targets.push(target.clone());
                }
            }
        }
    }
    let mut changed_index = false;
    for record in index.installations.values_mut() {
        if record.pending.as_ref().is_some_and(|p| targets.contains(p)) {
            record.pending = None;
            changed_index = true;
        }
    }
    if changed_index {
        index.save_locked(
            settings,
            &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            lock,
        )?;
    }
    for (path, bytes) in listing
        .entries
        .iter()
        .map(|e| (&e.path, e.bytes))
        .chain(listing.imports.iter().map(|e| (&e.path, e.bytes)))
    {
        fs::remove_dir_all(path).map_err(|source| io("removing", Path::new(path), source))?;
        listing.freed += bytes;
        listing.removed.push(path.clone());
    }
    Ok(())
}
