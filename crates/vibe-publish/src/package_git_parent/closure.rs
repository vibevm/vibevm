//! Bounded retained parent graph and no-external-object-source proof.
use super::git::ReadObjectDigest;
use super::{
    CLOSURE_DOMAIN, ParentInspection, ParentLimits, main_oid, tag_oid, target_identity,
    unavailable, uncertain,
};
use crate::package_git_operation::journal::{self, ParentPending, ParentReceipt, sha};
use crate::package_git_operation::{self, PendingProof};
use crate::package_git_plan::{PackageGitPlan, StagedCommit, TagRef};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_tree::git::LocalGit;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::{ChildKind, Project};

pub(super) struct Held {
    pub(super) run_id: String,
    pub(super) proof: PendingProof,
    pub(super) tree_bytes: Vec<u8>,
    pub(super) provenance_bytes: Vec<u8>,
    pub(super) objects: Project,
    pub(super) private_prefix: String,
}

pub(super) fn held(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    binary: &Path,
    limits: ParentLimits,
    ready: bool,
) -> Result<Held, ParentInspection> {
    let observed = if ready {
        package_git_provenance::inspect_no_gitlinks_ready_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            binary,
            limits.census,
        )
    } else {
        package_git_provenance::inspect_no_gitlinks_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            binary,
            limits.census,
        )
    };
    let run_id = match observed {
        ProvenanceInspection::NoGitlinksRetainedButNotReady { run_id, .. } => run_id,
        ProvenanceInspection::NotReady { run_id } => {
            return Err(ParentInspection::NotReady { run_id });
        }
        ProvenanceInspection::NeedsCommitTreeProof { run_id } => {
            return Err(unavailable(
                Some(&run_id),
                "populated Gitlinks need separate commit-tree byte proof",
            ));
        }
        ProvenanceInspection::Unavailable { run_id, reason } => {
            return Err(ParentInspection::Unavailable { run_id, reason });
        }
        ProvenanceInspection::Uncertain { run_id, reason } => {
            return Err(ParentInspection::Uncertain { run_id, reason });
        }
    };
    let (status, proof) = if ready {
        package_git_operation::ready_proof_cold(lease, namespace, scope, key, package_source)
    } else {
        package_git_operation::pending_proof_cold(lease, namespace, scope, key, package_source)
    }
    .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
    let Some(proof) = proof else {
        return Err(uncertain(
            Some(&run_id),
            format!("C.1 Pending proof changed: {status:?}"),
        ));
    };
    if proof.run_id != run_id {
        return Err(uncertain(
            Some(&run_id),
            "C.1 run changed during parent admission",
        ));
    }
    let Some(main) = main_oid(&proof.plan) else {
        return Err(unavailable(Some(&run_id), "existing main baseline absent"));
    };
    if target_identity(&proof.plan).is_none() {
        return Err(unavailable(
            Some(&run_id),
            "local-bare parent import requires DirectGitExisting",
        ));
    }
    match &proof.plan.staged.commit {
        StagedCommit::New {
            parent_oid: Some(parent),
            ..
        } if parent == main => {}
        StagedCommit::ReusedMain { oid } if oid == main => {}
        _ => {
            return Err(unavailable(
                Some(&run_id),
                "plan does not select exact existing main as parent/reused commit",
            ));
        }
    }
    let tree_bytes = journal::read_bytes(lease, &run_id, "tree")
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved tree receipt vanished"))?;
    let provenance_bytes = journal::read_bytes(lease, &run_id, "provenance")
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved provenance Landed vanished"))?;
    let objects = journal::objects(lease, &run_id, false)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "retained Git object root vanished"))?;
    LocalGit::new(binary, &objects, proof.plan.oid_hex_len)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
    Ok(Held {
        private_prefix: format!("refs/package-git-parent/{run_id}"),
        run_id,
        proof,
        tree_bytes,
        provenance_bytes,
        objects,
    })
}

