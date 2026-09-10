//! Compatibility wrapper for the operator-facing `vibe-index rebuild` verb.

use std::path::Path;

use anyhow::Result;

pub(crate) fn run_rebuild(check: bool, data_dir: &Path) -> Result<()> {
    vibe_index::cli::rebuild::run(vibe_index::cli::rebuild::Args {
        data_dir: data_dir.to_path_buf(),
        check,
    })?;
    Ok(())
}
