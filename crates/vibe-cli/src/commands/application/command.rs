//! Shell and native installer-file execution using the application index host.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::model::{ManagementEntry, ManagementRuntime};

const PROTOCOL: &str = "vibe-application-command-management/1";

/// An installer process. Only `Shell` evaluates shell syntax; file installers
/// receive structured arguments and a verified local file path.
pub enum Invocation<'a> {
    Shell(&'a str),
    Exe(&'a Path, &'a [String]),
    Msi(&'a Path, &'a [String]),
}

pub struct ApplyRequest<'a> {
    pub invocation: Invocation<'a>,
    pub app_root_template: &'a str,
    pub uninstall: Option<&'a str>,
    pub force_uninstall: Option<&'a str>,
    pub settings_root: &'a Path,
    pub host_root: &'a Path,
    pub application_id: &'a str,
    pub json_output: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandManagement {
    protocol: String,
    application_id: String,
    app_root: PathBuf,
    uninstall: Option<String>,
    #[serde(default)]
    force_uninstall: Option<String>,
}

pub struct CommandPublication {
    entry: PathBuf,
    backup: Option<PathBuf>,
}

impl CommandPublication {
    pub fn commit(self) {
        if let Some(backup) = self.backup {
            let _ = fs::remove_file(backup);
        }
    }

    pub fn rollback(self) -> Result<()> {
        fs::remove_file(&self.entry).context("removing new command management")?;
        if let Some(backup) = self.backup {
            fs::rename(backup, &self.entry).context("restoring prior command management")?;
        }
        Ok(())
    }
}

pub fn apply(request: ApplyRequest<'_>) -> Result<(ManagementEntry, PathBuf, CommandPublication)> {
    let ApplyRequest {
        invocation,
        app_root_template,
        uninstall,
        force_uninstall,
        settings_root,
        host_root,
        application_id,
        json_output,
    } = request;
    let expected_root = expand_app_root(app_root_template, settings_root)?;
    run_invocation(invocation, settings_root, &expected_root, json_output)?;
    let app_root = canonical_app_root(&expected_root)?;
    let management_dir = host_root.join("management");
    fs::create_dir_all(&management_dir).context("creating command management directory")?;
    let entry = management_dir.join("command.json");
    let backup = entry.with_file_name(format!(".command-{}.previous", std::process::id()));
    if backup.exists() {
        bail!("prior command management backup already exists");
    }
    let receipt = CommandManagement {
        protocol: PROTOCOL.into(),
        application_id: application_id.into(),
        app_root: app_root.clone(),
        uninstall: uninstall.map(str::to_owned),
        force_uninstall: force_uninstall.map(str::to_owned),
    };
    let bytes = serde_json::to_vec_pretty(&receipt)?;
    let temporary = management_dir.join(format!(".command-{}.json", std::process::id()));
    fs::write(&temporary, bytes).context("staging command management")?;
    let had_prior = entry.exists();
    if had_prior {
        fs::rename(&entry, &backup).context("retaining prior command management")?;
    }
    if let Err(error) = fs::rename(&temporary, &entry) {
        if had_prior {
            let _ = fs::rename(&backup, &entry);
        }
        return Err(error).context("publishing command management");
    }
    Ok((
        ManagementEntry {
            runtime: ManagementRuntime::Command,
            entry: entry.clone(),
        },
        app_root,
        CommandPublication {
            entry,
            backup: had_prior.then_some(backup),
        },
    ))
}

pub fn uninstall(
    entry: &ManagementEntry,
    application_id: &str,
    json_output: bool,
    force: bool,
) -> Result<String> {
    if entry.runtime != ManagementRuntime::Command {
        bail!("application management is not a command installer");
    }
    let bytes = fs::read(&entry.entry).context("reading retained command management")?;
    let management: CommandManagement =
        serde_json::from_slice(&bytes).context("parsing retained command management")?;
    if management.protocol != PROTOCOL || management.application_id != application_id {
        bail!("retained command management identity is invalid");
    }
    let script = if force {
        management
            .force_uninstall
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("selected installer has no force_uninstall command"))?
    } else {
        management
            .uninstall
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("selected installer has no uninstall command"))?
    };
    let current_dir = entry
        .entry
        .parent()
        .ok_or_else(|| anyhow::anyhow!("command management entry has no parent directory"))?;
    run_invocation(
        Invocation::Shell(script),
        current_dir,
        &management.app_root,
        json_output,
    )?;
    Ok("installer uninstall command completed".into())
}