pub(super) fn pending(
    held: &Held,
    source_identity: &str,
    source_token: &str,
) -> Result<ParentPending> {
    Ok(ParentPending {
        format: journal::PARENT_PENDING_FORMAT.into(),
        run_id: held.run_id.clone(),
        semantic_plan_sha256: held.proof.plan.digest()?,
        intent_sha256: held.proof.intent_sha256.clone(),
        payload_manifest_sha256: held.proof.payload_manifest_sha256.clone(),
        tree_receipt_sha256: sha(&held.tree_bytes),
        provenance_receipt_sha256: sha(&held.provenance_bytes),
        object_root_token: held.objects.root_correlation_token()?,
        local_bare_identity_sha256: source_identity.into(),
        local_bare_root_token: source_token.into(),
        private_prefix: held.private_prefix.clone(),
        observed_refs: held.proof.plan.before_refs.clone(),
    })
}

pub(super) fn receipt(
    row: &ParentPending,
    pending_bytes: &[u8],
    plan: &PackageGitPlan,
    closure: &Closure,
) -> Result<ParentReceipt> {
    Ok(ParentReceipt {
        format: journal::PARENT_FORMAT.into(),
        pending_sha256: sha(pending_bytes),
        run_id: row.run_id.clone(),
        semantic_plan_sha256: row.semantic_plan_sha256.clone(),
        intent_sha256: row.intent_sha256.clone(),
        payload_manifest_sha256: row.payload_manifest_sha256.clone(),
        tree_receipt_sha256: row.tree_receipt_sha256.clone(),
        provenance_receipt_sha256: row.provenance_receipt_sha256.clone(),
        object_root_token: row.object_root_token.clone(),
        local_bare_identity_sha256: row.local_bare_identity_sha256.clone(),
        local_bare_root_token: row.local_bare_root_token.clone(),
        private_prefix: row.private_prefix.clone(),
        observed_refs: row.observed_refs.clone(),
        parent_oid: main_oid(plan).context("main absent")?.into(),
        parent_tree_oid: closure.tree_oid.clone(),
        closure_sha256: closure.sha256.clone(),
        closure_objects: closure.objects,
        closure_bytes: closure.bytes,
    })
}

pub(super) fn recheck_prior(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    binary: &Path,
    limits: ParentLimits,
    run_id: &str,
    ready: bool,
) -> Result<()> {
    let observed = if ready {
        package_git_provenance::inspect_no_gitlinks_ready_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            binary,
            limits.census,
        )
    } else {
        package_git_provenance::inspect_no_gitlinks_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            binary,
            limits.census,
        )
    };
    match observed {
        ProvenanceInspection::NoGitlinksRetainedButNotReady {
            run_id: observed, ..
        } if observed == run_id => Ok(()),
        other => {
            bail!("C.1/payload/tree/no-Gitlinks proof changed during parent closure: {other:?}")
        }
    }
}

pub(super) fn no_external_object_sources(objects: &Project, limits: ParentLimits) -> Result<()> {
    let root = objects.root_dir()?;
    if objects.child_kind_nofollow(&root, "shallow")?.is_some() {
        bail!("retained Git repository is shallow");
    }
    if let Some(info) = objects.dir_if_present(&["objects", "info"])?
        && objects.child_kind_nofollow(&info, "alternates")?.is_some()
    {
        bail!("retained Git objects depend on an alternate object store");
    }
    if let Some(pack) = objects.dir_if_present(&["objects", "pack"])? {
        for name in objects.child_names_bounded(&pack, limits.max_pack_entries)? {
            if name.ends_with(".promisor") {
                bail!("retained Git pack is promisor-backed");
            }
            if objects.child_kind_nofollow(&pack, &name)? != Some(ChildKind::RegularFile) {
                bail!("retained Git pack entry is not a regular file");
            }
        }
    }
    let config = objects
        .read_file_bounded_in(&root, "config", limits.max_config_bytes)?
        .context("retained Git config absent")?;
    let lower = String::from_utf8(config)?.to_ascii_lowercase();
    if lower.contains("promisor") || lower.contains("partialclone") {
        bail!("retained Git config names a promisor/partial clone");
    }
    Ok(())
}

