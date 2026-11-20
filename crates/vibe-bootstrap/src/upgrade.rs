//! Explicit local newer-distribution upgrade through the existing CLI import owner.
use crate::{VerifiedBundle, file_verify};
use anyhow::{Context, Result, bail};
use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::{Command, Stdio},
};
use vibe_wire::behaviour::release_manifest::{
    DISTRIBUTION_COMPONENT_MAX_BYTES, DistributionComponentName,
};

fn query(mut command: Command) -> Result<serde_json::Value> {
    let mut output = tempfile::tempfile()?;
    let status = command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(Stdio::null())
        .status()?;
    if !status.success() || output.metadata()?.len() > 64 * 1024 {
        bail!("upgrade identity query refused");
    }
    output.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    output.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    serde_json::from_slice(&bytes).context("upgrade identity response invalid")
}

pub(crate) fn run(
    verified: &VerifiedBundle,
    kit: &Path,
    destination: &Path,
    settings: Option<&Path>,
    dry_run: bool,
) -> Result<()> {
    let executable = verified.directory.join("vibe");
    let child = || {
        let mut c = Command::new(&executable);
        c.current_dir(kit).env("VIBEVM_INSTALL_ROOT", destination);
        if let Some(s) = settings {
            c.env("VIBE_SETTINGS", s);
        }
        c
    };
    let mut current = child();
    current.args(["self", "current", "--json", "--unattended"]);
    let before = query(current)?;
    let selector = before["active_selector"]
        .as_str()
        .context("upgrade requires an existing active instance")?;
    let mut import = child();
    import.args(["self", "import"]).arg(&executable).args([
        "--tag",
        &verified.manifest.version,
        "--commit",
        &verified.manifest.source_commit,
        "--use",
        "--no-profile",
        "--expect-active",
        selector,
        "--unattended",
    ]);
    if dry_run {
        import.arg("--dry-run");
    }
    let status = import.stdin(Stdio::null()).status()?;
    if !status.success() {
        bail!("guarded local kit upgrade refused");
    }
    if dry_run {
        return Ok(());
    }
    let mut which = child();
    which.args([
        "self",
        "which",
        "vibe",
        "--active",
        "--json",
        "--unattended",
    ]);
    let identity = query(which)?;
    let installed = Path::new(
        identity["path"]
            .as_str()
            .context("upgraded payload path absent")?,
    );
    if !installed.is_absolute() || !installed.starts_with(destination) {
        bail!("upgraded payload is outside selected installation");
    }
    let component = verified
        .manifest
        .components
        .iter()
        .find(|c| c.name == DistributionComponentName::Vibe)
        .context("verified CLI component absent")?;
    let (_held, digest) = file_verify::open_hashed_regular_exact(
        installed,
        component.size,
        DISTRIBUTION_COMPONENT_MAX_BYTES,
        true,
    )?;
    if digest != component.digest {
        bail!("active upgraded payload differs from verified kit");
    }
    Ok(())
}
