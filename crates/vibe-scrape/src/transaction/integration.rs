//! One-way adapter from the already prepared product model into the durable
//! transaction model. No contract read, inventory walk, rewrite, or health
//! preparation occurs here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::BTreeMap;

use sha2::{Digest as _, Sha256};

use super::model as tx;
use super::traits::PreparedScrapeSource;
use crate::model::{Disposition, EntryKind, PreparedScrape, ScrapeMode};

mod contract;
mod tree;

#[cfg(test)]
mod tests;

use contract::{export_plan, in_place_plan, snapshots};
use tree::{digest_text, final_entries, inventory_manifest, manifest, rewrite_transaction_id};

impl PreparedScrapeSource for PreparedScrape {
    fn into_transaction(self) -> Result<tx::PreparedTransaction, tx::TransactionError> {
        prepared_transaction(self)
    }
}

pub fn prepared_transaction(
    prepared: PreparedScrape,
) -> Result<tx::PreparedTransaction, tx::TransactionError> {
    if !prepared.plan.blockers.is_empty() || !prepared.health.blockers.is_empty() {
        return Err(tx::TransactionError::InvalidPrepared(
            "a blocker-filled scrape plan cannot enter a transaction".to_owned(),
        ));
    }
    let project =
        vibe_safefs::Project::open(std::path::Path::new(&prepared.plan.project_display_root))
            .map_err(|error| {
                tx::TransactionError::Filesystem(format!("opening project: {error:#}"))
            })?;
    let project_identity_token = project.identity_token().map_err(|error| {
        tx::TransactionError::Filesystem(format!("sealing project identity: {error:#}"))
    })?;
    let source_tree = inventory_manifest(&prepared.inventory)?;
    let final_entries = final_entries(&prepared)?;
    let final_tree = manifest(final_entries.values().cloned().collect());
    let mut snapshots = snapshots(&prepared)?;
    let canonical_plan = snapshots
        .iter()
        .find(|snapshot| snapshot.kind == tx::SnapshotKind::CanonicalPlan)
        .map(|snapshot| snapshot.bytes.clone())
        .ok_or_else(|| {
            tx::TransactionError::InvalidPrepared(
                "prepared scrape has no canonical plan snapshot".to_owned(),
            )
        })?;
    for rewrite in &prepared.rewrites {
        let transaction_id = rewrite_transaction_id(rewrite);
        snapshots.push(tx::Snapshot {
            kind: tx::SnapshotKind::PreparedAfter,
            name: format!("after/{transaction_id}"),
            bytes: rewrite.after_bytes.clone(),
            mode: prepared
                .inventory
                .entries
                .iter()
                .find(|entry| entry.path == rewrite.path)
                .and_then(|entry| entry.unix_mode),
        });
    }
    let mode = match &prepared.plan.mode[..] {
        "export" => {
            tx::PreparedMode::Export(Box::new(export_plan(&prepared, source_tree, final_tree)?))
        }
        "in-place" => tx::PreparedMode::InPlace(Box::new(in_place_plan(
            &prepared,
            source_tree,
            final_tree,
            &final_entries,
        )?)),
        other => {
            return Err(tx::TransactionError::InvalidPrepared(format!(
                "unknown prepared mode `{other}`"
            )));
        }
    };
    Ok(tx::PreparedTransaction {
        project_identity_token,
        project_display_root: prepared.plan.project_display_root.clone(),
        plan_id: digest_text(&prepared.plan.plan_id)?,
        canonical_plan,
        snapshots,
        mode,
    })
}

pub fn project_identity_token(root: &std::path::Path) -> Result<String, tx::TransactionError> {
    let project = vibe_safefs::Project::open(root)
        .map_err(|error| tx::TransactionError::Filesystem(format!("opening project: {error:#}")))?;
    project.identity_token().map_err(|error| {
        tx::TransactionError::Filesystem(format!("sealing project identity: {error:#}"))
    })
}
