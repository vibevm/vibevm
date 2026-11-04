//! Before/after presentation of changed typed definition rows, never supplied answers.
use serde::Serialize;
use std::collections::BTreeMap;
use vibe_wire::generated::shared as w;

pub(super) fn summary(
    previous: &w::ConfigurationRecord,
    current: &w::ConfigurationRecord,
) -> Result<String, serde_json::Error> {
    let old = &previous.definition;
    let new = &current.definition;
    let mut lines = Vec::new();
    let before = &previous.evaluation;
    let after = &current.evaluation;
    if before.input.intent.profile != after.input.intent.profile {
        lines.push(format!(
            "Selected profile: {} → {}",
            before.input.intent.profile.as_deref().unwrap_or("custom"),
            after.input.intent.profile.as_deref().unwrap_or("custom")
        ));
        if let Some(profile) = after
            .input
            .intent
            .profile
            .as_ref()
            .and_then(|id| new.profiles.iter().find(|p| &p.id == id))
        {
            lines.push(format!(
                "New profile presets:\n{}",
                serde_json::to_string_pretty(profile)?
            ));
        }
    }
    if before.feature_result != after.feature_result {
        lines.push(format!(
            "Feature result before:\n{}\nFeature result proposed:\n{}",
            serde_json::to_string_pretty(&before.feature_result)?,
            serde_json::to_string_pretty(&after.feature_result)?
        ));
    }
    let parameter_id = |p: &w::ConfigurationParameterResult| match p {
        w::ConfigurationParameterResult::Public(p) => p.id.clone(),
        w::ConfigurationParameterResult::Secret(p) => p.id.clone(),
    };
    let before_parameters: BTreeMap<_, _> = before
        .parameters
        .iter()
        .map(|p| (parameter_id(p), p))
        .collect();
    let after_parameters: BTreeMap<_, _> = after
        .parameters
        .iter()
        .map(|p| (parameter_id(p), p))
        .collect();
    rows(
        "Parameter binding",
        before_parameters.iter().map(|(id, p)| (id.as_str(), *p)),
        after_parameters.iter().map(|(id, p)| (id.as_str(), *p)),
        &mut lines,
    )?;
    rows(
        "Feature",
        old.features.iter().map(|v| (v.id.as_str(), v)),
        new.features.iter().map(|v| (v.id.as_str(), v)),
        &mut lines,
    )?;
    let id = |v: &w::ConfigurationGroup| match v {
        w::ConfigurationGroup::Choice(v) => v.id.clone(),
        w::ConfigurationGroup::Closure(v) => v.id.clone(),
    };
    let old_groups: BTreeMap<_, _> = old.groups.iter().map(|v| (id(v), v)).collect();
    let new_groups: BTreeMap<_, _> = new.groups.iter().map(|v| (id(v), v)).collect();
    rows(
        "Group",
        old_groups.iter().map(|(id, v)| (id.as_str(), *v)),
        new_groups.iter().map(|(id, v)| (id.as_str(), *v)),
        &mut lines,
    )?;
    rows(
        "Parameter",
        old.parameters.iter().map(|v| (v.id.as_str(), v)),
        new.parameters.iter().map(|v| (v.id.as_str(), v)),
        &mut lines,
    )?;
    rows(
        "Profile",
        old.profiles.iter().map(|v| (v.id.as_str(), v)),
        new.profiles.iter().map(|v| (v.id.as_str(), v)),
        &mut lines,
    )?;
    rows(
        "Configuration value",
        old.config_values.iter().map(|(k, v)| (k.as_str(), v)),
        new.config_values.iter().map(|(k, v)| (k.as_str(), v)),
        &mut lines,
    )?;
    if lines.is_empty() {
        lines.push(
            "This request would begin relying on automatic choices that were not used previously."
                .into(),
        );
    }
    Ok(lines.join("\n\n"))
}
fn rows<'a, T: Serialize + PartialEq + 'a>(
    kind: &str,
    old: impl Iterator<Item = (&'a str, &'a T)>,
    new: impl Iterator<Item = (&'a str, &'a T)>,
    lines: &mut Vec<String>,
) -> Result<(), serde_json::Error> {
    let old: BTreeMap<_, _> = old.collect();
    let new: BTreeMap<_, _> = new.collect();
    for id in old
        .keys()
        .chain(new.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
    {
        match (old.get(id), new.get(id)) {
            (Some(before), Some(after)) if before != after => lines.push(format!(
                "{kind} `{id}` changed.\nBefore:\n{}\nAfter:\n{}",
                serde_json::to_string_pretty(before)?,
                serde_json::to_string_pretty(after)?
            )),
            (None, Some(after)) => lines.push(format!(
                "{kind} `{id}` added:\n{}",
                serde_json::to_string_pretty(after)?
            )),
            (Some(before), None) => lines.push(format!(
                "{kind} `{id}` removed:\n{}",
                serde_json::to_string_pretty(before)?
            )),
            _ => {}
        }
    }
    Ok(())
}
