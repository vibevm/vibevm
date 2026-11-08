//! Validate the selected original graph before the incumbent obsoletes projection.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::*;
use crate::source_candidates::{ActiveSourceEdge, EdgeOwner};
use model::{Model, Record};
use std::collections::{HashMap, HashSet};

pub(super) fn decode<P: SourceCandidateProvider>(
    provider: &P,
    model: &Model<'_>,
    ids: &[resolvo::SolvableId],
    request: &SourceSolveRequest<'_>,
) -> Result<SourceResolvedGraph, SolveError> {
    let selected_ids = ids
        .iter()
        .map(|id| model.pool.resolve_solvable(*id).record)
        .collect::<BTreeSet<_>>();
    let mut selected_before_projection = BTreeMap::new();
    let mut real = BTreeMap::new();
    for id in &selected_ids {
        if let Some(Record::Real { candidate, .. }) = model.records.get(*id) {
            let entry = &model.world.entries[*candidate];
            let key = coordinate(&entry.metadata);
            if real.insert(key.clone(), *id).is_some() {
                return Err(invalid("multiple selected candidates for one package name"));
            }
            if let Some(locked) = request
                .authoritative_locked
                .iter()
                .find(|row| row.group == key.0 && row.name == key.1)
                && !provider.respects_locked_selection(&entry.metadata.candidate, locked)?
            {
                return Err(invalid(
                    "selected source violates immutable lock/update authority",
                ));
            }
            selected_before_projection.insert(key, entry.metadata.clone());
        }
    }
    let mut reachable = BTreeSet::new();
    let mut active_edges = Vec::new();
    let mut pinned = BTreeSet::new();
    let mut root_order = Vec::new();
    let mut roots_seen = BTreeSet::new();
    // Traverse ORIGINAL edges independently of the rank support clauses.
    loop {
        let before = reachable.len();
        for id in &selected_ids {
            let Some(Record::Witness {
                edge,
                parent,
                target,
                ..
            }) = model.records.get(*id)
            else {
                continue;
            };
            if parent.is_some_and(|parent| !reachable.contains(&parent)) {
                continue;
            }
            reachable.insert(*target);
            if model.edges.get(*edge).is_none() {
                return Err(invalid("selected witness lost its original edge"));
            }
        }
        if before == reachable.len() {
            break;
        }
    }
    for id in &selected_ids {
        let Some(Record::Witness {
            edge,
            parent,
            target,
            alternative,
            pinned: has_pin,
        }) = model.records.get(*id)
        else {
            continue;
        };
        let original = &model.edges[*edge];
        if parent.is_some_and(|parent| !reachable.contains(&parent)) {
            return Err(invalid("pin witness has an unrooted parent"));
        }
        let Some(Record::Real { candidate, .. }) = model.records.get(*target) else {
            return Err(invalid("edge target is not a real package"));
        };
        let target_key = model.world.entries[*candidate].metadata.key();
        if *has_pin {
            pinned.insert(coordinate(&model.world.entries[*candidate].metadata));
        }
        let is_root = if parent.is_some() {
            false
        } else {
            match &original.id.owner {
                EdgeOwner::Root { .. } => true,
                EdgeOwner::Candidate(key) => request.activated.iter().any(|activation| {
                    activation.parent.key() == **key
                        && activation.edge.site == original.id.site
                        && activation
                            .introduced_targets
                            .contains(&(target_key.group.clone(), target_key.name.clone()))
                }),
            }
        };
        if is_root && roots_seen.insert((target_key.group.clone(), target_key.name.clone())) {
            root_order.push((target_key.group.clone(), target_key.name.clone()));
        }
        active_edges.push(ActiveSourceEdge {
            id: original.id.clone(),
            alternative: *alternative,
            target: target_key,
        });
    }
    for (key, id) in &real {
        if !reachable.contains(id) {
            return Err(invalid("selected package has no original rooted support"));
        }
        let Some(Record::Real { candidate, .. }) = model.records.get(*id) else {
            return Err(invalid("selected package record disappeared"));
        };
        if !model.world.entries[*candidate].default_eligible && !pinned.contains(key) {
            return Err(invalid(
                "unpinned candidate escaped the default source/version domain",
            ));
        }
    }
    decisions::annotate(
        &mut selected_before_projection,
        &active_edges,
        request,
        Some(model.world),
    )?;
    let selected_views = selected_before_projection
        .values()
        .map(|metadata| {
            (
                format!(
                    "{}/{}",
                    metadata.candidate.group(),
                    metadata.candidate.name()
                ),
                metadata.candidate.version().clone(),
                metadata.view.clone(),
            )
        })
        .collect::<Vec<_>>();
    crate::resolvo_engine::verify_source_capabilities(&selected_views)?;
    let mut chosen = HashMap::new();
    let mut obsolete = HashSet::new();
    for (key, metadata) in &selected_before_projection {
        for reference in &metadata.view.obsoletes.packages {
            if let Some(group) = &reference.group {
                obsolete.insert((group.clone(), reference.name.to_string()));
            }
        }
        chosen.insert(
            key.clone(),
            crate::Chosen {
                version: metadata.candidate.version().clone(),
                direct_deps: metadata.view.resolution_requires(),
                is_root: false,
            },
        );
    }
    // Keep root order tied to original request ordering rather than solver IDs.
    root_order.sort_by_key(|key| {
        request
            .roots
            .iter()
            .position(|root| {
                root.requirement.alternatives.iter().any(|choice| {
                    choice.reference.group.as_ref() == Some(&key.0)
                        && choice.reference.name == key.1
                })
            })
            .unwrap_or(request.roots.len())
    });
    let graph = crate::build_resolved_graph(&root_order, chosen, &obsolete);
    let selected = graph
        .packages
        .iter()
        .filter_map(|node| {
            selected_before_projection
                .get(&(node.group.clone(), node.name.clone()))
                .map(|metadata| ((node.group.clone(), node.name.clone()), metadata.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    for metadata in selected.values() {
        if let Some(error) = &metadata.configuration_error {
            return Err(DepProviderError::Configuration(error.clone()).into());
        }
    }
    for root in request.roots {
        for choice in &root.requirement.alternatives {
            if let Some(group) = &choice.reference.group
                && let Some(metadata) =
                    selected.get(&(group.clone(), choice.reference.name.to_string()))
                && let Some(kind) = choice.reference.kind
            {
                crate::assert_kind_matches(
                    Some(kind),
                    metadata
                        .candidate
                        .manifest()
                        .require_package()
                        .map_err(|error| DepProviderError::Other(error.to_string()))?
                        .kind,
                    &choice.reference.qualified_name(),
                )?;
            }
        }
    }
    if let Some(configuration) = request.configuration {
        configuration.refuse_unmerged_incoming_edges(&graph)?;
    }
    provider.validate_selected(&graph)?;
    Ok(SourceResolvedGraph {
        graph,
        selected,
        selected_before_projection,
        active_edges,
    })
}
