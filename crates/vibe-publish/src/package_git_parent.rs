//! Retained local-bare parent closure under package-Git C.1 Pending.
//! No hosted source, network, public push, C.1 Ready or parentless shortcut.

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_gitlink::CensusLimits;
use crate::package_git_operation::journal::{self, ParentPending, ParentReceipt, sha};
use crate::package_git_parent_source::SourceLimits;
use crate::package_git_plan::{PackageGitPlan, StagedCommit, TagRef};
use crate::package_git_tree::git::LocalGit;

mod closure;
mod identity;
mod source;
use identity::{main_oid, tag_oid, target_identity};
use source::observed_source;
mod git;
use closure::recheck_prior;
use closure::{
    Closure, Held, exact_private_refs, held, no_external_object_sources, pending, receipt,
    verify as verify_closure,
};

const CLOSURE_DOMAIN: &[u8] = b"vibevm/package-git-parent-closure/e1\0";

#[derive(Debug, Clone, Copy)]
pub(crate) struct ParentLimits {
    pub source: SourceLimits,
    pub census: CensusLimits,
    pub max_ref_output_bytes: usize,
    pub max_reachable_output_bytes: usize,
    pub max_objects: usize,
    pub max_object_bytes: u64,
    pub max_total_bytes: u64,
    pub max_parent_commit_bytes: usize,
    pub max_pack_entries: usize,
    pub max_config_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParentInspection {
    NotReady {
        run_id: Option<String>,
    },
    ClosureCompleteButNotReady {
        run_id: String,
        source_reobserved: bool,
        closure_sha256: String,
    },
    Unavailable {
        run_id: Option<String>,
        reason: String,
    },
    Uncertain {
        run_id: Option<String>,
        reason: String,
    },
}

fn uncertain(run: Option<&str>, reason: impl Into<String>) -> ParentInspection {
    ParentInspection::Uncertain {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

fn unavailable(run: Option<&str>, reason: impl Into<String>) -> ParentInspection {
    ParentInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

pub(crate) fn stage_local_parent_pending(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: &Project,
    binary: &Path,
    limits: ParentLimits,
) -> ParentInspection {
    stage_inner(
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
        || {},
    )
}

fn stage_inner(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: &Project,
    binary: &Path,
    limits: ParentLimits,
    after_fetch: impl FnOnce(),
) -> ParentInspection {
    let held = match held(
        lease,
        namespace,
        scope,
        key,
        Some(package_source),
        binary,
        limits,
        false,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run_id = held.run_id.clone();
    let prior_pending = journal::read_bytes(lease, &run_id, "parent-pending");
    let prior_landed = journal::read_bytes(lease, &run_id, "parent");
    match (prior_pending, prior_landed) {
        (Ok(None), Ok(None)) => {}
        (Ok(_), Ok(_)) => {
            return inspect_parent_cold(
                lease,
                namespace,
                scope,
                key,
                Some(package_source),
                Some(local_bare_source),
                binary,
                limits,
            );
        }
        (Err(error), _) | (_, Err(error)) => return uncertain(Some(&run_id), error.to_string()),
    }
    if limits.max_ref_output_bytes == 0
        || limits.max_reachable_output_bytes == 0
        || limits.max_objects == 0
        || limits.max_object_bytes == 0
        || limits.max_total_bytes == 0
        || limits.max_parent_commit_bytes == 0
        || limits.max_pack_entries == 0
        || limits.max_config_bytes == 0
    {
        return unavailable(
            Some(&run_id),
            "explicit parent closure limits must be nonzero",
        );
    }
    if let Err(error) = no_external_object_sources(local_bare_source, limits) {
        return unavailable(
            Some(&run_id),
            format!("local bare source is shallow/promisor/alternate: {error:#}"),
        );
    }
    let (source_token, source_identity) =
        match observed_source(local_bare_source, binary, &held.proof.plan, limits) {
            Ok(value) => value,
            Err(status) => return status,
        };
    let git = match LocalGit::new(binary, &held.objects, held.proof.plan.oid_hex_len) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if let Err(error) = no_external_object_sources(&held.objects, limits) {
        return uncertain(Some(&run_id), error.to_string());
    }
    let preflight = (|| -> Result<()> {
        if !git
            .private_refs(&held.private_prefix, limits.max_ref_output_bytes)?
            .is_empty()
        {
            bail!("unreceipted private parent refs already exist");
        }
        if git.object_present(main_oid(&held.proof.plan).context("main absent")?)?
            || tag_oid(&held.proof.plan).is_some_and(|oid| git.object_present(oid).unwrap_or(true))
        {
            bail!("unreceipted parent/tag object already occupies run store");
        }
        Ok(())
    })();
    if let Err(error) = preflight {
        return uncertain(Some(&run_id), error.to_string());
    }
    let pending_row = match pending(&held, &source_identity, &source_token) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let pending_bytes = match journal::publish(lease, &run_id, "parent-pending", &pending_row) {
        Ok(value) => value,
        Err(error) => {
            return uncertain(
                Some(&run_id),
                format!("parent Pending publication uncertain: {error:#}"),
            );
        }
    };
    // From here on, any failure leaves Pending and grants no automatic fetch retry.
    if let Err(error) = git.fetch_local_bare(
        local_bare_source.root_path(),
        tag_oid(&held.proof.plan).map(|_| held.proof.plan.version_tag_ref.as_str()),
        &held.private_prefix,
        limits.max_ref_output_bytes,
    ) {
        return uncertain(
            Some(&run_id),
            format!("local bare fetch may be partial: {error:#}"),
        );
    }
    after_fetch();
    if let Err(error) = no_external_object_sources(local_bare_source, limits) {
        return uncertain(
            Some(&run_id),
            format!("local bare source became shallow/promisor/alternate after fetch: {error:#}"),
        );
    }
    match observed_source(local_bare_source, binary, &held.proof.plan, limits) {
        Ok((token, identity)) if token == source_token && identity == source_identity => {}
        other => {
            return uncertain(
                Some(&run_id),
                format!("local bare changed after fetch: {other:?}"),
            );
        }
    }
    let verified = (|| -> Result<Closure> {
        exact_private_refs(
            &git,
            &held.proof.plan,
            &held.private_prefix,
            limits.max_ref_output_bytes,
        )?;
        verify_closure(&git, &held.objects, &held.proof.plan, limits)
    })();
    let closure = match verified {
        Ok(value) => value,
        Err(error) => {
            return uncertain(
                Some(&run_id),
                format!("fetched parent closure unproved: {error:#}"),
            );
        }
    };
    let landed = match receipt(&pending_row, &pending_bytes, &held.proof.plan, &closure) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if let Err(error) = recheck_prior(
        lease,
        namespace,
        scope,
        key,
        Some(package_source),
        binary,
        limits,
        &run_id,
        false,
    ) {
        return uncertain(Some(&run_id), error.to_string());
    }
    if let Err(error) = journal::publish(lease, &run_id, "parent", &landed) {
        return uncertain(
            Some(&run_id),
            format!("parent Landed publication uncertain: {error:#}"),
        );
    }
    inspect_parent_cold(
        lease,
        namespace,
        scope,
        key,
        Some(package_source),
        Some(local_bare_source),
        binary,
        limits,
    )
}

pub(crate) fn inspect_parent_cold(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ParentLimits,
) -> ParentInspection {
    inspect_parent_inner(
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
        false,
    )
}

pub(crate) fn inspect_parent_ready_cold(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ParentLimits,
) -> ParentInspection {
    inspect_parent_inner(
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
        true,
    )
}

fn inspect_parent_inner(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ParentLimits,
    ready: bool,
) -> ParentInspection {
    let held = match held(
        lease,
        namespace,
        scope,
        key,
        package_source,
        binary,
        limits,
        ready,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run_id = held.run_id.clone();
    let pending_bytes = match journal::read_bytes(lease, &run_id, "parent-pending") {
        Ok(Some(value)) => value,
        Ok(None) => {
            let landed = journal::read_bytes(lease, &run_id, "parent");
            let git = LocalGit::new(binary, &held.objects, held.proof.plan.oid_hex_len);
            return match (landed, git) {
                (Ok(None), Ok(git))
                    if git
                        .private_refs(&held.private_prefix, limits.max_ref_output_bytes)
                        .is_ok_and(|refs| refs.is_empty())
                        && git
                            .object_present(main_oid(&held.proof.plan).unwrap_or(""))
                            .is_ok_and(|present| !present) =>
                {
                    ParentInspection::NotReady {
                        run_id: Some(run_id),
                    }
                }
                _ => uncertain(
                    Some(&run_id),
                    "parent receipt/refs or object occupancy is uncertain",
                ),
            };
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let pending_row: ParentPending = match serde_json::from_slice(&pending_bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let expected_pending = match pending(
        &held,
        &pending_row.local_bare_identity_sha256,
        &pending_row.local_bare_root_token,
    ) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if pending_row != expected_pending
        || serde_json::to_vec(&pending_row).ok().as_deref() != Some(pending_bytes.as_slice())
        || target_identity(&held.proof.plan)
            != Some(pending_row.local_bare_identity_sha256.as_str())
    {
        return uncertain(
            Some(&run_id),
            "parent Pending differs from exact C.1/direct baseline",
        );
    }
    let landed_bytes = match journal::read_bytes(lease, &run_id, "parent") {
        Ok(Some(value)) => value,
        Ok(None) => {
            return uncertain(
                Some(&run_id),
                "parent fetch Pending has no independently verified Landed receipt",
            );
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let landed_row: ParentReceipt = match serde_json::from_slice(&landed_bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let git = match LocalGit::new(binary, &held.objects, held.proof.plan.oid_hex_len) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let verified = (|| -> Result<Closure> {
        exact_private_refs(
            &git,
            &held.proof.plan,
            &held.private_prefix,
            limits.max_ref_output_bytes,
        )?;
        verify_closure(&git, &held.objects, &held.proof.plan, limits)
    })();
    let closure = match verified {
        Ok(value) => value,
        Err(error) => {
            return uncertain(
                Some(&run_id),
                format!("retained parent closure differs: {error:#}"),
            );
        }
    };
    let expected_landed = match receipt(&pending_row, &pending_bytes, &held.proof.plan, &closure) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if landed_row != expected_landed
        || serde_json::to_vec(&landed_row).ok().as_deref() != Some(landed_bytes.as_slice())
    {
        return uncertain(
            Some(&run_id),
            "parent Landed differs from actual retained closure",
        );
    }
    if let Some(source) = local_bare_source {
        if let Err(error) = no_external_object_sources(source, limits) {
            return uncertain(
                Some(&run_id),
                format!("current local bare source is shallow/promisor/alternate: {error:#}"),
            );
        }
        match observed_source(source, binary, &held.proof.plan, limits) {
            Ok((token, identity))
                if token == pending_row.local_bare_root_token
                    && identity == pending_row.local_bare_identity_sha256 => {}
            other => {
                return uncertain(
                    Some(&run_id),
                    format!("current local bare differs from retained parent source: {other:?}"),
                );
            }
        }
    }
    if let Err(error) = recheck_prior(
        lease,
        namespace,
        scope,
        key,
        package_source,
        binary,
        limits,
        &run_id,
        ready,
    ) {
        return uncertain(Some(&run_id), error.to_string());
    }
    if !matches!(journal::read_bytes(lease, &run_id, "parent-pending"), Ok(Some(ref value)) if value == &pending_bytes)
        || !matches!(journal::read_bytes(lease, &run_id, "parent"), Ok(Some(ref value)) if value == &landed_bytes)
    {
        return uncertain(
            Some(&run_id),
            "parent receipts changed during cold verification",
        );
    }
    ParentInspection::ClosureCompleteButNotReady {
        run_id,
        source_reobserved: package_source.is_some() && local_bare_source.is_some(),
        closure_sha256: closure.sha256,
    }
}

#[cfg(test)]
#[path = "package_git_parent/tests.rs"]
mod tests;
