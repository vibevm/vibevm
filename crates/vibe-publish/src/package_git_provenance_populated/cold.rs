//! Read-only aggregate proof under an exact C.1 Pending or Ready admission.
use super::*;

pub(crate) fn inspect_complete_cold(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    approved: Option<&[ApprovedGitdir<'_>]>,
    binary: &Path,
    limits: SubmoduleLimits,
    admission: JournalAdmission,
) -> AggregateInspection {
    let proof = match prior(lease, namespace, scope, key, source, binary, admission) {
        Ok(v) => v,
        Err(s) => return s,
    };
    let run = proof.run_id.clone();
    let bytes = match journal::read_bytes(lease, &run, PENDING) {
        Ok(Some(v)) => v,
        Ok(None) => {
            return match journal::read_bytes(lease, &run, LANDED) {
                Ok(None) => AggregateInspection::NotReady { run_id: Some(run) },
                other => uncertain(
                    Some(&run),
                    format!("populated Landed exists without Pending: {other:?}"),
                ),
            };
        }
        Err(error) => return uncertain(Some(&run), error.to_string()),
    };
    let row: PopulatedPending = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => return uncertain(Some(&run), e.to_string()),
    };
    let tree_bytes = match journal::read_bytes(lease, &run, "tree") {
        Ok(Some(v)) => v,
        other => {
            return uncertain(
                Some(&run),
                format!("tree receipt unavailable cold: {other:?}"),
            );
        }
    };
    let plan_bytes = match serde_json::to_vec(&proof.plan) {
        Ok(v) => v,
        Err(e) => return uncertain(Some(&run), e.to_string()),
    };
    let semantic = match proof.plan.digest() {
        Ok(v) => v,
        Err(e) => return uncertain(Some(&run), e.to_string()),
    };
    if row.format != journal::POPULATED_PENDING_FORMAT
        || row.run_id != run
        || row.c1_plan_sha256 != sha(&plan_bytes)
        || row.semantic_plan_sha256 != semantic
        || row.intent_sha256 != proof.intent_sha256
        || row.payload_manifest_sha256 != proof.payload_manifest_sha256
        || row.tree_receipt_sha256 != sha(&tree_bytes)
        || row.source_root_token != proof.manifest.source_root_token
        || row.flattened_submodules_sha256 != proof.plan.flattened_submodules_sha256
        || serde_json::to_vec(&row).ok().as_deref() != Some(bytes.as_slice())
    {
        return uncertain(
            Some(&run),
            "populated Pending differs from exact C.1/plan/tree bytes",
        );
    }
    if row.links.is_empty()
        || row.links.len() != row.link_receipts.len()
        || row.links.windows(2).any(|p| p[0].path >= p[1].path)
        || row
            .links
            .iter()
            .zip(&row.link_receipts)
            .any(|(l, r)| l.path != r.path)
    {
        return uncertain(
            Some(&run),
            "populated Pending link set incomplete, duplicated or unordered",
        );
    }
    // Current-source collect proves the same exact retained Pending/Landed
    // bytes twice per link while also checking the approved checkout and full
    // index. The separate source-less pass is needed only on historical attach.
    let digest = match (source, approved) {
        (Some(source), Some(approved)) => {
            let current = match collect(
                lease, namespace, scope, key, &proof, source, approved, binary, limits, admission,
            ) {
                Ok(v) => v,
                Err(s) => {
                    return uncertain(
                        Some(&run),
                        format!("current populated source changed: {s:?}"),
                    );
                }
            };
            if pending_row(&proof, &tree_bytes, &current).ok().as_ref() != Some(&row) {
                return uncertain(
                    Some(&run),
                    "current complete Gitlink index differs from historical aggregate",
                );
            }
            current.flattened_sha256
        }
        (None, None) => {
            let index = GitIndexCensus::PopulatedCandidate {
                repository_root_token: row.index_repository_root_token.clone(),
                package_prefix: row.index_package_prefix.clone(),
                index_sha256: row.index_sha256.clone(),
                links: index_links(&row.links),
            };
            let mut proofs = Vec::with_capacity(row.links.len());
            for (link, expected) in row.links.iter().zip(&row.link_receipts) {
                let proved = match package_git_submodule_store::proved_link_receipt(
                    lease, namespace, scope, key, &link.path, None, None, binary, limits, admission,
                ) {
                    Ok(v) => v,
                    Err(other) => {
                        return uncertain(
                            Some(&run),
                            format!("indexed per-link cold proof missing: {other:?}"),
                        );
                    }
                };
                if sha(&proved.pending_bytes) != expected.pending_sha256
                    || sha(&proved.landed_bytes) != expected.landed_sha256
                {
                    return uncertain(
                        Some(&run),
                        "per-link record bytes differ from aggregate Pending",
                    );
                }
                proofs.push(proof_from_pending(&proved.pending));
            }
            let digest = match semantic_digest(&index, &proofs) {
                Ok(v) => v,
                Err(e) => return uncertain(Some(&run), e),
            };
            if digest != row.flattened_submodules_sha256 {
                return uncertain(Some(&run), "retained populated digest differs from plan");
            }
            digest
        }
        _ => {
            return unavailable(
                Some(&run),
                "current aggregate check needs both package source and all approved Git dirs",
            );
        }
    };
    let landed_bytes = match journal::read_bytes(lease, &run, LANDED) {
        Ok(Some(v)) => v,
        Ok(None) => {
            return uncertain(
                Some(&run),
                "populated Pending has no Landed, never auto-aggregate",
            );
        }
        Err(e) => return uncertain(Some(&run), e.to_string()),
    };
    let landed: PopulatedLanded = match serde_json::from_slice(&landed_bytes) {
        Ok(v) => v,
        Err(e) => return uncertain(Some(&run), e.to_string()),
    };
    if landed != landed_row(&row, &bytes)
        || serde_json::to_vec(&landed).ok().as_deref() != Some(landed_bytes.as_slice())
    {
        return uncertain(
            Some(&run),
            "populated Landed differs from independently proved aggregate",
        );
    }
    match prior(lease, namespace, scope, key, source, binary, admission) {
        Ok(again)
            if again.run_id == run
                && again.intent_sha256 == proof.intent_sha256
                && again.payload_manifest_sha256 == proof.payload_manifest_sha256
                && again.manifest == proof.manifest => {}
        other => {
            return uncertain(
                Some(&run),
                format!("C.1/payload/tree changed during aggregate cold proof: {other:?}"),
            );
        }
    }
    if !matches!(journal::read_bytes(lease, &run, PENDING), Ok(Some(ref now)) if now == &bytes)
        || !matches!(journal::read_bytes(lease, &run, LANDED), Ok(Some(ref now)) if now == &landed_bytes)
    {
        return uncertain(
            Some(&run),
            "populated aggregate receipts changed during cold verification",
        );
    }
    AggregateInspection::AggregateCompleteButNotReady {
        run_id: run,
        flattened_submodules_sha256: digest,
        source_reobserved: source.is_some(),
    }
}
