fn mutation(step: &MutationStep, snapshots: &BTreeSet<&str>) -> Result<(), TransactionError> {
    token(&step.id, "mutation id")?;
    match (&step.kind, &step.pair_id) {
        (MutationKind::CaptureBeforeImage | MutationKind::AtomicRewrite, Some(pair)) => {
            token(pair, "rewrite pair id")?
        }
        (MutationKind::CaptureBeforeImage | MutationKind::AtomicRewrite, None) => {
            return invalid("rewrite/capture operation requires pair_id");
        }
        (_, Some(_)) => return invalid("pair_id is forbidden outside rewrite/capture"),
        (_, None) => {}
    }
    for transition in &step.transitions {
        path(&transition.path)?;
        if transition.location == Location::Project && is_git(&transition.path) {
            return invalid("a project mutation addresses protected .git metadata");
        }
        state(&transition.before)?;
        state(&transition.after)?;
    }
    match step.kind {
        MutationKind::CaptureBeforeImage => validate_capture(step),
        MutationKind::AtomicRewrite => {
            let expected = format!("after/{}", step.id);
            if !snapshots.contains(expected.as_str()) {
                return invalid(format!(
                    "rewrite `{}` lacks prepared-after snapshot `{expected}`",
                    step.id
                ));
            }
            validate_rewrite(step)
        }
        MutationKind::CreateRelocationParent => validate_create_parent(step),
        MutationKind::Relocate => validate_move_pair(step, false),
        MutationKind::QuarantineFile => validate_move_pair(step, true),
        MutationKind::ContractDeleteLast => validate_contract_delete(step),
        MutationKind::ContractAncestorTreePark => validate_contract_tree_park(step),
        MutationKind::PruneEmptyDirectory => validate_prune(step),
        MutationKind::ContractExternalPreserve => {
            if step.transitions.is_empty() {
                Ok(())
            } else {
                invalid("external-preserve contract boundary mutates no path")
            }
        }
    }
}

fn reject_internal_mutation_id(id: &str) -> Result<(), TransactionError> {
    if id.starts_with("in-place/") || id.starts_with("export/") {
        invalid(format!(
            "mutation id `{id}` uses the transaction engine's reserved synthetic namespace"
        ))
    } else {
        Ok(())
    }
}

fn validate_capture(step: &MutationStep) -> Result<(), TransactionError> {
    if step.transitions.len() != 2
        || step.transitions[0].location != Location::Project
        || step.transitions[1].location != Location::Quarantine
    {
        return invalid("capture transition order must be project then quarantine");
    }
    let project = one_transition(step, Location::Project)?;
    let quarantine = one_transition(step, Location::Quarantine)?;
    let PathState::File(file) = &project.before else {
        return invalid("capture source must be a regular file");
    };
    if project.after != project.before
        || quarantine.before != PathState::Absent
        || quarantine.after != PathState::File(file.clone())
    {
        return invalid(
            "capture must preserve source and create its exact quarantine before-image",
        );
    }
    Ok(())
}

fn validate_rewrite(step: &MutationStep) -> Result<(), TransactionError> {
    if step.transitions.len() != 1 {
        return invalid("atomic rewrite must have one project transition");
    }
    let transition = &step.transitions[0];
    if transition.location != Location::Project
        || !matches!(transition.before, PathState::File(_))
        || !matches!(transition.after, PathState::File(_))
        || transition.before == transition.after
    {
        return invalid(
            "atomic rewrite must replace one regular before state with exact after state",
        );
    }
    Ok(())
}

fn validate_create_parent(step: &MutationStep) -> Result<(), TransactionError> {
    if step.transitions.len() != 1 {
        return invalid("relocation parent creation must have one transition");
    }
    let transition = &step.transitions[0];
    if transition.location != Location::Project
        || transition.before != PathState::Absent
        || !matches!(transition.after, PathState::EmptyDirectory { .. })
    {
        return invalid("relocation parent creation must be absent -> empty-directory");
    }
    Ok(())
}

