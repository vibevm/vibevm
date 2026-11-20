//! Thin ordinary-preview adapter; the shared operation owns source/evaluation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#SURFACE-PARITY");
use crate::{cli::ShowConfigurationPreviewArgs, output};
use anyhow::{Result, anyhow, bail};
use vibe_package_source::configuration_preview::{MAX_INPUT_BYTES, configuration_preview};
use vibe_wire::generated::ordinary_configuration_preview_request::{
    Input, OrdinaryConfigurationPreviewRequest,
};

pub(super) fn run(ctx: &output::Context, args: ShowConfigurationPreviewArgs) -> Result<()> {
    if args.interactive
        && (args.input.is_some()
            || ctx.is_json()
            || ctx.is_unattended()
            || !console::user_attended())
    {
        bail!(
            "interactive configuration preview requires an attended terminal without JSON or serialized input"
        );
    }
    // Preserve no-follow ancestry rather than canonicalizing user paths away.
    let root = std::path::absolute(&args.path)?;
    if args.interactive {
        return interactive(&root, args.package);
    }
    let input = args
        .input
        .map(|path| -> Result<Input> {
            let absolute = std::path::absolute(path)?;
            let parent = absolute
                .parent()
                .ok_or_else(|| anyhow!("input file has no parent"))?;
            let name = absolute
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("input file name is not UTF-8"))?;
            let scope = vibe_safefs::Project::open(parent)?;
            let snapshot = scope
                .read_file_snapshot_bounded(name, MAX_INPUT_BYTES)?
                .ok_or_else(|| anyhow!("input file is unavailable"))?;
            Ok(serde_json::from_slice(&snapshot.bytes)?)
        })
        .transpose()?;
    let result = configuration_preview(
        &root,
        &OrdinaryConfigurationPreviewRequest {
            package: args.package,
            input,
        },
    )?;
    if ctx.is_json() {
        ctx.emit_json(&result)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

fn interactive(root: &std::path::Path, package: String) -> Result<()> {
    let description = configuration_preview(
        root,
        &OrdinaryConfigurationPreviewRequest {
            package: package.clone(),
            input: None,
        },
    )?;
    let input = collect_answers(&description)?;
    let result = configuration_preview(
        root,
        &OrdinaryConfigurationPreviewRequest {
            package,
            input: Some(input),
        },
    )?;
    if result.source != description.source
        || result.version != description.version
        || result.description != description.description
    {
        bail!(
            "configuration preview source changed after questions were shown; no answers applied"
        );
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub(crate) fn collect_answers(
    description:&vibe_wire::generated::ordinary_configuration_preview_result::OrdinaryConfigurationPreviewResult,
) -> Result<Input> {
    use crate::output::prompts;
    use std::collections::BTreeMap;
    // The service owns admissible shape and question semantics; this adapter
    // only renders its typed rows and retains actual terminal selections.
    let mut selections = BTreeMap::new();
    let mut parameters = BTreeMap::new();
    for question in &description.questions {
        use vibe_wire::generated::ordinary_configuration_preview_result::QuestionKind;
        match question.kind {
            QuestionKind::Choice => {
                let group = description
                    .description
                    .choices
                    .iter()
                    .find(|group| group.id == question.id)
                    .ok_or_else(|| anyhow!("preview question has no described choice"))?;
                let choices = group
                    .members
                    .iter()
                    .map(|member| vibe_app::Choice::named(&member.name, &member.id))
                    .collect::<Vec<_>>();
                let selected = prompts::choose_opt_interruptible(
                    &format!("{} ({})", group.name, group.id),
                    &choices,
                    0,
                )?
                .ok_or_else(|| anyhow!("configuration preview cancelled; no answers applied"))?;
                selections.insert(group.id.clone(), group.members[selected].id.clone());
            }
            QuestionKind::Parameter => {
                let parameter = description
                    .description
                    .parameters
                    .iter()
                    .find(|parameter| parameter.id == question.id)
                    .ok_or_else(|| anyhow!("preview question has no described parameter"))?;
                // No Confirm default: only an actual selected true/false row
                // creates an answer. Escape/EOF is never false or consent.
                let choices = prompts::named(&["true", "false"]);
                let selected = prompts::choose_opt_interruptible(
                    &format!("{} ({})", parameter.name, parameter.id),
                    &choices,
                    0,
                )?
                .ok_or_else(|| anyhow!("configuration preview cancelled; no answers applied"))?;
                parameters.insert(parameter.id.clone(), selected == 0);
            }
        }
    }
    Ok(Input {
        selections,
        parameters,
    })
}
