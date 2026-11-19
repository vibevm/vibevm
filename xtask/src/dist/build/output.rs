use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub(super) fn write_output(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("distribution output has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating distribution output `{}`", parent.display()))?;
    let file_name = path
        .file_name()
        .context("distribution output has no file name")?
        .to_string_lossy();
    let temporary = parent.join(format!(".{file_name}.tmp.{}", std::process::id()));
    fs::write(&temporary, bytes)
        .with_context(|| format!("writing staged output `{}`", temporary.display()))?;
    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("removing prior derived output `{}`", path.display()))?;
    }
    fs::rename(&temporary, path)
        .with_context(|| format!("publishing derived output `{}`", path.display()))
}

pub(super) fn run_visible(command: &mut Command, label: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("spawning {label}"))?;
    if !status.success() {
        bail!("{label} failed (exit {:?})", status.code());
    }
    Ok(())
}

pub(super) fn write_native_installer(
    out_dir: &std::path::Path,
    target: &str,
    binaries: &std::collections::BTreeMap<String, std::path::PathBuf>,
) -> anyhow::Result<()> {
    let installer = binaries
        .get("vibe-installer")
        .context("Cargo emitted no native installer")?;
    let suffix = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let installer_path = out_dir.join(format!("vibe-installer-{target}{suffix}"));
    let installer_bytes = fs::read(installer)?;
    write_output(&installer_path, &installer_bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&installer_path, fs::Permissions::from_mode(0o755))?;
    }
    println!("dist build: {}", installer_path.display());
    Ok(())
}
