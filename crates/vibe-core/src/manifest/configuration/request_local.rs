//! One pure narrow public data-only request, without files, consent or history.
use super::{
    ConfigurationEffect, ConfigurationFeatureVisibility, ConfigurationGroupDecl,
    ConfigurationIntentDecl, PackageConfiguration,
};
use crate::manifest::{ConfigurationSelection, Manifest, ParameterValue};
use specmark::spec;
use std::collections::BTreeMap;
/// Evaluate one exact public unguarded exactly-one data-only choice.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-DEFINITIONS")]
pub fn evaluate_request_local_data(
    manifest: &Manifest,
    selections: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, ParameterValue>, &'static str> {
    let invalid = || "request-local data: unsupported definition or invalid exact choice; expected one public unguarded exactly-one group with data-only effects";
    let definition = manifest.configuration.as_ref().ok_or_else(invalid)?;
    let [
        ConfigurationGroupDecl::Choice {
            id,
            members,
            guards,
            cardinality,
            defaults,
            ..
        },
    ] = definition.groups.as_slice()
    else {
        return Err(invalid());
    };
    if *cardinality != crate::configuration::Cardinality::ExactlyOne
        || !guards.is_empty()
        || defaults.is_some()
        || !definition.parameters.is_empty()
        || !definition.profiles.is_empty()
        || selections.len() != 1
    {
        return Err(invalid());
    }
    let answer = selections.get(id).ok_or_else(invalid)?;
    if answer.len() != 1
        || !members.contains(&answer[0])
        || definition.features.len() != members.len()
        || definition.features.iter().any(|f| {
            f.visibility != ConfigurationFeatureVisibility::Public
                || !members.contains(&f.id)
                || f.parent.is_some()
                || f.required
                || f.default
                || !f.requires.is_empty()
                || !f.guards.is_empty()
                || f.effects
                    .iter()
                    .any(|e| !matches!(e, ConfigurationEffect::ConfigValue { .. }))
        })
    {
        return Err(invalid());
    }
    let original = ConfigurationIntentDecl {
        policy: crate::configuration::ChoicePolicy::RequireSelection,
        profile: None,
        features: vec![],
        default_features: false,
        selections: BTreeMap::from([(
            id.clone(),
            ConfigurationSelection {
                intent: crate::configuration::MergeIntent::Exact,
                members: Some(answer.clone()),
            },
        )]),
        parameters: BTreeMap::new(),
    };
    // An operation-local consumer is not authored or persisted project history.
    let record = PackageConfiguration::evaluate(
        manifest,
        &BTreeMap::from([("request-local data".into(), original)]),
    )
    .map_err(|_| invalid())?;
    record
        .selected_data(manifest.require_package().map_err(|_| invalid())?.kind)
        .map_err(|_| invalid())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn doc_and_remap_data_requests_share_narrow_exact_semantics() {
        for kind in ["doc", "remap"] {
            let extra = if kind == "doc" {
                "title='Manual'\nabstract='A synthetic manual.'\n[[documents]]\npackage='org.example/subject'\nversion='*'\n"
            } else {
                "[[remap]]\nprefix='https://example.invalid/'\ndeny=true\n"
            };
            let text = format!(
                "[package]\ngroup='org.example'\nname='data'\nversion='1.0.0'\nkind='{kind}'\n{extra}\n[configuration.config_values]\nselected=true\n[[configuration.features]]\nid='off'\nname='Off'\n[[configuration.features]]\nid='on'\nname='On'\neffects=[{{kind='config-value',name='selected'}}]\n[[configuration.groups]]\nmode='choice'\nid='mode'\nname='Mode'\ncardinality='exactly-one'\nmembers=['off','on']\n"
            );
            let manifest = Manifest::parse_str(&text).unwrap();
            for (member, expected) in [("off", false), ("on", true)] {
                let input = BTreeMap::from([("mode".into(), vec![member.into()])]);
                let values = evaluate_request_local_data(&manifest, &input).unwrap();
                assert_eq!(values.contains_key("selected"), expected);
            }
            for input in [
                BTreeMap::new(),
                BTreeMap::from([("mode".into(), vec![])]),
                BTreeMap::from([("mode".into(), vec!["off".into(), "on".into()])]),
                BTreeMap::from([("unknown".into(), vec!["on".into()])]),
            ] {
                assert!(evaluate_request_local_data(&manifest, &input).is_err());
            }
        }
    }
}
