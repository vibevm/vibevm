//! OpenCode file/member projection, ownership, merge and inverse.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use vibe_agent_projection::agents::Agent;
use vibe_core::manifest::SkillDecl;
use vibe_safefs::Project;

use super::artifact::AdmittedProjection;
use crate::mechanism::MechanismError;
use crate::mechanism::contain::{FileFault, checked_relative, digest_file, read_file_bounded};
use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployPlan, DeployTargetRequest, ObservedResource, PlannedDeployResource,
    RemoveReport,
};
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::vibebin::store;

const HOME: &str = "home:";
const SKILLS_RELATIVE: &str = ".config/opencode/skills";
const CONFIG_RELATIVE: &str = ".config/opencode/opencode.json";
const CONFIG_RESOURCE: &str = "home:.config/opencode/opencode.json";
const MCP_MARKER: &str = "home:.config/opencode/opencode.json#mcp/";
const FILE_CAP: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
struct Desired {
    resource: String,
    digest: String,
    kind: DesiredKind,
}

#[derive(Debug, Clone)]
enum DesiredKind {
    Skill { source: PathBuf, relative: String },
    Mcp { name: String, value: Value },
}

pub(crate) fn plan(
    request: &DeployTargetRequest<'_>,
    projection: &AdmittedProjection,
    config_digest: String,
) -> Result<DeployPlan, MechanismError> {
    prove_pure_paths(request)?;
    let desired = desired(projection)?;
    judge_occupants(request, &desired, true)?;
    let resources = desired
        .iter()
        .map(|item| PlannedDeployResource {
            resource: item.resource.clone(),
            desired_digest: item.digest.clone(),
        })
        .collect();
    let mut locks = BTreeSet::new();
    for item in &desired {
        match item.kind {
            DesiredKind::Skill { .. } => {
                locks.insert(item.resource.clone());
            }
            DesiredKind::Mcp { .. } => {
                locks.insert(CONFIG_RESOURCE.to_owned());
            }
        }
    }
    Ok(DeployPlan {
        resources,
        lock_resources: locks.into_iter().collect(),
        config_digest,
        reversible: request.prior_receipt.is_none(),
        summary: format!(
            "OpenCode plugin projection with {} exact skill/MCP member(s)",
            desired.len()
        ),
    })
}

pub(crate) fn apply(
    request: &DeployTargetRequest<'_>,
    projection: &AdmittedProjection,
    checkpoint: &mut CheckpointLedger<'_>,
) -> Result<ApplyReport, MechanismError> {
    let desired = desired(projection)?;
    judge_occupants(request, &desired, false)?;
    reconcile(request, &desired, checkpoint)?;
    Ok(ApplyReport {
        prior_state_handle: None,
        evidence: format!(
            "OpenCode: reconciled {} receipt-scoped skill/MCP member(s) and preserved foreign JSON values",
            desired.len()
        ),
    })
}

pub(crate) fn recover(
    request: &DeployTargetRequest<'_>,
    projection: &AdmittedProjection,
    checkpoint: &mut CheckpointLedger<'_>,
) -> Result<ApplyReport, MechanismError> {
    let desired = desired(projection)?;
    reconcile(request, &desired, checkpoint)?;
    Ok(ApplyReport {
        prior_state_handle: None,
        evidence: format!("OpenCode: rolled forward {} exact member(s)", desired.len()),
    })
}

pub(crate) fn verify_contained(
    request: &DeployTargetRequest<'_>,
    resources: &[String],
) -> Result<Vec<ObservedResource>, MechanismError> {
    let mut result = Vec::with_capacity(resources.len());
    for resource in resources {
        let digest = if let Some(relative) = contained_skill(resource) {
            let relative = relative.map_err(|_| remove_refusal(request, resource))?;
            match digest_file(&store::join(request.user_home, &relative)) {
                Ok((digest, _)) => Some(digest),
                Err(FileFault::Missing(_)) => None,
                Err(fault) => {
                    return Err(document_error(
                        request,
                        &store::join(request.user_home, &relative),
                        fault.reason(),
                    )
                    .into());
                }
            }
        } else if let Some(name) = resource.strip_prefix(MCP_MARKER) {
            if !SkillDecl::valid_name(name) {
                return Err(remove_refusal(request, resource));
            }
            read_document(request)?
                .get("mcp")
                .and_then(Value::as_object)
                .and_then(|mcp| mcp.get(name))
                .map(canonical_digest)
        } else {
            return Err(remove_refusal(request, resource));
        };
        result.push(ObservedResource {
            resource: resource.clone(),
            digest,
        });
    }
    Ok(result)
}

