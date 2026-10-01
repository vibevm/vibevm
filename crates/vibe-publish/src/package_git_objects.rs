//! Retained local commit and annotated tag for an e2 parentless package plan.
//! C.1 remains Pending; no remote repository or publication effect is invoked.

use std::path::Path;

use serde::{Deserialize, Serialize};
use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_gitlink::CensusLimits;
use crate::package_git_operation::journal::{self, ObjectReceipt, sha};
use crate::package_git_parent::{self, ParentInspection, ParentLimits};
use crate::package_git_provenance_populated::{self, AggregateInspection, ApprovedGitdir};
use crate::package_git_submodule::SubmoduleLimits;
use crate::package_git_tree::git::LocalGit;

mod proof;
use proof::{expected, held};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ObjectProvenance {
    NoGitlinks {
        landed_sha256: String,
    },
    Populated {
        pending_sha256: String,
        landed_sha256: String,
        flattened_submodules_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObjectInspection {
    NotReady {
        run_id: Option<String>,
    },
    ObjectsCompleteButNotReady {
        run_id: String,
        source_reobserved: bool,
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

fn uncertain(run: Option<&str>, reason: impl Into<String>) -> ObjectInspection {
    ObjectInspection::Uncertain {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

/// Write only absent commit/tag objects in the already staged, cold-proved
/// bare Git object root. A prior object without receipt is uncertain, not ours.
pub(crate) fn stage_parentless_objects_pending(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    git_binary: &Path,
    limits: CensusLimits,
) -> ObjectInspection {
    stage_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits,
        None,
        None,
        None,
        None,
    )
}

/// Parentless flattened output whose complete indexed Gitlinks have already
/// landed in separate retained byte stores. This still writes only at Pending.
pub(crate) fn stage_parentless_populated_objects_pending(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    approved: &[ApprovedGitdir<'_>],
    git_binary: &Path,
    limits: SubmoduleLimits,
) -> ObjectInspection {
    stage_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits.census,
        None,
        None,
        Some(limits),
        Some(approved),
    )
}

pub(crate) fn stage_existing_main_objects_pending(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: &Project,
    git_binary: &Path,
    limits: ParentLimits,
) -> ObjectInspection {
    stage_inner(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        git_binary,
        limits.census,
        Some(limits),
        Some(local_bare_source),
        None,
        None,
    )
}

fn stage_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    git_binary: &Path,
    limits: CensusLimits,
    parent_limits: Option<ParentLimits>,
    local_bare_source: Option<&Project>,
    populated_limits: Option<SubmoduleLimits>,
    approved: Option<&[ApprovedGitdir<'_>]>,
) -> ObjectInspection {
    let held = match held(
        lease,
        namespace_token,
        scope,
        key,
        Some(source),
        git_binary,
        limits,
        parent_limits,
        local_bare_source,
        populated_limits,
        approved,
        false,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    if parent_limits.is_some() && held.parent_oid.is_none() {
        return ObjectInspection::Unavailable {
            run_id: Some(held.run_id),
            reason: "existing-main object route selected a parentless plan".into(),
        };
    }
    let run_id = held.run_id.clone();
    match journal::read_bytes(lease, &run_id, "objects") {
        Ok(Some(_)) => {
            return inspect_inner(
                lease,
                namespace_token,
                scope,
                key,
                Some(source),
                git_binary,
                limits,
                parent_limits,
                local_bare_source,
                populated_limits,
                approved,
                false,
            );
        }
        Ok(None) => {}
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    }
    let git = match LocalGit::new(git_binary, &held.objects, held.proof.plan.oid_hex_len) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let planned = match expected(&held) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if let Some(parent_limits) = parent_limits {
        if !matches!(
            package_git_parent::inspect_parent_cold(
                lease,
                namespace_token,
                scope,
                key,
                Some(source),
                local_bare_source,
                git_binary,
                parent_limits
            ),
            ParentInspection::ClosureCompleteButNotReady { .. }
        ) {
            return uncertain(
                Some(&run_id),
                "existing-main closure changed before child object write",
            );
        }
        if !matches!(journal::read_bytes(lease, &run_id, "parent"), Ok(Some(ref bytes)) if Some(sha(bytes).as_str()) == held.parent_receipt_sha256.as_deref())
        {
            return uncertain(
                Some(&run_id),
                "parent receipt changed before child object write",
            );
        }
    }
    if let Some(populated_limits) = populated_limits {
        if !matches!(package_git_provenance_populated::inspect_complete_cold(
            lease, namespace_token, scope, key, Some(source), approved, git_binary,
            populated_limits, vibe_lifecycle::JournalAdmission::Pending,
        ), AggregateInspection::AggregateCompleteButNotReady { run_id: ref same, ref flattened_submodules_sha256, .. }
            if same == &run_id && flattened_submodules_sha256 == &held.proof.plan.flattened_submodules_sha256)
        {
            return uncertain(
                Some(&run_id),
                "complete populated Gitlink proof changed before object write",
            );
        }
    }
    let stage = (|| -> anyhow::Result<()> {
        if git.object_present(&planned.commit_oid)? || git.object_present(&planned.tag_oid)? {
            anyhow::bail!("unreceipted planned commit/tag object already exists; never restage");
        }
        if git.write_control_object("commit", &held.commit_bytes)? != planned.commit_oid {
            anyhow::bail!("Git wrote a commit OID different from e2 plan");
        }
        if git.verify_control_object("commit", &planned.commit_oid, &held.commit_bytes)?
            != planned.commit_sha256
        {
            anyhow::bail!("retained commit SHA-256 differs from e2 plan");
        }
        if git.write_control_object("tag", &held.tag_bytes)? != planned.tag_oid {
            anyhow::bail!("Git wrote a tag OID different from e2 plan");
        }
        if git.verify_control_object("tag", &planned.tag_oid, &held.tag_bytes)?
            != planned.tag_sha256
        {
            anyhow::bail!("retained tag SHA-256 differs from e2 plan");
        }
        Ok(())
    })();
    if let Err(error) = stage {
        return uncertain(
            Some(&run_id),
            format!("local commit/tag stage incomplete: {error:#}"),
        );
    }
    if let Some(populated_limits) = populated_limits {
        if !matches!(package_git_provenance_populated::inspect_complete_cold(
            lease, namespace_token, scope, key, Some(source), approved, git_binary,
            populated_limits, vibe_lifecycle::JournalAdmission::Pending,
        ), AggregateInspection::AggregateCompleteButNotReady { run_id: ref same, ref flattened_submodules_sha256, .. }
            if same == &run_id && flattened_submodules_sha256 == &held.proof.plan.flattened_submodules_sha256)
            || !matches!(journal::read_bytes(lease, &run_id, "gitlink-aggregate"), Ok(Some(ref bytes))
                if matches!(&held.provenance, ObjectProvenance::Populated { landed_sha256, .. }
                    if &sha(bytes) == landed_sha256))
        {
            return uncertain(
                Some(&run_id),
                "populated Gitlink proof changed after object write",
            );
        }
    }
    if let Err(error) = journal::publish(lease, &run_id, "objects", &planned) {
        return uncertain(
            Some(&run_id),
            format!("object receipt publication uncertain: {error:#}"),
        );
    }
    inspect_inner(
        lease,
        namespace_token,
        scope,
        key,
        Some(source),
        git_binary,
        limits,
        parent_limits,
        local_bare_source,
        populated_limits,
        approved,
        false,
    )
}

/// Read-only cold proof reopens both retained Git object contents. Historical
/// source-less attach reports its narrower witness explicitly.
pub(crate) fn inspect_objects_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
    limits: CensusLimits,
) -> ObjectInspection {
    inspect_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits,
        None,
        None,
        None,
        None,
        false,
    )
}

pub(crate) fn inspect_objects_ready_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
    limits: CensusLimits,
) -> ObjectInspection {
    inspect_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits,
        None,
        None,
        None,
        None,
        true,
    )
}

pub(crate) fn inspect_parentless_populated_objects_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    approved: Option<&[ApprovedGitdir<'_>]>,
    git_binary: &Path,
    limits: SubmoduleLimits,
) -> ObjectInspection {
    inspect_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits.census,
        None,
        None,
        Some(limits),
        approved,
        false,
    )
}

pub(crate) fn inspect_existing_main_objects_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ParentLimits,
) -> ObjectInspection {
    inspect_inner(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        git_binary,
        limits.census,
        Some(limits),
        local_bare_source,
        None,
        None,
        false,
    )
}

