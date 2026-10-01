//! Private atomic push owner; no live transport or public invocation route.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) repository: RepositoryObservation,
    pub(crate) refs: RefSnapshot,
}

pub(crate) struct PushRequest<'a> {
    pub(crate) objects: &'a Project,
    pub(crate) repository: &'a RepositoryIdentity,
    pub(crate) before_refs: &'a RefSnapshot,
    pub(crate) desired_refs: &'a RefSnapshot,
    pub(crate) version_tag_ref: &'a str,
    pub(crate) commit_oid: &'a str,
    pub(crate) tag_oid: &'a str,
}

/// Transport failures never retain provider text, URLs or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PushPortError {
    Uncertain,
}

pub(crate) trait AtomicPushPort {
    /// Observe only the exact proved repository and selected version tag.
    fn observe(
        &mut self,
        repository: &RepositoryIdentity,
        version_tag_ref: &str,
    ) -> Result<Snapshot, PushPortError>;

    /// Issue at most one --atomic push, with both exact raw leases and
    /// explicit retained commit/tag OID refspecs from request.objects.
    /// Never retry or fall back to sequential pushes. A successful receipt
    /// requires bounded strict porcelain with actual updates for both refs.
    fn atomic_push(&mut self, request: PushRequest<'_>) -> Result<PushReceipt, PushPortError>;
}

pub(crate) struct PushRun<'a> {
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

pub(crate) fn drive(input: &PushRun<'_>, port: &mut impl AtomicPushPort) -> PushInspection {
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
    let run = proof.run();
    let pending_before = journal::read_bytes(input.lease, run, PENDING);
    let landed_before = journal::read_bytes(input.lease, run, LANDED);
    if !matches!((&pending_before, &landed_before), (Ok(None), Ok(None))) {
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
    let baseline = Snapshot {
        repository: RepositoryObservation::Present(proof.repository.clone()),
        refs: proof.plan.before_refs.clone(),
    };
    let first = match port.observe(&proof.repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "first pre-observation uncertain"),
    };
    let second = match port.observe(&proof.repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "second pre-observation uncertain"),
    };
    if first != baseline || second != baseline {
        return recovery(
            Some(run),
            "raw repository/main/tag preconditions differ before Pending",
        );
    }
    let pending = expected_pending(&proof);
    let pending_bytes = match journal::publish(input.lease, run, PENDING, &pending) {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), format!("atomic-push Pending uncertain: {e:#}")),
    };
    if !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "atomic-push Pending changed before dispatch");
    }
    let immediate = match port.observe(&proof.repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "last pre-observation uncertain"),
    };
    if immediate != baseline {
        return recovery(
            Some(run),
            "raw refs changed after Pending before atomic push",
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
            "local Ready/approval/repo-create/Pending changed before push",
        );
    }
    let objects = match journal::objects(input.lease, run, false) {
        Ok(Some(value)) => value,
        _ => return recovery(Some(run), "retained object store unavailable before push"),
    };
    if !object_root_matches(input.lease, &proof, &objects) {
        return recovery(Some(run), "retained object root changed before push");
    }
    let request = PushRequest {
        objects: &objects,
        repository: &proof.repository,
        before_refs: &proof.plan.before_refs,
        desired_refs: &proof.desired_refs,
        version_tag_ref: &proof.plan.version_tag_ref,
        commit_oid: &proof.seal.selected_commit_oid,
        tag_oid: &proof.seal.selected_tag_oid,
    };
    let returned = port.atomic_push(request);
    if !object_root_matches(input.lease, &proof, &objects) {
        return recovery(Some(run), "retained object root changed during push");
    }
    let receipt = match returned {
        Ok(value) if exact_updates(&proof.plan, value) => value,
        Ok(_) => {
            return recovery(
                Some(run),
                "atomic-push callback did not prove both exact ref updates",
            );
        }
        Err(_) => return recovery(Some(run), "atomic-push callback uncertain; no retry"),
    };
    let after = match port.observe(&proof.repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "first post-observation uncertain"),
    };
    let after_again = match port.observe(&proof.repository, &proof.plan.version_tag_ref) {
        Ok(value) => value,
        Err(_) => return recovery(Some(run), "second post-observation uncertain"),
    };
    let desired = Snapshot {
        repository: RepositoryObservation::Present(proof.repository.clone()),
        refs: proof.desired_refs.clone(),
    };
    if after != desired || after_again != desired {
        return recovery(
            Some(run),
            "atomic push post-observation is not exact desired refs",
        );
    }
    if !matches!(journal::read_bytes(input.lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
    {
        return recovery(Some(run), "atomic-push Pending changed before Landed");
    }
    let landed = AtomicPushLanded {
        format: ATOMIC_PUSH_FORMAT.into(),
        run_id: run.into(),
        pending_sha256: sha(&pending_bytes),
        callback_exit_zero: true,
        main_update: receipt.main_update,
        tag_update: receipt.tag_update,
        observed_repository: after.repository,
        observed_refs: after.refs,
    };
    if let Err(e) = journal::publish(input.lease, run, LANDED, &landed) {
        return recovery(Some(run), format!("atomic-push Landed uncertain: {e:#}"));
    }
    let cold_profile = match input.profile {
        ReadyProfile::NoGitlinks { limits, .. } => ReadyProfile::NoGitlinks {
            limits,
            local_bare_source: None,
        },
        ReadyProfile::ParentlessPopulated { limits, .. } => ReadyProfile::ParentlessPopulated {
            limits,
            approved_gitdirs: None,
        },
    };
    inspect_cold_profile(
        input.home,
        input.lease,
        input.namespace,
        input.scope,
        input.key,
        None,
        input.git_binary,
        cold_profile,
        input.approval_id,
    )
}

/// Correlate the held handle and the journal-selected named root with Ready.
/// Repeated on either side of the callback; no cached root proof crosses it.
fn object_root_matches(lease: &LifecycleLease, proof: &Proved, objects: &Project) -> bool {
    let Ok(_) = objects.root_dir() else {
        return false;
    };
    let Ok(held) = objects.root_correlation_token() else {
        return false;
    };
    let Ok(Some(named)) = journal::objects(lease, proof.run(), false) else {
        return false;
    };
    held == proof.seal.object_root_token
        && matches!(named.root_correlation_token(), Ok(token) if token == held)
        && matches!(Project::open(objects.root_path()).and_then(|p| p.root_correlation_token()), Ok(token) if token == held)
}
