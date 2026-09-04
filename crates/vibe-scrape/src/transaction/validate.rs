//! Closed preparation and recovery-journal validation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::{BTreeMap, BTreeSet};

use super::model::*;
use super::sha256::digest as bytes_digest;

pub fn prepared(value: &PreparedTransaction) -> Result<(), TransactionError> {
    digest(&value.plan_id)?;
    if value.project_identity_token.is_empty() || value.project_display_root.is_empty() {
        return invalid("project identity token/display root is empty");
    }
    canonical_plan(
        &value.canonical_plan,
        &value.plan_id,
        &value.project_display_root,
        value.mode(),
    )?;
    let records = value
        .snapshots
        .iter()
        .map(|snapshot| SnapshotRecord {
            kind: snapshot.kind,
            name: snapshot.name.clone(),
            sha256: bytes_digest(&snapshot.bytes),
            bytes: snapshot.bytes.len() as u64,
            mode: snapshot.mode,
        })
        .collect::<Vec<_>>();
    snapshots(&records)?;
    let embedded_snapshot = value
        .snapshots
        .iter()
        .find(|snapshot| snapshot.kind == SnapshotKind::CanonicalPlan)
        .ok_or_else(|| invalid_error("canonical plan snapshot is absent"))?;
    if embedded_snapshot.bytes != value.canonical_plan {
        return invalid("embedded canonical plan differs from its snapshot");
    }
    execution(value.mode(), &value.mode, &snapshot_names(&records))?;
    prepared_after_records(&value.mode, &records)
}

pub fn journal(
    value: &Journal,
    expected_key: &ProjectKey,
    display_root: &str,
) -> Result<(), TransactionError> {
    if value.schema != 1 || &value.project_key != expected_key {
        return corrupt("journal schema/project key mismatch");
    }
    digest(&Digest(value.project_key.0.clone()))?;
    digest(&value.plan_id)?;
    canonical_plan(
        &value.canonical_plan,
        &value.plan_id,
        &value.project_display_root,
        value.mode,
    )?;
    transaction_id(&value.transaction_id)?;
    if value.project_display_root != display_root {
        return corrupt("journal display root differs from the locked recovery root");
    }
    snapshots(&value.snapshots)?;
    if value.snapshots_persisted > value.snapshots.len() {
        return corrupt("snapshot progress exceeds the expected snapshot set");
    }
    if let Some(active) = value.snapshot_active
        && (active != value.snapshots_persisted || active >= value.snapshots.len())
    {
        return corrupt("active snapshot is not the exact next durable-prefix member");
    }
    execution(
        value.mode,
        &value.execution,
        &snapshot_names(&value.snapshots),
    )?;
    prepared_after_records(&value.execution, &value.snapshots)?;
    state_for_mode(value.mode, &value.state)?;
    validate_names(value)?;
    validate_verification_workspace(value)?;
    validate_counters(value)?;
    validate_progress(value)?;
    validate_cleanup(value)?;
    validate_state_progress(value)?;
    validate_verification(value)?;
    validate_report(value)?;
    if value
        .settlement_intent
        .is_some_and(|outcome| outcome != Outcome::Refused)
        || (value.settlement_intent.is_some() && value.mode != TransactionMode::Export)
    {
        return corrupt("settlement intent is outside typed export refusal direction");
    }
    if let (Some(intent), Some(report)) = (value.settlement_intent, value.report.as_ref())
        && report.outcome != intent
        && !(intent == Outcome::Refused && report.outcome == Outcome::RollbackFailed)
    {
        return corrupt("embedded report contradicts durable settlement intent");
    }
    let incomplete_refusal = value.state == TransactionState::Complete
        && value
            .report
            .as_ref()
            .is_some_and(|report| report.outcome == Outcome::Refused);
    if value.state != TransactionState::Preparing
        && !incomplete_refusal
        && value.snapshots_persisted != value.snapshots.len()
    {
        return corrupt("non-preparing journal has incomplete snapshot progress");
    }
    if value.state != TransactionState::Preparing
        && !incomplete_refusal
        && value.snapshot_active.is_some()
    {
        return corrupt("non-preparing journal carries an active snapshot intent");
    }
    if value.state == TransactionState::Preparing
        && (value.candidate_name.is_some()
            || value.quarantine_name.is_some()
            || value.owned_tree_token.is_some()
            || value.owned_tree_seal.is_some()
            || value.cleanup_wal.is_some()
            || value.completed_steps != 0
            || value.active_step.is_some()
            || !value.actual_mutations.is_empty()
            || !value.verification.is_empty()
            || value.report.is_some())
    {
        return corrupt("preparation journal carries post-preparation state");
    }
    Ok(())
}

