use super::identity::proof_name;
use super::*;

pub(super) fn classify_entry(
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
