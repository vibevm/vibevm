//! Original-edge witnesses with a finite rooted support forest.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::*;
use resolvo::utils::{Pool, VersionSet};
use resolvo::{NameId, Requirement, VersionSetId};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Name {
    Package(Coordinate),
    Witness {
        edge: usize,
        parent_rank: Option<usize>,
    },
    Impossible,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Set(pub BTreeSet<usize>);
impl VersionSet for Set {
    type V = usize;
}
impl fmt::Display for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} retained choices", self.0.len())
    }
}
#[derive(Clone, Debug)]
pub(super) enum Record {
    Real {
        candidate: usize,
        rank: usize,
    },
    Witness {
        edge: usize,
        parent: Option<usize>,
        target: usize,
        alternative: usize,
        pinned: bool,
    },
}
#[derive(Clone)]
pub(super) struct Edge {
    pub id: crate::source_candidates::EdgeId,
    pub parent: Option<usize>,
    pub choices: Vec<Choice>,
}
#[derive(Clone)]
pub(super) struct Choice {
    pub target: usize,
    pub alternative: usize,
    pub pinned: bool,
}
pub(super) struct Model<'a> {
    pub pool: Pool<Set, Name>,
    pub world: &'a Universe,
    pub edges: Vec<Edge>,
    pub records: Vec<Record>,
    pub names: Vec<NameId>,
    pub by_name: HashMap<Name, Vec<usize>>,
    pub requirements: Vec<Vec<Requirement>>,
    pub constraints: Vec<Vec<VersionSetId>>,
    pub roots: Vec<Requirement>,
    pub root_constraints: Vec<VersionSetId>,
}
impl<'a> Model<'a> {
    pub fn new(
        world: &'a Universe,
        edges: Vec<Edge>,
        request: &SourceSolveRequest<'_>,
    ) -> Result<Self, SolveError> {
        let names = world
            .entries
            .iter()
            .map(|entry| coordinate(&entry.metadata))
            .collect::<BTreeSet<_>>();
        checked(
            SourceModelResource::PackageNames,
            names.len(),
            request.limits.package_names,
        )?;
        let rank_count = names.len();
        let ranked = world.entries.len().checked_mul(rank_count).ok_or_else(|| {
            limit(
                SourceModelResource::RankedCandidates,
                usize::MAX,
                request.limits.ranked_candidates,
            )
        })?;
        checked(
            SourceModelResource::RankedCandidates,
            ranked,
            request.limits.ranked_candidates,
        )?;
        let mut witness_count = 0usize;
        for edge in &edges {
            let per_choice = if edge.parent.is_some() {
                rank_count.checked_mul(rank_count)
            } else {
                Some(1)
            };
            let count = per_choice
                .and_then(|n| n.checked_mul(edge.choices.len()))
                .ok_or_else(|| {
                    limit(
                        SourceModelResource::EdgeWitnesses,
                        usize::MAX,
                        request.limits.edge_witnesses,
                    )
                })?;
            witness_count = witness_count.checked_add(count).ok_or_else(|| {
                limit(
                    SourceModelResource::EdgeWitnesses,
                    usize::MAX,
                    request.limits.edge_witnesses,
                )
            })?;
            checked(
                SourceModelResource::EdgeWitnesses,
                witness_count,
                request.limits.edge_witnesses,
            )?;
        }
        let mut model = Self {
            pool: Pool::new(),
            world,
            edges,
            records: Vec::new(),
            names: Vec::new(),
            by_name: HashMap::new(),
            requirements: Vec::new(),
            constraints: Vec::new(),
            roots: Vec::new(),
            root_constraints: Vec::new(),
        };
        let mut real = Vec::new();
        for (candidate, entry) in world.entries.iter().enumerate() {
            let mut ranked_ids = Vec::new();
            for rank in 0..rank_count {
                ranked_ids.push(model.add(
                    Name::Package(coordinate(&entry.metadata)),
                    Record::Real { candidate, rank },
                ));
            }
            real.push(ranked_ids);
        }
        let real_count = model.records.len();
        for edge_index in 0..model.edges.len() {
            let edge = model.edges[edge_index].clone();
            let parents = match edge.parent {
                Some(parent) => real[parent].iter().copied().map(Some).collect(),
                None => vec![None],
            };
            for parent in parents {
                let parent_rank = parent.and_then(|id| match model.records[id] {
                    Record::Real { rank, .. } => Some(rank),
                    _ => None,
                });
                let name = Name::Witness {
                    edge: edge_index,
                    parent_rank,
                };
                for choice in &edge.choices {
                    for (rank, target) in real[choice.target].iter().copied().enumerate() {
                        if parent.is_none() && rank != 0 {
                            continue;
                        }
                        let id = model.add(
                            name.clone(),
                            Record::Witness {
                                edge: edge_index,
                                parent,
                                target,
                                alternative: choice.alternative,
                                pinned: choice.pinned,
                            },
                        );
                        if let Some(parent) = parent {
                            let requirement = model.exact(parent);
                            model.requirements[id].push(requirement);
                        }
                        let requirement = model.exact(target);
                        model.requirements[id].push(requirement);
                    }
                }
                let requirement = model.any(&name);
                if let Some(parent) = parent {
                    model.requirements[parent].push(requirement);
                } else {
                    model.roots.push(requirement);
                }
            }
        }
        for id in 0..real_count {
            let Record::Real { candidate, rank } = model.records[id] else {
                return Err(invalid("real candidate layout changed"));
            };
            let mut support = Vec::new();
            let mut pins = Vec::new();
            for (witness, record) in model.records.iter().enumerate().skip(real_count) {
                if let Record::Witness {
                    parent,
                    target,
                    pinned,
                    ..
                } = record
                {
                    if *target != id {
                        continue;
                    }
                    let smaller=parent.is_none()||parent.is_some_and(|parent|matches!(model.records[parent],Record::Real{rank:prior,..} if prior<rank));
                    if smaller {
                        support.push(witness);
                    }
                    if *pinned {
                        pins.push(witness);
                    }
                }
            }
            let requirement = model.union(support);
            model.requirements[id].push(requirement);
            if !world.entries[candidate].default_eligible {
                let requirement = model.union(pins);
                model.requirements[id].push(requirement);
            }
            for conflict in &world.entries[candidate].metadata.view.conflicts.packages {
                if let Some(group) = &conflict.group {
                    let constraint = model.set(
                        Name::Package((group.clone(), conflict.name.to_string())),
                        [],
                    );
                    model.constraints[id].push(constraint);
                } else {
                    return Err(invalid("unqualified conflict in retained metadata"));
                }
            }
        }
        for held in request.held_roots {
            let Some(group) = &held.group else {
                return Err(invalid("unqualified held root"));
            };
            let name = Name::Package((group.clone(), held.name.to_string()));
            let ids = model
                .by_name
                .get(&name)
                .into_iter()
                .flatten()
                .copied()
                .filter(|id| match model.records[*id] {
                    Record::Real { candidate, .. } => held
                        .version
                        .matches(world.entries[candidate].metadata.candidate.version()),
                    _ => false,
                })
                .collect::<Vec<_>>();
            let constraint = model.set(name, ids);
            model.root_constraints.push(constraint);
        }
        Ok(model)
    }
    fn add(&mut self, name: Name, record: Record) -> usize {
        let id = self.records.len();
        let name_id = self.pool.intern_package_name(name.clone());
        self.pool.intern_solvable(name_id, id);
        self.names.push(name_id);
        self.records.push(record);
        self.by_name.entry(name).or_default().push(id);
        self.requirements.push(Vec::new());
        self.constraints.push(Vec::new());
        id
    }
    fn set(&self, name: Name, ids: impl IntoIterator<Item = usize>) -> VersionSetId {
        self.pool.intern_version_set(
            self.pool.intern_package_name(name),
            Set(ids.into_iter().collect()),
        )
    }
    fn any(&self, name: &Name) -> Requirement {
        self.set(
            name.clone(),
            self.by_name.get(name).cloned().unwrap_or_default(),
        )
        .into()
    }
    fn exact(&self, id: usize) -> Requirement {
        self.pool
            .intern_version_set(self.names[id], Set(BTreeSet::from([id])))
            .into()
    }
    fn union(&self, ids: Vec<usize>) -> Requirement {
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        // Numeric record-name representatives avoid ordering canonical source URLs.
        for id in ids {
            let representative = self
                .by_name
                .get(self.pool.resolve_package_name(self.names[id]))
                .and_then(|ids| ids.first())
                .copied()
                .unwrap_or(id);
            groups.entry(representative).or_default().push(id);
        }
        let sets = groups
            .into_iter()
            .map(|(id, ids)| {
                self.pool
                    .intern_version_set(self.names[id], Set(ids.into_iter().collect()))
            })
            .collect::<Vec<_>>();
        match sets.split_first() {
            Some((first, rest)) => self
                .pool
                .intern_version_set_union(*first, rest.iter().copied())
                .into(),
            None => self.any(&Name::Impossible),
        }
    }
}
