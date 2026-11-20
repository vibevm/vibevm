//! Opt-in metadata-only setup after successful mandatory installation.
//! Terminal answers stay with the existing CLI; no planner or secret reader here.
use crate::file_verify;
use anyhow::{Context, Result, bail};
use std::{
    io::IsTerminal,
    path::Path,
    process::{Command, Stdio},
};
use vibe_wire::behaviour::release_manifest::{
    DISTRIBUTION_COMPONENT_MAX_BYTES, DistributionComponent,
};

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    InputRequired,
    CompletedUnconfirmed,
    Failed,
}

/// Optional failure is reported separately and never reverses mandatory success.
pub(crate) fn after_install(
    staged: &Path,
    component: &DistributionComponent,
    kit: &Path,
    destination: Option<&Path>,
    settings: Option<&Path>,
    unattended: bool,
) {
    let attended = !unattended
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::io::stderr().is_terminal();
    let status = if !attended {
        Status::InputRequired
    } else {
        match invoke(staged, component, kit, destination, settings) {
            Ok(true) => Status::CompletedUnconfirmed,
            Ok(false) | Err(_) => Status::Failed,
        }
    };
    // The current setup interface does not distinguish cancel/no-op/applied.
    // Exit zero is deliberately NOT reported as applied.
    println!(
        "{}",
        serde_json::json!({
            "protocol":"vibe-installer-optional-setup/1",
            "installation":"completed",
            "optional_setup":{"status":status,"recovery_command":"vibe ai setup"}
        })
    );
}

fn scoped(
    program: &Path,
    kit: &Path,
    destination: Option<&Path>,
    settings: Option<&Path>,
) -> Command {
    let mut command = Command::new(program);
    command.current_dir(kit);
    if let Some(path) = destination {
        command.env("VIBEVM_INSTALL_ROOT", path);
    }
    if let Some(path) = settings {
        command.env("VIBE_SETTINGS", path);
    }
    command
}

fn invoke(
    staged: &Path,
    component: &DistributionComponent,
    kit: &Path,
    destination: Option<&Path>,
    settings: Option<&Path>,
) -> Result<bool> {
    // Ask the incumbent version-store owner; never parse its layout here or
    // launch a command found via PATH. Query output contains identity only.
    let output = tempfile::tempfile()?;
    let result = scoped(staged, kit, destination, settings)
        .args([
            "self",
            "which",
            "vibe",
            "--active",
            "--json",
            "--unattended",
        ])
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(Stdio::null())
        .status()?;
    if !result.success() || output.metadata()?.len() > 64 * 1024 {
        bail!("installed CLI identity unavailable");
    }
    use std::io::{Read, Seek, SeekFrom};
    let mut output = output;
    output.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    output.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if value["ok"] != true
        || value["command"] != "self:which"
        || value["component"] != "vibe"
        || !value["selector"].as_str().is_some_and(|s| !s.is_empty())
    {
        bail!("installed CLI identity incomplete");
    }
    let installed = Path::new(value["path"].as_str().context("installed path absent")?);
    if !installed.is_absolute() || installed == staged {
        bail!("installed CLI path unavailable");
    }
    let (_held, digest) = file_verify::open_hashed_regular_exact(
        installed,
        component.size,
        DISTRIBUTION_COMPONENT_MAX_BYTES,
        true,
    )?;
    if digest != component.digest {
        bail!("installed CLI differs from verified distribution");
    }
    // No --unattended, raw output capture, answer logging, synthetic approval,
    // provider launch or terminal-mode changes. The accepted setup owns consent.
    Ok(scoped(installed, kit, destination, settings)
        .args(["ai", "setup"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?
        .success())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use vibe_wire::behaviour::release_manifest::DistributionComponentName;

    #[test]
    fn mismatched_installed_digest_refuses_before_setup_child() {
        let root = tempfile::tempdir().unwrap();
        let installed = root.path().join("installed");
        let marker = root.path().join("executed");
        let program = format!("#!/bin/sh\ntouch '{}'\n", marker.display());
        std::fs::write(&installed, &program).unwrap();
        std::fs::set_permissions(&installed, std::fs::Permissions::from_mode(0o755)).unwrap();
        let staged = root.path().join("staged");
        let identity = serde_json::json!({"ok":true,"command":"self:which","component":"vibe","selector":"tag:1#1","path":installed});
        std::fs::write(
            &staged,
            format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", identity),
        )
        .unwrap();
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
        let component = DistributionComponent {
            name: DistributionComponentName::Vibe,
            path: "vibe".into(),
            size: program.len() as u64,
            digest: format!("sha256:{}", "0".repeat(64)),
        };
        assert!(invoke(&staged, &component, root.path(), None, None).is_err());
        assert!(!marker.exists());
    }
}
