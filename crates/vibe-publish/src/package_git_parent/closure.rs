//! Bounded retained parent graph and no-external-object-source proof.
use super::git::ReadObjectDigest;
use super::{CLOSURE_DOMAIN, ParentLimits, main_oid, tag_oid};
use crate::package_git_plan::{PackageGitPlan, StagedCommit, TagRef};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_tree::git::LocalGit;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::{ChildKind, Project};

pub(super) fn recheck_prior(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    binary: &Path,
    limits: ParentLimits,
    run_id: &str,
) -> Result<()> {
    match package_git_provenance::inspect_no_gitlinks_cold(
        lease,
        namespace,
        scope,
        key,
        package_source,
        binary,
        limits.census,
    ) {
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
