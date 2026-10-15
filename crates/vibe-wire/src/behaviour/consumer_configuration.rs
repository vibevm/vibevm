//! Admission next to generated shapes; no definition lookup or effects.
use crate::generated::consumer_configuration::e1::{request as wire, result as output};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::environment::*;
use vibe_core::manifest::{feature_constraints as core, feature_effects::*};

fn reference(s: &str) -> Result<Reference, String> {
    Reference::new(s).map_err(|e| e.to_string())
}
fn features(values: &[String]) -> Result<BTreeSet<LocalFeatureRef>, String> {
    let set = values
        .iter()
        .map(|s| LocalFeatureRef::new(s).map_err(|e| e.to_string()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if set.len() != values.len() {
        return Err("duplicate member".into());
    }
    Ok(set)
}
pub fn admit_request(request: &wire::Request) -> Result<Vec<core::OwnerEdge>, String> {
    if request.edges.is_empty() {
        return Err("empty consumer request".into());
    }
    let mut out = Vec::new();
    for e in &request.edges {
        let d = &e.domain;
        let domain = core::ResolutionDomain {
            definition: DefinitionId::new(&d.definition.reference, &d.definition.producer)
                .map_err(|e| e.to_string())?,
            lock: d
                .lock
                .as_ref()
                .map(|l| LockId::new(&l.reference, &l.producer).map_err(|e| e.to_string()))
                .transpose()?,
            target: TargetInstanceId::new(&d.target.reference, &d.target.producer)
                .map_err(|e| e.to_string())?,
            target_role: match d.target_role {
                wire::DomainTargetRole::Executor => LockTargetRole::Executor,
                wire::DomainTargetRole::ProducedToolRuntime => LockTargetRole::ProducedToolRuntime,
                wire::DomainTargetRole::EmittedCodeTarget => LockTargetRole::EmittedCodeTarget,
            },
            identity: reference(&d.identity)?,
        };
        let package = match &e.package {
            wire::Package::Vibe(p) => {
                core::PackageIdentity::Vibe(p.reference.parse().map_err(|e| format!("{e}"))?)
            }
            wire::Package::Native(p) => {
                if p.version.is_empty() {
                    return Err("empty native version".into());
                }
                core::PackageIdentity::Native {
                    backend: reference(&p.backend)?,
                    package: reference(&p.package)?,
                    version: p.version.clone(),
                }
            }
        };
        let mut selections = BTreeMap::new();
        for (group, s) in &e.selections {
            selections.insert(
                LocalFeatureRef::new(group).map_err(|e| e.to_string())?,
                core::RequestedSelection {
                    selection: match &s.members {
                        None => core::Selection::Omitted,
                        Some(m) => core::Selection::Explicit(features(m)?),
                    },
                    intent: match s.intent {
                        wire::SelectionIntent::Additive => core::SelectionIntent::Additive,
                        wire::SelectionIntent::Exact => core::SelectionIntent::Exact,
                    },
                    provenance: reference(&s.provenance)?,
                },
            );
        }
        let edge = core::OwnerEdge {
            consumer: reference(&e.consumer)?,
            dependency: DependencyDeclarationId::new(&e.dependency).map_err(|e| e.to_string())?,
            role: match e.role {
                wire::EdgeRole::Build => DependencyRole::Build,
                wire::EdgeRole::Runtime => DependencyRole::Runtime,
                wire::EdgeRole::BuildAndRuntime => DependencyRole::BuildAndRuntime,
            },
            instance: core::ConfigurationInstance {
                domain,
                identity: reference(&e.instance)?,
                configuration: reference(&e.configuration)?,
                package,
            },
            policy: match e.policy {
                wire::EdgePolicy::RequireSelection => core::DefaultPolicy::RequireSelection,
                wire::EdgePolicy::UseDeclaredDefaults => core::DefaultPolicy::UseDeclaredDefaults,
            },
            selections,
            features: features(&e.features)?,
            all_features: e.all_features,
        };
        // Domain and instance identify one configuration/package, independently
        // of consumer selection. Result admission uses this same boundary.
        if let Some((index, _)) =
            out.iter()
                .enumerate()
                .find(|(_, prior): &(_, &core::OwnerEdge)| {
                    prior.instance.domain == edge.instance.domain
                        && prior.instance.identity == edge.instance.identity
                        && prior.instance != edge.instance
                })
        {
            return Err(format!(
                "instance identity disagrees on configuration/package: edges {index} and {}",
                out.len()
            ));
        }
        if out.iter().any(|prior: &core::OwnerEdge| {
            prior.consumer == edge.consumer
                && prior.dependency == edge.dependency
                && prior.role == edge.role
                && prior.instance == edge.instance
        }) {
            return Err("duplicate consumer edge".into());
        }
        out.push(edge);
    }
    Ok(out)
}
fn reject_null(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Null => Err("null is not omission".into()),
        serde_json::Value::Array(a) => a.iter().try_for_each(reject_null),
        serde_json::Value::Object(o) => o.values().try_for_each(reject_null),
        _ => Ok(()),
    }
}
pub fn decode_request(bytes: &[u8]) -> Result<wire::Request, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    reject_null(&value)?;
    let request = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    admit_request(&request)?;
    Ok(request)
}
pub fn result_for(
    request: &wire::Request,
    outcomes: Vec<output::Outcome>,
) -> Result<output::Result, String> {
    // Both families own generated shapes. The conversion copies their common
    // edge vocabulary losslessly, with no independently maintained DTO.
    let result = output::Result {
        epoch: output::ResultEpoch::DefaultName,
        edges: serde_json::from_value(
            serde_json::to_value(&request.edges).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?,
        outcomes,
    };
    admit_result(&result)?;
    Ok(result)
}
pub fn admit_result(result: &output::Result) -> Result<(), String> {
    let request = wire::Request {
        epoch: wire::RequestEpoch::DefaultName,
        edges: serde_json::from_value(
            serde_json::to_value(&result.edges).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?,
    };
    let edges = admit_request(&request)?;
    let mut covered = BTreeSet::new();
    let mut instances = Vec::new();
    for o in &result.outcomes {
        let representative = edges
            .get(o.representative as usize)
            .ok_or("invalid representative")?;
        if !covered.insert(o.representative) {
            return Err("duplicate outcome".into());
        }
        if instances.contains(&representative.instance) {
            return Err("duplicate instance outcome".into());
        }
        instances.push(representative.instance.clone());
        features(&o.active_features)?;
        for (group, members) in &o.selected {
            LocalFeatureRef::new(group).map_err(|e| e.to_string())?;
            features(members)?;
        }
        features(&o.needs)?;
        let mut conflicts = BTreeSet::new();
        for &index in &o.conflicting_edges {
            let edge = edges.get(index as usize).ok_or("invalid conflict edge")?;
            if edge.instance != representative.instance || !conflicts.insert(index) {
                return Err("conflict domain/duplicate".into());
            }
        }
        for applied in &o.defaults_used {
            let edge = edges
                .get(applied.edge as usize)
                .ok_or("invalid default edge")?;
            reference(&applied.provenance)?;
            LocalFeatureRef::new(&applied.group).map_err(|e| e.to_string())?;
            if edge.instance != representative.instance
                || edge.policy != core::DefaultPolicy::UseDeclaredDefaults
                || edge
                    .selections
                    .get(&LocalFeatureRef::new(&applied.group).unwrap())
                    .is_some_and(|s| matches!(s.selection, core::Selection::Explicit(_)))
            {
                return Err("default without consumer permission".into());
            }
        }
        match o.status {
            output::OutcomeStatus::Resolved if !o.needs.is_empty() || !conflicts.is_empty() => {
                return Err("resolved with pending/conflict".into());
            }
            output::OutcomeStatus::NeedsSelection
                if o.needs.is_empty() || !conflicts.is_empty() || !o.active_features.is_empty() =>
            {
                return Err("invalid NeedsSelection".into());
            }
            output::OutcomeStatus::Conflict
                if conflicts.len() < 2 || !o.active_features.is_empty() =>
            {
                return Err("invalid conflict".into());
            }
            _ => {}
        }
    }
    if result.outcomes.is_empty() {
        return Err("empty result".into());
    }
    if edges.iter().any(|e| !instances.contains(&e.instance)) {
        return Err("missing instance outcome".into());
    }
    Ok(())
}
pub fn decode_result(bytes: &[u8]) -> Result<output::Result, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    reject_null(&value)?;
    let result = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    admit_result(&result)?;
    Ok(result)
}

#[cfg(test)]
mod m_05_b_2_cases;
