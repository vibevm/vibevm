//! Classification lattice and canonical plan construction.

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

pub fn build(
    project: &vibe_safefs::Project,
    request: &ScrapeRequest,
    snapshot: &ContractSnapshot,
    inventory: &Inventory,
    rewrites: &[PreparedRewrite],
    rewrite_blockers: Vec<Blocker>,
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

    if !contract.healthcheck.is_empty() {
        blockers.push(Blocker::new(
            "health-preparation-required",
            "health argv, executable identities, and verifier snapshots are not prepared yet",
        ));
    }
    if rewrites.iter().any(|rewrite| is_native_lock(&rewrite.path)) {
        blockers.push(Blocker::new(
            "native-lock-evidence-required",
            "a selected rewrite changes a native lockfile but graph evidence is not prepared yet",
        ));
    }

    let projected = build_projected_final(project, inventory, &items, rewrites, contract)?;
    if let Err(error) = crate::rewrite::validate_projected_final(contract, &projected) {
        blockers.push(Blocker::new(
            "projected-final-invalid",
            format!("projected scraped tree fails residual validation: {error}"),
        ));
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
        prepared_healthchecks: None,
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

fn build_projected_final(
    project: &vibe_safefs::Project,
    inventory: &Inventory,
    items: &[PlanItem],
    rewrites: &[PreparedRewrite],
    contract: &crate::contract::Contract,
) -> Result<Vec<crate::rewrite::ProjectedEntry>, ScrapeError> {
    let inventory_by_path = inventory
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut projected = BTreeMap::<String, crate::rewrite::ProjectedEntry>::new();
    for item in items {
        let path = match item.disposition {
            Disposition::Delete | Disposition::DeleteLast => continue,
            Disposition::Relocate => {
                let row = contract
                    .relocate
                    .iter()
                    .find(|row| at_or_below(&item.path, &row.from))
                    .ok_or_else(|| {
                        ScrapeError::inventory(format!(
                            "relocated plan item `{}` has no relocation row",
                            item.path
                        ))
                    })?;
                let suffix = item.path.strip_prefix(&row.from).ok_or_else(|| {
                    ScrapeError::inventory(format!(
                        "relocated plan item `{}` is outside `{}`",
                        item.path, row.from
                    ))
                })?;
                format!("{}{}", row.to, suffix)
            }
            Disposition::Keep | Disposition::Rewrite => item.path.clone(),
        };
        let source = inventory_by_path.get(item.path.as_str()).ok_or_else(|| {
            ScrapeError::inventory(format!(
                "plan item `{}` is absent from inventory",
                item.path
            ))
        })?;
        let bytes = if item.entry_kind == EntryKind::Directory {
            None
        } else if let Some(rewrite) = rewrites
            .iter()
            .rev()
            .find(|rewrite| rewrite.path == item.path)
        {
            Some(rewrite.after_bytes.clone())
        } else if item.disposition == Disposition::Rewrite {
            return Err(ScrapeError::rewrite(format!(
                "rewrite plan item `{}` has no prepared rewrite",
                item.path
            )));
        } else {
            Some(read_inventoried_bytes(project, source)?)
        };
        projected.insert(
            path.clone(),
            crate::rewrite::ProjectedEntry {
                path,
                kind: item.entry_kind,
                bytes,
                unix_mode: item.unix_mode,
            },
        );
    }

    let paths = projected.keys().cloned().collect::<Vec<_>>();
    for path in paths {
        let mut current = path.as_str();
        while let Some((parent, _)) = current.rsplit_once('/') {
            projected
                .entry(parent.to_owned())
                .or_insert_with(|| crate::rewrite::ProjectedEntry {
                    path: parent.to_owned(),
                    kind: EntryKind::Directory,
                    bytes: None,
                    unix_mode: inventory_by_path
                        .get(parent)
                        .and_then(|entry| entry.unix_mode),
                });
            current = parent;
        }
    }
    Ok(projected.into_values().collect())
}

fn read_inventoried_bytes(
    project: &vibe_safefs::Project,
    entry: &crate::model::InventoryEntry,
) -> Result<Vec<u8>, ScrapeError> {
    let expected_size = entry.bytes.ok_or_else(|| {
        ScrapeError::inventory(format!("file `{}` has no inventoried size", entry.path))
    })?;
    let cap = usize::try_from(expected_size).map_err(|_| {
        ScrapeError::inventory(format!("file `{}` is too large to project", entry.path))
    })?;
    let snapshot = project
        .read_file_snapshot_bounded(&entry.path, cap)
        .map_err(|error| {
            ScrapeError::inventory(format!(
                "re-reading `{}` for final projection: {error:#}",
                entry.path
            ))
        })?
        .ok_or_else(|| ScrapeError::inventory(format!("`{}` disappeared", entry.path)))?;
    if snapshot.size != expected_size
        || Some(snapshot.identity) != entry.identity
        || snapshot.unix_mode != entry.unix_mode
        || entry.sha256.as_deref() != Some(&format!("sha256:{}", snapshot.sha256))
    {
        return Err(ScrapeError::inventory(format!(
            "`{}` changed after inventory while projecting the final tree",
            entry.path
        )));
    }
    Ok(snapshot.bytes)
}

fn classify_entry(
    entry: &crate::model::InventoryEntry,
    rules: &[&ClassifyRule],
    baselines: &BTreeMap<&str, &str>,
    in_closed_root: bool,
    baseline_used: &mut BTreeSet<String>,
) -> (PlanItem, Vec<Blocker>) {
    let mut blockers = Vec::new();
    let mut keep = Vec::new();
    let mut delete = Vec::new();
    let mut generated = Vec::new();
    for rule in rules {
        match rule {
            ClassifyRule::Keep { .. } => keep.push(*rule),
            ClassifyRule::Delete { .. } => delete.push(*rule),
            ClassifyRule::Generated { .. } => generated.push(*rule),
        }
    }
    if entry.kind == EntryKind::File {
        for rule in rules {
            let uses_sha = matches!(
                rule,
                ClassifyRule::Delete {
                    proof: Proof::Sha256V1,
                    ..
                } | ClassifyRule::Generated {
                    proof: Proof::Sha256V1,
                    ..
                }
            );
            if uses_sha {
                match baselines.get(entry.path.as_str()) {
                    Some(_) => {
                        baseline_used.insert(entry.path.clone());
                    }
                    None => blockers.push(
                        Blocker::new(
                            "missing-baseline",
                            "sha256-v1 regular file has no exact baseline",
                        )
                        .at(&entry.path)
                        .rule(rule.id()),
                    ),
                }
            }
        }
    }
    if keep.is_empty() && delete.is_empty() && generated.is_empty() {
        if in_closed_root {
            blockers.push(
                Blocker::new(
                    "unclassified-closed-root",
                    "entry below a closed root has no classification",
                )
                .at(&entry.path),
            );
        }
        return (
            PlanItem {
                path: entry.path.clone(),
                entry_kind: entry.kind,
                disposition: Disposition::Keep,
                class: if in_closed_root {
                    FileClass::Unknown
                } else {
                    FileClass::AuthoredProduct
                },
                proof: None,
                modification: ModificationState::NotApplicable,
                owner: crate::contract::Owner::Project,
                sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                unix_mode: entry.unix_mode,
                rule_ids: Vec::new(),
            },
            blockers,
        );
    }
    if !keep.is_empty() && !generated.is_empty() {
        blockers.push(
            Blocker::new(
                "keep-generated-overlap",
                "keep plus generated classification is invalid",
            )
            .at(&entry.path),
        );
    }
    check_same_kind(&delete, &entry.path, &mut blockers);
    check_same_kind(&generated, &entry.path, &mut blockers);
    let mut rule_ids = rules
        .iter()
        .map(|rule| rule.id().to_owned())
        .collect::<Vec<_>>();
    rule_ids.sort();
    if !keep.is_empty() {
        return (
            PlanItem {
                path: entry.path.clone(),
                entry_kind: entry.kind,
                disposition: Disposition::Keep,
                class: FileClass::AuthoredProduct,
                proof: None,
                modification: ModificationState::NotApplicable,
                owner: crate::contract::Owner::Project,
                sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                unix_mode: entry.unix_mode,
                rule_ids,
            },
            blockers,
        );
    }
    let selected = generated
        .first()
        .or_else(|| delete.first())
        .expect("nonempty classification");
    let (proof, modified, class) = match selected {
        ClassifyRule::Generated {
            proof, modified, ..
        } => (*proof, *modified, FileClass::GeneratedOwned),
        ClassifyRule::Delete {
            proof, modified, ..
        } => (*proof, *modified, FileClass::AuthoredMetadata),
        ClassifyRule::Keep { .. } => unreachable!(),
    };
    let modification = if entry.kind == EntryKind::Directory {
        ModificationState::NotApplicable
    } else {
        match proof {
            Proof::ContractAssertionV1 => ModificationState::Unknown,
            Proof::VibeGeneratedV1 => {
                blockers.push(
                    Blocker::new(
                        "generated-proof-unavailable",
                        "no exact receipt/artifact digest proves generated ownership",
                    )
                    .at(&entry.path)
                    .rule(selected.id()),
                );
                ModificationState::Unknown
            }
            Proof::Sha256V1 => match baselines.get(entry.path.as_str()) {
                Some(expected) => {
                    baseline_used.insert(entry.path.clone());
                    if entry.sha256.as_deref() == Some(*expected) {
                        ModificationState::Unmodified
                    } else {
                        ModificationState::Modified
                    }
                }
                None => {
                    blockers.push(
                        Blocker::new(
                            "missing-baseline",
                            "sha256-v1 regular file has no exact baseline",
                        )
                        .at(&entry.path)
                        .rule(selected.id()),
                    );
                    ModificationState::Unknown
                }
            },
        }
    };
    let disposition = match (entry.kind, modification, modified) {
        (EntryKind::Directory, _, _) => Disposition::Delete,
        (_, ModificationState::Unmodified, _) => Disposition::Delete,
        (_, ModificationState::Modified | ModificationState::Unknown, ModifiedPolicy::Keep) => {
            Disposition::Keep
        }
        (_, ModificationState::Modified | ModificationState::Unknown, ModifiedPolicy::Delete) => {
            Disposition::Delete
        }
        (_, ModificationState::Modified | ModificationState::Unknown, ModifiedPolicy::Refuse) => {
            blockers.push(
                Blocker::new(
                    "modified-policy-refusal",
                    format!(
                        "{:?} content is refused by its modification policy",
                        modification
                    ),
                )
                .at(&entry.path)
                .rule(selected.id()),
            );
            Disposition::Delete
        }
        (_, ModificationState::NotApplicable, _) => Disposition::Delete,
    };
    (
        PlanItem {
            path: entry.path.clone(),
            entry_kind: entry.kind,
            disposition,
            class,
            proof: Some(proof_name(proof).to_owned()),
            modification,
            owner: crate::contract::Owner::Vibe,
            sha256: entry.sha256.clone(),
            bytes: entry.bytes,
            unix_mode: entry.unix_mode,
            rule_ids,
        },
        blockers,
    )
}

fn check_same_kind(rules: &[&ClassifyRule], path: &str, blockers: &mut Vec<Blocker>) {
    let Some(first) = rules.first() else { return };
    let signature = classification_signature(first);
    if rules[1..]
        .iter()
        .any(|rule| classification_signature(rule) != signature)
    {
        blockers.push(
            Blocker::new(
                "inconsistent-same-kind-overlap",
                "same-kind classifications disagree on owner, proof or modified policy",
            )
            .at(path),
        );
    }
}

fn classification_signature(rule: &ClassifyRule) -> String {
    match rule {
        ClassifyRule::Keep { owner, .. } => format!("keep:{owner:?}"),
        ClassifyRule::Delete {
            owner,
            proof,
            modified,
            ..
        } => format!("delete:{owner:?}:{proof:?}:{modified:?}"),
        ClassifyRule::Generated {
            owner,
            proof,
            modified,
            ..
        } => format!("generated:{owner:?}:{proof:?}:{modified:?}"),
    }
}

fn validate_relocations(
    contract: &crate::contract::Contract,
    inventory: &Inventory,
    items: &[PlanItem],
    blockers: &mut Vec<Blocker>,
) {
    for row in &contract.relocate {
        let found = inventory
            .entries
            .iter()
            .any(|entry| at_or_below(&entry.path, &row.from));
        if row.required && !found {
            blockers.push(
                Blocker::new(
                    "required-relocation-absent",
                    "required relocation source is absent",
                )
                .at(&row.from)
                .rule(&row.id),
            );
        }
        if inventory
            .entries
            .iter()
            .any(|entry| at_or_below(&entry.path, &row.to))
        {
            blockers.push(
                Blocker::new(
                    "relocation-destination-exists",
                    "relocation destination already exists",
                )
                .at(&row.to)
                .rule(&row.id),
            );
        }
        if inventory.entries.iter().any(|entry| {
            entry.kind == EntryKind::File
                && entry.path != row.to
                && at_or_below(&row.to, &entry.path)
        }) {
            blockers.push(
                Blocker::new(
                    "relocation-destination-ancestor-file",
                    "a relocation destination ancestor is an existing file",
                )
                .at(&row.to)
                .rule(&row.id),
            );
        }
        let under_delete = items.iter().any(|item| {
            item.disposition == Disposition::Delete && at_or_below(&row.to, &item.path)
        });
        if under_delete {
            blockers.push(
                Blocker::new(
                    "relocation-destination-deleted",
                    "relocation destination lies below a deletion root",
                )
                .at(&row.to)
                .rule(&row.id),
            );
        }
    }
}

fn retain_delete_ancestors(items: &mut [PlanItem]) {
    let retained = items
        .iter()
        .filter(|item| {
            matches!(
                item.disposition,
                Disposition::Keep | Disposition::Rewrite | Disposition::Relocate
            )
        })
        .map(|item| item.path.clone())
        .collect::<Vec<_>>();
    for item in items {
        if item.entry_kind == EntryKind::Directory
            && item.disposition == Disposition::Delete
            && retained
                .iter()
                .any(|path| path.starts_with(&(item.path.clone() + "/")))
        {
            item.disposition = Disposition::Keep;
        }
    }
}

fn validate_contract_last(
    snapshot: &ContractSnapshot,
    inventory: &Inventory,
    items: &mut [PlanItem],
    rewrites: &[PreparedRewrite],
    contract: &crate::contract::Contract,
    blockers: &mut Vec<Blocker>,
) -> ContractBoundary {
    let path = &snapshot.display_path;
    let entry = inventory.entries.iter().find(|entry| &entry.path == path);
    match entry {
        Some(entry) if entry.kind == EntryKind::File => {}
        Some(_) => blockers.push(
            Blocker::new(
                "contract-not-regular",
                "contained contract is not a regular file",
            )
            .at(path),
        ),
        None => blockers.push(
            Blocker::new(
                "contract-not-inventory",
                "contained contract is absent from inventory",
            )
            .at(path),
        ),
    }
    if entry.and_then(|entry| entry.sha256.as_deref()) != Some(snapshot.sha256.as_str()) {
        blockers.push(
            Blocker::new(
                "contract-snapshot-drift",
                "inventoried contract digest differs from the validated snapshot",
            )
            .at(path),
        );
    }
    if entry.and_then(|entry| entry.identity) != Some(snapshot.identity) {
        blockers.push(
            Blocker::new(
                "contract-snapshot-identity-drift",
                "inventoried contract identity differs from the validated snapshot",
            )
            .at(path),
        );
    }
    match items.iter_mut().find(|item| &item.path == path) {
        Some(item) if item.disposition == Disposition::Delete => {
            item.disposition = Disposition::DeleteLast
        }
        Some(_) => blockers.push(
            Blocker::new(
                "contract-not-effective-delete",
                "contained contract must resolve to exactly one effective-delete file",
            )
            .at(path),
        ),
        None => {}
    }
    if rewrites.iter().any(|row| &row.path == path) {
        blockers.push(
            Blocker::new(
                "contract-rewrite-conflict",
                "delete-last contract cannot be a rewrite target",
            )
            .at(path),
        );
    }
    if contract
        .relocate
        .iter()
        .any(|row| &row.from == path || &row.to == path)
    {
        blockers.push(
            Blocker::new(
                "contract-relocation-conflict",
                "delete-last contract cannot be relocation source/destination",
            )
            .at(path),
        );
    }
    if contract.baseline.iter().any(|row| &row.path == path) {
        blockers.push(
            Blocker::new(
                "contract-baseline-conflict",
                "delete-last contract cannot be a baseline target",
            )
            .at(path),
        );
    }
    let mut ancestors = Vec::new();
    let mut current = path.as_str();
    while let Some((parent, _)) = current.rsplit_once('/') {
        let surviving_descendant = items.iter().any(|item| {
            item.path != *path
                && item.path != parent
                && at_or_below(&item.path, parent)
                && matches!(item.disposition, Disposition::Keep | Disposition::Rewrite)
        }) || contract
            .relocate
            .iter()
            .any(|row| at_or_below(&row.to, parent));
        if surviving_descendant {
            break;
        }
        ancestors.push(parent.to_owned());
        current = parent;
    }
    ContractBoundary::DeleteLast {
        path: path.clone(),
        empty_ancestors: ancestors,
    }
}

fn summarize(items: &[PlanItem]) -> PlanSummary {
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

fn plan_identity(
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
        assertions: &'a [String],
        healthchecks: &'a [String],
        contract_boundary: &'a ContractBoundary,
        blockers: &'a [Blocker],
        summary: &'a PlanSummary,
        prepared_healthchecks: Option<&'a [vibe_wire::generated::scrape::e1::plan::Healthcheck]>,
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
        assertions: &plan.assertions,
        healthchecks: &plan.healthchecks,
        contract_boundary: &plan.contract_boundary,
        blockers: &plan.blockers,
        summary: &plan.summary,
        prepared_healthchecks: plan.prepared_healthchecks.as_deref(),
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

fn is_native_lock(path: &str) -> bool {
    path.ends_with("Cargo.lock")
        || path.ends_with("package-lock.json")
        || path.ends_with("pnpm-lock.yaml")
        || path.ends_with("yarn.lock")
        || path.ends_with("go.sum")
}

fn mode_name(mode: &ScrapeMode) -> String {
    match mode {
        ScrapeMode::InPlace => "in-place",
        ScrapeMode::Export { .. } => "export",
    }
    .to_owned()
}
fn at_or_below(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&(root.to_owned() + "/"))
}
fn proof_name(proof: Proof) -> &'static str {
    match proof {
        Proof::ContractAssertionV1 => "contract-assertion-v1",
        Proof::Sha256V1 => "sha256-v1",
        Proof::VibeGeneratedV1 => "vibe-generated-v1",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::InventoryEntry;

    #[test]
    fn kept_descendant_shields_delete_directory() {
        let mut items = vec![
            PlanItem {
                path: "vibevm".into(),
                entry_kind: EntryKind::Directory,
                disposition: Disposition::Delete,
                class: FileClass::AuthoredMetadata,
                proof: None,
                modification: ModificationState::NotApplicable,
                owner: crate::contract::Owner::Vibe,
                sha256: None,
                bytes: None,
                unix_mode: None,
                rule_ids: vec![],
            },
            PlanItem {
                path: "vibevm/keep".into(),
                entry_kind: EntryKind::File,
                disposition: Disposition::Keep,
                class: FileClass::AuthoredProduct,
                proof: None,
                modification: ModificationState::NotApplicable,
                owner: crate::contract::Owner::Project,
                sha256: None,
                bytes: Some(0),
                unix_mode: None,
                rule_ids: vec![],
            },
        ];
        retain_delete_ancestors(&mut items);
        assert_eq!(items[0].disposition, Disposition::Keep);
    }

    #[test]
    fn inventory_type_is_canonical_input() {
        let inventory = Inventory {
            entries: vec![InventoryEntry {
                path: "a".into(),
                kind: EntryKind::File,
                sha256: Some(format!("sha256:{}", "0".repeat(64))),
                bytes: Some(0),
                unix_mode: None,
                identity: None,
            }],
            tree_digest: "sha256:x".into(),
        };
        assert_eq!(inventory.entries[0].path, "a");
    }
}
