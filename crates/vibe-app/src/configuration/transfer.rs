//! Reassess transported common history against an explicitly selected target.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT");
use super::{EvaluatedConfiguration, EvaluationRequest, HistoryUse, evaluate, portable};
use crate::{HostFacts, ParameterPrompt};
use std::collections::BTreeSet;
use vibe_core::manifest::ConfigurationDecl;
use vibe_wire::generated::shared as w;

pub(crate) struct TransferRequest<'a> {
    pub definition: &'a ConfigurationDecl,
    pub history: &'a w::PortableConfigurationHistory,
    pub target: &'a w::PortableTargetInput,
    pub ephemeral: &'a [(String, String)],
    pub facts: HostFacts<'a>,
    pub prompt: Option<ParameterPrompt<'a>>,
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT"
)]
pub(crate) fn prepare(request: TransferRequest<'_>) -> Result<EvaluatedConfiguration, String> {
    prepare_required(request, &[])
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT"
)]
pub(crate) fn prepare_required(
    request: TransferRequest<'_>,
    required_secrets: &[String],
) -> Result<EvaluatedConfiguration, String> {
    let history = portable::decode(request.history)?;
    for parameter in &history.parameters {
        if let w::PortableParameterHistory::Public(p) = parameter
            && (p.source.is_some() || p.realization.is_some())
            && request
                .definition
                .parameters
                .iter()
                .any(|target| target.id == p.id && target.secret)
        {
            return Err(format!(
                "historically public parameter `{}` is now secret; public transfer is refused",
                p.id
            ));
        }
    }
    let current = request
        .definition
        .semantic_fingerprint()
        .map_err(|e| format!("{}: {e}", e.field))?;
    let replacement = request.target.configuration.as_ref();
    if let Some(value) = replacement
        && (value.protocol != "vibe-configuration-input/1" || value.expected_definition != current)
    {
        return Err(
            "target replacement must name the exact current configuration definition".into(),
        );
    }
    if request
        .target
        .review_definition
        .as_ref()
        .is_some_and(|expected| expected != &current)
    {
        return Err(
            "target review does not name the exact current configuration definition".into(),
        );
    }
    let input = replacement.map(|v| &v.input).unwrap_or(&history.input);
    evaluate(EvaluationRequest {
        definition: request.definition,
        input,
        previous: Some(history.view()),
        history_use: HistoryUse::Transfer {
            path_bindings: &request.target.path_bindings,
            required_secrets,
        },
        replacement: replacement.is_some(),
        reviewed: request.target.review_definition.is_some(),
        expected_definition: Some(&current),
        fresh_parameters: &BTreeSet::new(),
        ephemeral: request.ephemeral,
        facts: request.facts,
        prompt: request.prompt,
    })
}

#[cfg(test)]
#[path = "transfer/tests.rs"]
mod tests;

/// Public transfer facts consumed by the ordinary installer planner. Answers are
/// separate ephemeral parameters; this value contains only historical/public data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
pub struct PortableTransfer {
    pub history: w::PortableConfigurationHistory,
    pub target: w::PortableTargetInput,
    pub required_secrets: Vec<String>,
}