pub(super) fn exact_private_refs(
    git: &LocalGit<'_>,
    plan: &PackageGitPlan,
    prefix: &str,
    cap: usize,
) -> Result<()> {
    let rows = git.private_refs(prefix, cap)?;
    let mut expected = vec![(
        format!("{prefix}/main"),
        main_oid(plan).context("main absent")?.to_owned(),
    )];
    if let Some(tag) = tag_oid(plan) {
        expected.push((format!("{prefix}/tag"), tag.to_owned()));
    }
    let mut rows = rows;
    rows.sort();
    expected.sort();
    if rows != expected {
        bail!("retained private parent refs differ from approved raw baseline");
    }
    if let TagRef::Present {
        peeled_commit_oid, ..
    } = &plan.before_refs.version_tag
    {
        let value = git.output(
            &["rev-parse", &format!("{prefix}/tag^{{commit}}")],
            &[],
            128,
        )?;
        if value != format!("{peeled_commit_oid}\n").into_bytes() {
            bail!("retained private tag peels to a different commit");
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Closure {
    pub(super) sha256: String,
    pub(super) objects: usize,
    pub(super) bytes: u64,
    pub(super) tree_oid: String,
}

fn frame(hash: &mut Sha256, tag: &[u8], data: &[u8]) {
    hash.update((tag.len() as u64).to_be_bytes());
    hash.update(tag);
    hash.update((data.len() as u64).to_be_bytes());
    hash.update(data);
}

pub(super) fn verify(
    git: &LocalGit<'_>,
    objects: &Project,
    plan: &PackageGitPlan,
    limits: ParentLimits,
) -> Result<Closure> {
    no_external_object_sources(objects, limits)?;
    let parent = main_oid(plan).context("main absent")?;
    let mut oids: BTreeSet<String> = git
        .reachable_oids(
            parent,
            limits.max_objects,
            limits.max_reachable_output_bytes,
        )?
        .into_iter()
        .collect();
    if let Some(tag) = tag_oid(plan) {
        oids.insert(tag.to_owned());
    }
    if oids.len() > limits.max_objects {
        bail!("parent closure object count exceeds host-selected cap");
    }
    let mut digest = Sha256::new();
    digest.update(CLOSURE_DOMAIN);
    let mut total = 0_u64;
    for oid in &oids {
        let row: ReadObjectDigest = git.object_digest_streamed(oid, limits.max_object_bytes)?;
        total = total
            .checked_add(row.size_bytes)
            .context("parent closure byte count overflow")?;
        if total > limits.max_total_bytes {
            bail!("parent closure exceeds host-selected aggregate byte cap");
        }
        frame(&mut digest, b"oid", row.oid.as_bytes());
        frame(&mut digest, b"kind", row.kind.as_bytes());
        frame(&mut digest, b"size", &row.size_bytes.to_be_bytes());
        frame(&mut digest, b"sha256", row.sha256.as_bytes());
        if oid == parent && row.kind != "commit" {
            bail!("retained main parent is not a commit");
        }
    }
    let tree_oid = git.commit_tree_oid(parent, limits.max_parent_commit_bytes)?;
    if matches!(plan.staged.commit, StagedCommit::ReusedMain { .. }) {
        if tree_oid != plan.staged.git_tree_oid {
            bail!("reused-main no-op tree differs from vaulted tree");
        }
        if !matches!(&plan.before_refs.version_tag, TagRef::Present { peeled_commit_oid, .. } if peeled_commit_oid == parent)
        {
            bail!("reused-main no-op tag does not peel to main");
        }
    }
    frame(&mut digest, b"count", &(oids.len() as u64).to_be_bytes());
    frame(&mut digest, b"total", &total.to_be_bytes());
    Ok(Closure {
        sha256: format!("sha256:{:x}", digest.finalize()),
        objects: oids.len(),
        bytes: total,
        tree_oid,
    })
}