pub(crate) fn remove(
    request: &DeployTargetRequest<'_>,
    resources: &[String],
) -> Result<RemoveReport, MechanismError> {
    let receipt = request.prior_receipt.ok_or_else(|| {
        MechanismError::Deploy(DeployProviderError::RemoveNotOwned {
            target: request.target.id.clone(),
            resource: resources
                .first()
                .cloned()
                .unwrap_or_else(|| "<none>".to_owned()),
        })
    })?;
    let owned: BTreeSet<&str> = receipt
        .resources
        .iter()
        .map(|item| item.resource.as_str())
        .collect();
    let mut skills = Vec::new();
    let mut mcp = Vec::new();
    for resource in resources {
        if !owned.contains(resource.as_str()) {
            return Err(DeployProviderError::RemoveNotOwned {
                target: request.target.id.clone(),
                resource: resource.clone(),
            }
            .into());
        }
        if let Some(relative) = contained_skill(resource) {
            skills.push((
                resource.clone(),
                relative.map_err(|_| remove_refusal(request, resource))?,
            ));
        } else if let Some(name) = resource.strip_prefix(MCP_MARKER) {
            if !SkillDecl::valid_name(name) {
                return Err(remove_refusal(request, resource));
            }
            mcp.push((resource.clone(), name.to_owned()));
        } else {
            return Err(remove_refusal(request, resource));
        }
    }
    let mut removed = Vec::new();
    for (resource, relative) in skills {
        if store::remove_resource(&request.target.id, request.user_home, &relative)? {
            removed.push(resource);
        }
        prune_skill(request, &relative)?;
    }
    if !mcp.is_empty() {
        let mut document = read_document(request)?;
        let mut changed = false;
        if let Some(Value::Object(entries)) = document.get_mut("mcp") {
            for (resource, name) in &mcp {
                if entries.remove(name).is_some() {
                    removed.push(resource.clone());
                    changed = true;
                }
            }
        }
        if changed {
            write_document(request, &document)?;
        }
    }
    Ok(RemoveReport {
        removed,
        evidence: "OpenCode: removed only receipt-owned contained skill files/MCP members; immutable support and foreign neighbours were preserved".to_owned(),
    })
}

fn desired(projection: &AdmittedProjection) -> Result<Vec<Desired>, MechanismError> {
    let mut desired = Vec::new();
    for file in &projection.files {
        let Some(tail) = file.relative.strip_prefix("skills/") else {
            continue;
        };
        let relative = checked_relative(&format!("{SKILLS_RELATIVE}/{tail}")).map_err(|fault| {
            DeployProviderError::PluginArtifact {
                target: "<projection>".to_owned(),
                artifact: "<projection>".to_owned(),
                provider: crate::mechanism::BUILTIN_OPENCODE_PLUGIN_PIN,
                reason: fault.reason().to_owned(),
            }
        })?;
        desired.push(Desired {
            resource: format!("{HOME}{relative}"),
            digest: file.digest.clone(),
            kind: DesiredKind::Skill {
                source: file.absolute.clone(),
                relative,
            },
        });
    }
    for (name, value) in &projection.opencode_mcp {
        desired.push(Desired {
            resource: format!("{MCP_MARKER}{name}"),
            digest: canonical_digest(value),
            kind: DesiredKind::Mcp {
                name: name.clone(),
                value: value.clone(),
            },
        });
    }
    desired.sort_by(|left, right| left.resource.cmp(&right.resource));
    Ok(desired)
}

fn judge_occupants(
    request: &DeployTargetRequest<'_>,
    desired: &[Desired],
    allow_intent: bool,
) -> Result<(), MechanismError> {
    for item in desired {
        let Some(observed) = observe(request, item)? else {
            continue;
        };
        let prior = request.prior_receipt.and_then(|receipt| {
            receipt
                .resources
                .iter()
                .find(|owned| owned.resource == item.resource)
        });
        if prior.is_some_and(|owned| owned.post_digest == observed) {
            continue;
        }
        if allow_intent && interrupted(request, item, &observed) {
            continue;
        }
        let reason = prior.map_or_else(
            || format!("unowned occupant has digest `{observed}`"),
            |owned| {
                format!(
                    "receipt recorded `{}`, independently observed `{observed}`",
                    owned.post_digest
                )
            },
        );
        return Err(DeployProviderError::PluginOccupancy {
            target: request.target.id.clone(),
            resource: item.resource.clone(),
            reason,
        }
        .into());
    }
    Ok(())
}