fn validate_move_pair(step: &MutationStep, quarantine: bool) -> Result<(), TransactionError> {
    if step.transitions.len() != 2 {
        return invalid("move operation must have exact source/destination transition pair");
    }
    let source = step
        .transitions
        .iter()
        .find(|transition| {
            transition.location == Location::Project && transition.after == PathState::Absent
        })
        .ok_or_else(|| invalid_error("move has no project source -> absent transition"))?;
    let target_location = if quarantine {
        Location::Quarantine
    } else {
        Location::Project
    };
    if step.transitions[0].location != Location::Project
        || step.transitions[1].location != target_location
    {
        return invalid("move transition order must be source then destination");
    }
    let target = step
        .transitions
        .iter()
        .find(|transition| {
            transition.location == target_location && transition.before == PathState::Absent
        })
        .ok_or_else(|| invalid_error("move has no absent -> target transition"))?;
    if source.before == PathState::Absent || target.after != source.before {
        return invalid("move destination does not receive the exact source state");
    }
    if quarantine && !matches!(source.before, PathState::File(_)) {
        return invalid("quarantine removal moves one regular file, never a directory tree");
    }
    Ok(())
}

fn validate_prune(step: &MutationStep) -> Result<(), TransactionError> {
    if step.transitions.len() != 1 {
        return invalid("prune must have one project transition");
    }
    let transition = &step.transitions[0];
    if transition.location != Location::Project
        || !matches!(transition.before, PathState::EmptyDirectory { .. })
        || transition.after != PathState::Absent
    {
        return invalid("prune must remove one independently observed empty directory");
    }
    Ok(())
}

fn validate_contract_delete(step: &MutationStep) -> Result<(), TransactionError> {
    let quarantine = one_transition(step, Location::Quarantine)?;
    let sources = step
        .transitions
        .iter()
        .filter(|transition| {
            transition.location == Location::Project
                && matches!(transition.before, PathState::File(_))
                && transition.after == PathState::Absent
        })
        .collect::<Vec<_>>();
    if step.transitions.len() != 2
        || sources.len() != 1
        || quarantine.before != PathState::Absent
        || quarantine.after != sources[0].before
    {
        return invalid(
            "contract delete-last must move exactly one regular contract to quarantine",
        );
    }
    if !std::ptr::eq(&step.transitions[0], sources[0])
        || step.transitions.get(1) != Some(quarantine)
    {
        return invalid("contract boundary must contain only the contract file move");
    }
    Ok(())
}

fn validate_contract_tree_park(step: &MutationStep) -> Result<(), TransactionError> {
    if step.transitions.len() != 2
        || step.transitions[0].location != Location::Project
        || step.transitions[1].location != Location::Quarantine
        || step.transitions[0].after != PathState::Absent
        || step.transitions[1].before != PathState::Absent
        || step.transitions[1].after != step.transitions[0].before
        || !matches!(step.transitions[0].before, PathState::Tree(_))
    {
        return invalid(
            "contract ancestor cleanup must atomically park one exact project tree in quarantine",
        );
    }
    Ok(())
}

fn one_transition(
    step: &MutationStep,
    location: Location,
) -> Result<&PathTransition, TransactionError> {
    let matches = step
        .transitions
        .iter()
        .filter(|transition| transition.location == location)
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Ok(matches[0])
    } else {
        invalid("operation does not have exactly one transition at the required location")
    }
}

fn project_transition(step: &MutationStep) -> Result<&PathTransition, TransactionError> {
    one_transition(step, Location::Project)
}

fn mutation_rank(kind: MutationKind) -> u8 {
    match kind {
        MutationKind::CaptureBeforeImage | MutationKind::AtomicRewrite => 0,
        MutationKind::CreateRelocationParent => 1,
        MutationKind::Relocate => 2,
        MutationKind::QuarantineFile => 3,
        MutationKind::PruneEmptyDirectory => 4,
        MutationKind::ContractDeleteLast
        | MutationKind::ContractAncestorTreePark
        | MutationKind::ContractExternalPreserve => 5,
    }
}

