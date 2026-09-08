fn canonical_plan(
    bytes: &[u8],
    plan_id: &Digest,
    display_root: &str,
    mode: TransactionMode,
) -> Result<(), TransactionError> {
    if bytes.is_empty() || bytes.len() > MAX_CANONICAL_PLAN_BYTES {
        return invalid("embedded canonical plan is empty or exceeds 16 MiB");
    }
    let plan: vibe_wire::generated::scrape::e1::plan::Plan = serde_json::from_slice(bytes)
        .map_err(|error| invalid_error(format!("embedded canonical plan is invalid: {error}")))?;
    let canonical = serde_json::to_vec(&plan)
        .map_err(|error| invalid_error(format!("canonical plan serialization failed: {error}")))?;
    if canonical != bytes {
        return invalid("embedded canonical plan bytes are not canonical generated JSON");
    }
    let expected_mode = match mode {
        TransactionMode::Export => vibe_wire::generated::scrape::e1::plan::Mode::Export,
        TransactionMode::InPlace => vibe_wire::generated::scrape::e1::plan::Mode::InPlace,
    };
    if plan.schema != 1
        || plan.plan_id != plan_id.0
        || plan.project.display_root != display_root
        || plan.mode != expected_mode
        || plan.command != vibe_wire::generated::scrape::e1::plan::Command::Scrape
    {
        return invalid("embedded canonical plan identity/mode/schema differs from transaction");
    }
    Ok(())
}

fn execution(
    mode: TransactionMode,
    value: &PreparedMode,
    snapshots: &BTreeSet<&str>,
) -> Result<(), TransactionError> {
    match (mode, value) {
        (TransactionMode::Export, PreparedMode::Export(plan)) => export(plan, snapshots),
        (TransactionMode::InPlace, PreparedMode::InPlace(plan)) => in_place(plan, snapshots),
        _ => corrupt("journal mode and executable-plan variant differ"),
    }
}

fn snapshots(records: &[SnapshotRecord]) -> Result<(), TransactionError> {
    let mut names = BTreeSet::new();
    let mut kinds = BTreeMap::<SnapshotKind, usize>::new();
    for record in records {
        token(&record.name, "snapshot name")?;
        digest(&record.sha256)?;
        if !names.insert(record.name.as_str()) {
            return invalid("snapshot names must be globally unique");
        }
        *kinds.entry(record.kind).or_default() += 1;
    }
    if kinds.get(&SnapshotKind::Contract) != Some(&1)
        || kinds.get(&SnapshotKind::CanonicalContract) != Some(&1)
        || kinds.get(&SnapshotKind::CanonicalPlan) != Some(&1)
    {
        return invalid(
            "exactly one raw contract, canonical contract and canonical plan are required",
        );
    }
    Ok(())
}

