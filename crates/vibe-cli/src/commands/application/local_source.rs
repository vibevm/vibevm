//! The `--local-source` expansion, kept at the application-source boundary.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#local-source-shorthand");

use anyhow::{Context, Result, bail};

use crate::cli::InstallArgs;

pub(super) fn expand(args: &mut InstallArgs) -> Result<()> {
    if !args.local_source {
        return Ok(());
    }
    let workspace = vibe_workspace::Workspace::discover(&args.path)
        .context("locating the checkout for --local-source")?;
    let registry = workspace.root.join("vibevm").join("vibepacks");
    if !registry.is_dir() {
        bail!(
            "--local-source expected this checkout's package registry at `{}`",
            registry.display()
        );
    }
    args.registry = Some(registry);
    args.from_source = true;
    args.offline = true;
    Ok(())
}
