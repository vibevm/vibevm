//! Exact held local-bare source reobservation for parent admission.
use super::{ParentInspection, ParentLimits, unavailable, uncertain};
use crate::package_git_parent_source::{self, ParentSourceObservation};
use crate::package_git_plan::PackageGitPlan;
use std::path::Path;
use vibe_safefs::Project;

pub(super) fn observed_source(
    source: &Project,
    binary: &Path,
    plan: &PackageGitPlan,
    limits: ParentLimits,
) -> Result<(String, String), ParentInspection> {
    match package_git_parent_source::observe_local_bare(source, binary, plan, limits.source) {
        ParentSourceObservation::Exact {
            repository_root_token,
            local_bare_identity_sha256,
            ..
        } => Ok((repository_root_token, local_bare_identity_sha256)),
        ParentSourceObservation::Unavailable(reason) => Err(unavailable(None, reason)),
        ParentSourceObservation::Drift(reason) => Err(uncertain(None, reason)),
    }
}
