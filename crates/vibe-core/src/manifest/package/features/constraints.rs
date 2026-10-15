//! Explicit consumer choices layered on the existing feature algebra.
use super::effects::{
    DependencyDeclarationId, EffectError, FeatureDeclarations, FeatureProgram, LocalFeatureRef,
};
use crate::environment::{
    DefinitionId, DependencyRole, LockId, LockTargetRole, Reference, TargetInstanceId,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
    ExactlyOne,
}
impl Cardinality {
    pub fn accepts(self, count: usize) -> bool {
        match self {
            Self::ZeroOrMore => true,
            Self::OneOrMore => count > 0,
            Self::ZeroOrOne => count <= 1,
            Self::ExactlyOne => count == 1,
        }
    }
}

/// Positive references only. Parsing never accepts expressions or package paths.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Guard {
    Group(LocalFeatureRef),
    Member(LocalFeatureRef),
}
impl Guard {
    pub fn parse(kind: &str, name: &str) -> Result<Self, ConstraintError> {
        let name = LocalFeatureRef::new(name)?;
        match kind {
            "group" => Ok(Self::Group(name)),
            "member" => Ok(Self::Member(name)),
            _ => Err(ConstraintError::Invalid("unsupported guard kind".into())),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstrainedGroup {
    pub cardinality: Cardinality,
    pub members: BTreeSet<LocalFeatureRef>,
    pub guards: Vec<Guard>,
    pub defaults: Option<BTreeSet<LocalFeatureRef>>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Constraints {
    pub groups: BTreeMap<LocalFeatureRef, ConstrainedGroup>,
    pub member_guards: BTreeMap<LocalFeatureRef, Vec<Guard>>,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConstraintError {
    #[error(transparent)]
    Effect(#[from] EffectError),
    #[error("invalid constrained definition: {0}")]
    Invalid(String),
    #[error("cyclic guard dependency: {0:?}")]
    GuardCycle(Guard),
}
impl Constraints {
    /// Validate dormant definitions too, without selecting any alternatives.
    /// Edges here are guard dependencies, never ordinary feature effects.
    pub fn order(
        &self,
        program: &FeatureProgram,
        declarations: &FeatureDeclarations,
    ) -> Result<Vec<Guard>, ConstraintError> {
        program.validate(declarations)?;
        let mut graph: BTreeMap<Guard, Vec<Guard>> = BTreeMap::new();
        let mut members = BTreeSet::new();
        for (name, group) in &self.groups {
            if group.members.is_empty() {
                return Err(ConstraintError::Invalid("empty group".into()));
            }
            if let Some(defaults) = &group.defaults {
                if !defaults.is_subset(&group.members) || !group.cardinality.accepts(defaults.len())
                {
                    return Err(ConstraintError::Invalid("invalid declared defaults".into()));
                }
            }
            graph.insert(Guard::Group(name.clone()), group.guards.clone());
            for member in &group.members {
                if !program.features.contains_key(member)
                    || member.as_str().starts_with('_')
                    || !members.insert(member.clone())
                {
                    return Err(ConstraintError::Invalid(format!(
                        "unknown, private or multiply owned member {}",
                        member.as_str()
                    )));
                }
                let mut dependencies = self.member_guards.get(member).cloned().unwrap_or_default();
                dependencies.push(Guard::Group(name.clone()));
                graph.insert(Guard::Member(member.clone()), dependencies);
            }
        }
        if self.member_guards.keys().any(|m| !members.contains(m)) {
            return Err(ConstraintError::Invalid("dangling member guard".into()));
        }
        for dependency in graph.values().flatten() {
            if !graph.contains_key(dependency) {
                return Err(ConstraintError::Invalid(format!(
                    "unknown guard {dependency:?}"
                )));
            }
        }
        fn visit(
            node: &Guard,
            graph: &BTreeMap<Guard, Vec<Guard>>,
            visiting: &mut BTreeSet<Guard>,
            done: &mut BTreeSet<Guard>,
            out: &mut Vec<Guard>,
        ) -> Result<(), ConstraintError> {
            if done.contains(node) {
                return Ok(());
            }
            if !visiting.insert(node.clone()) {
                return Err(ConstraintError::GuardCycle(node.clone()));
            }
            for next in &graph[node] {
                visit(next, graph, visiting, done, out)?;
            }
            visiting.remove(node);
            done.insert(node.clone());
            out.push(node.clone());
            Ok(())
        }
        let mut out = Vec::new();
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        for node in graph.keys() {
            visit(node, &graph, &mut visiting, &mut done, &mut out)?;
        }
        Ok(out)
    }
    pub fn members(&self) -> BTreeSet<LocalFeatureRef> {
        self.groups
            .values()
            .flat_map(|g| g.members.iter().cloned())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Omitted,
    Explicit(BTreeSet<LocalFeatureRef>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionIntent {
    Additive,
    Exact,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultPolicy {
    RequireSelection,
    UseDeclaredDefaults,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedSelection {
    pub selection: Selection,
    pub intent: SelectionIntent,
    pub provenance: Reference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionDomain {
    pub definition: DefinitionId,
    pub lock: Option<LockId>,
    pub target: TargetInstanceId,
    pub target_role: LockTargetRole,
    pub identity: Reference,
}
/// Native identifiers never pass through Vibe's PackageRef/SemVer parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageIdentity {
    Vibe(crate::PackageRef),
    Native {
        backend: Reference,
        package: Reference,
        version: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationInstance {
    pub domain: ResolutionDomain,
    pub identity: Reference,
    pub configuration: Reference,
    pub package: PackageIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerEdge {
    pub consumer: Reference,
    pub dependency: DependencyDeclarationId,
    pub role: DependencyRole,
    pub instance: ConfigurationInstance,
    pub policy: DefaultPolicy,
    pub selections: BTreeMap<LocalFeatureRef, RequestedSelection>,
    pub features: BTreeSet<LocalFeatureRef>,
    pub all_features: bool,
}

#[cfg(test)]
mod m_05_b_2_cases;