fn validate_verification_workspace(value: &Journal) -> Result<(), TransactionError> {
    let Some(workspace) = &value.verification_workspace else {
        if value.state != TransactionState::Preparing {
            return corrupt("non-preparing journal has no verification workspace");
        }
        return Ok(());
    };
    if workspace.name != "v"
        || workspace.display_root.is_empty()
        || !std::path::Path::new(&workspace.display_root).is_absolute()
    {
        return corrupt("verification workspace name/display root is invalid");
    }
    digest(&Digest(workspace.ownership_token.clone()))?;
    let mut material = b"vibe-scrape-verification-workspace-e1\0".to_vec();
    material.extend_from_slice(value.project_key.0.as_bytes());
    material.push(0);
    material.extend_from_slice(value.transaction_id.0.as_bytes());
    if workspace.ownership_token != bytes_digest(&material).0 {
        return corrupt("verification workspace ownership token is not transaction-derived");
    }
    Ok(())
}

const MAX_CANONICAL_PLAN_BYTES: usize = 16 * 1024 * 1024;

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

fn validate_names(value: &Journal) -> Result<(), TransactionError> {
    let (expected_name, forbidden_name, role) = match value.mode {
        TransactionMode::Export => (
            value.candidate_name.as_deref(),
            value.quarantine_name.as_deref(),
            "export",
        ),
        TransactionMode::InPlace => (
            value.quarantine_name.as_deref(),
            value.candidate_name.as_deref(),
            "quarantine",
        ),
    };
    if forbidden_name.is_some() {
        return corrupt("journal carries the other mode's owned sibling name");
    }
    if expected_name.is_some() != value.owned_tree_token.is_some() {
        return corrupt("owned sibling name/token presence differs");
    }
    if let Some(name) = expected_name {
        let prefix = match value.mode {
            TransactionMode::Export => ".vibe-scrape-candidate-",
            TransactionMode::InPlace => ".vibe-scrape-quarantine-",
        };
        if name != format!("{prefix}{}", value.transaction_id.0) {
            return corrupt("owned sibling name does not derive from transaction id");
        }
        let expected = ownership_token(value, role);
        if value.owned_tree_token.as_deref() != Some(expected.as_str()) {
            return corrupt("owned sibling token does not derive from journal identity");
        }
    }
    Ok(())
}

fn validate_counters(value: &Journal) -> Result<(), TransactionError> {
    let maximum = match &value.execution {
        PreparedMode::Export(plan) => plan.entries.len(),
        PreparedMode::InPlace(plan) => {
            plan.steps.len() + 1 + usize::from(plan.contract_cleanup_step.is_some())
        }
    };
    if value.completed_steps > maximum {
        return corrupt("completed step count exceeds executable plan");
    }
    if let Some(active) = value.active_step
        && (active >= maximum || active != value.completed_steps)
    {
        return corrupt("active step is not the exact next uncheckpointed step");
    }
    Ok(())
}

