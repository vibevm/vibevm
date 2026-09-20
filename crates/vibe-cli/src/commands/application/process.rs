//! Structured native runtime dispatch for package-declared application installers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#context");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#reply");

use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::progress::{Progress, ProgressDiagnosticLevel};
use vibe_wire::generated::application::e1::reply as reply_wire;

use crate::output::sanitize_progress_text;

use super::model::{
    ApplicationContext, ApplicationOperation, ApplicationReply, ApplicationStatus, ManagementEntry,
    RESULT_PROTOCOL,
};

const LIVE_LOG_POLL: Duration = Duration::from_millis(100);
const MAX_LIVE_LINE_BYTES: usize = 8_192;

#[derive(Clone, Copy)]
enum LiveLogKind {
    Stdout,
    Stderr,
}

struct LiveLogTail {
    file: File,
    pending: Vec<u8>,
    kind: LiveLogKind,
    readable: bool,
}

impl LiveLogTail {
    fn open(path: &Path, kind: LiveLogKind) -> Option<Self> {
        OpenOptions::new()
            .read(true)
            .open(path)
            .ok()
            .map(|file| Self {
                file,
                pending: Vec::new(),
                kind,
                readable: true,
            })
    }

    /// Observe bytes already written to the owned capture file. Read failures
    /// disable live detail only: progress is ancillary and can never change the
    /// child process, reply protocol, or final captured diagnostic.
    fn poll(&mut self, task: &vibe_core::progress::ProgressTask, final_poll: bool) {
        if !self.readable {
            return;
        }
        let mut chunk = [0u8; 4 * 1024];
        loop {
            match self.file.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => self.pending.extend_from_slice(&chunk[..read]),
                Err(_) => {
                    self.readable = false;
                    return;
                }
            }
            self.emit_complete_lines(task);
            while self.pending.len() >= MAX_LIVE_LINE_BYTES {
                let bounded: Vec<_> = self.pending.drain(..MAX_LIVE_LINE_BYTES).collect();
                self.emit(task, &bounded);
            }
        }
        self.emit_complete_lines(task);
        if final_poll && !self.pending.is_empty() {
            let tail = std::mem::take(&mut self.pending);
            self.emit(task, &tail);
        }
    }

    fn emit_complete_lines(&mut self, task: &vibe_core::progress::ProgressTask) {
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<_> = self.pending.drain(..=end).collect();
            self.emit(task, &line);
        }
    }

    fn emit(&self, task: &vibe_core::progress::ProgressTask, bytes: &[u8]) {
        let text = sanitize_progress_text(String::from_utf8_lossy(bytes).trim());
        if text.is_empty() {
            return;
        }
        let lower = text.to_ascii_lowercase();
        match self.kind {
            LiveLogKind::Stderr if lower.contains("error") => {
                task.diagnostic(ProgressDiagnosticLevel::Error, text)
            }
            LiveLogKind::Stderr if lower.contains("warning") => {
                task.diagnostic(ProgressDiagnosticLevel::Warning, text)
            }
            LiveLogKind::Stdout | LiveLogKind::Stderr => task.detail(text),
        }
    }
}

#[spec(
    deviates = "spec://org.vibevm.core/vibevm/common/PROP-059#context",
    reason = "The trusted CLI process resolves native Node from its own inherited PATH immediately before launching the package-declared installer; the browser and application context cannot supply or override this environment value."
)]
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
    progress: &Progress,
) -> Result<ApplicationReply> {
    let task = progress.task(format!(
        "Running source application {} for {}",
        match context.operation {
            ApplicationOperation::Install => "installer",
            ApplicationOperation::Update => "updater",
            ApplicationOperation::Uninstall => "uninstaller",
        },
        context.application.id
    ));
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
    // Separate read handles keep the exact capture files as the child stream
    // destinations while allowing bounded, sanitized observation during the
    // wait. The structured application reply remains a different file.
    let mut stdout_tail = LiveLogTail::open(&stdout_path, LiveLogKind::Stdout);
    let mut stderr_tail = LiveLogTail::open(&stderr_path, LiveLogKind::Stderr);
    let node = resolve_node()?;
    let mut child = Command::new(&node)
        .arg(entry)
        .current_dir(current_dir)
        .env("VIBE_APPLICATION_CONTEXT", context_path)
        .env("VIBE_APPLICATION_REPLY", reply_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("starting application installer `{}`", entry.display()))?;
    let status = loop {
        if let Some(tail) = &mut stdout_tail {
            tail.poll(&task, false);
        }
        if let Some(tail) = &mut stderr_tail {
            tail.poll(&task, false);
        }
        if let Some(status) = child
            .try_wait()
            .context("waiting for the application installer")?
        {
            break status;
        }
        std::thread::sleep(LIVE_LOG_POLL);
    };
    if let Some(tail) = &mut stdout_tail {
        tail.poll(&task, true);
    }
    if let Some(tail) = &mut stderr_tail {
        tail.poll(&task, true);
    }
    let diagnostic = sanitize_progress_text(&bounded_diagnostic(&stderr_path, &stdout_path));
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
        let message = sanitize_progress_text(reply_message.unwrap_or(&diagnostic));
        task.diagnostic(ProgressDiagnosticLevel::Error, message.clone());
        task.fail("source application process failed");
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
            task.fail("source application reply was invalid");
            cleanup();
            return Err(error);
        }
    };
    if let Err(error) = validate_reply(context, &reply) {
        task.fail("source application reply was invalid");
        cleanup();
        return Err(error);
    }
    cleanup();
    task.finish();
    Ok(reply)
}