/// Run a package-authored pre/post install hook through the same shell path.
pub fn run_shell_hook(
    script: &str,
    package_root: &Path,
    app_root: &Path,
    json_output: bool,
) -> Result<()> {
    run_invocation(
        Invocation::Shell(script),
        package_root,
        app_root,
        json_output,
    )
}

/// Resolve an external root before an installer creates it.
pub fn preview_app_root(template: &str, settings_root: &Path) -> Result<PathBuf> {
    expand_app_root(template, settings_root)
}

fn run_invocation(
    invocation: Invocation<'_>,
    current_dir: &Path,
    app_root: &Path,
    json_output: bool,
) -> Result<()> {
    let mut command = match invocation {
        Invocation::Shell(script) => {
            if script.trim().is_empty() {
                bail!("installer shell command is empty");
            }
            #[cfg(windows)]
            let mut command = {
                let mut command = Command::new("cmd.exe");
                command.args(["/D", "/S", "/C", script]);
                command
            };
            #[cfg(not(windows))]
            let mut command = {
                let mut command = Command::new("/bin/sh");
                command.args(["-c", script]);
                command
            };
            command.env("VIBE_APP_ROOT", app_root);
            command
        }
        Invocation::Exe(file, args) => {
            if !cfg!(windows) {
                bail!("EXE installer can run only on Windows");
            }
            let mut command = Command::new(file);
            command.args(args);
            command
        }
        Invocation::Msi(file, args) => {
            if !cfg!(windows) {
                bail!("MSI installer can run only on Windows");
            }
            let mut command = Command::new("msiexec.exe");
            command.arg("/i").arg(file).args(args);
            command
        }
    };
    command
        .current_dir(current_dir)
        .env("VIBE_APP_ROOT", app_root)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = if json_output {
        // Keep Vibe's JSON stdout machine-readable while showing the child
        // stdout on the invoking console's stderr stream.
        let mut child = command
            .stdout(Stdio::piped())
            .spawn()
            .context("starting application installer")?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("installer stdout pipe is unavailable"))?;
        let relay = std::thread::spawn(move || -> io::Result<()> {
            let mut input = stdout;
            let mut output = io::stderr().lock();
            let mut buffer = [0u8; 8192];
            loop {
                let read = input.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                output.write_all(&buffer[..read])?;
                output.flush()?;
            }
            Ok(())
        });
        let status = child.wait().context("waiting for application installer")?;
        relay
            .join()
            .map_err(|_| anyhow::anyhow!("installer stdout relay failed"))??;
        status
    } else {
        command
            .stdout(Stdio::inherit())
            .status()
            .context("starting application installer")?
    };
    if !status.success() {
        bail!(
            "application installer exited {}",
            status
                .code()
                .map_or_else(|| "without a code".into(), |code| code.to_string())
        );
    }
    Ok(())
}

fn expand_app_root(template: &str, settings_root: &Path) -> Result<PathBuf> {
    let expanded = if let Some(rest) = template.strip_prefix("${vibe_home}") {
        format!("{}{rest}", settings_root.display())
    } else if let Some(rest) = template.strip_prefix("$vibe_home") {
        format!("{}{rest}", settings_root.display())
    } else {
        template.to_owned()
    };
    let path = PathBuf::from(expanded);
    if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
        bail!("installer app_root must resolve to an absolute path without `..`");
    }
    Ok(path)
}

fn canonical_app_root(path: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path).context("reading installed app_root")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("installed app_root is not a real directory");
    }
    Ok(crate::commands::init::strip_unc_public(fs::canonicalize(
        path,
    )?))
}
