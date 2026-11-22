//! Source-bound ordinary read-only configuration preview.
//!
//! Description and evaluation are not a solve, installation plan or permission
//! to execute effects. Source admission is kept separate from this pure kernel.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#DESCRIBE-EVALUATE-PLAN");

use specmark::spec;
use std::collections::BTreeMap;
use vibe_core::configuration::{Cardinality, ChoicePolicy, MergeIntent};
use vibe_core::manifest::{
    ConfigurationFeatureVisibility, ConfigurationGroupDecl, ConfigurationIntentDecl,
    ConfigurationSelection, Manifest, PackageConfiguration, PackageConfigurationError,
    ParameterType, ParameterValue,
};
use vibe_wire::generated::ordinary_configuration_preview_request::OrdinaryConfigurationPreviewRequest;
use vibe_wire::generated::ordinary_configuration_preview_result as wire;

/// Bound both adapters before parsing or returning an answer document.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
pub mod integer;

fn unsupported(package: &str, reason: &str) -> PackageConfigurationError {
    PackageConfigurationError::Unsupported {
        package: package.into(),
        reason: reason.into(),
    }
}

/// Evaluate only the first admitted definition shape, through the existing
/// ordinary normalizer/evaluator. Original answers remain separate from output.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#EVALUATION")]
pub(crate) fn evaluate(
    manifest: &Manifest,
    request: &OrdinaryConfigurationPreviewRequest,
    source: wire::Source,
) -> Result<wire::OrdinaryConfigurationPreviewResult, PackageConfigurationError> {
    let package = &request.package;
    let metadata = manifest
        .require_package()
        .map_err(|_| unsupported(package, "preview requires an ordinary package definition"))?;
    if metadata.kind != vibe_core::PackageKind::Flow || manifest.template.is_some() {
        return Err(unsupported(
            package,
            "preview supports ordinary non-template flows only",
        ));
    }
    let definition = manifest.configuration.as_ref().ok_or_else(|| {
        unsupported(
            package,
            "the selected source has no common configuration definition",
        )
    })?;
    if definition.groups.len() != 1
        || definition.parameters.len() != 1
        || !definition.profiles.is_empty()
    {
        return Err(unsupported(
            package,
            "preview requires one visible exactly-one choice and one required public bool without defaults or profiles",
        ));
    }
    let ConfigurationGroupDecl::Choice {
        id,
        name,
        cardinality: Cardinality::ExactlyOne,
        members,
        defaults: None,
        guards,
        ..
    } = &definition.groups[0]
    else {
        return Err(unsupported(
            package,
            "preview requires an unguarded exactly-one choice without defaults",
        ));
    };
    if !guards.is_empty()
        || definition.features.len() != members.len()
        || definition.features.iter().any(|feature| {
            feature.visibility != ConfigurationFeatureVisibility::Public
                || feature.parent.is_some()
                || feature.required
                || feature.default
                || !feature.requires.is_empty()
                || !feature.guards.is_empty()
                || !members.contains(&feature.id)
        })
    {
        return Err(unsupported(
            package,
            "preview does not support hidden, guarded, implicit or additional feature selections",
        ));
    }
    let parameter = &definition.parameters[0];
    if parameter.kind != ParameterType::Bool
        || parameter.secret
        || !parameter.required
        || parameter.default.is_some()
    {
        return Err(unsupported(
            package,
            "preview requires a required public bool without a default",
        ));
    }
    if !manifest.requires_any.is_empty()
        || !manifest.conditional_deps.is_empty()
        || !manifest.requires.capabilities.is_empty()
        || !vibe_core::manifest::application_requirements(&manifest.installer_options).is_empty()
        || definition
            .features
            .iter()
            .flat_map(|feature| &feature.effects)
            .any(|effect| {
                matches!(
                    effect,
                    vibe_core::manifest::ConfigurationEffect::Forward { .. }
                        | vibe_core::manifest::ConfigurationEffect::Subskill { .. }
                )
            })
    {
        return Err(unsupported(
            package,
            "preview does not support forwarding, subskill or conditional source effects",
        ));
    }
    definition
        .compile_choices()
        .map_err(|error| PackageConfigurationError::Invalid {
            package: package.clone(),
            reason: error.to_string(),
        })?;
    let normalized_input = request.input.as_ref().map(|input| ConfigurationIntentDecl {
        policy: ChoicePolicy::RequireSelection,
        profile: None,
        features: Vec::new(),
        default_features: false,
        selections: input
            .selections
            .iter()
            .map(|(group, member)| {
                (
                    group.clone(),
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
            .map(|(id, value)| (id.clone(), ParameterValue::Bool(*value)))
            .collect(),
    });
    let resolved = normalized_input
        .as_ref()
        .map(|input| {
            let record = PackageConfiguration::evaluate(
                manifest,
                &BTreeMap::from([("preview".into(), input.clone())]),
            )?;
            let parameters = record
                .selected_parameters(metadata.kind)?
                .into_iter()
                .map(|(id, value)| {
                    let ParameterValue::Bool(value) = value else {
                        unreachable!("admitted bool parameter")
                    };
                    (id, value)
                })
                .collect();
            Ok::<_, PackageConfigurationError>(wire::Resolved {
                features: record.features,
                parameters,
            })
        })
        .transpose()?;
    Ok(wire::OrdinaryConfigurationPreviewResult {
        protocol: "vibe-ordinary-configuration-preview/1".into(),
        project_witness: None,
        package: package.clone(),
        version: metadata.version.to_string(),
        source,
        description: wire::Description {
            choices: vec![wire::Choice {
                id: id.clone(),
                name: name.clone(),
                members: members
                    .iter()
                    .map(|member| {
                        let feature = definition
                            .features
                            .iter()
                            .find(|feature| &feature.id == member)
                            .expect("compiled choice member");
                        wire::Member {
                            id: member.clone(),
                            name: feature.name.clone(),
                        }
                    })
                    .collect(),
            }],
            parameters: vec![wire::Parameter {
                id: parameter.id.clone(),
                name: parameter.name.clone(),
                required: true,
                type_: wire::ParameterType::Bool,
            }],
        },
        questions: if resolved.is_some() {
            Vec::new()
        } else {
            vec![
                wire::Question {
                    id: id.clone(),
                    kind: wire::QuestionKind::Choice,
                    members: members.clone(),
                },
                wire::Question {
                    id: parameter.id.clone(),
                    kind: wire::QuestionKind::Parameter,
                    members: Vec::new(),
                },
            ]
        },
        original: request.input.as_ref().map(|input| wire::Input {
            selections: input.selections.clone(),
            parameters: input.parameters.clone(),
        }),
        resolved,
    })
}

/// Describe/evaluate one compulsory exact ordinary root, without acquisition.
/// The trusted root is supplied by the host, never by the MCP request, and so
/// is `default_registry_isolated`: whether the host's composition root observed
/// the isolated no-default-registry posture (`VIBE_NO_DEFAULT_REGISTRY` exactly
/// `1`). This crate reads no environment; without that posture the preview
/// refuses.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#DESCRIBE-EVALUATE-PLAN")]
pub fn configuration_preview(
    trusted_root: &std::path::Path,
    request: &OrdinaryConfigurationPreviewRequest,
    default_registry_isolated: bool,
) -> anyhow::Result<wire::OrdinaryConfigurationPreviewResult> {
    let (mut result, witness) = observe(
        trusted_root,
        &request.package,
        serde_json::to_vec(request)?.len(),
        default_registry_isolated,
        |manifest, source| evaluate(manifest, request, source),
    )?;
    result.project_witness = Some(witness);
    Ok(result)
}

pub(super) fn observe<T>(
    trusted_root: &std::path::Path,
    package: &str,
    request_bytes: usize,
    default_registry_isolated: bool,
    evaluate: impl FnOnce(&Manifest, wire::Source) -> Result<T, PackageConfigurationError>,
) -> anyhow::Result<(T, String)> {
    use anyhow::{anyhow, bail};
    use std::path::Component;
    use vibe_core::manifest::RegistryType;
    let refuse = |reason: &str| anyhow::Error::new(unsupported(package, reason));
    if request_bytes > MAX_INPUT_BYTES {
        return Err(refuse("preview answers exceed the bounded input limit"));
    }
    // The host decides this posture at its composition root from the same
    // suppression condition its embedded-registry startup uses, not a second
    // best-effort discovery routine. A host without it cannot prove ambient
    // source equivalence in this slice.
    if !default_registry_isolated {
        return Err(refuse(
            "embedded source influence is not excluded; preview requires the isolated no-default-registry posture",
        ));
    }
    let locator = vibe_core::remap::active();
    if locator.policy() != vibe_core::remap::UnmatchedPolicy::Allow
        || locator
            .rows()
            .iter()
            .any(|row| row.only != Some(vibe_core::remap::ArtifactKind::RawFile))
        || locator
            .sources()
            .iter()
            .any(|(layer, _, _)| *layer == vibe_core::remap::Layer::Invocation)
    {
        return Err(refuse(
            "active remap source influence is outside this preview slice",
        ));
    }
    let project = vibe_safefs::Project::open(trusted_root)?;
    let snapshot = project
        .read_file_snapshot_bounded(Manifest::FILENAME, MAX_INPUT_BYTES)?
        .ok_or_else(|| refuse("selected project manifest is unavailable"))?;
    let manifest = Manifest::parse_str(std::str::from_utf8(&snapshot.bytes)?)?;
    if manifest.workspace.is_some()
        || !manifest.source_policy.is_empty()
        || !manifest.mirrors.is_empty()
        || !manifest.overrides.is_empty()
        || !manifest.requires.git_packages.is_empty()
        || !manifest.requires.path_packages.is_empty()
        || !manifest.requires_any.is_empty()
        || !manifest.conditional_deps.is_empty()
        || !manifest.requires.remaps.is_empty()
    {
        return Err(refuse(
            "preview source equivalence is not established for workspace or competing source policies",
        ));
    }
    // Do not read ambient configuration/authentication. Its presence is enough
    // to make source equivalence unsupported in this first slice.
    for path in [
        vibe_core::settings::registry_config_path(),
        vibe_core::settings::site_profile_path(),
        vibe_core::settings::settings_toml_path(),
        vibe_core::settings::settings_dir().map(|root| root.join("cache")),
    ] {
        let path = path.ok_or_else(|| refuse("ambient source location is unknown"))?;
        vibe_safefs::Project::pin_absent_path(&path).map_err(|_| {
            refuse("ambient source layer is present or cannot be proven absent no-follow")
        })?;
    }
    vibe_safefs::Project::pin_absent_path(&trusted_root.join(".vibe/settings.local.toml"))
        .map_err(|_| refuse("project remap settings are present or cannot be proven absent"))?;
    if project
        .read_file_snapshot_bounded(".vibe/registry.local.toml", MAX_INPUT_BYTES)?
        .is_some()
        || project
            .read_file_snapshot_bounded("vibe.lock", MAX_INPUT_BYTES)?
            .is_some()
    {
        return Err(refuse(
            "preview requires a cold project without local registry overrides or a retained lock",
        ));
    }
    let reference = manifest
        .requires
        .packages
        .iter()
        .find(|reference| {
            reference
                .group
                .as_ref()
                .is_some_and(|group| format!("{group}/{}", reference.name) == package)
        })
        .ok_or_else(|| refuse("package is not an exact selected project requirement"))?;
    if manifest.requires.optional.get(package) == Some(&true)
        || manifest.requires.configurations.contains_key(package)
    {
        return Err(refuse(
            "preview requires a compulsory unanswered project root",
        ));
    }
    let registries = manifest
        .registries
        .iter()
        .filter(|registry| registry.enabled)
        .collect::<Vec<_>>();
    if registries.len() != 1 {
        return Err(refuse(
            "preview requires one explicit admitted local project registry",
        ));
    }
    let registry = registries[0];
    if registry.checked_type()? != RegistryType::Localdir
        || registry.token_env.is_some()
        || registry
            .auth
            .is_some_and(|auth| auth != vibe_core::manifest::AuthKind::None)
    {
        return Err(refuse(
            "preview supports unauthenticated local directory registry only",
        ));
    }
    if let Some(pin) = manifest.requires.registry_pins.get(package)
        && pin.as_str() != registry.name
    {
        return Err(refuse("project requirement names a different registry"));
    }
    let root = vibe_registry::localdir_path(&registry.url)?;
    let relative = root
        .strip_prefix(trusted_root)
        .map_err(|_| refuse("external local registry is outside the trusted project boundary"))?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(name) => name
                .to_str()
                .ok_or_else(|| refuse("non-UTF8 registry path")),
            _ => Err(refuse("noncanonical registry path")),
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    if components.is_empty() {
        return Err(refuse(
            "registry must be nested beneath the trusted project root",
        ));
    }
    if let Some(local) = project.dir_if_present(&["vibevm", "vibepacks"])? {
        let selected = project
            .dir_if_present(&components)?
            .ok_or_else(|| refuse("local registry is unavailable"))?;
        if local.identity()? != selected.identity()? {
            return Err(refuse(
                "competing project-local registry is outside this preview slice",
            ));
        }
    }
    let observation =
        vibe_registry::local_preview::observe_local_preview(&project, &components, reference)
            .map_err(|error| {
                refuse(&format!("local source unavailable or unsupported: {error}"))
            })?;
    let result = evaluate(
        &observation.manifest,
        wire::Source {
            registry: registry.name.clone(),
            content_hash: observation.content_hash,
        },
    )?;
    let after = project
        .read_file_snapshot_bounded(Manifest::FILENAME, MAX_INPUT_BYTES)?
        .ok_or_else(|| anyhow!("selected project manifest disappeared"))?;
    if after != snapshot {
        bail!("selected project changed during configuration preview");
    }
    let witness = vibe_core::manifest::submitted_project_witness(trusted_root, &snapshot.bytes)
        .map_err(anyhow::Error::msg)?;
    Ok((result, witness))
}