fn prepared_after_records(
    mode: &PreparedMode,
    records: &[SnapshotRecord],
) -> Result<(), TransactionError> {
    let by_name = records
        .iter()
        .map(|record| (record.name.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    match mode {
        PreparedMode::Export(plan) => {
            for entry in &plan.entries {
                let Some(ExportPayload::PreparedAfter { snapshot_name }) = &entry.payload else {
                    continue;
                };
                let record = by_name.get(snapshot_name.as_str()).ok_or_else(|| {
                    invalid_error("export prepared-after snapshot record is absent")
                })?;
                let final_entry = plan
                    .final_manifest
                    .entries
                    .iter()
                    .find(|final_entry| final_entry.path == entry.target_path)
                    .ok_or_else(|| invalid_error("export final entry is absent"))?;
                if record.kind != SnapshotKind::PreparedAfter
                    || final_entry.sha256.as_ref() != Some(&record.sha256)
                    || final_entry.bytes != Some(record.bytes)
                    || final_entry.mode != record.mode
                {
                    return invalid("export prepared-after record differs from final file state");
                }
            }
        }
        PreparedMode::InPlace(plan) => {
            validate_contract_binding(plan, records)?;
            for step in plan
                .steps
                .iter()
                .filter(|step| step.kind == MutationKind::AtomicRewrite)
            {
                let name = format!("after/{}", step.id);
                let record = by_name.get(name.as_str()).ok_or_else(|| {
                    invalid_error("rewrite prepared-after snapshot record is absent")
                })?;
                let transition = project_transition(step)?;
                let PathState::File(after) = &transition.after else {
                    return invalid("rewrite after state is not a regular file");
                };
                if record.kind != SnapshotKind::PreparedAfter
                    || record.sha256 != after.sha256
                    || record.bytes != after.bytes
                    || record.mode != after.mode
                {
                    return invalid("rewrite prepared-after record differs from after state");
                }
            }
        }
    }
    Ok(())
}

fn snapshot_names(records: &[SnapshotRecord]) -> BTreeSet<&str> {
    records.iter().map(|record| record.name.as_str()).collect()
}

fn export(plan: &ExportPlan, snapshots: &BTreeSet<&str>) -> Result<(), TransactionError> {
    token(&plan.output_name, "output name")?;
    if plan.output_identity.is_empty()
        || plan.output_parent_identity.is_empty()
        || plan.output_display_path.is_empty()
    {
        return invalid("export output, parent and display identities must be sealed");
    }
    canonical_tree(&plan.source_tree)?;
    canonical_tree(&plan.final_manifest)?;
    let mut previous: Option<&str> = None;
    for entry in &plan.entries {
        path(&entry.target_path)?;
        if is_git(&entry.target_path) {
            return invalid(".git is not an export entry");
        }
        if previous.is_some_and(|prior| prior.as_bytes() >= entry.target_path.as_bytes()) {
            return invalid("export entries must be unique and byte-sorted");
        }
        previous = Some(&entry.target_path);
        match (&entry.kind, &entry.payload) {
            (TreeEntryKind::Directory, None) => {}
            (
                TreeEntryKind::File,
                Some(ExportPayload::Source {
                    source_path,
                    before,
                }),
            ) => {
                path(source_path)?;
                file_state(before)?;
                let source = plan
                    .source_tree
                    .entries
                    .iter()
                    .find(|source| source.path == *source_path)
                    .ok_or_else(|| invalid_error("export source is absent from source seal"))?;
                if !entry_matches_file(source, before) {
                    return invalid("export source payload differs from the source seal");
                }
            }
            (TreeEntryKind::File, Some(ExportPayload::PreparedAfter { snapshot_name })) => {
                if !snapshots.contains(snapshot_name.as_str()) {
                    return invalid("export prepared-after payload names an absent snapshot");
                }
            }
            _ => return invalid("export file/directory payload shape is inconsistent"),
        }
    }
    let manifest_paths = plan
        .final_manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    let entry_paths = plan
        .entries
        .iter()
        .map(|entry| entry.target_path.as_str())
        .collect::<Vec<_>>();
    if manifest_paths != entry_paths {
        return invalid("export entry list must equal the complete final manifest");
    }
    Ok(())
}

fn in_place(plan: &InPlacePlan, snapshots: &BTreeSet<&str>) -> Result<(), TransactionError> {
    if plan.quarantine_parent_identity.is_empty() {
        return invalid("quarantine parent identity is not sealed");
    }
    canonical_tree(&plan.before_tree)?;
    canonical_tree(&plan.pre_contract_tree)?;
    canonical_tree(&plan.post_contract_tree)?;
    canonical_tree(&plan.after_tree)?;
    let mut ids = BTreeSet::new();
    let mut prior_rank = 0_u8;
    let mut prior_rewrite_path: Option<String> = None;
    let mut relocation_parent_paths = Vec::new();
    let mut relocation_paths = Vec::new();
    let mut removal_paths = Vec::new();
    let mut prune_paths = Vec::new();
    for (index, step) in plan.steps.iter().enumerate() {
        mutation(step, snapshots)?;
        reject_internal_mutation_id(&step.id)?;
        if !ids.insert(step.id.as_str()) {
            return invalid("mutation ids must be unique");
        }
        if matches!(
            step.kind,
            MutationKind::ContractDeleteLast
                | MutationKind::ContractAncestorTreePark
                | MutationKind::ContractExternalPreserve
        ) {
            return invalid("contract mutation may occur only in the final boundary slot");
        }
        let rank = mutation_rank(step.kind);
        if rank < prior_rank {
            return invalid("mutation order must be rewrite, relocation, removal, prune");
        }
        prior_rank = rank;
        match step.kind {
            MutationKind::CaptureBeforeImage => {
                let next = plan.steps.get(index + 1).ok_or_else(|| {
                    invalid_error("capture-before-image must be followed by atomic rewrite")
                })?;
                if next.kind != MutationKind::AtomicRewrite || next.pair_id != step.pair_id {
                    return invalid("capture-before-image/rewrite pair is not adjacent and exact");
                }
                let captured = project_transition(step)?;
                let rewritten = project_transition(next)?;
                if captured.path != rewritten.path || captured.before != rewritten.before {
                    return invalid(
                        "rewrite before state/path differs from its captured quarantine before-image",
                    );
                }
                let path = captured.path.clone();
                if prior_rewrite_path
                    .as_ref()
                    .is_some_and(|prior| prior.as_bytes() >= path.as_bytes())
                {
                    return invalid("rewrite pairs must be byte-sorted by project path");
                }
                prior_rewrite_path = Some(path);
            }
            MutationKind::AtomicRewrite => {
                let prior = index.checked_sub(1).and_then(|at| plan.steps.get(at));
                if prior.is_none_or(|prior| {
                    prior.kind != MutationKind::CaptureBeforeImage || prior.pair_id != step.pair_id
                }) {
                    return invalid("atomic rewrite has no adjacent capture-before-image pair");
                }
            }
            MutationKind::QuarantineFile => {
                removal_paths.push(project_transition(step)?.path.clone())
            }
            MutationKind::PruneEmptyDirectory => {
                prune_paths.push(project_transition(step)?.path.clone())
            }
            MutationKind::CreateRelocationParent => {
                relocation_parent_paths.push(project_transition(step)?.path.clone())
            }
            MutationKind::Relocate => relocation_paths.push(step.transitions[0].path.clone()),
            MutationKind::ContractDeleteLast
            | MutationKind::ContractAncestorTreePark
            | MutationKind::ContractExternalPreserve => {
                unreachable!()
            }
        }
    }
    if !removal_paths
        .windows(2)
        .all(|pair| pair[0].as_bytes() < pair[1].as_bytes())
    {
        return invalid("file removals must be in canonical byte order");
    }
    if !relocation_parent_paths.windows(2).all(|pair| {
        path_depth(&pair[0]) < path_depth(&pair[1])
            || (path_depth(&pair[0]) == path_depth(&pair[1])
                && pair[0].as_bytes() < pair[1].as_bytes())
    }) {
        return invalid("relocation parents must be shallowest-first then byte-sorted");
    }
    if !relocation_paths
        .windows(2)
        .all(|pair| pair[0].as_bytes() < pair[1].as_bytes())
    {
        return invalid("relocations must be byte-sorted by source path");
    }
    if !prune_paths.windows(2).all(|pair| {
        path_depth(&pair[0]) > path_depth(&pair[1])
            || (path_depth(&pair[0]) == path_depth(&pair[1])
                && pair[0].as_bytes() < pair[1].as_bytes())
    }) {
        return invalid("directory pruning must be deepest-first then byte-sorted");
    }
    mutation(&plan.contract_step, snapshots)?;
    reject_internal_mutation_id(&plan.contract_step.id)?;
    if !ids.insert(plan.contract_step.id.as_str()) {
        return invalid("contract mutation id duplicates an ordinary mutation");
    }
    match plan.contract_step.kind {
        MutationKind::ContractDeleteLast | MutationKind::ContractExternalPreserve => {}
        _ => return invalid("contract boundary slot has a non-contract mutation"),
    }
    match (&plan.contract, plan.contract_step.kind) {
        (ContractCommit::DeleteLast { .. }, MutationKind::ContractDeleteLast)
        | (ContractCommit::ExternalPreserve, MutationKind::ContractExternalPreserve) => {}
        _ => return invalid("contract identity and contract mutation kind differ"),
    }
    match (&plan.contract, &plan.contract_cleanup_step) {
        (
            ContractCommit::DeleteLast {
                empty_ancestors, ..
            },
            Some(step),
        ) if !empty_ancestors.is_empty() && step.kind == MutationKind::ContractAncestorTreePark => {
            mutation(step, snapshots)?;
            reject_internal_mutation_id(&step.id)?;
            if !ids.insert(step.id.as_str()) {
                return invalid("contract cleanup mutation id is not unique");
            }
        }
        (
            ContractCommit::DeleteLast {
                empty_ancestors, ..
            },
            None,
        ) if empty_ancestors.is_empty() => {}
        (ContractCommit::ExternalPreserve, None) => {}
        _ => return invalid("contract cleanup step does not match contract ancestor policy"),
    }

    let projected = apply_project_steps(&plan.before_tree, &plan.steps)?;
    if projected.entries != plan.pre_contract_tree.entries {
        return invalid("ordinary mutation algebra does not yield pre-contract tree seal");
    }
    let projected = apply_project_steps(&projected, std::slice::from_ref(&plan.contract_step))?;
    if projected.entries != plan.post_contract_tree.entries {
        return invalid("contract file move algebra does not yield post-contract tree seal");
    }
    let projected = if let Some(step) = &plan.contract_cleanup_step {
        apply_project_steps(&projected, std::slice::from_ref(step))?
    } else {
        projected
    };
    if projected.entries != plan.after_tree.entries {
        return invalid("contract cleanup algebra does not yield final tree seal");
    }
    Ok(())
}

fn validate_contract_binding(
    plan: &InPlacePlan,
    records: &[SnapshotRecord],
) -> Result<(), TransactionError> {
    let contract = records
        .iter()
        .find(|record| record.kind == SnapshotKind::Contract)
        .ok_or_else(|| invalid_error("contract snapshot is absent"))?;
    match &plan.contract {
        ContractCommit::ExternalPreserve => {
            if plan.contract_step.kind != MutationKind::ContractExternalPreserve
                || !plan.contract_step.transitions.is_empty()
            {
                return invalid("external preserve must have no project contract transition");
            }
        }
        ContractCommit::DeleteLast {
            path: contract_path,
            empty_ancestors,
        } => {
            path(contract_path)?;
            let source = plan
                .contract_step
                .transitions
                .iter()
                .find(|transition| {
                    transition.location == Location::Project
                        && matches!(transition.before, PathState::File(_))
                        && transition.after == PathState::Absent
                })
                .ok_or_else(|| invalid_error("contract delete source is absent"))?;
            let PathState::File(state) = &source.before else {
                unreachable!()
            };
            if source.path != *contract_path
                || state.sha256 != contract.sha256
                || state.bytes != contract.bytes
                || state.mode != contract.mode
            {
                return invalid(
                    "contract delete source is not the exact durable contract snapshot file",
                );
            }
            let mut current = contract_path.as_str();
            for ancestor in empty_ancestors {
                let expected = current
                    .rsplit_once('/')
                    .map(|(parent, _)| parent)
                    .ok_or_else(|| {
                        invalid_error("contract empty ancestor chain extends above project root")
                    })?;
                if ancestor != expected {
                    return invalid(
                        "contract empty ancestors are not an exact deepest-first chain",
                    );
                }
                current = ancestor;
            }
            match (empty_ancestors.last(), &plan.contract_cleanup_step) {
                (None, None) => {}
                (Some(topmost), Some(cleanup)) => {
                    let project = &cleanup.transitions[0];
                    if project.path != *topmost {
                        return invalid(
                            "contract cleanup tree does not start at the topmost empty ancestor",
                        );
                    }
                }
                _ => return invalid("contract empty ancestors lack one cleanup tree step"),
            }
        }
    }
    Ok(())
}
