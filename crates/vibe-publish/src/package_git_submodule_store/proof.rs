//! Two-pass read-only proof of one retained Gitlink receipt.
use super::*;

/// Return exact canonical records only after the source-owned cold inspector
/// has rehashed the retained byte store. A second inspection brackets reads.
pub(crate) fn proved_link_receipt(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    path: &str,
    package_source: Option<&Project>,
    approved_gitdir: Option<&Project>,
    binary: &Path,
    limits: SubmoduleLimits,
    admission: JournalAdmission,
) -> Result<ProvedLinkReceipt, LinkStoreInspection> {
    let first = inspect_one_cold(
        lease,
        namespace,
        scope,
        key,
        path,
        package_source,
        approved_gitdir,
        binary,
        limits,
        admission,
    );
    let LinkStoreInspection::ByteStoreCompleteButNotReady { run_id, .. } = first else {
        return Err(first);
    };
    let (pending_name, landed_name) = suffixes(&path_sha(path));
    let pending_bytes = journal::read_bytes(lease, &run_id, &pending_name)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved Gitlink Pending vanished"))?;
    let landed_bytes = journal::read_bytes(lease, &run_id, &landed_name)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved Gitlink Landed vanished"))?;
    let pending: GitlinkPending = serde_json::from_slice(&pending_bytes)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
    if pending.path != path
        || serde_json::to_vec(&pending).ok().as_deref() != Some(pending_bytes.as_slice())
    {
        return Err(uncertain(
            Some(&run_id),
            "Gitlink Pending changed after cold proof",
        ));
    }
    if !matches!(inspect_one_cold(lease, namespace, scope, key, path, package_source, approved_gitdir, binary, limits, admission), LinkStoreInspection::ByteStoreCompleteButNotReady { run_id: ref again, .. } if again == &run_id)
        || !matches!(journal::read_bytes(lease, &run_id, &pending_name), Ok(Some(ref now)) if now == &pending_bytes)
        || !matches!(journal::read_bytes(lease, &run_id, &landed_name), Ok(Some(ref now)) if now == &landed_bytes)
    {
        return Err(uncertain(
            Some(&run_id),
            "Gitlink records changed during proved receipt read",
        ));
    }
    Ok(ProvedLinkReceipt {
        path: path.into(),
        pending_bytes,
        landed_bytes,
        pending,
    })
}
