//! Scoped joins to existing domain data, never an operation or journal owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CANONICAL-DOMAIN-LINKS");
use super::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use vibe_wire::generated::{
    capability::e1::coverage::{Coverage, Surface},
    machine::e1::operation_descriptor::OperationDescriptor,
    preview::e1::{
        release_impact::ReleaseImpact,
        transition::{CapturedSource, SourceDigestRef},
    },
};

fn fault(subject: &str, reason: &str) -> CheckFailure {
    CheckFailure::new("domain-binding", subject, reason)
}
fn token(value: &impl Serialize) -> String {
    serde_json::to_value(value)
        .expect("generated enum")
        .as_str()
        .expect("enum token")
        .into()
}
fn retained(
    root: &vibe_safefs::Project,
    reference: &SourceDigestRef,
) -> Result<Vec<u8>, CheckFailure> {
    // A fixed ceiling as well as retained no-follow snapshot identity. The
    // owner source text is selected by reference, never read from the workspace.
    let components: Vec<_> = reference.source.split('/').collect();
    let (name, parents) = components
        .split_last()
        .ok_or_else(|| fault("domain-evidence", "Invalid retained path"))?;
    let directory = root
        .dir_if_present(parents)
        .map_err(|_| fault("domain-evidence", "Retained directory refused"))?
        .ok_or_else(|| fault("domain-evidence", "Retained directory missing"))?;
    let bytes = root
        .read_file_snapshot_bounded_in(&directory, name, 4 * 1024 * 1024)
        .map_err(|_| fault("domain-evidence", "Retained file refused or exceeds 4 MiB"))?
        .ok_or_else(|| fault("domain-evidence", "Retained file missing"))?
        .bytes;
    if capture::sha256(&bytes) != reference.sha256 {
        return Err(fault("domain-evidence", "Retained evidence digest differs"));
    }
    Ok(bytes)
}
fn inventory(
    root: &vibe_safefs::Project,
    reference: &SourceDigestRef,
) -> Result<Coverage, CheckFailure> {
    let value: Coverage = serde_json::from_slice(&retained(root, reference)?)
        .map_err(|_| fault("capability-inventory", "Invalid registered inventory shape"))?;
    value
        .validate_capability_coverage()
        .map_err(|e| fault(&e.subject, e.reason))?;
    Ok(value)
}

/// A captured source always resolves a regular blob, not a mutable path or symlink.
fn blob(root: &Path, commit: &str, path: &str, digest: &str) -> Result<Vec<u8>, CheckFailure> {
    let census = capture::git(root, &["ls-tree", "-z", commit, "--", path])?;
    let mut entries = super::git_entry::rows(&census);
    let entry = entries
        .next()
        .ok_or_else(|| fault(path, "Captured source is missing"))?;
    let entry =
        super::git_entry::decode(entry).map_err(|_| fault(path, "Invalid captured entry"))?;
    if entries.next().is_some() || !entry.regular() || entry.repository_path != path {
        return Err(fault(
            path,
            "Captured source must be one exact regular blob",
        ));
    }
    let bytes = capture::git(root, &["cat-file", "blob", entry.oid])?;
    if capture::sha256(&bytes) != digest {
        return Err(fault(
            path,
            "Captured source is stale or has a different digest",
        ));
    }
    Ok(bytes)
}
fn captured(
    root: &Path,
    references: &BTreeMap<String, String>,
    source: &CapturedSource,
) -> Result<Vec<u8>, CheckFailure> {
    let commit = references
        .get(&source.capture_id)
        .ok_or_else(|| fault(&source.capture_id, "Capture is not selected"))?;
    blob(root, commit, &source.path, &source.sha256)
}
fn surface_sources(
    root: &Path,
    commit: &str,
    inventory: &Coverage,
    surface: &Surface,
) -> Result<(), CheckFailure> {
    for id in &surface.source_ids {
        let source = inventory
            .sources
            .iter()
            .find(|source| &source.source_id == id)
            .ok_or_else(|| fault(id, "Canonical source row missing"))?;
        blob(root, commit, &source.path, &source.sha256)?;
    }
    Ok(())
}