fn interrupted(request: &DeployTargetRequest<'_>, desired: &Desired, observed: &str) -> bool {
    let Some(intent) = request.recovery_intent else {
        return false;
    };
    intent.prior_generation == request.prior_receipt.map(|receipt| receipt.generation)
        && intent.resources.iter().any(|planned| {
            planned.resource == desired.resource
                && planned.desired_digest == observed
                && desired.digest == observed
        })
}

fn observe(
    request: &DeployTargetRequest<'_>,
    desired: &Desired,
) -> Result<Option<String>, MechanismError> {
    match &desired.kind {
        DesiredKind::Skill { relative, .. } => {
            let path = store::join(request.user_home, relative);
            match digest_file(&path) {
                Ok((digest, _)) => Ok(Some(digest)),
                Err(FileFault::Missing(_)) => Ok(None),
                Err(fault) => Err(document_error(request, &path, fault.reason()).into()),
            }
        }
        DesiredKind::Mcp { name, .. } => {
            let document = read_document(request)?;
            Ok(document
                .get("mcp")
                .and_then(Value::as_object)
                .and_then(|mcp| mcp.get(name))
                .map(canonical_digest))
        }
    }
}

fn reconcile(
    request: &DeployTargetRequest<'_>,
    desired: &[Desired],
    checkpoint: &mut CheckpointLedger<'_>,
) -> Result<(), MechanismError> {
    for item in desired
        .iter()
        .filter(|item| matches!(item.kind, DesiredKind::Skill { .. }))
    {
        let DesiredKind::Skill { source, relative } = &item.kind else {
            unreachable!()
        };
        if observe(request, item)?.as_ref() != Some(&item.digest) {
            let bytes = read_file_bounded(source, FILE_CAP).map_err(|fault| {
                DeployProviderError::OpenCodeDocument {
                    target: request.target.id.clone(),
                    path: source.display().to_string(),
                    reason: fault.reason(),
                }
            })?;
            if format!("{:x}", Sha256::digest(&bytes)) != item.digest {
                return Err(DeployProviderError::PluginOccupancy {
                    target: request.target.id.clone(),
                    resource: item.resource.clone(),
                    reason: "the projected source changed after admission".to_owned(),
                }
                .into());
            }
            store::place_resource(
                &request.target.id,
                request.user_home,
                request.staging,
                relative,
                &bytes,
                false,
            )?;
        }
        checkpoint.completed(&item.resource)?;
    }
    let mcp: Vec<&Desired> = desired
        .iter()
        .filter(|item| matches!(item.kind, DesiredKind::Mcp { .. }))
        .collect();
    if !mcp.is_empty() {
        let mut document = read_document(request)?;
        let entries = document
            .entry("mcp")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| {
                document_error(
                    request,
                    &config_path(request),
                    "`mcp` must be an object".to_owned(),
                )
            })?;
        let mut changed = false;
        for item in &mcp {
            let DesiredKind::Mcp { name, value } = &item.kind else {
                unreachable!()
            };
            if entries.get(name) != Some(value) {
                entries.insert(name.clone(), value.clone());
                changed = true;
            }
        }
        if changed {
            write_document(request, &document)?;
        }
        for item in mcp {
            checkpoint.completed(&item.resource)?;
        }
    }
    Ok(())
}

fn read_document(request: &DeployTargetRequest<'_>) -> Result<Map<String, Value>, MechanismError> {
    let path = config_path(request);
    let bytes = match read_file_bounded(&path, FILE_CAP) {
        Ok(bytes) => bytes,
        Err(FileFault::Missing(_)) => return Ok(Map::new()),
        Err(fault) => return Err(document_error(request, &path, fault.reason()).into()),
    };
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| document_error(request, &path, format!("not JSON: {error}")))?;
    let document = value
        .as_object()
        .cloned()
        .ok_or_else(|| document_error(request, &path, "root must be an object".to_owned()))?;
    if document.get("mcp").is_some_and(|value| !value.is_object()) {
        return Err(document_error(
            request,
            &path,
            "`mcp`, when present, must be an object".to_owned(),
        )
        .into());
    }
    Ok(document)
}

