//! Existing SafeFS publication shared by explicit composition and launch carriers.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#LAYERED-PREFIX-LOCATION");
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use vibe_workspace::system_prompt::sha256_hex;
pub(super) fn publish(node: &Path, kind: &str, text: &str) -> Result<PathBuf> {
    let project = vibe_safefs::Project::open(node)?;
    let mut directory = project.root_dir()?;
    for component in vibe_core::layout::current_boot_dir()
        .join("local")
        .components()
    {
        directory = directory.ensure_child(
            component
                .as_os_str()
                .to_str()
                .context("unsupported local boot path")?,
        )?;
    }
    match project.read_file_snapshot_bounded_in(&directory, ".gitignore", 16)? {
        Some(snapshot) if snapshot.bytes == b"*\n" => {}
        Some(_) => bail!("personal boot local .gitignore already exists with different contents"),
        None => project.publish_new_in(&directory, ".gitignore", b"*\n")?,
    }
    let name = format!("{kind}-{}.md", sha256_hex(text.as_bytes()));
    match project.read_file_snapshot_bounded_in(&directory, &name, text.len())? {
        Some(snapshot) if snapshot.bytes == text.as_bytes() => {}
        Some(_) => bail!("personal boot content-addressed artifact has unexpected contents"),
        None => project.publish_new_in(&directory, &name, text.as_bytes())?,
    }
    Ok(directory.join(&name))
}
