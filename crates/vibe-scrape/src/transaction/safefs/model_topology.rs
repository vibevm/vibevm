fn set_flat_state(entries: &mut BTreeMap<String, TreeEntry>, path: &str, state: &PathState) {
    entries.retain(|candidate, _| candidate != path && !candidate.starts_with(&format!("{path}/")));
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
            for child in &tree.descendants {
                let child_path = format!("{path}/{}", child.relative_path);
                entries.insert(
                    child_path.clone(),
                    TreeEntry {
                        path: child_path,
                        kind: child.kind,
                        sha256: child.sha256.clone(),
                        bytes: child.bytes,
                        mode: child.mode,
                    },
                );
            }
        }
    }
}

fn model_digest(value: &str) -> Digest {
    if value.starts_with("sha256:") {
        Digest(value.to_owned())
    } else {
        Digest(format!("sha256:{value}"))
    }
}

fn digest_bytes(bytes: &[u8]) -> Digest {
    Digest(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn one_at(
    step: &MutationStep,
    location: Location,
) -> Result<&super::PathTransition, TransactionError> {
    let mut found = step
        .transitions
        .iter()
        .filter(|transition| transition.location == location);
    let answer = found
        .next()
        .ok_or_else(|| invalid_step_error(step, "required transition is absent"))?;
    if found.next().is_some() {
        return invalid_step(step, "required transition is not unique");
    }
    Ok(answer)
}

fn move_pair(
    step: &MutationStep,
) -> Result<(&super::PathTransition, &super::PathTransition), TransactionError> {
    let source = step
        .transitions
        .iter()
        .find(|transition| {
            transition.before != PathState::Absent && transition.after == PathState::Absent
        })
        .ok_or_else(|| invalid_step_error(step, "move source is absent"))?;
    let destination = step
        .transitions
        .iter()
        .find(|transition| {
            transition.before == PathState::Absent && transition.after == source.before
        })
        .ok_or_else(|| invalid_step_error(step, "move destination is absent"))?;
    Ok((source, destination))
}

fn invalid_step<T>(step: &MutationStep, detail: &str) -> Result<T, TransactionError> {
    Err(invalid_step_error(step, detail))
}

fn invalid_step_error(step: &MutationStep, detail: &str) -> TransactionError {
    TransactionError::InvalidPrepared(format!("step `{}`: {detail}", step.id))
}

fn third_step<T>(step: &MutationStep, detail: &str) -> Result<T, TransactionError> {
    Err(TransactionError::ThirdState(format!(
        "step `{}`: {detail}",
        step.id
    )))
}

fn parked_directory(step: &MutationStep) -> String {
    format!("pruned/{}", step.id)
}

fn quarantine_topology(plan: &InPlacePlan) -> Vec<String> {
    let mut directories = BTreeSet::new();
    let mut add_parents = |path: &str| {
        let components = path.split('/').collect::<Vec<_>>();
        for count in 1..components.len() {
            directories.insert(components[..count].join("/"));
        }
    };
    for step in plan
        .steps
        .iter()
        .chain(std::iter::once(&plan.contract_step))
        .chain(plan.contract_cleanup_step.iter())
    {
        for transition in step
            .transitions
            .iter()
            .filter(|transition| transition.location == Location::Quarantine)
        {
            add_parents(&transition.path);
        }
        if step.kind == MutationKind::PruneEmptyDirectory {
            add_parents(&parked_directory(step));
        }
    }
    let mut directories = directories.into_iter().collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        path_depth(left)
            .cmp(&path_depth(right))
            .then_with(|| left.as_bytes().cmp(right.as_bytes()))
    });
    directories
}

fn path_depth(path: &str) -> usize {
    path.bytes().filter(|byte| *byte == b'/').count()
}
