//! Structural admission of historical configuration; never rerun host checks on index read.
use super::{definition, vocabulary};
use vibe_core::manifest::ParameterType;
use vibe_wire::generated::shared as w;

pub(crate) fn validate(record: &w::ConfigurationRecord) -> Result<(), String> {
    let d = definition::read_definition(&record.definition)?;
    let expected = d
        .semantic_fingerprint()
        .map_err(|e| format!("{}: {e}", e.field))?;
    if expected != record.evaluation.definition {
        return Err("configuration record definition differs from its semantic basis".into());
    }
    let input = vocabulary::read_input(&record.evaluation.input.intent)?;
    d.normalize_input(&input, "installer")
        .map_err(|e| format!("{}: {e}", e.field))?;
    let result = record
        .evaluation
        .resolved
        .as_ref()
        .ok_or("installed configuration has no resolved result")?;
    if !record.evaluation.questions.is_empty() || !record.evaluation.problems.is_empty() {
        return Err("installed configuration retains unresolved questions or problems".into());
    }
    if record
        .evaluation
        .feature_result
        .as_ref()
        .is_some_and(|features| features != &result.features)
    {
        return Err("ready configuration differs from its feature component result".into());
    }
    if result.parameters != record.evaluation.parameters {
        return Err("configuration parameter result disagrees with its evaluation".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for p in &record.evaluation.parameters {
        let id = match p {
            w::ConfigurationParameterResult::Public(p) => &p.id,
            w::ConfigurationParameterResult::Secret(p) => &p.id,
        };
        if !seen.insert(id) {
            return Err(format!("duplicate recorded parameter `{id}`"));
        }
        let decl = d
            .parameters
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| format!("unknown recorded parameter `{id}`"))?;
        match p {
            w::ConfigurationParameterResult::Secret(p) => {
                if !decl.secret
                    || p.resupply
                    || p.provided
                        != matches!(p.origin, w::ConfigurationParameterOrigin::Explicit(_))
                {
                    return Err(format!(
                        "recorded secret parameter `{id}` has incompatible metadata"
                    ));
                }
            }
            w::ConfigurationParameterResult::Public(p) => {
                if decl.secret || vocabulary::parameter_type(&decl.kind) != p.type_ {
                    return Err(format!(
                        "recorded parameter `{id}` has incompatible privacy or type"
                    ));
                }
                if let Some(value) = &p.value {
                    let value = vocabulary::read_value(value)?;
                    decl.canonical(&value.to_string()).map_err(|_| {
                        format!("recorded parameter `{id}` violates its historical rules")
                    })?;
                    if decl.kind == ParameterType::Path
                        && !std::path::Path::new(&value.to_string()).is_absolute()
                    {
                        return Err(format!("recorded path parameter `{id}` is not absolute"));
                    }
                }
                if let Some(value) = &p.source {
                    vocabulary::read_value(value)?;
                }
            }
        }
    }
    if seen.len() != d.parameters.len() {
        return Err("configuration record omits declared parameter metadata".into());
    }
    Ok(())
}

pub(crate) fn validate_origins(
    origins: &std::collections::BTreeMap<String, w::ConfigurationParameterOrigin>,
) -> Result<(), String> {
    for (id, origin) in origins {
        match origin {
            w::ConfigurationParameterOrigin::Profile(_) => {
                return Err(format!(
                    "secret parameter `{id}` cannot have a public profile origin"
                ));
            }
            w::ConfigurationParameterOrigin::Explicit(_)
            | w::ConfigurationParameterOrigin::Default(_)
            | w::ConfigurationParameterOrigin::Absent(_)
            | w::ConfigurationParameterOrigin::LegacyEffective(_) => {}
        }
    }
    Ok(())
}