/// This route is deliberately finite: the owner maps the MCP coverage axis,
/// and the selected release relationship names either that transport or its
/// underlying Rust service. Neither relationship is inferred from ID equality.
fn native_planning(
    release: &vibe_safefs::Project,
    inventory: &Coverage,
    link: &vibe_wire::generated::preview::e1::transition::DomainEffect,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    use vibe_orchestrator::machine_operation::{
        MachineCoverageRegistry, MachineOperationService, PlannerCoverageBinding,
        PurePlannerAdmission,
    };
    use vibe_wire::generated::migration::e1::plan_report::PlanReport;
    use vibe_workspace::migration_planning::{
        MigrationPlanningResponse, decode_migration_request, validate_report_for_request,
    };
    let selected = link.native_planning.as_ref().expect("selected by caller");
    let expected = match token(&selected.relationship).as_str() {
        "reviewed-envelope" => (
            "mcp:plan_manifest_migration_envelope",
            "pure-machine-planner-transport",
        ),
        "reviewed-service" => (
            "rust:vibe-orchestrator:machine_operation",
            "pure-machine-planner-service",
        ),
        _ => return Err(fault(&link.surface_id, "Unknown reviewed relationship")),
    };
    if (link.interface_id.as_str(), link.capability_id.as_str()) != expected
        || link.change_id != "pure-machine-service-and-envelope-tool"
        || link.migration_descriptor.is_some()
        || !link.format_breaks.is_empty()
    {
        return Err(fault(
            &link.surface_id,
            "Selected release relationship differs from reviewed planner contribution",
        ));
    }
    // Explicit local admission is for pure text replay only; no application or
    // production registration/host permission is inferred from this selection.
    let service = MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted)
        .map_err(|e| fault(&link.surface_id, &e.to_string()))?;
    let registry = MachineCoverageRegistry::from_coverage(inventory)
        .map_err(|e| fault(&link.surface_id, &e.to_string()))?;
    let PlannerCoverageBinding::Bound {
        coverage_identity,
        mapping,
        descriptor,
        source_bytes_verified: false,
    } = registry
        .reviewed_planner_binding(&service)
        .map_err(|e| fault(&link.surface_id, &e.to_string()))?
    else {
        return Err(fault(
            &link.surface_id,
            "Reviewed planner mapping is not bound",
        ));
    };
    if mapping.surface_id.as_deref() != Some(link.surface_id.as_str())
        || descriptor.domain_owner != selected.domain_owner
        || format!("{}/e1", descriptor.input_format_id.as_deref().unwrap_or(""))
            != selected.request_format
        || format!(
            "{}/e1",
            descriptor.result_format_id.as_deref().unwrap_or("")
        ) != selected.report_format
    {
        return Err(fault(
            &link.surface_id,
            "Reviewed native owner/format/surface differs",
        ));
    }
    vibe_orchestrator::machine_operation::validate_descriptor_metadata(&descriptor)
        .map_err(|e| fault(&link.surface_id, &e.to_string()))?;
    let request_bytes = retained(release, &selected.request)?;
    let request = decode_migration_request(
        serde_json::from_slice(&request_bytes)
            .map_err(|_| fault(&link.surface_id, "Invalid native request JSON"))?,
    )
    .map_err(|_| {
        fault(
            &link.surface_id,
            "Native request owner refused selected bytes",
        )
    })?;
    let report_bytes = retained(release, &selected.report)?;
    let retained_report: PlanReport = serde_json::from_slice(&report_bytes)
        .map_err(|_| fault(&link.surface_id, "Invalid registered native report"))?;
    validate_report_for_request(&request, &retained_report)
        .map_err(|e| fault(&link.surface_id, &e.to_string()))?;
    let actual = service
        .plan_migration(
            descriptor.operation_id.as_deref().expect("owner operation"),
            request,
        )
        .map_err(|e| fault(&link.surface_id, &e.to_string()))?;
    if actual != MigrationPlanningResponse::Report(retained_report.clone()) {
        return Err(fault(
            &link.surface_id,
            "Retained report differs from current native service replay",
        ));
    }
    use vibe_wire::generated::migration::e1::plan_report::Outcome;
    let outcome = match retained_report.outcome {
        Outcome::Planned(_) => "planned",
        Outcome::NoOp(_) => "no-op",
        Outcome::NeedsInput(_) => "needs-input",
        Outcome::Refused(_) => "refused",
    };
    report.checked_claims.push(format!(
        "reviewed-planner-mapping:{}:{}:{}:{}:{}:{}",
        link.interface_id,
        link.capability_id,
        coverage_identity,
        descriptor.operation_id.as_deref().unwrap(),
        descriptor.source_id,
        descriptor.capability_id.as_deref().unwrap()
    ));
    report.checked_claims.push(format!(
        "current-native-planning-replay:{}:{}:{}:{}:{}",
        link.change_id,
        selected.capture_id,
        selected.request.sha256,
        selected.report.sha256,
        outcome
    ));
    report.checked_claims.push(format!(
        "automatic-file-application:{}:unsupported",
        link.surface_id
    ));
    Ok(())
}

