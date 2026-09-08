use super::*;

pub(super) fn summarize(items: &[PlanItem]) -> PlanSummary {
    let mut summary = PlanSummary::default();
    for item in items {
        match item.disposition {
            Disposition::Keep => summary.keep += 1,
            Disposition::Rewrite => summary.rewrite += 1,
            Disposition::Relocate => summary.relocate += 1,
            Disposition::DeleteLast => summary.delete_last += 1,
            Disposition::Delete => match item.modification {
                ModificationState::Unmodified | ModificationState::NotApplicable => {
                    summary.delete_unmodified += 1
                }
                ModificationState::Modified => summary.delete_modified += 1,
                ModificationState::Unknown => summary.delete_unknown += 1,
            },
        }
    }
    summary
}

pub(super) fn plan_identity(
    plan: &ScrapePlan,
    snapshot: &ContractSnapshot,
    output_identity: Option<&str>,
) -> Result<String, ScrapeError> {
    #[derive(Serialize)]
    struct IdentityProjection<'a> {
        schema: u32,
        command: &'a str,
        mode: &'a str,
        platform_os: &'static str,
        platform_arch: &'static str,
        tree_digest: &'a str,
        contract_sha256: &'a str,
        items: &'a [PlanItem],
        rewrites: &'a [PreparedRewrite],
        relocations: &'a [PlannedRelocation],
        native_lock_changes: &'a [crate::model::NativeLockChange],
        assertions: &'a [String],
        healthchecks: &'a [String],
        contract_boundary: &'a ContractBoundary,
        blockers: &'a [Blocker],
        summary: &'a PlanSummary,
        prepared_health: &'a crate::health::PreparedHealth,
        output_identity: Option<&'a str>,
    }
    let projection = IdentityProjection {
        schema: plan.schema,
        command: &plan.command,
        mode: &plan.mode,
        platform_os: std::env::consts::OS,
        platform_arch: std::env::consts::ARCH,
        tree_digest: &plan.tree_digest,
        contract_sha256: &plan.contract_sha256,
        items: &plan.items,
        rewrites: &plan.rewrites,
        relocations: &plan.relocations,
        native_lock_changes: &plan.native_lock_changes,
        assertions: &plan.assertions,
        healthchecks: &plan.healthchecks,
        contract_boundary: &plan.contract_boundary,
        blockers: &plan.blockers,
        summary: &plan.summary,
        prepared_health: &plan.prepared_health,
        output_identity,
    };
    let encoded = serde_json::to_vec(&projection).map_err(|error| {
        ScrapeError::contract(format!("serializing canonical plan identity: {error}"))
    })?;
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-plan-e1\0");
    hash.update(&snapshot.bytes);
    hash.update(b"\0");
    hash.update(encoded);
    Ok(format!("sha256:{:x}", hash.finalize()))
}

pub(super) fn is_native_lock(path: &str) -> bool {
    path.ends_with("Cargo.lock")
        || path.ends_with("package-lock.json")
        || path.ends_with("pnpm-lock.yaml")
        || path.ends_with("yarn.lock")
        || path.ends_with("go.sum")
}

pub(super) fn mode_name(mode: &ScrapeMode) -> String {
    match mode {
        ScrapeMode::InPlace => "in-place",
        ScrapeMode::Export { .. } => "export",
    }
    .to_owned()
}
pub(super) fn at_or_below(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&(root.to_owned() + "/"))
}
pub(super) fn proof_name(proof: Proof) -> &'static str {
    match proof {
        Proof::ContractAssertionV1 => "contract-assertion-v1",
        Proof::Sha256V1 => "sha256-v1",
        Proof::VibeGeneratedV1 => "vibe-generated-v1",
    }
}
