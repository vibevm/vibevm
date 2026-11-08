//! Existing resolvo callbacks over a frozen source-aware model.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::{
    model::{Model, Name, Record},
    *,
};
use resolvo::{
    Candidates, Condition, ConditionId, Dependencies, DependencyProvider,
    HintDependenciesAvailable, Interner, KnownDependencies, NameId, SolvableId, SolverCache,
    StringId, VersionSetId, VersionSetUnionId,
};
use std::cell::RefCell;
use std::fmt;

pub(super) struct Provider<'a> {
    pub model: Model<'a>,
    pub fault: RefCell<Option<String>>,
}
impl Provider<'_> {
    fn fail(&self, reason: &str) {
        let mut fault = self.fault.borrow_mut();
        if fault.is_none() {
            *fault = Some(reason.into());
        }
    }
    fn record(&self, id: SolvableId) -> Option<&Record> {
        let index = self.model.pool.resolve_solvable(id).record;
        self.model.records.get(index)
    }
    fn order(&self, id: SolvableId) -> Option<(semver::Version, usize, usize, usize)> {
        let record = self.record(id)?;
        let (candidate, rank) = match record {
            Record::Real { candidate, rank } => (*candidate, *rank),
            Record::Witness { target, .. } => match self.model.records.get(*target)? {
                Record::Real { candidate, rank } => (*candidate, *rank),
                _ => return None,
            },
        };
        let entry = self.model.world.entries.get(candidate)?;
        Some((
            entry.metadata.candidate.version().clone(),
            entry.preference_rank,
            entry.registry_order,
            rank,
        ))
    }
}
impl Interner for Provider<'_> {
    type NameId = NameId;
    type SolvableId = SolvableId;
    fn display_solvable(&self, id: SolvableId) -> impl fmt::Display + '_ {
        match self.record(id) {
            Some(Record::Real { candidate, rank }) => {
                let item = &self.model.world.entries[*candidate].metadata.candidate;
                format!(
                    "{}/{}@{} [{}] support-depth={rank}",
                    item.group(),
                    item.name(),
                    item.version(),
                    item.binding().registry_name().unwrap_or("non-registry")
                )
            }
            Some(Record::Witness { edge, .. }) => {
                format!("original edge {:?}", self.model.edges[*edge].id)
            }
            None => "invalid retained source record".into(),
        }
    }
    fn display_name(&self, id: NameId) -> impl fmt::Display + '_ {
        format!("{:?}", self.model.pool.resolve_package_name(id))
    }
    fn display_version_set(&self, id: VersionSetId) -> impl fmt::Display + '_ {
        self.model.pool.resolve_version_set(id)
    }
    fn display_string(&self, id: StringId) -> impl fmt::Display + '_ {
        self.model.pool.resolve_string(id)
    }
    fn version_set_name(&self, id: VersionSetId) -> NameId {
        self.model.pool.resolve_version_set_package_name(id)
    }
    fn solvable_name(&self, id: SolvableId) -> NameId {
        self.model.pool.resolve_solvable(id).name
    }
    fn version_sets_in_union(&self, id: VersionSetUnionId) -> impl Iterator<Item = VersionSetId> {
        self.model.pool.resolve_version_set_union(id)
    }
    fn resolve_condition(&self, _: ConditionId) -> Condition {
        self.fail("source model unexpectedly requested a final-truth condition; historical activation belongs to planning");
        let name = self.model.pool.intern_package_name(Name::Impossible);
        Condition::Requirement(
            self.model
                .pool
                .intern_version_set(name, model::Set(Default::default())),
        )
    }
}
impl DependencyProvider for Provider<'_> {
    async fn filter_candidates(
        &self,
        ids: &[SolvableId],
        set: VersionSetId,
        inverse: bool,
    ) -> Vec<SolvableId> {
        ids.iter()
            .copied()
            .filter(|id| {
                self.model
                    .pool
                    .resolve_version_set(set)
                    .0
                    .contains(&self.model.pool.resolve_solvable(*id).record)
                    != inverse
            })
            .collect()
    }
    async fn get_candidates(&self, name: NameId) -> Option<Candidates> {
        Some(Candidates {
            candidates: self
                .model
                .by_name
                .get(self.model.pool.resolve_package_name(name))
                .into_iter()
                .flatten()
                .map(|id| self.model.pool.intern_solvable(name, *id))
                .collect(),
            favored: None,
            locked: None,
            hint_dependencies_available: HintDependenciesAvailable::All,
            excluded: Vec::new(),
        })
    }
    async fn sort_candidates(&self, _: &SolverCache<Self>, ids: &mut [SolvableId]) {
        ids.sort_by(|a, b| match (self.order(*a), self.order(*b)) {
            (Some(a), Some(b)) => {
                b.0.cmp(&a.0)
                    .then_with(|| a.1.cmp(&b.1))
                    .then_with(|| a.2.cmp(&b.2))
                    .then_with(|| a.3.cmp(&b.3))
            }
            _ => {
                self.fail("source candidate sort lost its retained metadata");
                std::cmp::Ordering::Equal
            }
        });
    }
    async fn get_dependencies(&self, id: SolvableId) -> Dependencies {
        let index = self.model.pool.resolve_solvable(id).record;
        match (
            self.model.requirements.get(index),
            self.model.constraints.get(index),
        ) {
            (Some(requirements), Some(constrains)) => Dependencies::Known(KnownDependencies {
                requirements: requirements.iter().copied().map(Into::into).collect(),
                constrains: constrains.clone(),
            }),
            _ => {
                self.fail("source dependency lookup lost its retained record");
                Dependencies::Unknown(self.model.pool.intern_string("invalid source model record"))
            }
        }
    }
}
