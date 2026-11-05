//! Typed portable history admission, independent of a source-host ready record.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
mod definition;
mod input;
use super::{ConfigurationHistory, HistoricalParameter};
use vibe_wire::generated::shared as w;

/// Owned normalized representation of historical meaning, not an executable result.
#[derive(Debug, Clone)]
pub(crate) struct TransportedHistory {
    pub definition: w::ConfigurationDefinition,
    pub fingerprint: String,
    pub input: w::ConfigurationInput,
    pub reliance: Vec<String>,
    /// Source realization remains diagnostic; view never exposes it for local reuse.
    pub parameters: Vec<w::PortableParameterHistory>,
}
impl TransportedHistory {
    pub(crate) fn view(&self) -> ConfigurationHistory<'_> {
        ConfigurationHistory {
            definition: &self.definition,
            fingerprint: &self.fingerprint,
            input: &self.input,
            reliance: &self.reliance,
            parameters: self
                .parameters
                .iter()
                .map(|p| match p {
                    w::PortableParameterHistory::Public(p) => HistoricalParameter::Public {
                        id: &p.id,
                        kind: &p.type_,
                        origin: &p.origin,
                        source: p.source.as_ref(),
                        reusable_local_realization: None,
                    },
                    w::PortableParameterHistory::Secret(p) => HistoricalParameter::Secret {
                        id: &p.id,
                        origin: &p.origin,
                        provided: p.provided,
                    },
                })
                .collect(),
        }
    }
}
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub(crate) fn encode(record: &w::ConfigurationRecord) -> w::PortableConfigurationHistory {
    w::PortableConfigurationHistory {
        definition: definition::to_portable(&record.definition),
        fingerprint: record.evaluation.definition.clone(),
        input: input::to_portable(&record.evaluation.input),
        reliance: record.evaluation.reliance.clone(),
        parameters: record
            .evaluation
            .parameters
            .iter()
            .map(|p| match p {
                w::ConfigurationParameterResult::Public(p) => w::PortableParameterHistory::Public(
                    Box::new(w::PortableParameterHistoryPublic {
                        id: p.id.clone(),
                        type_: p.type_.clone(),
                        origin: p.origin.clone(),
                        source: p.source.clone(),
                        realization: p.value.clone(),
                    }),
                ),
                w::ConfigurationParameterResult::Secret(p) => w::PortableParameterHistory::Secret(
                    Box::new(w::PortableParameterHistorySecret {
                        id: p.id.clone(),
                        origin: p.origin.clone(),
                        provided: p.provided,
                    }),
                ),
            })
            .collect(),
    }
}
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub(crate) fn decode(
    history: &w::PortableConfigurationHistory,
) -> Result<TransportedHistory, String> {
    let result = TransportedHistory {
        definition: definition::from_portable(&history.definition),
        fingerprint: history.fingerprint.clone(),
        input: input::from_portable(&history.input),
        reliance: history.reliance.clone(),
        parameters: history.parameters.clone(),
    };
    let d = super::definition::read_definition(&result.definition)?;
    if d.semantic_fingerprint()
        .map_err(|e| format!("{}: {e}", e.field))?
        != result.fingerprint
    {
        return Err("portable history differs from its typed definition fingerprint".into());
    }
    let original = super::vocabulary::read_input(&result.input.intent)?;
    d.normalize_input(&original, "installer")
        .map_err(|e| format!("{}: {e}", e.field))?;
    for id in &result.input.disabled_features {
        if !d.features.iter().any(|f| {
            &f.id == id
                && f.visibility == vibe_core::manifest::ConfigurationFeatureVisibility::Public
        }) {
            return Err(format!(
                "portable disabled feature `{id}` is not a public declared feature"
            ));
        }
    }
    for reason in &result.reliance {
        let known = matches!(reason.as_str(), "profile" | "feature-defaults")
            || ["choice:", "closure:", "parameter:"].iter().any(|prefix| {
                reason
                    .strip_prefix(prefix)
                    .is_some_and(|id| !id.trim().is_empty() && !id.chars().any(char::is_control))
            });
        if !known {
            return Err("portable history has an unknown or empty reliance reason".into());
        }
    }
    let profile = original
        .profile
        .as_ref()
        .and_then(|id| d.profiles.iter().find(|p| &p.id == id));
    let mut seen = std::collections::BTreeSet::new();
    for p in &result.parameters {
        let id = match p {
            w::PortableParameterHistory::Public(p) => &p.id,
            w::PortableParameterHistory::Secret(p) => &p.id,
        };
        if !seen.insert(id) {
            return Err(format!("duplicate portable parameter `{id}`"));
        }
        let declared = d
            .parameters
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| format!("unknown portable parameter `{id}`"))?;
        match p {
            w::PortableParameterHistory::Secret(p) => {
                if !declared.secret
                    || matches!(p.origin, w::ConfigurationParameterOrigin::Profile(_))
                    || p.provided
                        != matches!(p.origin, w::ConfigurationParameterOrigin::Explicit(_))
                {
                    return Err(format!(
                        "portable secret parameter `{id}` has inconsistent provision metadata"
                    ));
                }
            }
            w::PortableParameterHistory::Public(p) => {
                if declared.secret || p.type_ != super::vocabulary::parameter_type(&declared.kind) {
                    return Err(format!(
                        "portable parameter `{id}` has incompatible privacy or type"
                    ));
                }
                let (origin, source) =
                    super::parameters::public_source(declared, &original, profile);
                if p.origin != origin || p.source != source.as_ref().map(super::vocabulary::value) {
                    return Err(format!(
                        "portable parameter `{id}` has an inconsistent historical origin or lexical source"
                    ));
                }
                if declared.kind != vibe_core::manifest::ParameterType::Path
                    && p.realization
                        .as_ref()
                        .is_some_and(|value| Some(value) != p.source.as_ref())
                {
                    return Err(format!(
                        "portable parameter `{id}` has an inconsistent source realization"
                    ));
                }
                for value in [p.source.as_ref(), p.realization.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    let value = super::vocabulary::read_value(value)?;
                    let canonical = declared.canonical(&value.to_string()).map_err(|_| {
                        format!("portable parameter `{id}` violates its historical rules")
                    })?;
                    if super::parameters::typed(declared.kind, &canonical)? != value {
                        return Err(format!(
                            "portable parameter `{id}` has a mismatched typed value"
                        ));
                    }
                }
            }
        }
    }
    if seen.len() != d.parameters.len() {
        return Err("portable history omits declared parameter metadata".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
