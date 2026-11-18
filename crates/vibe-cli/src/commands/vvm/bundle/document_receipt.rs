//! Placement receipt for outer release documents not named by platform manifest.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#remove");
use anyhow::Result;
use std::path::Path;

pub(super) fn write(staging: &Path) -> Result<()> {
    let documents = crate::commands::vvm::placer::manifest_for(&[
        (staging.join("LICENSE.md"), "LICENSE.md".to_owned()),
        (staging.join("README.md"), "README.md".to_owned()),
    ])?;
    std::fs::write(
        staging.join(".vvm-manifest.toml"),
        toml::to_string(&documents)?,
    )?;
    Ok(())
}