pub(crate) fn inspect_existing_main_objects_ready_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ParentLimits,
) -> ObjectInspection {
    inspect_inner(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        git_binary,
        limits.census,
        Some(limits),
        local_bare_source,
        None,
        None,
        true,
    )
}

fn inspect_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
    limits: CensusLimits,
    parent_limits: Option<ParentLimits>,
    local_bare_source: Option<&Project>,
    populated_limits: Option<SubmoduleLimits>,
    approved: Option<&[ApprovedGitdir<'_>]>,
    ready: bool,
) -> ObjectInspection {
    let held = match held(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits,
        parent_limits,
        local_bare_source,
        populated_limits,
        approved,
        ready,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    if parent_limits.is_some() && held.parent_oid.is_none() {
        return ObjectInspection::Unavailable {
            run_id: Some(held.run_id),
            reason: "existing-main cold route selected a parentless plan".into(),
        };
    }
    let run_id = held.run_id.clone();
    let bytes = match journal::read_bytes(lease, &run_id, "objects") {
        Ok(Some(value)) => value,
        Ok(None) => {
            let git = match LocalGit::new(git_binary, &held.objects, held.proof.plan.oid_hex_len) {
                Ok(value) => value,
                Err(error) => return uncertain(Some(&run_id), error.to_string()),
            };
            match (
                git.object_present(&held.proof.plan.staged.tag_target_commit_oid),
                held.proof
                    .plan
                    .staged
                    .annotated_tag_oid
                    .as_deref()
                    .map(|oid| git.object_present(oid)),
            ) {
                (Ok(false), Some(Ok(false))) => {
                    return ObjectInspection::NotReady {
                        run_id: Some(run_id),
                    };
                }
                _ => {
                    return uncertain(
                        Some(&run_id),
                        "planned Git object exists or occupancy uncertain without a receipt",
                    );
                }
            }
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let row: ObjectReceipt = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let expected = match expected(&held) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if row != expected || serde_json::to_vec(&row).ok().as_deref() != Some(bytes.as_slice()) {
        return uncertain(
            Some(&run_id),
            "object receipt differs from exact C.1/e2 plan witnesses",
        );
    }
    let git = match LocalGit::new(git_binary, &held.objects, held.proof.plan.oid_hex_len) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if git
        .verify_control_object("commit", &row.commit_oid, &held.commit_bytes)
        .ok()
        .as_deref()
        != Some(row.commit_sha256.as_str())
        || git
            .verify_control_object("tag", &row.tag_oid, &held.tag_bytes)
            .ok()
            .as_deref()
            != Some(row.tag_sha256.as_str())
    {
        return uncertain(
            Some(&run_id),
            "actual retained commit/tag type, size or bytes differ from e2 plan",
        );
    }
    if !matches!(journal::read_bytes(lease, &run_id, "objects"), Ok(Some(ref current)) if current == &bytes)
    {
        return uncertain(
            Some(&run_id),
            "object receipt changed during cold verification",
        );
    }
    ObjectInspection::ObjectsCompleteButNotReady {
        run_id,
        source_reobserved: source.is_some()
            && (held.parent_oid.is_none() || local_bare_source.is_some()),
    }
}

#[cfg(test)]
#[path = "package_git_objects/tests.rs"]
mod tests;
