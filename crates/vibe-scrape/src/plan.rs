//! Classification lattice and canonical plan construction.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::contract::{ClassifyRule, ModifiedPolicy, Proof};
use crate::glob::Glob;
use crate::model::{
    Blocker, ContractBoundary, ContractSnapshot, Disposition, EntryKind, FileClass, Inventory,
    MappedRelocation, ModificationState, PlanItem, PlanSummary, PlannedRelocation, PreparedRewrite,
    ScrapeError, ScrapeMode, ScrapePlan, ScrapeRequest,
};

mod classify;
mod identity;
mod projected;
mod relocation;

use classify::classify_entry;
use identity::{at_or_below, is_native_lock, mode_name, plan_identity, summarize};
use projected::{build_projected_final, validate_contract_last};
use relocation::{retain_delete_ancestors, validate_relocations};

#[allow(clippy::too_many_arguments)]
pub fn build(
    project: &vibe_safefs::Project,
    request: &ScrapeRequest,
    snapshot: &ContractSnapshot,
    inventory: &Inventory,
    rewrites: &[PreparedRewrite],
    rewrite_blockers: Vec<Blocker>,
    prepared_health: &crate::health::PreparedHealth,
    output_identity: Option<&str>,
) -> Result<ScrapePlan, ScrapeError> {
    let contract = &snapshot.value;
    let compiled = contract
        .classify
        .iter()
        .map(|rule| {
            let patterns = rule
                .patterns()
                .iter()
                .map(|pattern| Glob::parse(pattern))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((rule, patterns))
        })
        .collect::<Result<Vec<_>, ScrapeError>>()?;
    let baseline = contract
        .baseline
        .iter()
        .map(|row| (row.path.as_str(), row.sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    let rewrite_paths = rewrites
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>();
    let relocation_sources = contract
        .relocate
        .iter()
        .map(|row| row.from.as_str())
        .collect::<Vec<_>>();
    let mut baseline_used = BTreeSet::new();
    let mut rule_matches = vec![0_u64; contract.classify.len()];
    let mut blockers = rewrite_blockers;
    let mut items = Vec::new();

    for entry in &inventory.entries {
        let mut matched = Vec::new();
        for (index, (rule, patterns)) in compiled.iter().enumerate() {
            if patterns.iter().any(|pattern| pattern.matches(&entry.path)) {
                rule_matches[index] += 1;
                matched.push(*rule);
            }
        }
        let in_closed_root = contract
            .scope
            .closed_roots
            .iter()
            .any(|root| at_or_below(&entry.path, root));
        let (mut item, mut local) = classify_entry(
            entry,
            &matched,
            &baseline,
            in_closed_root,
            &mut baseline_used,
        );
        blockers.append(&mut local);

        if rewrite_paths.contains(entry.path.as_str()) {
            if item.disposition == Disposition::Keep {
                item.disposition = Disposition::Rewrite;
                item.class = FileClass::AuthoredMetadata;
            } else {
                blockers.push(
                    Blocker::new(
                        "rewrite-classification-conflict",
                        "rewrite target is not effectively kept",
                    )
                    .at(&entry.path),
                );
            }
        }
        if relocation_sources
            .iter()
            .any(|source| at_or_below(&entry.path, source))
        {
            if matches!(item.disposition, Disposition::Keep | Disposition::Rewrite) {
                item.disposition = Disposition::Relocate;
            } else {
                blockers.push(
                    Blocker::new(
                        "relocation-source-not-kept",
                        "relocation source and descendants must be effectively kept",
                    )
                    .at(&entry.path),
                );
            }
        }
        items.push(item);
    }

    for (index, rule) in contract.classify.iter().enumerate() {
        if rule.require_match() && rule_matches[index] == 0 {
            blockers.push(
                Blocker::new(
                    "required-classification-empty",
                    "classification rule requires at least one match",
                )
                .rule(rule.id()),
            );
        }
    }
    for rule in &contract.rewrite {
        if let crate::contract::RewriteRule::ManagedBlockRemoveV1 { paths, .. } = rule {
            for path in paths {
                if baseline.contains_key(path.as_str()) {
                    baseline_used.insert(path.clone());
                }
            }
        }
    }
    for path in baseline.keys() {
        if !baseline_used.contains(*path) {
            blockers.push(
                Blocker::new(
                    "unused-baseline",
                    "baseline is selected by no sha256-v1 classification",
                )
                .at(*path),
            );
        }
    }

    validate_relocations(contract, inventory, &items, &mut blockers);
    retain_delete_ancestors(&mut items);

    let contract_boundary = if snapshot.contained {
        validate_contract_last(
            snapshot,
            inventory,
            &mut items,
            rewrites,
            contract,
            &mut blockers,
        )
    } else {
        ContractBoundary::Preserve
    };

    for blocker in &prepared_health.blockers {
        let mut projected = Blocker::new(&blocker.code, &blocker.message);
        if let Some(check_id) = &blocker.check_id {
            projected = projected.rule(check_id);
        }
        blockers.push(projected);
    }
    for rewrite in rewrites {
        if is_native_lock(&rewrite.path) && rewrite.native_lock_change.is_none() {
            blockers.push(
                Blocker::new(
                    "native-lock-evidence-required",
                    "a selected rewrite changes a native lockfile but carries no exact dependency-graph evidence",
                )
                .at(&rewrite.path)
                .rule(&rewrite.id),
            );
        }
    }

    let projected = build_projected_final(project, inventory, &items, rewrites, contract)?;
    if let Err(error) = crate::rewrite::validate_projected_final(contract, &projected) {
        blockers.push(Blocker::new(
            "projected-final-invalid",
            format!("projected scraped tree fails residual validation: {error}"),
        ));
    }
    for blocker in crate::health::validate_projected_final(contract, prepared_health, &projected) {
        let mut projected = Blocker::new(&blocker.code, &blocker.message);
        if let Some(check_id) = blocker.check_id {
            projected = projected.rule(check_id);
        }
        blockers.push(projected);
    }

    items.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    blockers.sort_by(|left, right| {
        (&left.code, &left.path, &left.rule_id, &left.message).cmp(&(
            &right.code,
            &right.path,
            &right.rule_id,
            &right.message,
        ))
    });
    blockers.dedup();
    let relocations = contract
        .relocate
        .iter()
        .map(|row| PlannedRelocation {
            id: row.id.clone(),
            from: row.from.clone(),
            to: row.to.clone(),
            required: row.required,
            mapped_descendants: inventory
                .entries
                .iter()
                .filter(|entry| at_or_below(&entry.path, &row.from))
                .map(|entry| {
                    let suffix = entry.path.strip_prefix(&row.from).unwrap_or_default();
                    let rewritten = rewrites
                        .iter()
                        .rev()
                        .find(|rewrite| rewrite.path == entry.path);
                    MappedRelocation {
                        from: entry.path.clone(),
                        to: format!("{}{}", row.to, suffix),
                        entry_kind: entry.kind,
                        sha256: rewritten
                            .map(|rewrite| rewrite.after_sha256.clone())
                            .or_else(|| entry.sha256.clone()),
                        bytes: rewritten
                            .map(|rewrite| rewrite.after_bytes.len() as u64)
                            .or(entry.bytes),
                        unix_mode: entry.unix_mode,
                    }
                })
                .collect(),
        })
        .collect();
    let mut native_lock_changes = rewrites
        .iter()
        .filter_map(|rewrite| rewrite.native_lock_change.clone())
        .collect::<Vec<_>>();
    native_lock_changes.sort_by(|left, right| {
        (&left.manager, &left.path, &left.authorizing_rewrite_id).cmp(&(
            &right.manager,
            &right.path,
            &right.authorizing_rewrite_id,
        ))
    });
    for pair in native_lock_changes.windows(2) {
        if pair[0].manager == pair[1].manager && pair[0].path == pair[1].path {
            blockers.push(
                Blocker::new(
                    "native-lock-authorization-ambiguous",
                    "one native lockfile is changed by more than one authorizing rewrite",
                )
                .at(&pair[0].path),
            );
        }
    }
    blockers.sort_by(|left, right| {
        (&left.code, &left.path, &left.rule_id, &left.message).cmp(&(
            &right.code,
            &right.path,
            &right.rule_id,
            &right.message,
        ))
    });
    blockers.dedup();
    let mut plan = ScrapePlan {
        schema: 1,
        command: "scrape".to_owned(),
        mode: mode_name(&request.mode),
        plan_id: String::new(),
        tree_digest: inventory.tree_digest.clone(),
        contract_sha256: snapshot.sha256.clone(),
        items,
        rewrites: rewrites.to_vec(),
        relocations,
        native_lock_changes,
        assertions: contract
            .assertions
            .iter()
            .map(|row| row.id().to_owned())
            .collect(),
        healthchecks: contract
            .healthcheck
            .iter()
            .map(|row| row.id().to_owned())
            .collect(),
        contract_boundary,
        blockers,
        summary: PlanSummary::default(),
        prepared_health: prepared_health.clone(),
        project_display_root: request.root.display().to_string(),
        contract_display_path: snapshot.display_path.clone(),
        contract_contained: snapshot.contained,
        contract_action: contract.commit.contract,
        contract_value: contract.clone(),
    };
    plan.assertions.sort();
    plan.healthchecks.sort();
    plan.relocations.sort_by(|a, b| a.id.cmp(&b.id));
    plan.summary = summarize(&plan.items);
    plan.plan_id = plan_identity(&plan, snapshot, output_identity)?;
    Ok(plan)
}

#[cfg(test)]
mod tests;