pub(super) fn check(
    path: &Path,
    root: &Path,
    transition: &Transition,
    records: &[ReleaseImpact],
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let Some(binding) = &transition.domain_bindings else {
        report.verdict = CheckReportVerdict::Pending;
        report.findings.push(Finding { code: "bindings-domain-selection-missing".into(), subject: "bindings".into(),
            reason: "Select exact canonical inventories and per-effect domain links for this contribution scope.".into(), state: FindingState::Pending });
        return Ok(());
    };
    // Pure validation also guards entrypoints used directly by focused owner tests.
    transition
        .validate_preview()
        .map_err(|e| fault(e.field, e.reason))?;
    let release = capture::open_project(path.parent().unwrap_or(Path::new(".")))?;
    let before = inventory(&release, &binding.before_inventory)?;
    let after = inventory(&release, &binding.after_inventory)?;
    let references: BTreeMap<_, _> = transition
        .capture_references()
        .map_err(|e| fault(e.field, e.reason))?
        .into_iter()
        .map(|reference| (reference.capture_id, reference.commit_sha1))
        .collect();
    let before_commit = references
        .get(&binding.before_capture_id)
        .ok_or_else(|| fault("bindings", "Before capture missing"))?;
    let after_commit = references
        .get(&binding.after_capture_id)
        .ok_or_else(|| fault("bindings", "After capture missing"))?;
    for (inventory, commit) in [(&before, before_commit), (&after, after_commit)] {
        for source in &inventory.sources {
            blob(root, commit, &source.path, &source.sha256)?;
        }
    }
    report
        .checked_claims
        .push("selected-inventory-endpoint-source-bytes:verified".into());
    let old: BTreeMap<_, _> = before
        .surfaces
        .iter()
        .map(|s| (s.surface_id.as_str(), s))
        .collect();
    let new: BTreeMap<_, _> = after
        .surfaces
        .iter()
        .map(|s| (s.surface_id.as_str(), s))
        .collect();
    let delta: BTreeSet<_> = old
        .keys()
        .chain(new.keys())
        .copied()
        .filter(|id| old.get(id) != new.get(id))
        .collect();
    if let Some(ids) = &binding.selected_changes {
        if ids
            .iter()
            .any(|id| !records.iter().any(|record| &record.change_id == id))
        {
            return Err(fault("bindings", "Selected native impact missing"));
        }
    }
    let mut covered = BTreeSet::new();
    let mut joined = BTreeSet::new();
    for link in &binding.effects {
        let record = records
            .iter()
            .find(|record| record.change_id == link.change_id)
            .ok_or_else(|| fault(&link.change_id, "Selected native impact missing"))?;
        let key = (
            &link.interface_id,
            &link.capability_id,
            &link.audience,
            &link.platform,
        );
        let effects: Vec<_> = record
            .effects
            .iter()
            .filter(|effect| {
                (
                    &effect.interface_id,
                    &effect.capability_id,
                    &effect.audience,
                    &effect.platform,
                ) == key
            })
            .collect();
        if effects.len() != 1
            || record.before_capture_id != binding.before_capture_id
            || record.after_capture_id != binding.after_capture_id
        {
            return Err(fault(
                &link.change_id,
                "Effect or exact capture scope does not join uniquely",
            ));
        }
        joined.insert((&record.change_id, key));
        let effect = effects[0];
        let prior = old.get(link.surface_id.as_str()).copied();
        let current = new.get(link.surface_id.as_str()).copied();
        let kind = token(&effect.kind);
        let coherent = match (prior, current) {
            (None, None) => return Err(fault(&link.surface_id, "Canonical surface missing")),
            (None, Some(_)) => kind == "introduced",
            (Some(_), None) => kind == "removed",
            // Metadata changes do not prove behavioral changes, and equal
            // schema/catalogue rows do not prove equal semantics. Retain the
            // independently reviewed effect rather than deriving it here.
            (Some(_), Some(_)) => matches!(kind.as_str(), "changed" | "fixed" | "unchanged"),
        };
        if !coherent {
            return Err(fault(
                &link.surface_id,
                "Effect kind contradicts canonical surface membership",
            ));
        }
        covered.insert(link.surface_id.as_str());
        if let Some(surface) = prior {
            surface_sources(root, before_commit, &before, surface)?;
        }
        if let Some(surface) = current {
            surface_sources(root, after_commit, &after, surface)?;
        }
        if token(&link.action) != token(&record.user_impact.migration_action) {
            return Err(fault(
                &link.change_id,
                "Migration action differs from retained impact",
            ));
        }
        let action = token(&link.action);
        let class = token(&link.data_class);
        if (action == "rebuild" && class != "derived")
            || (action == "automated" && class != "authored")
            || (class == "derived" && action == "manual")
        {
            return Err(fault(
                &link.change_id,
                "Action contradicts authored/derived data ownership",
            ));
        }
        if kind != "unchanged"
            && (token(&record.documentation_impact.state) != "impact"
                || token(&record.documentation_impact.documentation_action) == "none")
        {
            return Err(fault(
                &link.surface_id,
                "Changed or new surface lacks documentation obligation",
            ));
        }
        for source in &link.format_breaks {
            captured(root, &references, source)?;
            report.checked_claims.push(format!(
                "break-note-byte-identity:{}:{}",
                source.path, source.sha256
            ));
        }
        if link.native_planning.is_some() {
            native_planning(&release, &after, link, report)?;
        } else if link.change_id == "pure-machine-service-and-envelope-tool" {
            report.verdict = CheckReportVerdict::Pending;
            report.findings.push(Finding {
                code: "bindings-native-planning-selection-missing".into(),
                subject: link.interface_id.clone(),
                reason: "Select exact native request/report bytes for the reviewed planner relationship.".into(),
                state: FindingState::Pending,
            });
        }
        if let Some(source) = &link.migration_descriptor {
            let descriptor: OperationDescriptor = serde_json::from_slice(&captured(root, &references, source)?)
                .map_err(|_| fault(&link.surface_id, "Descriptor is not the registered machine descriptor (journals/recipes are not descriptors)"))?;
            vibe_orchestrator::machine_operation::validate_descriptor_metadata(&descriptor)
                .map_err(|error| fault(&link.surface_id, &error.to_string()))?;
            let (inventory, surface) = if source.capture_id == binding.before_capture_id {
                (&before, prior)
            } else {
                (&after, current)
            };
            let surface = surface
                .ok_or_else(|| fault(&link.surface_id, "Descriptor endpoint has no surface"))?;
            if descriptor.operation_id.is_none()
                || descriptor.operation_id != surface.operation_id
                || descriptor.capability_id.as_deref() != Some(link.capability_id.as_str())
                || !surface.source_ids.contains(&descriptor.source_id)
                || !inventory
                    .sources
                    .iter()
                    .any(|row| row.source_id == descriptor.source_id)
            {
                return Err(fault(
                    &link.surface_id,
                    "Descriptor operation/capability/source identity does not join",
                ));
            }
            report.checked_claims.push(format!(
                "domain-descriptor:{}:{}",
                link.surface_id, source.sha256
            ));
        } else if action == "automated" && link.native_planning.is_none() {
            return Err(fault(
                &link.surface_id,
                "Automatic conversion has no exact domain descriptor",
            ));
        }
        if action == "automated" {
            // Deliberately do not promote producer labels or corpus shapes to
            // an implemented, precondition-checked conversion command.
            report.verdict = CheckReportVerdict::Pending;
            report.findings.push(Finding { code: "bindings-automatic-relationship-unselected".into(), subject: link.surface_id.clone(),
                reason: "Exact descriptor metadata resolves, but this supplement selects no exact native migration request/report, source/target profile, recipe or application prerequisites. Break-note byte identity cannot supply that relationship. The pure planner is available separately; no applied converter or callable coverage is established.".into(), state: FindingState::Unsupported });
        }
        report.checked_claims.push(format!(
            "domain-effect:{}:{}:{}:{}:{}:{}:{}",
            link.change_id,
            link.interface_id,
            link.capability_id,
            link.audience,
            link.platform,
            action,
            class
        ));
        report.checked_claims.push(format!(
            "documentation-obligation:{}:{}",
            link.surface_id, link.topic_id
        ));
    }
    if delta.iter().any(|id| !covered.contains(id)) {
        return Err(fault(
            "capability-delta",
            "Canonical new/changed/removed surface is missing a release impact",
        ));
    }
    for record in records {
        if binding
            .selected_changes
            .as_ref()
            .is_some_and(|ids| !ids.contains(&record.change_id))
        {
            continue;
        }
        for effect in &record.effects {
            if !joined.contains(&(
                &record.change_id,
                (
                    &effect.interface_id,
                    &effect.capability_id,
                    &effect.audience,
                    &effect.platform,
                ),
            )) {
                return Err(fault(
                    &record.change_id,
                    "Release effect has no canonical surface join",
                ));
            }
        }
    }
    report
        .checked_claims
        .push(format!("canonical-surface-delta:{}", delta.len()));
    report.findings.push(Finding { code: "bindings-operational-coverage-unverified".into(), subject: "bindings".into(),
        reason: "Exact source/descriptor joins establish reference consistency only; registry admission, callable availability, host grants and executed conformance remain independent domain claims.".into(), state: FindingState::Pending });
    Ok(())
}
