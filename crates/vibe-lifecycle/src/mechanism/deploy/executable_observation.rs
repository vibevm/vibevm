//! Shared finite executable continuity observation; never a PATH resolver.
use std::path::PathBuf;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableObservation {
    pub(crate) launcher: PathBuf,
    pub(crate) resolved: PathBuf,
    pub(crate) state: vibe_safefs::StableFileState,
    pub(crate) identity: String,
}
#[derive(Debug)]
pub(crate) enum ObservationError {
    Absent,
    Unavailable,
    Changed,
}
pub(crate) fn observe(path: &std::path::Path) -> Result<ExecutableObservation, ObservationError> {
    let absent = || ObservationError::Absent;
    let failed = || ObservationError::Unavailable;
    if !path.is_absolute() {
        return Err(failed());
    }
    let resolve = || {
        std::fs::canonicalize(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                absent()
            } else {
                failed()
            }
        })
    };
    // Resolve only the admitted launcher, never search PATH. Parent and leaf
    // links participate. The original launcher still goes to ProcessSpec.
    let resolved = resolve()?;
    let parent = resolved.parent().ok_or_else(failed)?;
    let name = resolved
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(failed)?;
    let project = vibe_safefs::Project::open(parent).map_err(|_| failed())?;
    let (state, identity) = project
        .stable_file_state_with_identity(name)
        .map_err(|_| failed())?
        .ok_or_else(absent)?;
    if resolve()? != resolved {
        return Err(ObservationError::Changed);
    }
    Ok(ExecutableObservation {
        launcher: path.into(),
        resolved,
        state,
        identity: identity.observation_correlation_token(),
    })
}
