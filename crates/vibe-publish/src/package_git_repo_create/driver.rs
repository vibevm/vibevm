//! Private create owner. No live port or public invocation route is installed.

use super::*;
use crate::package_git_plan::{CreateDecision, classify_create};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteSnapshot {
    pub(crate) repository: RepositoryObservation,
    pub(crate) refs: RefSnapshot,
}

/// Transport failures carry no provider text, URLs or credential material.
/// Every failure is uncertain and gives no automatic retry authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreatePortError {
    Uncertain,
}

pub(crate) trait CreatePort {
    /// Observe exactly this hosted repository and the C.1-selected tag ref.
    fn observe(
        &mut self,
        repository: &HostedRepository,
        version_tag_ref: &str,
    ) -> Result<RemoteSnapshot, CreatePortError>;

    /// Return a stable identity only from an actually successful create
    /// response. Discovering an existing repository is never create success.
    fn create(
        &mut self,
        repository: &HostedRepository,
        creation: &CreationOptions,
    ) -> Result<RepositoryIdentity, CreatePortError>;
}

pub(crate) struct CreateRun<'a> {
    pub(crate) home: &'a ApprovalHome,
    pub(crate) lease: &'a LifecycleLease,
    pub(crate) namespace: &'a SubmissionNamespace,
    pub(crate) scope: &'a SubmissionScope,
    pub(crate) key: &'a str,
    pub(crate) package_source: &'a Project,
    pub(crate) git_binary: &'a Path,
    pub(crate) profile: ReadyProfile<'a>,
    pub(crate) approval_id: &'a str,
}

pub(crate) fn drive(input: &CreateRun<'_>, port: &mut impl CreatePort) -> CreateInspection {
    let proof = match proved_profile(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        Some(input.package_source),
        input.git_binary,
        input.profile,
        input.approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run = proof.run_id.as_str();
    let (repository, creation) = match create_shape(&proof) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let pending = journal::read_bytes(input.lease, run, PENDING);
    let landed = journal::read_bytes(input.lease, run, LANDED);
    if !matches!((&pending, &landed), (Ok(None), Ok(None))) {
        return inspect_cold_profile(
            input.home,
            input.lease,
            input.namespace,
            input.scope,
            input.key,
            Some(input.package_source),
            input.git_binary,
            input.profile,
            input.approval_id,
        );
    }
    let baseline = RemoteSnapshot {
        repository: proof.plan.before_repo.clone(),
        refs: proof.plan.before_refs.clone(),
    };
    let first = match port.observe(repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "first pre-observation uncertain"),
    };
    let second = match port.observe(repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "second pre-observation uncertain"),
    };
    if first != baseline || second != baseline {
        return recovery(
            Some(run),
            "pre-effect repository/refs differ from approved baseline",
        );
    }
    let pending = expected_pending(&proof, repository, creation);
    let pending_bytes = match journal::publish(input.lease, run, PENDING, &pending) {
        Ok(value) => value,
        Err(e) => {
            return recovery(
                Some(run),
                format!("create Pending publication uncertain: {e:#}"),
            );
        }
    };
    if !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "create Pending changed before dispatch");
    }
    let immediate = match port.observe(repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "last pre-observation uncertain"),
    };
    if immediate != baseline {
        return recovery(
            Some(run),
            "remote changed after Pending before create; no dispatch",
        );
    }
    let reproved = match proved_profile(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        Some(input.package_source),
        input.git_binary,
        input.profile,
        input.approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    if reproved != proof
        || !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(
            Some(run),
            "local Ready/approval/Pending changed before create",
        );
    }
    let returned = match port.create(repository, creation) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "create callback uncertain; no retry"),
    };
    let after = match port.observe(repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "first post-observation uncertain"),
    };
    let after_again = match port.observe(repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "second post-observation uncertain"),
    };
    if after != after_again || after.refs != empty_refs() {
        return recovery(
            Some(run),
            "created repository/refs changed during post-observation",
        );
    }
    if !matches!(
        classify_create(&proof.plan, true, Some(&returned), &after.repository),
        Ok(CreateDecision::Landed(_))
    ) {
        return recovery(
            Some(run),
            "create response and independent repository identity differ",
        );
    }
    if !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "create Pending changed before Landed");
    }
    let landed = RepoCreateLanded {
        format: REPO_CREATE_FORMAT.into(),
        run_id: run.into(),
        pending_sha256: sha(&pending_bytes),
        returned_identity: returned,
        observed_repo: after.repository,
        observed_refs: after.refs,
    };
    if let Err(e) = journal::publish(input.lease, run, LANDED, &landed) {
        return recovery(Some(run), format!("create Landed uncertain: {e:#}"));
    }
    inspect_cold_profile(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        Some(input.package_source),
        input.git_binary,
        input.profile,
        input.approval_id,
    )
}
