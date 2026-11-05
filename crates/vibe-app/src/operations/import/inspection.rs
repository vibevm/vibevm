//! The finite cohort's effect footprint includes retained plans not yet started.
use super::{document::ImportWork, pending, store};
use crate::{AppError, SettingsRoot};
use std::collections::BTreeSet;
use vibe_wire::generated::shared as w;

pub(crate) fn effect_keys(
    settings: &SettingsRoot,
    document: &ImportWork,
) -> Result<BTreeSet<String>, AppError> {
    let mut keys = document
        .affected_keys()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut seen = BTreeSet::new();
    if let Some(target) = document.current() {
        visit(
            settings,
            document,
            target,
            &pending::import_reference(document.target(), document.cursor()),
            &mut keys,
            &mut seen,
        )?;
    }
    for result in document.results() {
        if !document.inspect_result(settings, result)? {
            continue;
        }
        if let (Some(target), Some(row)) = (&result.target, result.row_index) {
            visit(
                settings,
                document,
                target,
                &pending::import_reference(document.target(), row),
                &mut keys,
                &mut seen,
            )?;
        }
    }
    for (target, row) in document.receipt_targets() {
        visit(
            settings,
            document,
            target,
            &pending::import_reference(document.target(), row),
            &mut keys,
            &mut seen,
        )?;
    }
    Ok(keys)
}
fn visit(
    settings: &SettingsRoot,
    document: &ImportWork,
    target: &w::ApplicationPendingTarget,
    parent: &w::ApplicationContinuationTarget,
    keys: &mut BTreeSet<String>,
    seen: &mut BTreeSet<(String, String)>,
) -> Result<(), AppError> {
    if !seen.insert((target.key.clone(), target.operation.clone())) {
        return Ok(());
    }
    keys.insert(target.key.clone());
    let state = pending::load(
        settings,
        &pending::coordinate(&target.key)?,
        Some(&target.operation),
    )?;
    let op = state.pending(settings)?;
    if op.parent.as_ref() != Some(parent) {
        return Err(store::failure(
            settings,
            &document.target().operation,
            "retained effect plan has another owner",
        ));
    }
    for child in &op.children {
        // Even before target allocation, the ordinary planner has selected an exact key.
        keys.insert(child.key.clone());
        if let Some(target) = &child.target {
            let path = pending::work(settings, &pending::coordinate(&target.key)?);
            if document.full().is_none() && child.completed && !path.exists() {
                continue;
            }
            visit(
                settings,
                document,
                target,
                &pending::application_reference(&op.target),
                keys,
                seen,
            )?;
        }
    }
    Ok(())
}
