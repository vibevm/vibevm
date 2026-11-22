//! Recreate derived gate dependencies inside the owned source snapshot.

use std::path::Path;

use anyhow::{Result, bail};

/// Recreate the host's derived application slot from the same pinned source.
/// The authored app is in the archive; its development slot is intentionally
/// absent from Git. This preparation applies only to the owned gate scratch.
pub(super) fn prepare_self_check_dependencies(root: &Path) -> Result<()> {
    let source = root.join("vibevm/vibepacks/org.vibevm.doc/web/v1.0.12");
    let slot = root.join("vibevm/vibedeps/org.vibevm.doc.web/1.0.12");
    if source.is_dir() && !slot.exists() {
        copy_gate_dependency(&source, &slot)?;
    }
    Ok(())
}

fn copy_gate_dependency(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_gate_dependency(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), target)?;
        } else {
            bail!(
                "unsupported entry in pinned gate dependency: {}",
                entry.path().display()
            );
        }
    }
    Ok(())
}
