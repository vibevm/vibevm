//! Inspect terminal work before reconstructing execution and restore exact full scope.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION");
use super::{pending, store};
use crate::{AppError, ImportScope, ImportSecrets, SettingsRoot};
use vibe_wire::generated::shared as w;
#[derive(Debug, Clone)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION")]
pub struct ImportResumeInfo {
    pub context: w::ApplicationImportContext,
    pub full: bool,
    pub cleanup_only: bool,
    /// Durably settled prefix, including refusals; waiting input is not a completed row.
    pub results: Vec<w::ApplicationPendingImportResult>,
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION"
)]
pub fn inspect_import_resume(
    settings: &SettingsRoot,
    id: &str,
) -> Result<ImportResumeInfo, AppError> {
    let document = super::load(settings, id)?;
    let mut cleanup_only = document.phase() != &w::ApplicationImportPhase::Running
        || document.cursor() as usize == document.root_count();
    if !cleanup_only
        && document.cursor() as usize + 1 == document.root_count()
        && let Some(target) = document.current()
    {
        let state = pending::load(
            settings,
            &pending::coordinate(&target.key)?,
            Some(&target.operation),
        )?;
        cleanup_only = state.pending(settings)?.phase
            == w::ApplicationPendingOperationPhase::Complete
            || super::super::update::retirement::cleanup_only(state.pending(settings)?);
    }
    Ok(ImportResumeInfo {
        context: document.context().clone(),
        results: document.results().to_vec(),
        full: document.full().is_some(),
        cleanup_only,
    })
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION"
)]
pub(crate) fn scope_from_state<'a>(
    settings: &SettingsRoot,
    state: &crate::steps::StepsFile,
    secrets: &'a ImportSecrets,
) -> Result<Option<ImportScope<'a>>, AppError> {
    let Some(target) = pending::full_import_owner(settings, state)? else {
        return Ok(None);
    };
    let document = super::load(settings, &target.operation)?;
    let full = document.full().ok_or_else(|| {
        store::failure(settings, &target.operation, "full import scope disappeared")
    })?;
    ImportScope::restored(
        full.target.clone(),
        full.cohort.roots.clone(),
        full.cohort.bindings.clone(),
        full.cohort.secret_requirements.clone(),
        secrets,
        &state.package.key(),
    )
    .map(Some)
}