fn validate_progress(value: &Journal) -> Result<(), TransactionError> {
    let expected = expected_progress(&value.execution);
    if value.mutation_progress.len() != expected.len() {
        return corrupt("mutation progress does not cover the exact planned mutation set");
    }
    for (actual, expected) in value.mutation_progress.iter().zip(expected) {
        if actual.id != expected.id || actual.kind != expected.kind {
            return corrupt("mutation progress order/identity differs from executable plan");
        }
    }
    for actual in &value.actual_mutations {
        if !value
            .mutation_progress
            .iter()
            .any(|planned| planned.id == actual.id && planned.kind == actual.kind)
        {
            return corrupt("actual mutation evidence names an unplanned mutation");
        }
        match (actual.direction, actual.status) {
            (MutationDirection::Apply, MutationStatus::Applied)
            | (MutationDirection::Rollback, MutationStatus::RolledBack) => {}
            _ => return corrupt("actual mutation evidence carries intent/planned status"),
        }
    }
    for progress in &value.mutation_progress {
        let applied = value.actual_mutations.iter().any(|actual| {
            actual.id == progress.id
                && actual.direction == MutationDirection::Apply
                && actual.status == MutationStatus::Applied
        });
        let rolled_back = value.actual_mutations.iter().any(|actual| {
            actual.id == progress.id
                && actual.direction == MutationDirection::Rollback
                && actual.status == MutationStatus::RolledBack
        });
        match progress.status {
            MutationStatus::Planned | MutationStatus::NoMutation | MutationStatus::ApplyIntent
                if rolled_back =>
            {
                return corrupt("planned/apply-intent mutation has rollback evidence");
            }
            MutationStatus::NoMutation if applied => {
                return corrupt("no-mutation step carries applied evidence");
            }
            MutationStatus::Applied if !applied || rolled_back => {
                return corrupt("applied mutation evidence/status is inconsistent");
            }
            MutationStatus::RollbackIntent if !applied || rolled_back => {
                return corrupt("rollback intent lacks exactly one applied direction");
            }
            MutationStatus::RolledBack if !applied || !rolled_back => {
                return corrupt("rolled-back mutation lacks both actual directions");
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_cleanup(value: &Journal) -> Result<(), TransactionError> {
    let Some(wal) = &value.cleanup_wal else {
        return Ok(());
    };
    let refusal_cleanup = value.mode == TransactionMode::Export
        && value.state == TransactionState::Candidate
        && value.settlement_intent == Some(Outcome::Refused);
    if !matches!(
        value.state,
        TransactionState::RollingBack
            | TransactionState::CleanupPending
            | TransactionState::RolledBack
            | TransactionState::RollbackFailed
            | TransactionState::Complete
    ) && !refusal_cleanup
    {
        return corrupt("owned-tree cleanup WAL exists outside cleanup-capable state");
    }
    let seal = value
        .owned_tree_seal
        .as_ref()
        .ok_or_else(|| invalid_error("cleanup WAL has no owned-tree seal"))?;
    if value.owned_tree_token.is_none()
        || wal.directory_identity != seal.directory_identity
        || wal.manifest_digest != seal.manifest_digest
    {
        return corrupt("cleanup WAL binding differs from owned-tree evidence");
    }
    let expected_name = match value.mode {
        TransactionMode::Export => value.candidate_name.as_deref(),
        TransactionMode::InPlace => value.quarantine_name.as_deref(),
    };
    if expected_name != Some(wal.name.as_str()) {
        return corrupt("cleanup WAL name differs from the journaled owned sibling");
    }
    let order = cleanup_order(seal);
    if wal.completed.len() > order.len()
        || wal
            .completed
            .iter()
            .zip(&order)
            .any(|(actual, expected)| actual != expected)
    {
        return corrupt("cleanup completion list is not the canonical prefix");
    }
    if let Some(active) = &wal.active {
        if wal.completed.len() >= order.len()
            || active.progress_key != order[wal.completed.len()]
            || active.expected.path != active.path
            || active.intent_token.is_empty()
        {
            return corrupt("active cleanup intent is not the exact next canonical entry");
        }
        if active.root {
            if active.progress_key != "root"
                || active.path != wal.name
                || active.expected.kind != TreeEntryKind::Directory
            {
                return corrupt("root cleanup intent has an invalid shape");
            }
        } else {
            let expected = seal
                .entries
                .iter()
                .find(|entry| cleanup_key(entry) == active.progress_key)
                .ok_or_else(|| invalid_error("cleanup intent entry is absent from its seal"))?;
            if &active.expected != expected {
                return corrupt("cleanup intent expected state differs from its sealed entry");
            }
        }
    }
    Ok(())
}

fn cleanup_order(seal: &OwnedTreeSeal) -> Vec<String> {
    let mut files = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::File)
        .map(cleanup_key)
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let mut directories = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        path_depth(&right.path)
            .cmp(&path_depth(&left.path))
            .then_with(|| right.path.as_bytes().cmp(left.path.as_bytes()))
    });
    files.extend(directories.into_iter().map(cleanup_key));
    files.push("root".to_owned());
    files
}

fn cleanup_key(entry: &OwnedEntrySeal) -> String {
    format!(
        "{}:{}",
        match entry.kind {
            TreeEntryKind::File => "file",
            TreeEntryKind::Directory => "directory",
        },
        entry.path
    )
}

fn validate_state_progress(value: &Journal) -> Result<(), TransactionError> {
    let status = |id: &str| {
        value
            .mutation_progress
            .iter()
            .find(|progress| progress.id == id)
            .map(|progress| progress.status)
    };
    if value.state == TransactionState::Preparing
        && value
            .mutation_progress
            .iter()
            .any(|progress| progress.status != MutationStatus::Planned)
    {
        return corrupt("preparation journal carries mutation progress");
    }
    match (&value.execution, &value.state) {
        (PreparedMode::Export(plan), TransactionState::Candidate) => {
            if value.completed_steps != plan.entries.len() || value.active_step.is_some() {
                return corrupt("candidate state does not have a complete export entry prefix");
            }
        }
        (
            PreparedMode::Export(plan),
            TransactionState::PublishedPendingVerify
            | TransactionState::Verified
            | TransactionState::CleanupPending,
        ) => {
            if value.completed_steps != plan.entries.len()
                || value.active_step.is_some()
                || status("export/publish") != Some(MutationStatus::Applied)
            {
                return corrupt("published/verified export lacks applied publication evidence");
            }
        }
        (PreparedMode::InPlace(_), TransactionState::BeforePassed) => {
            if value.completed_steps != 0 || value.active_step.is_some() {
                return corrupt("before-passed state carries project mutation progress");
            }
        }
        (PreparedMode::InPlace(plan), TransactionState::ContractBoundary(_)) => {
            if value.completed_steps < plan.steps.len() + 1 {
                return corrupt("contract boundary precedes the contract file checkpoint");
            }
        }
        (
            PreparedMode::InPlace(plan),
            TransactionState::Verified | TransactionState::CleanupPending,
        ) => {
            let expected = plan.steps.len() + 1 + usize::from(plan.contract_cleanup_step.is_some());
            if value.completed_steps != expected || value.active_step.is_some() {
                return corrupt(
                    "verified in-place state lacks complete contract-boundary progress",
                );
            }
        }
        _ => {}
    }
    let owned_removed = value.mutation_progress.iter().any(|progress| {
        matches!(
            progress.id.as_str(),
            "export/candidate" | "in-place/quarantine"
        ) && matches!(
            progress.status,
            MutationStatus::RollbackIntent | MutationStatus::RolledBack
        )
    });
    if matches!(
        value.state,
        TransactionState::Candidate
            | TransactionState::PublishedPendingVerify
            | TransactionState::Verified
            | TransactionState::CleanupPending
    ) && (value.owned_tree_token.is_none()
        || (value.owned_tree_seal.is_none() && !owned_removed))
    {
        return corrupt("owned mutation state has no sibling ownership token");
    }
    if value.mode == TransactionMode::InPlace
        && matches!(
            value.state,
            TransactionState::Mutating
                | TransactionState::ContractBoundary(_)
                | TransactionState::Verified
                | TransactionState::CleanupPending
        )
        && (value.owned_tree_token.is_none() || (value.owned_tree_seal.is_none() && !owned_removed))
    {
        return corrupt("in-place mutation state has no quarantine ownership token");
    }
    if value.owned_tree_seal.is_some() && value.owned_tree_token.is_none() {
        return corrupt("owned tree seal has no ownership seed");
    }
    if let Some(seal) = &value.owned_tree_seal {
        if seal.directory_identity.is_empty() || seal.manifest_digest.is_empty() {
            return corrupt("owned tree seal has incomplete identities");
        }
        let mut prior = None;
        for entry in &seal.entries {
            path(&entry.path)?;
            if entry.identity.is_empty()
                || prior.is_some_and(|path: &str| path.as_bytes() >= entry.path.as_bytes())
            {
                return corrupt("owned tree seal entries are incomplete or non-canonical");
            }
            prior = Some(entry.path.as_str());
        }
    }
    Ok(())
}

fn expected_progress(mode: &PreparedMode) -> Vec<PlannedMutationEvidence> {
    match mode {
        PreparedMode::Export(plan) => {
            let mut values = vec![PlannedMutationEvidence {
                id: "export/candidate".to_owned(),
                kind: PlannedMutationKind::ExportCandidateCreate,
            }];
            values.extend(plan.entries.iter().enumerate().map(|(index, entry)| {
                PlannedMutationEvidence {
                    id: format!("export/entry/{index}/{}", entry.target_path),
                    kind: PlannedMutationKind::ExportEntry,
                }
            }));
            values.push(PlannedMutationEvidence {
                id: "export/publish".to_owned(),
                kind: PlannedMutationKind::ExportPublish,
            });
            values
        }
        PreparedMode::InPlace(plan) => {
            let mut values = vec![PlannedMutationEvidence {
                id: "in-place/quarantine".to_owned(),
                kind: PlannedMutationKind::InPlaceQuarantineCreate,
            }];
            values.extend(
                plan.steps
                    .iter()
                    .chain(std::iter::once(&plan.contract_step))
                    .chain(plan.contract_cleanup_step.iter())
                    .map(|step| PlannedMutationEvidence {
                        id: step.id.clone(),
                        kind: PlannedMutationKind::InPlace(step.kind),
                    }),
            );
            values
        }
    }
}

fn validate_verification(value: &Journal) -> Result<(), TransactionError> {
    let mut seen = BTreeSet::new();
    for record in &value.verification {
        digest(&record.evidence_sha256)?;
        if record.evidence_sha256 != bytes_digest(&record.evidence.canonical_evidence) {
            return corrupt("verification evidence digest mismatch");
        }
        if !seen.insert(record.phase) {
            return corrupt("verification phase is recorded more than once");
        }
    }
    let expected = match value.mode {
        TransactionMode::Export => vec![
            VerificationPhase::Before,
            VerificationPhase::FinalResidual,
            VerificationPhase::AfterHealth,
            VerificationPhase::FinalTree,
            VerificationPhase::SourceUnchanged,
        ],
        TransactionMode::InPlace => vec![
            VerificationPhase::Before,
            VerificationPhase::PreContractResidual,
            VerificationPhase::FinalResidual,
            VerificationPhase::AfterHealth,
            VerificationPhase::FinalTree,
        ],
    };
    let actual = value
        .verification
        .iter()
        .map(|record| record.phase)
        .collect::<Vec<_>>();
    if !expected.starts_with(&actual) {
        return corrupt("verification records are not a canonical phase prefix");
    }
    let verified_commit = matches!(
        value.state,
        TransactionState::Verified | TransactionState::CleanupPending
    ) || (value.state == TransactionState::Complete
        && value
            .report
            .as_ref()
            .is_some_and(|report| report.outcome == Outcome::Verified));
    if verified_commit
        && (actual != expected
            || value
                .verification
                .iter()
                .any(|record| !record.evidence.accepted))
    {
        return corrupt(
            "verified transaction lacks the exact complete accepted commit-gating verification set",
        );
    }
    Ok(())
}

fn validate_report(value: &Journal) -> Result<(), TransactionError> {
    let Some(report) = &value.report else {
        if matches!(
            value.state,
            TransactionState::Complete | TransactionState::RollbackFailed
        ) {
            return corrupt("terminal journal has no report");
        }
        return Ok(());
    };
    if report.project_key != value.project_key
        || report.transaction_id != value.transaction_id
        || report.plan_id != value.plan_id
        || report.mode != value.mode
        || report.snapshots != value.snapshots
        || report.verification != value.verification
        || report.actual_mutations != value.actual_mutations
    {
        return corrupt("report identity/evidence differs from journal");
    }
    let expected_before = match &value.execution {
        PreparedMode::Export(plan) => &plan.source_tree.digest,
        PreparedMode::InPlace(plan) => &plan.before_tree.digest,
    };
    if report.before_tree.as_ref() != Some(expected_before) {
        return corrupt("report before tree differs from executable plan");
    }
    let expected_after = match (report.outcome, &value.execution) {
        (Outcome::Refused, PreparedMode::Export(_)) => None,
        (Outcome::Refused, PreparedMode::InPlace(plan)) => Some(&plan.before_tree.digest),
        (Outcome::Verified | Outcome::RolledBack | Outcome::RollbackFailed, _) => {
            value.delivered_tree.as_ref()
        }
    };
    if report.after_tree.as_ref() != expected_after {
        return corrupt("report after tree differs from its outcome direction");
    }
    let expected_assurance = if cfg!(windows) && report.outcome == Outcome::Verified
        || value
            .verification
            .iter()
            .any(|record| record.evidence.assurance == Assurance::Reduced)
    {
        Assurance::Reduced
    } else {
        Assurance::Full
    };
    if report.assurance != expected_assurance {
        return corrupt("report assurance differs from verification evidence");
    }
    if value.state == TransactionState::Complete && report.cleanup != Cleanup::Complete {
        return corrupt("complete journal has a cleanup-pending report");
    }
    let planned = value
        .mutation_progress
        .iter()
        .map(|progress| PlannedMutationEvidence {
            id: progress.id.clone(),
            kind: progress.kind,
        })
        .collect::<Vec<_>>();
    if report.planned_mutations != planned {
        return corrupt("report planned mutations differ from journal");
    }
    match (&value.state, report.outcome) {
        (TransactionState::Verified | TransactionState::CleanupPending, Outcome::Verified)
        | (TransactionState::RollbackFailed, Outcome::RollbackFailed)
        | (TransactionState::RolledBack, Outcome::RolledBack)
        | (TransactionState::Complete, _) => Ok(()),
        _ => corrupt("report outcome is inconsistent with journal state"),
    }
}

fn state_for_mode(mode: TransactionMode, state: &TransactionState) -> Result<(), TransactionError> {
    let legal = match mode {
        TransactionMode::Export => !matches!(
            state,
            TransactionState::BeforePassed
                | TransactionState::Mutating
                | TransactionState::ContractBoundary(_)
        ),
        TransactionMode::InPlace => !matches!(
            state,
            TransactionState::Candidate | TransactionState::PublishedPendingVerify
        ),
    };
    if legal {
        Ok(())
    } else {
        corrupt("journal state is illegal for its mode")
    }
}

fn canonical_tree(tree: &TreeManifest) -> Result<(), TransactionError> {
    digest(&tree.digest)?;
    let mut previous: Option<&str> = None;
    for entry in &tree.entries {
        path(&entry.path)?;
        if is_git(&entry.path) {
            return invalid("a sealed tree contains protected .git metadata");
        }
        if previous.is_some_and(|prior| prior.as_bytes() >= entry.path.as_bytes()) {
            return invalid("tree manifest entries must be unique and byte-sorted");
        }
        previous = Some(&entry.path);
        match entry.kind {
            TreeEntryKind::File if entry.sha256.is_none() || entry.bytes.is_none() => {
                return invalid("manifest file lacks digest or byte count");
            }
            TreeEntryKind::Directory if entry.sha256.is_some() || entry.bytes.is_some() => {
                return invalid("manifest directory carries file evidence");
            }
            _ => {}
        }
        if let Some(value) = &entry.sha256 {
            digest(value)?;
        }
    }
    Ok(())
}

fn state(value: &PathState) -> Result<(), TransactionError> {
    match value {
        PathState::File(file) => file_state(file),
        PathState::Tree(tree) => canonical_subtree(tree),
        PathState::Absent | PathState::EmptyDirectory { .. } => Ok(()),
    }
}

fn canonical_subtree(tree: &SubtreeState) -> Result<(), TransactionError> {
    digest(&tree.digest)?;
    let mut previous: Option<&str> = None;
    for entry in &tree.descendants {
        path(&entry.relative_path)?;
        if previous.is_some_and(|prior| prior.as_bytes() >= entry.relative_path.as_bytes()) {
            return invalid("subtree descendants must be unique and byte-sorted");
        }
        previous = Some(&entry.relative_path);
        match entry.kind {
            TreeEntryKind::File if entry.sha256.is_none() || entry.bytes.is_none() => {
                return invalid("subtree file lacks digest or byte count");
            }
            TreeEntryKind::Directory if entry.sha256.is_some() || entry.bytes.is_some() => {
                return invalid("subtree directory carries file evidence");
            }
            _ => {}
        }
        if let Some(value) = &entry.sha256 {
            digest(value)?;
        }
    }
    Ok(())
}

fn file_state(value: &FileState) -> Result<(), TransactionError> {
    digest(&value.sha256)
}

fn entry_matches_file(entry: &TreeEntry, state: &FileState) -> bool {
    entry.kind == TreeEntryKind::File
        && entry.sha256.as_ref() == Some(&state.sha256)
        && entry.bytes == Some(state.bytes)
        && entry.mode == state.mode
}

fn digest(value: &Digest) -> Result<(), TransactionError> {
    let Some(hex) = value.0.strip_prefix("sha256:") else {
        return invalid("digest must use sha256:<64-lowercase-hex>");
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return invalid("digest must use sha256:<64-lowercase-hex>");
    }
    Ok(())
}

fn transaction_id(value: &TransactionId) -> Result<(), TransactionError> {
    if !(6..=64).contains(&value.0.len())
        || !value.0.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return corrupt("transaction id is outside 6..64 ASCII alphanumerics");
    }
    Ok(())
}

