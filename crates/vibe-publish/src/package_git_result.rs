//! Retained historical result for the private package-Git fake owner chain.
//! No remote observation or effect is made while sealing or cold attaching.

use std::path::Path;

use serde::{Deserialize, Serialize};
use vibe_lifecycle::{LifecycleLease, SubmissionNamespace, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_approval::{ApprovalHome, LinkInspection};
use crate::package_git_operation::journal::{self, AtomicPushPending, ReadySeal, sha};
use crate::package_git_plan::RepositoryIdentity;
use crate::package_git_push::{self, PushInspection};
use crate::package_git_ready::{self, ReadyInspection, ReadyLimits};
use crate::package_git_repo_create::{self, CreateInspection};

const RESULT: &str = "result";
const FORMAT: &str = "vibevm.package-git-result/e2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResultInspection {
    AwaitingPush {
        run_id: String,
    },
    PartialRecovery {
        run_id: String,
        push_pending_sha256: String,
    },
    ReadyToSeal {
        run_id: String,
    },
    Completed {
        run_id: String,
        result_sha256: String,
    },
    Unavailable {
        run_id: Option<String>,
        reason: String,
    },
    RecoveryRequired {
        run_id: Option<String>,
        reason: String,
    },
}

fn recovery(run: Option<&str>, reason: impl Into<String>) -> ResultInspection {
    ResultInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn unavailable(run: Option<&str>, reason: impl Into<String>) -> ResultInspection {
    ResultInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultRow {
    format: String,
    run_id: String,
    namespace_id: String,
    scope_context: String,
    scope_target: String,
    scope_principal: String,
    key_sha256: String,
    lease_root_token: String,
    semantic_plan_sha256: String,
    c1_plan_sha256: String,
    request_sha256: String,
    ready_seal_sha256: String,
    approval_link_landed_sha256: String,
    repository: RepositoryIdentity,
    repo_create_pending_sha256: Option<String>,
    repo_create_landed_sha256: Option<String>,
    atomic_push_pending_sha256: String,
    atomic_push_landed_sha256: String,
    selected_tree_oid: String,
    selected_commit_oid: String,
    selected_tag_oid: String,
}

#[allow(clippy::too_many_arguments)]
fn complete_chain(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ReadyLimits,
    approval_id: &str,
) -> Result<ResultRow, ResultInspection> {
    let (run, ready_sha) = match package_git_ready::inspect_ready_seal_cold(
        lease,
        namespace.id(),
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
    ) {
        ReadyInspection::ReadyForApproval {
            run_id,
            seal_sha256,
        } => (run_id, seal_sha256),
        ReadyInspection::Unavailable { run_id, reason } => {
            return Err(unavailable(run_id.as_deref(), reason));
        }
        other => {
            return Err(recovery(
                None,
                format!("C.1 Ready chain unavailable: {other:?}"),
            ));
        }
    };
    if !matches!(home.inspect_link_cold(
        lease, namespace, scope, key, package_source, local_bare_source, git_binary, limits,
        approval_id,
    ), LinkInspection::Landed { run_id: linked, .. } if linked == run)
    {
        return Err(recovery(
            Some(&run),
            "approval-link Landed cold proof unavailable",
        ));
    }
    let push_landed_sha = match package_git_push::inspect_cold(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
    ) {
        PushInspection::Landed {
            run_id: pushed,
            landed_sha256,
        } if pushed == run => landed_sha256,
        PushInspection::NotStarted { run_id: pending } if pending == run => {
            return Err(ResultInspection::AwaitingPush { run_id: run });
        }
        PushInspection::PendingRecoveryRequired {
            run_id: pending,
            pending_sha256,
        } if pending == run => {
            return Err(ResultInspection::PartialRecovery {
                run_id: run,
                push_pending_sha256: pending_sha256,
            });
        }
        PushInspection::Unavailable { run_id, reason } => {
            return Err(unavailable(run_id.as_deref(), reason));
        }
        other => {
            return Err(recovery(
                Some(&run),
                format!("atomic-push Landed unavailable: {other:?}"),
            ));
        }
    };
    let bytes = |suffix: &str| -> Result<Vec<u8>, ResultInspection> {
        journal::read_bytes(lease, &run, suffix)
            .map_err(|e| recovery(Some(&run), e.to_string()))?
            .ok_or_else(|| recovery(Some(&run), format!("{suffix} receipt disappeared")))
    };
    let ready_bytes = bytes("ready")?;
    let seal: ReadySeal =
        serde_json::from_slice(&ready_bytes).map_err(|e| recovery(Some(&run), e.to_string()))?;
    let link_bytes = bytes("approval-link")?;
    let push_pending = bytes("atomic-push-pending")?;
    let push_landed = bytes("atomic-push")?;
    let push_row: AtomicPushPending =
        serde_json::from_slice(&push_pending).map_err(|e| recovery(Some(&run), e.to_string()))?;
    let (create_pending, create_landed, create_landed_sha) = match &push_row.repository {
        RepositoryIdentity::Hosted { id, .. } => {
            let landed_sha = match package_git_repo_create::inspect_cold(
                home,
                lease,
                namespace,
                scope,
                key,
                package_source,
                local_bare_source,
                git_binary,
                limits,
                approval_id,
            ) {
                CreateInspection::Landed {
                    run_id: created,
                    repository_id,
                    landed_sha256,
                } if created == run && repository_id == *id && *id > 0 => landed_sha256,
                other => {
                    return Err(recovery(
                        Some(&run),
                        format!("repo-create Landed unavailable: {other:?}"),
                    ));
                }
            };
            (
                Some(bytes("repo-create-pending")?),
                Some(bytes("repo-create")?),
                Some(landed_sha),
            )
        }
        RepositoryIdentity::Direct { .. } => {
            if !matches!(
                journal::read_bytes(lease, &run, "repo-create-pending"),
                Ok(None)
            ) || !matches!(journal::read_bytes(lease, &run, "repo-create"), Ok(None))
            {
                return Err(recovery(
                    Some(&run),
                    "DirectGitExisting has foreign create records",
                ));
            }
            (None, None, None)
        }
    };
    if sha(&ready_bytes) != ready_sha
        || create_landed.as_ref().map(|bytes| sha(bytes)) != create_landed_sha
        || sha(&push_landed) != push_landed_sha
        || serde_json::to_vec(&seal).ok().as_deref() != Some(ready_bytes.as_slice())
    {
        return Err(recovery(
            Some(&run),
            "exact chain changed after owner cold proof",
        ));
    }
    let row = ResultRow {
        format: FORMAT.into(),
        run_id: run.clone(),
        namespace_id: namespace.id().into(),
        scope_context: scope.context.clone(),
        scope_target: scope.target.clone(),
        scope_principal: scope.principal.clone(),
        key_sha256: seal.key_sha256.clone(),
        lease_root_token: seal.lease_root_token.clone(),
        semantic_plan_sha256: seal.semantic_plan_sha256.clone(),
        c1_plan_sha256: seal.c1_plan_sha256.clone(),
        request_sha256: seal.request_sha256.clone(),
        ready_seal_sha256: ready_sha,
        approval_link_landed_sha256: sha(&link_bytes),
        repository: push_row.repository,
        repo_create_pending_sha256: create_pending.as_ref().map(|bytes| sha(bytes)),
        repo_create_landed_sha256: create_landed_sha,
        atomic_push_pending_sha256: sha(&push_pending),
        atomic_push_landed_sha256: push_landed_sha,
        selected_tree_oid: seal.selected_tree_oid,
        selected_commit_oid: seal.selected_commit_oid,
        selected_tag_oid: seal.selected_tag_oid,
    };
    let unchanged_optional = |suffix: &str, expected: &Option<Vec<u8>>| matches!(journal::read_bytes(lease, &run, suffix), Ok(now) if &now == expected);
    if !matches!(package_git_ready::inspect_ready_seal_cold(
        lease, namespace.id(), scope, key, package_source, local_bare_source, git_binary, limits,
    ), ReadyInspection::ReadyForApproval { run_id: ref same, seal_sha256: ref hash }
        if same == &run && hash == &row.ready_seal_sha256)
        || !matches!(journal::read_bytes(lease, &run, "approval-link"), Ok(Some(ref now)) if now == &link_bytes)
        || !unchanged_optional("repo-create-pending", &create_pending)
        || !unchanged_optional("repo-create", &create_landed)
        || !matches!(journal::read_bytes(lease, &run, "atomic-push-pending"), Ok(Some(ref now)) if now == &push_pending)
        || !matches!(journal::read_bytes(lease, &run, "atomic-push"), Ok(Some(ref now)) if now == &push_landed)
    {
        return Err(recovery(
            Some(&run),
            "retained chain changed during terminal proof",
        ));
    }
    Ok(row)
}

/// No remote call. A later ref move does not rewrite the historical result.
#[allow(clippy::too_many_arguments)]
pub(crate) fn inspect_cold(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ReadyLimits,
    approval_id: &str,
) -> ResultInspection {
    let result = match complete_chain(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
    ) {
        Ok(value) => value,
        Err(ResultInspection::PartialRecovery {
            run_id,
            push_pending_sha256,
        }) => {
            return match journal::read_bytes(lease, &run_id, RESULT) {
                Ok(None) => ResultInspection::PartialRecovery {
                    run_id,
                    push_pending_sha256,
                },
                _ => recovery(Some(&run_id), "terminal result exists over partial push"),
            };
        }
        Err(ResultInspection::AwaitingPush { run_id }) => {
            return match journal::read_bytes(lease, &run_id, RESULT) {
                Ok(None) => ResultInspection::AwaitingPush { run_id },
                _ => recovery(Some(&run_id), "terminal result exists before push"),
            };
        }
        Err(status) => return status,
    };
    let run = result.run_id.as_str();
    let bytes = match journal::read_bytes(lease, run, RESULT) {
        Ok(Some(value)) => value,
        Ok(None) => return ResultInspection::ReadyToSeal { run_id: run.into() },
        Err(e) => return recovery(Some(run), e.to_string()),
    };
    let recorded: ResultRow = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), e.to_string()),
    };
    if recorded != result || serde_json::to_vec(&recorded).ok().as_deref() != Some(bytes.as_slice())
    {
        return recovery(
            Some(run),
            "terminal result differs from complete retained chain",
        );
    }
    if !matches!(journal::read_bytes(lease, run, RESULT), Ok(Some(ref now)) if now == &bytes) {
        return recovery(Some(run), "terminal result changed during cold attach");
    }
    ResultInspection::Completed {
        run_id: run.into(),
        result_sha256: sha(&bytes),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn seal_result(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ReadyLimits,
    approval_id: &str,
) -> ResultInspection {
    seal_inner(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn seal_inner(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ReadyLimits,
    approval_id: &str,
    stop_after_publish: bool,
) -> ResultInspection {
    let initial = inspect_cold(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
    );
    let ResultInspection::ReadyToSeal { run_id } = initial else {
        return initial;
    };
    let row = match complete_chain(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
    ) {
        Ok(value) if value.run_id == run_id => value,
        Ok(_) => return recovery(Some(&run_id), "run changed before result publication"),
        Err(status) => return status,
    };
    if !matches!(journal::read_bytes(lease, &run_id, RESULT), Ok(None)) {
        return recovery(
            Some(&run_id),
            "result appeared before create-new publication",
        );
    }
    if let Err(e) = journal::publish(lease, &run_id, RESULT, &row) {
        return recovery(
            Some(&run_id),
            format!("terminal result publication uncertain: {e:#}"),
        );
    }
    if stop_after_publish {
        return recovery(Some(&run_id), "injected lost result publication response");
    }
    inspect_cold(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
    )
}

#[cfg(test)]
mod tests;
