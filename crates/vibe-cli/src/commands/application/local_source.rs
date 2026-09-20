//! The `--local-source` expansion, kept at the application-source boundary.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#local-source-shorthand");

use anyhow::{Context, Result, bail};

use std::path::{Path, PathBuf};

use crate::cli::{InstallArgs, UpdateArgs};

pub(super) fn expand_install(args: &mut InstallArgs) -> Result<()> {
    if !args.local_source {
        return Ok(());
    }
    args.registry = Some(registry(&args.path)?);
    args.from_source = true;
    args.offline = true;
    Ok(())
}

/// Expands the update shorthand and returns the additional offline posture.
pub(super) fn expand_update(args: &mut UpdateArgs) -> Result<bool> {
    if !args.local_source {
        return Ok(false);
    }
    args.registry = Some(registry(&args.path)?);
    args.from_source = true;
    Ok(true)
}

fn registry(path: &Path) -> Result<PathBuf> {
    let selected = std::fs::canonicalize(path)
        .with_context(|| format!("resolving --local-source path `{}`", path.display()))?;
    let start = if selected.is_file() {
        selected
            .parent()
            .ok_or_else(|| anyhow::anyhow!("--local-source path has no parent"))?
    } else {
        selected.as_path()
    };
    for ancestor in start.ancestors() {
        let registry = ancestor.join("vibevm").join("vibepacks");
        if registry.is_dir() {
            return Ok(registry);
        }
    }
    bail!(
        "--local-source found no `vibevm/vibepacks` at or above `{}`",
        path.display()
    )
}
