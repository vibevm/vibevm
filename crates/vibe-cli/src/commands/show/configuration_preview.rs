//! Thin ordinary-preview adapter; the shared operation owns source/evaluation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#SURFACE-PARITY");
use crate::{cli::ShowConfigurationPreviewArgs, output};
use anyhow::{Result, anyhow};
use vibe_package_source::configuration_preview::{MAX_INPUT_BYTES, configuration_preview};
use vibe_wire::generated::ordinary_configuration_preview_request::{
    Input, OrdinaryConfigurationPreviewRequest,
};

pub(super) fn run(ctx: &output::Context, args: ShowConfigurationPreviewArgs) -> Result<()> {
    // Preserve no-follow ancestry rather than canonicalizing user paths away.
    let root = std::path::absolute(&args.path)?;
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
