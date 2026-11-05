//! Same-cohort reuse is proved by retained exact attempts and publication identity.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-INSTALLED-REUSE");
use super::{pending, store};
use crate::{AppError, Index, SettingsRoot};
use vibe_wire::generated::shared as w;
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-INSTALLED-REUSE"
)]
pub(crate) fn completed_binding(
    settings: &SettingsRoot,
    target: &w::ApplicationImportTarget,
    key: &str,
    index: &Index,
    fingerprint: &str,
) -> Result<bool, AppError> {
    let owner = super::load(settings, &target.operation)?;
    if owner.target() != target || owner.full().is_none() {
        return Err(store::failure(
            settings,
            &target.operation,
            "binding scope is not this exact full cohort",
        ));
    }
    if let Some(full) = owner.full()
        && super::receipts::completed(settings, full, key, index, fingerprint)?
    {
        return Ok(true);
    }
    let mut roots = owner
        .results()
        .iter()
        .filter(|r| r.action == w::ApplicationPendingImportResultAction::Installed)
        .filter_map(|r| r.target.as_ref().zip(r.row_index))
        .map(|(t, row)| (t.clone(), row))
        .collect::<Vec<_>>();
    if let Some(current) = owner.current() {
        roots.push((current.clone(), owner.cursor()));
    }
    let mut seen = std::collections::BTreeSet::new();
    for (root, row) in roots {
        if walk(
            settings,
            index,
            &root,
            &pending::import_reference(target, row),
            key,
            fingerprint,
            &mut seen,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-INSTALLED-REUSE"
)]
fn walk(
    settings: &SettingsRoot,
    index: &Index,
    target: &w::ApplicationPendingTarget,
    parent: &w::ApplicationContinuationTarget,
    wanted: &str,
    fingerprint: &str,
    seen: &mut std::collections::BTreeSet<(String, String)>,
) -> Result<bool, AppError> {
    if !seen.insert((target.key.clone(), target.operation.clone())) {
        return Ok(false);
    }
    let state = pending::load(
        settings,
        &pending::coordinate(&target.key)?,
        Some(&target.operation),
    )?;
    let op = state.pending(settings)?;
    if op.parent.as_ref() != Some(parent) || !pending::matches_import_scope(&state, fingerprint) {
        return Err(store::failure(
            settings,
            &target.operation,
            "completed binding lineage or immutable scope differs",
        ));
    }
    if op.phase == w::ApplicationPendingOperationPhase::Complete {
        let record = index.installations.get(&target.key).ok_or_else(|| {
            store::failure(
                settings,
                &target.operation,
                "completed installation disappeared",
            )
        })?;
        let actual = pending::record_identity(record).map_err(|_| {
            store::failure(
                settings,
                &target.operation,
                "cannot identify completed installation",
            )
        })?;
        if op.publication_record.as_deref() != Some(actual.as_str()) || record.pending.is_some() {
            return Err(store::failure(
                settings,
                &target.operation,
                "completed installation differs from exact published result",
            ));
        }
        if target.key == wanted {
            return Ok(true);
        }
    }
    for child in &op.children {
        if child.completed
            && let Some(next) = &child.target
        {
            let record = index.installations.get(&next.key).ok_or_else(|| {
                store::failure(
                    settings,
                    &target.operation,
                    "completed dependency disappeared",
                )
            })?;
            let actual = pending::record_identity(record).map_err(|_| {
                store::failure(
                    settings,
                    &target.operation,
                    "cannot identify completed dependency",
                )
            })?;
            if child.record_identity.as_deref() != Some(actual.as_str()) {
                return Err(store::failure(
                    settings,
                    &target.operation,
                    "dependency publication identity changed",
                ));
            }
            if walk(
                settings,
                index,
                next,
                &pending::application_reference(&op.target),
                wanted,
                fingerprint,
                seen,
            )? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