pub fn validate_management(
    host_root: &Path,
    management: &ManagementEntry,
) -> Result<ManagementEntry> {
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
    validate_launchers(context, reply)?;
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

fn validate_launchers(context: &ApplicationContext, reply: &ApplicationReply) -> Result<()> {
    let root = context.settings_root.join("opt").join("bin");
    let mut destinations = std::collections::BTreeSet::new();
    for launcher in &reply.launchers {
        let Some(name) = launcher
            .destination
            .file_name()
            .and_then(|value| value.to_str())
        else {
            bail!("application reply launcher has no portable file name");
        };
        if launcher.destination.parent() != Some(root.as_path())
            || !launcher_belongs_to_command(name, &context.application.commands)
            || !destinations.insert(launcher.destination.clone())
            || launcher.sha256.len() != 64
            || !launcher
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("application reply launcher ownership is invalid");
        }
        let metadata = fs::symlink_metadata(&launcher.destination)
            .context("reading application reply launcher")?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || hash_file(&launcher.destination)? != launcher.sha256
        {
            bail!("application reply launcher does not match deployed regular bytes");
        }
    }
    Ok(())
}

fn launcher_belongs_to_command(name: &str, commands: &[String]) -> bool {
    commands.iter().any(|command| {
        name == command
            || [".cmd", ".ps1", ".sh"]
                .iter()
                .any(|suffix| name == format!("{command}{suffix}"))
    })
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        use std::io::Read;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn read_reply(path: &Path) -> Result<ApplicationReply> {
    let bytes = fs::read(path).context("application installer wrote no reply")?;
    let wire: reply_wire::ApplicationReply =
        serde_json::from_slice(&bytes).context("application installer reply is malformed")?;
    Ok(ApplicationReply::from_wire(wire))
}

fn write_new_json(path: &Path, value: &ApplicationContext) -> Result<()> {
    let bytes = serde_json::to_vec(&value.to_wire()).context("serializing application context")?;
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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use vibe_core::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};

    use super::{LiveLogKind, LiveLogTail};

    #[derive(Default)]
    struct Recorded(Mutex<Vec<ProgressEvent>>);

    impl ProgressObserver for Recorded {
        fn observe(&self, event: ProgressEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    #[test]
    fn fake_node_log_is_reported_before_child_exit() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("fake-node.stdout");
        let mut writer = File::create(&log).unwrap();
        let mut tail = LiveLogTail::open(&log, LiveLogKind::Stdout).unwrap();
        let observer = Arc::new(Recorded::default());
        let progress = Progress::new(observer.clone());
        let child = progress.task("Running fake Node installer");

        writer.write_all(b"building provider\n").unwrap();
        writer.flush().unwrap();
        tail.poll(&child, false);

        let events = observer.0.lock().unwrap();
        assert!(events.iter().any(|event| matches!(
            &event.kind,
            ProgressEventKind::Detail { message } if message == "building provider"
        )));
        assert!(
            !events
                .iter()
                .any(|event| matches!(&event.kind, ProgressEventKind::Finished)),
            "the live line arrives before the child task is marked complete",
        );
        drop(events);
        child.finish();
    }

    #[test]
    fn quiet_and_json_contexts_keep_application_progress_inert() {
        use crate::cli::AgentModeArg;
        use crate::output::{Context, ProgressMode};

        for (quiet, json) in [(true, false), (false, true)] {
            let ctx = Context::from_flags(quiet, json, None, false, AgentModeArg::Cli)
                .with_progress(true, ProgressMode::Plain);
            let task = ctx.progress().task("hidden fake Node installer");
            assert_eq!(task.id().get(), 0);
            task.finish();
        }
    }
}