fn apply_project_steps(
    initial: &TreeManifest,
    steps: &[MutationStep],
) -> Result<TreeManifest, TransactionError> {
    let mut entries = initial
        .entries
        .iter()
        .map(|entry| (entry.path.clone(), entry.clone()))
        .collect::<BTreeMap<_, _>>();
    for step in steps {
        for transition in step
            .transitions
            .iter()
            .filter(|transition| transition.location == Location::Project)
        {
            if !tree_has_state(&entries, &transition.path, &transition.before) {
                return invalid(format!(
                    "step `{}` before state is not present in projected tree at `{}`",
                    step.id, transition.path
                ));
            }
            set_tree_state(&mut entries, &transition.path, &transition.after)?;
        }
    }
    let entries = entries.into_values().collect::<Vec<_>>();
    Ok(TreeManifest {
        // The plan owns the canonical digest. Equality below compares entries;
        // replace the digest at the comparison boundary.
        digest: tree_digest(&entries),
        entries,
    })
}

fn tree_has_state(entries: &BTreeMap<String, TreeEntry>, path: &str, state: &PathState) -> bool {
    match state {
        PathState::Absent => !entries.keys().any(|entry| at_or_below(entry, path)),
        PathState::File(file) => entries
            .get(path)
            .is_some_and(|entry| entry_matches_file(entry, file)),
        PathState::EmptyDirectory { mode } => {
            entries
                .get(path)
                .is_some_and(|entry| entry.kind == TreeEntryKind::Directory && entry.mode == *mode)
                && !entries
                    .keys()
                    .any(|entry| entry != path && at_or_below(entry, path))
        }
        PathState::Tree(tree) => subtree_matches(entries, path, tree),
    }
}

fn set_tree_state(
    entries: &mut BTreeMap<String, TreeEntry>,
    path: &str,
    state: &PathState,
) -> Result<(), TransactionError> {
    let removals = entries
        .keys()
        .filter(|entry| at_or_below(entry, path))
        .cloned()
        .collect::<Vec<_>>();
    for removal in removals {
        entries.remove(&removal);
    }
    match state {
        PathState::Absent => {}
        PathState::File(file) => {
            entries.insert(
                path.to_owned(),
                TreeEntry {
                    path: path.to_owned(),
                    kind: TreeEntryKind::File,
                    sha256: Some(file.sha256.clone()),
                    bytes: Some(file.bytes),
                    mode: file.mode,
                },
            );
        }
        PathState::EmptyDirectory { mode } => {
            entries.insert(
                path.to_owned(),
                TreeEntry {
                    path: path.to_owned(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: *mode,
                },
            );
        }
        PathState::Tree(tree) => {
            entries.insert(
                path.to_owned(),
                TreeEntry {
                    path: path.to_owned(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: tree.root_mode,
                },
            );
            for entry in &tree.descendants {
                let absolute = format!("{path}/{}", entry.relative_path);
                entries.insert(
                    absolute.clone(),
                    TreeEntry {
                        path: absolute,
                        kind: entry.kind,
                        sha256: entry.sha256.clone(),
                        bytes: entry.bytes,
                        mode: entry.mode,
                    },
                );
            }
        }
    }
    Ok(())
}

fn subtree_matches(
    entries: &BTreeMap<String, TreeEntry>,
    root: &str,
    expected: &SubtreeState,
) -> bool {
    let Some(root_entry) = entries.get(root) else {
        return false;
    };
    if root_entry.kind != TreeEntryKind::Directory || root_entry.mode != expected.root_mode {
        return false;
    }
    let prefix = format!("{root}/");
    let actual = entries
        .values()
        .filter_map(|entry| {
            let relative = entry.path.strip_prefix(&prefix)?;
            Some(SubtreeEntry {
                relative_path: relative.to_owned(),
                kind: entry.kind,
                sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                mode: entry.mode,
            })
        })
        .collect::<Vec<_>>();
    actual == expected.descendants
}

fn tree_digest(entries: &[TreeEntry]) -> Digest {
    logical_tree_manifest(entries.to_vec()).digest
}