fn path(value: &str) -> Result<(), TransactionError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains(['\\', ':', '\0'])
        || value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return invalid(format!("non-portable transaction path `{value}`"));
    }
    Ok(())
}

fn token(value: &str, label: &str) -> Result<(), TransactionError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        return invalid(format!("{label} is not a portable token"));
    }
    Ok(())
}

fn ownership_token(journal: &Journal, role: &str) -> String {
    let material = format!(
        "vibe-scrape-owner-e1\0{}\0{}\0{role}",
        journal.project_key.0, journal.transaction_id.0
    );
    bytes_digest(material.as_bytes()).0
}

fn path_depth(path: &str) -> usize {
    path.split('/').count()
}

fn at_or_below(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&(root.to_owned() + "/"))
}

fn is_git(value: &str) -> bool {
    value == ".git" || value.starts_with(".git/")
}

fn invalid_error(message: impl Into<String>) -> TransactionError {
    TransactionError::InvalidPrepared(message.into())
}

fn invalid<T>(message: impl Into<String>) -> Result<T, TransactionError> {
    Err(invalid_error(message))
}

fn corrupt<T>(message: impl Into<String>) -> Result<T, TransactionError> {
    Err(TransactionError::Store(format!(
        "invalid transaction journal: {}",
        message.into()
    )))
}
