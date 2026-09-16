//! Structured native runtime dispatch for package-declared application installers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#context");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#reply");

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::model::{
    ApplicationContext, ApplicationOperation, ApplicationReply, ApplicationStatus, ManagementEntry,
    ManagementRuntime, RESULT_PROTOCOL,
};

pub fn resolve_node() -> Result<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let names: &[&str] = if cfg!(windows) {
        &["node.exe"]
    } else {
        &["node"]
    };
    for directory in std::env::split_paths(&path) {
        for name in names {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return fs::canonicalize(&candidate)
                    .map(crate::commands::init::strip_unc_public)
                    .with_context(|| format!("resolving Node runtime `{}`", candidate.display()));
            }
        }
    }
    bail!("global application installer requires native Node.js on PATH")
}

pub fn dispatch(
    entry: &Path,
    current_dir: &Path,
    context: &ApplicationContext,
    context_path: &Path,
    reply_path: &Path,
) -> Result<ApplicationReply> {
    for path in [context_path, reply_path] {
        vibe_safefs::ensure_no_follow_walk(&context.settings_root, path, true)?;
    }
    write_new_json(context_path, context)?;
    remove_file_if_present(reply_path)?;
    let stdout_path = context_path.with_extension("stdout");
    let stderr_path = context_path.with_extension("stderr");
    for path in [&stdout_path, &stderr_path] {
        vibe_safefs::ensure_no_follow_walk(&context.settings_root, path, true)?;
    }
    remove_file_if_present(&stdout_path)?;
    remove_file_if_present(&stderr_path)?;
    let stdout = File::create(&stdout_path).context("creating application installer stdout")?;
    let stderr = File::create(&stderr_path).context("creating application installer stderr")?;
    let node = resolve_node()?;
    let status = Command::new(&node)
        .arg(entry)
        .current_dir(current_dir)
        .env("VIBE_APPLICATION_CONTEXT", context_path)
        .env("VIBE_APPLICATION_REPLY", reply_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .status()
        .with_context(|| format!("starting application installer `{}`", entry.display()))?;
    let diagnostic = bounded_diagnostic(&stderr_path, &stdout_path);
    let parsed = read_reply(reply_path);
    let cleanup = || {
        let _ = fs::remove_file(context_path);
        let _ = fs::remove_file(reply_path);
        let _ = fs::remove_file(&stdout_path);
        let _ = fs::remove_file(&stderr_path);
    };
    if !status.success() {
        let reply_message = parsed
            .as_ref()
            .ok()
            .map(|reply| reply.message.as_str())
            .filter(|value| !value.is_empty());
        let message = reply_message.unwrap_or(&diagnostic);
        cleanup();
        bail!(
            "application installer exited {}: {}",
            status
                .code()
                .map_or_else(|| "without a code".into(), |code| code.to_string()),
            message
        );
    }
    let reply = match parsed {
        Ok(reply) => reply,
        Err(error) => {
            cleanup();
            return Err(error);
        }
    };
    validate_reply(context, &reply)?;
    cleanup();
    Ok(reply)
}

pub fn validate_management(
    host_root: &Path,
    management: &ManagementEntry,
) -> Result<ManagementEntry> {
    if management.runtime != ManagementRuntime::Node {
        bail!("application management runtime differs from its declaration");
    }
    let host = crate::commands::init::strip_unc_public(
        fs::canonicalize(host_root).context("resolving the owned application host")?,
    );
    let metadata = fs::symlink_metadata(&management.entry)
        .context("reading the retained application management entry")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("application management entry is not a retained regular file");
    }
    let entry = crate::commands::init::strip_unc_public(
        fs::canonicalize(&management.entry)
            .context("resolving the retained application management entry")?,
    );
    if !entry.starts_with(&host) || entry == host {
        bail!("application management entry is outside the owned application host");
    }
    Ok(ManagementEntry {
        runtime: management.runtime,
        entry,
    })
}

fn validate_reply(context: &ApplicationContext, reply: &ApplicationReply) -> Result<()> {
    if reply.protocol != RESULT_PROTOCOL
        || reply.operation != context.operation
        || reply.application_id != context.application.id
        || reply.host_root != context.host_root
        || reply.commands != context.application.commands
    {
        bail!("application installer reply does not match its dispatched context");
    }
    match (context.operation, &reply.status) {
        (
            ApplicationOperation::Install | ApplicationOperation::Update,
            ApplicationStatus::Ready,
        )
        | (ApplicationOperation::Uninstall, ApplicationStatus::Undeployed)
        | (_, ApplicationStatus::Failed) => Ok(()),
        _ => bail!("application installer reply status is invalid for the requested operation"),
    }
}

fn read_reply(path: &Path) -> Result<ApplicationReply> {
    let bytes = fs::read(path).context("application installer wrote no reply")?;
    serde_json::from_slice(&bytes).context("application installer reply is malformed")
}

fn write_new_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value).context("serializing application context")?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    let mut file = options
        .open(path)
        .with_context(|| format!("creating application context `{}`", path.display()))?;
    use std::io::Write;
    file.write_all(&bytes)
        .context("writing application context")?;
    file.sync_all().context("syncing application context")?;
    Ok(())
}

fn bounded_diagnostic(stderr: &Path, stdout: &Path) -> String {
    for path in [stderr, stdout] {
        if let Ok(bytes) = fs::read(path) {
            let start = bytes.len().saturating_sub(8_192);
            let text = String::from_utf8_lossy(&bytes[start..]).trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }
    }
    "no diagnostic".into()
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("removing `{}`", path.display())),
    }
}
