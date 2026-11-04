//! Source-independent submitted input and CLI edits of retained original answers.
use super::{
    ConfigurationSubmission,
    parameters::typed,
    vocabulary::{source_input, value},
};
use crate::FeatureRequest;
use std::collections::BTreeSet;
use vibe_core::{configuration as k, manifest as m};
use vibe_wire::generated::shared as w;

/// Strict generated input admission. Invalid payload errors never echo supplied values.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-JOURNEY"
)]
pub fn parse_submission(bytes: &[u8]) -> Result<ConfigurationSubmission, String> {
    let v: ConfigurationSubmission = serde_json::from_slice(bytes).map_err(|e| {
        format!(
            "invalid configuration input at line {}, column {}",
            e.line(),
            e.column()
        )
    })?;
    if v.protocol != "vibe-configuration-input/1" {
        return Err("unsupported configuration input protocol".into());
    }
    Ok(v)
}
pub(super) struct EditedInput {
    pub input: w::ConfigurationInput,
    pub ephemeral: Vec<(String, String)>,
    pub fresh: BTreeSet<String>,
}
pub(super) fn edit(
    d: &m::ConfigurationDecl,
    submission: Option<&ConfigurationSubmission>,
    previous: Option<&w::ConfigurationRecord>,
    features: &FeatureRequest,
    set: &[(String, String)],
) -> Result<EditedInput, String> {
    let mut input = submission
        .map(|s| s.input.clone())
        .or_else(|| previous.map(|p| p.evaluation.input.clone()))
        .unwrap_or_else(|| w::ConfigurationInput {
            intent: source_input(&m::ConfigurationIntentDecl {
                policy: k::ChoicePolicy::RequireSelection,
                profile: None,
                features: vec![],
                default_features: true,
                selections: Default::default(),
                parameters: Default::default(),
            }),
            disabled_features: vec![],
        });
    if let Some(profile) = &features.profile {
        input.intent.profile = Some(profile.clone());
    }
    for id in &features.enable {
        if features.disable.contains(id) {
            return Err(format!(
                "feature `{id}` is both enabled and disabled in this request"
            ));
        }
    }
    if !features.enable.is_empty() || !features.disable.is_empty() {
        let model = d
            .compile_choices()
            .map_err(|e| format!("{}: {e}", e.field))?;
        input.intent.features = model
            .prepare_feature_delta(&k::FeatureInput {
                base: k::FeatureBase::Selected(input.intent.features),
                additions: features.enable.clone(),
                disabled: features.disable.clone(),
            })
            .map_err(|e| e.to_string())?;
        input
            .disabled_features
            .retain(|id| !features.enable.contains(id));
        input
            .disabled_features
            .extend(features.disable.iter().cloned());
        input.disabled_features.sort();
        input.disabled_features.dedup();
    }
    let mut ephemeral = Vec::new();
    let mut fresh = BTreeSet::new();
    for (id, raw) in set {
        let p = d
            .parameters
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| format!("unknown parameter `{id}`"))?;
        if p.secret {
            ephemeral.push((id.clone(), raw.clone()));
            continue;
        }
        let canonical = p.canonical(raw)?;
        input
            .intent
            .parameters
            .insert(id.clone(), value(&typed(p.kind, &canonical)?));
        fresh.insert(id.clone());
    }
    Ok(EditedInput {
        input,
        ephemeral,
        fresh,
    })
}
