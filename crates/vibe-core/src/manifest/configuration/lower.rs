//! Lower one retained ordinary declaration into the existing pure evaluators.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-DEFINITIONS");
use super::*;
use crate::configuration::{
    ChoiceGroup, ChoiceModel, ClosureGroup, FeatureModel, FeatureNode, MemberGuards,
};
use crate::manifest::{FieldError, FieldPath};

impl ConfigurationDecl {
    /// Compile local feature/choice semantics. Parameters and typed effects remain
    /// in this declaration; this result does not claim their runtime evaluation
    /// or foreign source admission. Manifest validation binds dependency targets.
    /// ```
    /// use vibe_core::manifest::ConfigurationDecl;
    /// use vibe_core::configuration::{FeatureInput, FeatureBase, ChoiceOutcome};
    /// let definition: ConfigurationDecl = toml::from_str("")?;
    /// let model = definition.compile_choices()?;
    /// let result = model.evaluate(&FeatureInput { base: FeatureBase::Defaults,
    ///     additions: vec![], disabled: vec![] }, &[]);
    /// assert!(matches!(result.outcome, ChoiceOutcome::Resolved { .. }));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[track_caller]
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-DEFINITIONS"
    )]
    pub fn compile_choices(&self) -> Result<ChoiceModel, FieldError> {
        self.compile_choices_at(&FieldPath::root().table("configuration"))
    }

    pub(crate) fn compile_choices_at(&self, at: &FieldPath) -> Result<ChoiceModel, FieldError> {
        self.validate_local(at)?;
        let nodes = self
            .features
            .iter()
            .map(|f| FeatureNode {
                id: f.id.clone(),
                default: f.default,
                parent: f.parent.clone(),
                required: f.required,
                requires: f.requires.clone(),
            })
            .collect();
        let mut closure = Vec::new();
        let mut choices = Vec::new();
        for group in &self.groups {
            match group {
                ConfigurationGroupDecl::Choice {
                    id,
                    members,
                    cardinality,
                    defaults,
                    guards,
                    ..
                } => choices.push(ChoiceGroup {
                    id: id.clone(),
                    members: members.clone(),
                    cardinality: *cardinality,
                    defaults: defaults.clone(),
                    guards: guards.iter().map(Into::into).collect(),
                }),
                ConfigurationGroupDecl::Closure {
                    id,
                    members,
                    cardinality,
                    fallback,
                    ..
                } => closure.push(ClosureGroup {
                    id: id.clone(),
                    members: members.clone(),
                    cardinality: *cardinality,
                    fallback: fallback.clone(),
                }),
            }
        }
        let features = FeatureModel::new(nodes, closure)
            .map_err(|e| FieldError::new(at.table("features"), e.to_string()))?;
        let guards = self
            .features
            .iter()
            .filter(|f| !f.guards.is_empty())
            .map(|f| MemberGuards {
                member: f.id.clone(),
                guards: f.guards.iter().map(Into::into).collect(),
            })
            .collect();
        ChoiceModel::new(features, choices, guards)
            .map_err(|e| FieldError::new(at.table("groups"), e.to_string()))
    }
}
