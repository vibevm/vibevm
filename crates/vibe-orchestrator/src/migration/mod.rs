//! Redbook recipe facade; M13 remains the planning and saved-apply owner.
use vibe_core::manifest::{
    self, ManifestMigrationOutcome, ManifestMigrationPlan, MigrationSelection,
};
use vibe_workspace::migration_application::{
    OwnedManifestMigrationFile, execute_manifest_migration_json,
};

pub fn propose_redbook_consumer(text: &str) -> ManifestMigrationOutcome {
    manifest::plan_manifest_migration(
        text,
        MigrationSelection {
            source_profile: Some(manifest::redbook::LEGACY),
            target_profile: Some(manifest::redbook::SELECTED),
            recipe: Some(manifest::redbook::RECIPE),
            source_decisions: &[],
        },
    )
}

/// The caller approves the exact proposal; saved apply re-plans and checks
/// original bytes, identities, grant, journal and before-image barriers.
pub fn apply_redbook_proposal(
    grant: &OwnedManifestMigrationFile,
    operation_id: &str,
    proposal: &ManifestMigrationPlan,
) -> anyhow::Result<serde_json::Value> {
    anyhow::ensure!(
        proposal.recipe == manifest::redbook::RECIPE,
        "not a Redbook proposal"
    );
    anyhow::ensure!(
        propose_redbook_consumer(&proposal.before)
            == ManifestMigrationOutcome::Planned(proposal.clone()),
        "proposal differs from exact recipe replay"
    );
    let approved = serde_json::json!({"schema":1,"source_text":proposal.before,
        "source_profile":proposal.source_profile,"target_profile":proposal.target_profile,"recipe":proposal.recipe});
    let operation = serde_json::json!({"schema":1,"operation":"apply","operation_id":operation_id,
        "authority":{"scope":"owned_authored_file","project_root":grant.project().root_path(),
            "recovery_store_root":grant.recovery_store_root(),"target_relative":grant.target_relative()},
        "approved_request_json":approved.to_string(),"before_content_id":proposal.before_identity,
        "after_content_id":proposal.after_identity,"diff_id":proposal.diff_identity});
    Ok(serde_json::to_value(execute_manifest_migration_json(
        &operation.to_string(),
        grant,
    )?)?)
}

#[cfg(test)]
mod m_05_f_2_cases;
