//! Pure consumer resolution: explicit identity, choice policy, then the B1 evaluator.
use super::{FeatureExpansion, FeatureRequest, expand_feature_effects};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
use vibe_wire::behaviour::consumer_configuration as codec;
use vibe_wire::generated::consumer_configuration::e1::{
    request::Request,
    result::{Outcome, OutcomeDefaultsUsed, OutcomeStatus},
};

/// Admitted source definitions, supplied by the caller; never looked up globally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationDefinition {
    pub instance: ConfigurationInstance,
    pub program: FeatureProgram,
    pub declarations: FeatureDeclarations,
    pub constraints: Constraints,
}
pub struct ConfigurationResolution {
    pub result: vibe_wire::generated::consumer_configuration::e1::result::Result,
    /// Full B1 effects, aligned with result outcomes. Unresolved choices have none.
    pub expansions: Vec<Option<FeatureExpansion>>,
}

pub fn resolve_consumer_configuration(
    request: &Request,
    definitions: &[ConfigurationDefinition],
) -> Result<ConfigurationResolution, String> {
    let edges = codec::admit_request(request)?;
    let mut instances: Vec<(ConfigurationInstance, Vec<usize>)> = Vec::new();
    for (i, edge) in edges.iter().enumerate() {
        if let Some((instance, indices)) = instances
            .iter_mut()
            .find(|(k, _)| k.domain == edge.instance.domain && k.identity == edge.instance.identity)
        {
            if *instance != edge.instance {
                return Err(format!(
                    "instance identity disagrees on configuration/package: edges {} and {i}",
                    indices[0]
                ));
            }
            indices.push(i);
        } else {
            instances.push((edge.instance.clone(), vec![i]));
        }
    }
    let mut outcomes = Vec::new();
    let mut expansions = Vec::new();
    for (instance, indices) in instances {
        let matching: Vec<_> = definitions
            .iter()
            .filter(|d| d.instance == instance)
            .collect();
        if matching.len() != 1 {
            return Err("missing or ambiguous admitted definition".into());
        }
        let definition = matching[0];
        let constraints = &definition.constraints;
        let order = constraints
            .order(&definition.program, &definition.declarations)
            .map_err(|e| e.to_string())?;
        for &i in &indices {
            if edges[i].features.iter().any(|f| {
                f.as_str().starts_with('_') || !definition.program.features.contains_key(f)
            }) {
                return Err("unknown or private requested feature".into());
            }
            if edges[i].all_features && !constraints.groups.is_empty() {
                return Err("all-features cannot select constrained members".into());
            }
            if edges[i]
                .features
                .iter()
                .any(|f| constraints.members().contains(f))
            {
                return Err("constrained members require group selection".into());
            }
            for (group, selection) in &edges[i].selections {
                let declared = constraints
                    .groups
                    .get(group)
                    .ok_or("unknown requested group")?;
                if let Selection::Explicit(members) = &selection.selection {
                    if !members.is_subset(&declared.members) {
                        return Err("unknown requested member".into());
                    }
                }
            }
        }
        let mut outcome = Outcome {
            representative: indices[0] as u32,
            status: OutcomeStatus::Resolved,
            active_features: vec![],
            selected: BTreeMap::new(),
            needs: vec![],
            conflicting_edges: vec![],
            defaults_used: vec![],
        };
        let mut active = BTreeSet::new();
        let mut requested: BTreeMap<LocalFeatureRef, BTreeSet<LocalFeatureRef>> = BTreeMap::new();
        let mut selected = BTreeSet::new();
        let mut group_requests = BTreeMap::new();
        for node in order {
            match &node {
                Guard::Group(name) => {
                    let group = &constraints.groups[name];
                    if !group.guards.iter().all(|g| active.contains(g)) {
                        continue;
                    }
                    active.insert(node.clone());
                    let mut requests: Vec<(usize, SelectionIntent, BTreeSet<LocalFeatureRef>)> =
                        Vec::new();
                    for &i in &indices {
                        let edge = &edges[i];
                        let choice = edge.selections.get(name);
                        let selection = choice.map(|c| &c.selection).unwrap_or(&Selection::Omitted);
                        let members = match selection {
                            Selection::Explicit(members) => Some(members.clone()),
                            Selection::Omitted
                                if edge.policy == DefaultPolicy::UseDeclaredDefaults =>
                            {
                                if group.defaults.is_some() {
                                    outcome.defaults_used.push(OutcomeDefaultsUsed {
                                        edge: i as u32,
                                        group: name.as_str().into(),
                                        provenance: choice
                                            .map(|c| c.provenance.as_str())
                                            .unwrap_or("consumer-policy:use_declared_defaults")
                                            .into(),
                                    });
                                }
                                group.defaults.clone()
                            }
                            Selection::Omitted => None,
                        };
                        if let Some(members) = members {
                            requests.push((
                                i,
                                choice
                                    .map(|c| c.intent)
                                    .unwrap_or(SelectionIntent::Additive),
                                members,
                            ));
                        }
                    }
                    requested.insert(
                        name.clone(),
                        requests
                            .iter()
                            .flat_map(|(_, _, m)| m.iter().cloned())
                            .collect(),
                    );
                    group_requests.insert(name.clone(), requests);
                }
                Guard::Member(member) => {
                    let (group, _) = constraints
                        .groups
                        .iter()
                        .find(|(_, g)| g.members.contains(member))
                        .unwrap();
                    if active.contains(&Guard::Group(group.clone()))
                        && requested.get(group).is_some_and(|m| m.contains(member))
                        && constraints
                            .member_guards
                            .get(member)
                            .is_none_or(|guards| guards.iter().all(|g| active.contains(g)))
                    {
                        active.insert(node.clone());
                        selected.insert(member.clone());
                    }
                }
            }
        }
        for (name, group) in &constraints.groups {
            if active.contains(&Guard::Group(name.clone())) {
                // Dormant member intent is retained in the echoed edges. It
                // becomes a constraint only when its positive guard is active.
                let eligible: BTreeSet<_> = group
                    .members
                    .iter()
                    .filter(|m| {
                        constraints
                            .member_guards
                            .get(*m)
                            .is_none_or(|guards| guards.iter().all(|g| active.contains(g)))
                    })
                    .cloned()
                    .collect();
                let requests: Vec<_> = group_requests[name]
                    .iter()
                    .map(|(i, intent, m)| {
                        (
                            *i,
                            *intent,
                            m.intersection(&eligible).cloned().collect::<BTreeSet<_>>(),
                        )
                    })
                    .collect();
                for (offset, (left, intent, members)) in requests.iter().enumerate() {
                    for (right, other_intent, other) in &requests[offset + 1..] {
                        let conflict = match (intent, other_intent) {
                            (SelectionIntent::Exact, SelectionIntent::Exact) => members != other,
                            (SelectionIntent::Exact, _) => !other.is_subset(members),
                            (_, SelectionIntent::Exact) => !members.is_subset(other),
                            _ => false,
                        };
                        if conflict {
                            outcome
                                .conflicting_edges
                                .extend([*left as u32, *right as u32]);
                        }
                    }
                }
                let members: Vec<_> = group
                    .members
                    .intersection(&selected)
                    .map(|m| m.as_str().to_owned())
                    .collect();
                if !group.cardinality.accepts(members.len()) {
                    if members.is_empty() {
                        outcome.needs.push(name.as_str().into());
                    } else if outcome.conflicting_edges.is_empty() {
                        return Err(format!("cardinality exceeded for {}", name.as_str()));
                    }
                }
                outcome.selected.insert(name.as_str().into(), members);
            }
        }
        if !outcome.conflicting_edges.is_empty() {
            outcome.conflicting_edges.sort_unstable();
            outcome.conflicting_edges.dedup();
            outcome.status = OutcomeStatus::Conflict;
            expansions.push(None);
        } else if !outcome.needs.is_empty() {
            outcome.status = OutcomeStatus::NeedsSelection;
            expansions.push(None);
        } else {
            let constrained = constraints.members();
            let mut features = selected
                .iter()
                .map(|f| f.as_str().to_owned())
                .collect::<BTreeSet<_>>();
            for &i in &indices {
                let edge = &edges[i];
                features.extend(edge.features.iter().map(|f| f.as_str().to_owned()));
                if edge.all_features {
                    features.extend(
                        definition
                            .program
                            .features
                            .keys()
                            .filter(|f| !f.as_str().starts_with('_'))
                            .map(|f| f.as_str().to_owned()),
                    );
                }
            }
            let expansion = expand_feature_effects(
                &definition.program,
                &definition.declarations,
                &FeatureRequest {
                    explicit: features.into_iter().collect(),
                    no_defaults: true,
                    all: false,
                },
                &BTreeSet::new(),
            )
            .map_err(|e| e.to_string())?;
            // Ordinary feature effects must not bypass explicit group policy.
            if constrained
                .iter()
                .any(|m| expansion.active_features.contains(m.as_str()) && !selected.contains(m))
            {
                return Err("feature closure implicitly selects a constrained member".into());
            }
            outcome.active_features = expansion.active_features.iter().cloned().collect();
            expansions.push(Some(expansion));
        }
        outcomes.push(outcome);
    }
    let result = codec::result_for(request, outcomes)?;
    Ok(ConfigurationResolution { result, expansions })
}

#[cfg(test)]
mod m_05_b_2_cases;
