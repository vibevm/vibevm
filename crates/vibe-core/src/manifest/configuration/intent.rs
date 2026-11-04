//! Authored consumer input and target-aware normalization, without evaluation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#INPUT-NORMALIZATION");
use super::*;
use crate::configuration::{
    ChoiceAnswer, ChoicePolicy, ConsumerChoices, FeatureBase, FeatureInput, Selection,
    valid_identifier,
};
use crate::manifest::{FieldError, FieldPath};
use std::collections::BTreeSet;

fn yes() -> bool {
    true
}
fn is_yes(value: &bool) -> bool {
    *value
}

/// Original request on a real requires edge. Policy is deliberately mandatory.
/// ```
/// use vibe_core::manifest::ConfigurationIntentDecl;
/// let input: ConfigurationIntentDecl=toml::from_str("policy='require-selection'\nselections.mode={intent='exact',members=[]}").unwrap();
/// assert_eq!(input.selections["mode"].members,Some(vec![]));
/// assert!(input.default_features);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-INTENT")]
pub struct ConfigurationIntentDecl {
    #[serde(with = "super::vocabulary::policy")]
    pub policy: ChoicePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub default_features: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub selections: BTreeMap<String, ConfigurationSelection>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, ParameterValue>,
}

/// Inputs for the existing evaluator; not resolved, executable or candidate-bound.
/// The caller retains the original authored request separately.
/// ```
/// use vibe_core::manifest::{ConfigurationDecl,ConfigurationIntentDecl};
/// let d=ConfigurationDecl::default();
/// let i:ConfigurationIntentDecl=toml::from_str("policy='require-selection'").unwrap();
/// let n=d.normalize_input(&i,"project/dependency").unwrap();
/// assert!(n.parameters.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#INPUT-NORMALIZATION")]
pub struct NormalizedConfigurationInput {
    pub features: FeatureInput,
    pub choices: ConsumerChoices,
    pub parameters: BTreeMap<String, ParameterValue>,
}
impl ConfigurationIntentDecl {
    pub(crate) fn validate_shape_at(&self, at: &FieldPath) -> Result<(), FieldError> {
        let check = |id: &str, field: FieldPath| {
            if valid_identifier(id) {
                Ok(())
            } else {
                Err(FieldError::new(
                    field,
                    "identifier must be nonblank without control characters",
                ))
            }
        };
        if let Some(profile) = &self.profile {
            check(profile, at.field("profile"))?;
        }
        let mut seen = BTreeSet::new();
        for id in &self.features {
            check(id, at.field("features"))?;
            if !seen.insert(id) {
                return Err(FieldError::new(
                    at.field("features"),
                    format!("duplicate feature `{id}`"),
                ));
            }
        }
        for (id, answer) in &self.selections {
            let row = at.table("selections").field(id);
            check(id, row.clone())?;
            let mut seen = BTreeSet::new();
            for member in answer.members.iter().flatten() {
                check(member, row.field("members"))?;
                if !seen.insert(member) {
                    return Err(FieldError::new(
                        row.field("members"),
                        format!("duplicate member `{member}`"),
                    ));
                }
            }
        }
        for id in self.parameters.keys() {
            check(id, at.table("parameters").field(id))?;
        }
        Ok(())
    }
}
impl ConfigurationDecl {
    /// Validate target references and overlay explicit answers on a named profile.
    /// Missing values/defaults and questions remain for the actual domain owner.
    /// ```
    /// use vibe_core::manifest::{ConfigurationDecl,ConfigurationIntentDecl};
    /// use vibe_core::configuration::FeatureBase;
    /// let d=ConfigurationDecl::default();
    /// let i:ConfigurationIntentDecl=toml::from_str("policy='require-selection'\ndefault_features=false")?;
    /// let n=d.normalize_input(&i,"project/edge")?;
    /// assert_eq!(n.features.base,FeatureBase::Selected(vec![]));
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    #[track_caller]
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#INPUT-NORMALIZATION"
    )]
    pub fn normalize_input(
        &self,
        input: &ConfigurationIntentDecl,
        consumer: &str,
    ) -> Result<NormalizedConfigurationInput, FieldError> {
        self.compile_choices()?;
        let at = FieldPath::root().table("configuration");
        input.validate_shape_at(&at)?;
        if consumer.trim().is_empty() || consumer.chars().any(char::is_control) {
            return Err(FieldError::new(
                FieldPath::root().field("consumer"),
                "diagnostic consumer key must be nonblank without controls",
            ));
        }
        let profile = input
            .profile
            .as_ref()
            .map(|id| {
                self.profiles.iter().find(|p| &p.id == id).ok_or_else(|| {
                    FieldError::new(at.field("profile"), format!("unknown profile `{id}`"))
                })
            })
            .transpose()?;
        for id in &input.features {
            let feature = self.features.iter().find(|f| &f.id == id).ok_or_else(|| {
                FieldError::new(at.field("features"), format!("unknown feature `{id}`"))
            })?;
            if feature.visibility == ConfigurationFeatureVisibility::Internal {
                return Err(FieldError::new(
                    at.field("features"),
                    format!("internal feature `{id}` cannot be requested directly"),
                ));
            }
        }
        let mut selections = profile.map(|p| p.selections.clone()).unwrap_or_default();
        for (id, answer) in &input.selections {
            let row = at.table("selections").field(id);
            let Some(ConfigurationGroupDecl::Choice { members, .. }) =
                self.groups.iter().find(|g| match g {
                    ConfigurationGroupDecl::Choice { id: group, .. }
                    | ConfigurationGroupDecl::Closure { id: group, .. } => group == id,
                })
            else {
                return Err(FieldError::new(row, format!("unknown choice group `{id}`")));
            };
            for member in answer.members.iter().flatten() {
                if !members.contains(member) {
                    return Err(FieldError::new(
                        row.field("members"),
                        format!("unknown choice member `{member}`"),
                    ));
                }
            }
            selections.insert(id.clone(), answer.clone());
        }
        let mut parameters = profile.map(|p| p.parameters.clone()).unwrap_or_default();
        for (id, value) in &input.parameters {
            let row = at.table("parameters").field(id);
            let parameter = self
                .parameters
                .iter()
                .find(|p| &p.id == id)
                .ok_or_else(|| FieldError::new(row.clone(), format!("unknown parameter `{id}`")))?;
            if parameter.secret {
                return Err(FieldError::new(
                    row,
                    "secret parameter requires an ephemeral operation input, not an authored value",
                ));
            }
            let mut with_value = parameter.clone();
            with_value.default = Some(value.clone());
            super::super::package::application::validate_parameter(
                &with_value,
                "authored parameter input",
                &row,
            )
            .map_err(|error| FieldError::new(row, error.message))?;
            parameters.insert(id.clone(), value.clone());
        }
        let base = match profile {
            Some(p) => FeatureBase::Selected(p.features.clone()),
            None if input.default_features => FeatureBase::Defaults,
            None => FeatureBase::Selected(vec![]),
        };
        let answers = selections
            .into_iter()
            .map(|(id, answer)| {
                (
                    id,
                    ChoiceAnswer {
                        intent: answer.intent,
                        selection: match answer.members {
                            Some(v) => Selection::Explicit(v),
                            None => Selection::Omitted,
                        },
                    },
                )
            })
            .collect();
        Ok(NormalizedConfigurationInput {
            features: FeatureInput {
                base,
                additions: input.features.clone(),
                disabled: vec![],
            },
            choices: ConsumerChoices {
                key: consumer.into(),
                policy: input.policy,
                answers,
            },
            parameters,
        })
    }
}

#[cfg(test)]
mod tests;
