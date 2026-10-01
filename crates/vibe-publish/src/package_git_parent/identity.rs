//! Exact selected parent and raw tag identities from the e2 plan.
use crate::package_git_plan::{PackageGitPlan, RawRef, RepositoryTarget, TagRef};

pub(super) fn main_oid(plan: &PackageGitPlan) -> Option<&str> {
    match &plan.before_refs.main {
        RawRef::Present { oid } => Some(oid),
        RawRef::Absent => None,
    }
}
pub(super) fn tag_oid(plan: &PackageGitPlan) -> Option<&str> {
    match &plan.before_refs.version_tag {
        TagRef::Present { raw_oid, .. } => Some(raw_oid),
        TagRef::Absent => None,
    }
}
pub(super) fn target_identity(plan: &PackageGitPlan) -> Option<&str> {
    match &plan.target {
        RepositoryTarget::DirectGitExisting {
            remote_identity_sha256,
        } => Some(remote_identity_sha256),
        _ => None,
    }
}
