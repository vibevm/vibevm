//! Versioned narrow integer/profile vertical; source gate and normalizer stay shared.
use super::*;
use specmark::spec;
use vibe_wire::generated::{
    ordinary_integer_preview_request as request, ordinary_integer_preview_result as result,
};

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#ORIGINAL-INTENT")]
pub fn original(input: &request::Input) -> ConfigurationIntentDecl {
    ConfigurationIntentDecl {
        policy: ChoicePolicy::RequireSelection,
        profile: input.profile.clone(),
        features: Vec::new(),
        default_features: false,
        selections: input
            .selections
            .iter()
            .map(|(id, member)| {
                (
                    id.clone(),
                    ConfigurationSelection {
                        intent: MergeIntent::Exact,
                        members: Some(vec![member.clone()]),
                    },
                )
            })
            .collect(),
        parameters: input
            .parameters
            .iter()
            .map(|(id, value)| (id.clone(), ParameterValue::Integer(*value)))
            .collect(),
    }
}

/// The integer/profile counterpart of [`super::configuration_preview`]; the
/// host supplies the same `default_registry_isolated` posture.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#DESCRIBE-EVALUATE-PLAN")]
pub fn preview(
    root: &std::path::Path,
    req: &request::OrdinaryIntegerPreviewRequest,
    default_registry_isolated: bool,
) -> anyhow::Result<result::OrdinaryIntegerPreviewResult> {
    anyhow::ensure!(
        req.protocol == "vibe-ordinary-integer-preview/2",
        "integer preview protocol mismatch"
    );
    let (mut out, witness) = super::observe(
        root,
        &req.package,
        serde_json::to_vec(req)?.len(),
        default_registry_isolated,
        |manifest, source| {
            let fail = |why: &str| unsupported(&req.package, why);
            let metadata = manifest
                .require_package()
                .map_err(|_| fail("integer preview package missing"))?;
            let d = manifest
                .configuration
                .as_ref()
                .ok_or_else(|| fail("integer preview definition missing"))?;
            if metadata.kind != vibe_core::PackageKind::Flow
                || manifest.template.is_some()
                || !vibe_core::manifest::integer_profile_shape(d)
            {
                return Err(fail(
                    "integer preview requires one bounded public integer, one parameter preset and one explicit public choice",
                ));
            }
            let p = &d.parameters[0];
            let profile = &d.profiles[0];
            let ConfigurationGroupDecl::Choice {
                id, name, members, ..
            } = &d.groups[0]
            else {
                unreachable!()
            };
            let Some(ParameterValue::Integer(preset)) = profile.parameters.get(&p.id) else {
                return Err(fail("integer preset must be exact signed integer"));
            };
            let effective = req
                .input
                .as_ref()
                .map(|input| {
                    if input.profile.as_deref() != Some(profile.id.as_str()) {
                        return Err(fail(
                            "integer preview v2 requires the declared selected preset",
                        ));
                    }
                    let original = original(input);
                    let record = PackageConfiguration::evaluate(
                        manifest,
                        &BTreeMap::from([("preview".into(), original)]),
                    )?;
                    record
                        .selected_parameters(metadata.kind)?
                        .into_iter()
                        .map(|(id, value)| match value {
                            ParameterValue::Integer(value) => Ok((id, value)),
                            _ => Err(fail("noninteger effective value")),
                        })
                        .collect::<Result<BTreeMap<_, _>, _>>()
                })
                .transpose()?;
            Ok(result::OrdinaryIntegerPreviewResult {
                protocol: "vibe-ordinary-integer-preview/2".into(),
                package: req.package.clone(),
                version: metadata.version.to_string(),
                source: result::Source {
                    registry: source.registry,
                    content_hash: source.content_hash,
                },
                project_witness: String::new(),
                definition: result::Description {
                    parameter: p.id.clone(),
                    name: p.name.clone(),
                    min: p
                        .min
                        .ok_or_else(|| fail("integer preview parameter has no lower bound"))?,
                    max: p
                        .max
                        .ok_or_else(|| fail("integer preview parameter has no upper bound"))?,
                    profile: profile.id.clone(),
                    profile_name: profile.name.clone(),
                    preset: *preset,
                    group: id.clone(),
                    group_name: name.clone(),
                    members: members.clone(),
                },
                original: req.input.as_ref().map(|i| result::Input {
                    profile: i.profile.clone(),
                    selections: i.selections.clone(),
                    parameters: i.parameters.clone(),
                }),
                effective,
            })
        },
    )?;
    out.project_witness = witness;
    Ok(out)
}
