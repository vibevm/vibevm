//! Ordinary-only answer intake; source-bound preview is not apply permission.
use crate::{cli::InstallArgs, output};
use anyhow::{Result, anyhow, bail, ensure};
use specmark::spec;
use vibe_wire::generated::{
    ordinary_configuration_preview_request::{Input, OrdinaryConfigurationPreviewRequest},
    ordinary_install_answers::OrdinaryInstallAnswers,
};

#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub(crate) fn prepare(ctx: &output::Context, args: &mut InstallArgs) -> Result<()> {
    if args.ordinary_answers.is_none() && !args.ordinary_interactive {
        return Ok(());
    }
    ensure!(
        !args.global && args.packages.is_empty() && !args.plan_only && args.registry.is_none(),
        "ordinary answers require the existing standalone root and declared registry"
    );
    ensure!(
        args.features.is_empty() && !args.no_default_features && !args.all_features,
        "ordinary answers cannot mix legacy feature flags"
    );
    if args.ordinary_interactive
        && (ctx.is_unattended() || ctx.is_json() || !console::user_attended())
    {
        bail!("ordinary interactive answers require an attended terminal");
    }
    if !args.ordinary_interactive
        && !args.assume_yes
        && (ctx.is_unattended() || ctx.is_json() || !console::user_attended())
    {
        bail!(
            "ordinary submitted install requires explicit --assume-yes or attended apply confirmation"
        );
    }
    let root = super::resolve_project_root(&args.path)?;
    let isolated = output::default_registry_isolated();
    let project = vibe_safefs::Project::open(&root)?;
    let bytes = project
        .read_file_snapshot_bounded("vibe.toml", 64 * 1024)?
        .ok_or_else(|| anyhow!("ordinary project manifest unavailable"))?;
    let manifest = vibe_core::manifest::Manifest::parse_str(std::str::from_utf8(&bytes.bytes)?)?;
    ensure!(
        manifest.requires.packages.len() == 1,
        "ordinary answers require one actual root declaration"
    );
    let requirement = manifest.requires.packages[0].clone();
    let package = requirement.qualified_name();
    if let Some(path) = &args.ordinary_answers {
        let absolute = std::path::absolute(path)?;
        let parent = absolute
            .parent()
            .ok_or_else(|| anyhow!("ordinary answers parent absent"))?;
        let name = absolute
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("ordinary answers name invalid"))?;
        let scope = vibe_safefs::Project::open(parent)?;
        let content = scope
            .read_file_snapshot_bounded(name, 64 * 1024)?
            .ok_or_else(|| anyhow!("ordinary answers missing"))?;
        let value: serde_json::Value = serde_json::from_slice(&content.bytes)
            .map_err(|_| anyhow!("ordinary install answers: invalid shape; expected protocol/package/project_witness strings, source registry/content_hash strings, input.selections string map and input.parameters boolean map"))?;
        if value.get("protocol").and_then(|p| p.as_str())
            == Some("vibe-ordinary-integer-install-answers/2")
        {
            let envelope: vibe_wire::generated::ordinary_integer_install_answers::OrdinaryIntegerInstallAnswers = serde_json::from_slice(&content.bytes)
                .map_err(|_|anyhow!("integer install answers v2 invalid shape; values omitted"))?;
            let input = vibe_wire::generated::ordinary_integer_preview_request::Input {
                profile: envelope.input.profile.clone(),
                selections: envelope.input.selections.clone(),
                parameters: envelope.input.parameters.clone(),
            };
            let result = vibe_package_source::configuration_preview::integer::preview(&root,
                &vibe_wire::generated::ordinary_integer_preview_request::OrdinaryIntegerPreviewRequest {
                    protocol:"vibe-ordinary-integer-preview/2".into(),package:package.clone(),input:Some(input.clone()),
                }, isolated)?;
            ensure!(
                envelope.package == package
                    && envelope.project_witness == result.project_witness
                    && envelope.source.registry == result.source.registry
                    && envelope.source.content_hash == result.source.content_hash,
                "integer submitted project/source changed"
            );
            args.parsed_ordinary_submission = Some(vibe_core::manifest::SubmittedInput {
                original: vibe_package_source::configuration_preview::integer::original(&input),
                requirement,
                project_witness: envelope.project_witness,
                expected_source: vibe_core::manifest::SubmittedSource {
                    registry: envelope.source.registry,
                    content_hash: vibe_core::ContentHash::parse(&envelope.source.content_hash)?,
                },
            });
            return Ok(());
        }
    }
    let request = OrdinaryConfigurationPreviewRequest {
        package: package.clone(),
        input: None,
    };
    let description = vibe_package_source::configuration_preview::configuration_preview(
        &root, &request, isolated,
    )?;
    let envelope = if args.ordinary_interactive {
        let input = crate::commands::show::configuration_preview::collect_answers(&description)?;
        OrdinaryInstallAnswers {
            protocol: "vibe-ordinary-install-answers/1".into(),
            package: package.clone(),
            project_witness: description
                .project_witness
                .clone()
                .ok_or_else(|| anyhow!("preview project witness absent"))?,
            source: vibe_wire::generated::ordinary_install_answers::Source {
                registry: description.source.registry.clone(),
                content_hash: description.source.content_hash.clone(),
            },
            input: vibe_wire::generated::ordinary_install_answers::Input {
                selections: input.selections,
                parameters: input.parameters,
            },
        }
    } else {
        let answers = args
            .ordinary_answers
            .as_ref()
            .ok_or_else(|| anyhow!("ordinary install answers: no answer file was given"))?;
        let path = std::path::absolute(answers)?;
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("answer file has no parent"))?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("answer filename invalid"))?;
        let scope = vibe_safefs::Project::open(parent)?;
        let content = scope
            .read_file_snapshot_bounded(name, 64 * 1024)?
            .ok_or_else(|| anyhow!("answer file unavailable"))?;
        vibe_wire::behaviour::ordinary_install_answers::parse(&content.bytes)?
    };
    ensure!(
        envelope.protocol == "vibe-ordinary-install-answers/1"
            && envelope.package == package
            && Some(&envelope.project_witness) == description.project_witness.as_ref()
            && envelope.source.registry == description.source.registry
            && envelope.source.content_hash == description.source.content_hash,
        "ordinary answer subject/project/source changed"
    );
    let result = vibe_package_source::configuration_preview::configuration_preview(
        &root,
        &OrdinaryConfigurationPreviewRequest {
            package: package.clone(),
            input: Some(Input {
                selections: envelope.input.selections.clone(),
                parameters: envelope.input.parameters.clone(),
            }),
        },
        isolated,
    )?;
    ensure!(
        result.source == description.source
            && result.version == description.version
            && result.project_witness == description.project_witness,
        "ordinary source or project changed during answer collection"
    );
    let original = vibe_core::manifest::ConfigurationIntentDecl {
        policy: vibe_core::configuration::ChoicePolicy::RequireSelection,
        profile: None,
        features: Vec::new(),
        default_features: false,
        selections: envelope
            .input
            .selections
            .into_iter()
            .map(|(id, member)| {
                (
                    id,
                    vibe_core::manifest::ConfigurationSelection {
                        intent: vibe_core::configuration::MergeIntent::Exact,
                        members: Some(vec![member]),
                    },
                )
            })
            .collect(),
        parameters: envelope
            .input
            .parameters
            .into_iter()
            .map(|(id, value)| (id, vibe_core::manifest::ParameterValue::Bool(value)))
            .collect(),
    };
    args.parsed_ordinary_submission = Some(vibe_core::manifest::SubmittedInput {
        original,
        requirement,
        project_witness: envelope.project_witness,
        expected_source: vibe_core::manifest::SubmittedSource {
            registry: envelope.source.registry,
            content_hash: vibe_core::ContentHash::parse(&envelope.source.content_hash)?,
        },
    });
    Ok(())
}