fn write_document(
    request: &DeployTargetRequest<'_>,
    document: &Map<String, Value>,
) -> Result<(), MechanismError> {
    let bytes = canonical_document(document)?;
    let project = Project::open(request.user_home)
        .map_err(|error| document_error(request, &config_path(request), format!("{error:#}")))?;
    project
        .write_atomic(CONFIG_RELATIVE, &bytes)
        .map_err(|error| {
            document_error(
                request,
                &config_path(request),
                format!("{:#}", error.into_report()),
            )
        })?;
    Ok(())
}

fn canonical_document(document: &Map<String, Value>) -> Result<Vec<u8>, MechanismError> {
    let mut bytes =
        serde_json::to_vec_pretty(&sorted(&Value::Object(document.clone()))).map_err(|error| {
            MechanismError::Deploy(DeployProviderError::OpenCodeDocument {
                target: "<encode>".to_owned(),
                path: CONFIG_RELATIVE.to_owned(),
                reason: error.to_string(),
            })
        })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn canonical_digest(value: &Value) -> String {
    let bytes = serde_json::to_vec(&sorted(value)).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}

fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let ordered: BTreeMap<&String, &Value> = map.iter().collect();
            Value::Object(
                ordered
                    .into_iter()
                    .map(|(key, value)| (key.clone(), sorted(value)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
        scalar => scalar.clone(),
    }
}

fn contained_skill(resource: &str) -> Option<Result<String, ()>> {
    let tail = resource.strip_prefix(HOME)?;
    if !tail.starts_with(&format!("{SKILLS_RELATIVE}/")) {
        return None;
    }
    let below = tail.strip_prefix(&format!("{SKILLS_RELATIVE}/"))?;
    let mut parts = below.split('/');
    if parts.next().is_none_or(|name| !SkillDecl::valid_name(name)) || parts.next().is_none() {
        return Some(Err(()));
    }
    Some(checked_relative(tail).map_err(|_| ()))
}

fn prune_skill(request: &DeployTargetRequest<'_>, relative: &str) -> Result<(), MechanismError> {
    let tail = relative
        .strip_prefix(&format!("{SKILLS_RELATIVE}/"))
        .ok_or_else(|| remove_refusal(request, relative))?;
    let skill = tail
        .split('/')
        .next()
        .ok_or_else(|| remove_refusal(request, relative))?;
    let boundary = Agent::opencode_user_skills_root_from_home(request.user_home).join(skill);
    let mut current = store::join(request.user_home, relative)
        .parent()
        .map(Path::to_path_buf);
    while let Some(directory) = current {
        if !directory.starts_with(&boundary) {
            break;
        }
        match std::fs::remove_dir(&directory) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) =>
            {
                break;
            }
            Err(error) => return Err(document_error(request, &directory, error.to_string()).into()),
        }
        if directory == boundary {
            break;
        }
        current = directory.parent().map(Path::to_path_buf);
    }
    Ok(())
}

fn prove_pure_paths(request: &DeployTargetRequest<'_>) -> Result<(), MechanismError> {
    let skills = Agent::opencode_user_skills_root_from_home(request.user_home);
    let config = Agent::opencode_user_config_from_home(request.user_home);
    if skills
        != request
            .user_home
            .join(".config")
            .join("opencode")
            .join("skills")
        || config != config_path(request)
    {
        return Err(document_error(
            request,
            &config,
            "pure injected-home OpenCode paths disagree".to_owned(),
        )
        .into());
    }
    Ok(())
}

fn config_path(request: &DeployTargetRequest<'_>) -> PathBuf {
    Agent::opencode_user_config_from_home(request.user_home)
}

fn remove_refusal(request: &DeployTargetRequest<'_>, resource: &str) -> MechanismError {
    DeployProviderError::RemoveNotOwned {
        target: request.target.id.clone(),
        resource: resource.to_owned(),
    }
    .into()
}

fn document_error(
    request: &DeployTargetRequest<'_>,
    path: &Path,
    reason: String,
) -> DeployProviderError {
    DeployProviderError::OpenCodeDocument {
        target: request.target.id.clone(),
        path: path.display().to_string(),
        reason,
    }
}
