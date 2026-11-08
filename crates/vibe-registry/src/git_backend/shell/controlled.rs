//! Cooperative control of metadata Git children.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use command_group::CommandGroup;
use std::{
    io::{self, Read},
    process::{Command, Output, Stdio},
    time::Duration,
};
use vibe_core::cancellation::{CancellationToken, Cancelled};

/// Run one child in its own process group/job and reap it on cancellation.
/// ```no_run
/// let token = vibe_core::cancellation::CancellationToken::new();
/// let output = vibe_registry::git_backend::shell::controlled::output(
///     &mut std::process::Command::new("git"), &token)?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn output(command: &mut Command, token: &CancellationToken) -> io::Result<Output> {
    if !token.has_control() {
        return command.output();
    }
    token.checkpoint().map_err(io::Error::other)?;
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = command.group_spawn()?;
    let stdout = child.inner().stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).map(|_| bytes)
        })
    });
    let stderr = child.inner().stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).map(|_| bytes)
        })
    });
    let mut cancelled = false;
    let mut observed_status = None;
    let status = loop {
        if token.checkpoint().is_err() {
            cancelled = true;
            child.kill()?;
            break child.wait()?;
        }
        if observed_status.is_none() {
            observed_status = child.try_wait()?;
        }
        let drained = stdout.as_ref().is_none_or(|reader| reader.is_finished())
            && stderr.as_ref().is_none_or(|reader| reader.is_finished());
        if let Some(status) = observed_status.as_ref()
            && drained
        {
            break *status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let collect =
        |reader: Option<std::thread::JoinHandle<io::Result<Vec<u8>>>>| -> io::Result<Vec<u8>> {
            match reader {
                Some(reader) => reader
                    .join()
                    .map_err(|_| io::Error::other("Git output reader panicked"))?,
                None => Ok(Vec::new()),
            }
        };
    let stdout = collect(stdout)?;
    let stderr = collect(stderr)?;
    if cancelled {
        return Err(io::Error::other(Cancelled));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn cancellation_reaps_a_shell_and_its_pipe_holding_descendant() {
        let token = CancellationToken::new();
        let signal = token.clone();
        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            signal.cancel();
        });
        let start = std::time::Instant::now();
        let result = output(
            Command::new("/bin/sh").args(["-c", "sleep 30 & wait"]),
            &token,
        );
        sender.join().unwrap();
        assert!(result.is_err());
        assert!(token.acknowledged());
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}

pub(super) enum TransferChild {
    Plain(std::process::Child),
    Group(command_group::GroupChild),
}
pub(super) fn spawn_transfer(
    command: &mut Command,
    token: &CancellationToken,
) -> io::Result<TransferChild> {
    if token.has_control() {
        command.group_spawn().map(TransferChild::Group)
    } else {
        command.spawn().map(TransferChild::Plain)
    }
}
impl TransferChild {
    pub(super) fn inner(&mut self) -> &mut std::process::Child {
        match self {
            Self::Plain(child) => child,
            Self::Group(child) => child.inner(),
        }
    }
    pub(super) fn kill(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(child) => child.kill(),
            Self::Group(child) => child.kill(),
        }
    }
    pub(super) fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        match self {
            Self::Plain(child) => child.wait(),
            Self::Group(child) => child.wait(),
        }
    }
}

pub(super) fn git_error(source: std::io::Error, cmd: String) -> super::GitError {
    if source
        .get_ref()
        .is_some_and(|error| error.is::<Cancelled>())
    {
        super::GitError::ContactRefused(crate::ContactRefusal::OperationCancelled)
    } else {
        super::GitError::Io { cmd, source }
    }
}

/// Fixed metadata evaluation with bounded lifetime and captured stdout. The
/// stderr policy already selected by the caller is retained.
pub fn bounded_output(
    command: &mut Command,
    token: &CancellationToken,
    timeout: Duration,
    cap: usize,
) -> io::Result<Output> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    token.checkpoint().map_err(io::Error::other)?;
    command.stdout(Stdio::piped()).stdin(Stdio::null());
    let mut child = command.group_spawn()?;
    let oversized = Arc::new(AtomicBool::new(false));
    let over = oversized.clone();
    let stdout = child.inner().stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let mut chunk = [0_u8; 4096];
            loop {
                let count = pipe.read(&mut chunk)?;
                if count == 0 {
                    break;
                }
                let remaining = cap.saturating_add(1).saturating_sub(bytes.len());
                bytes.extend_from_slice(&chunk[..count.min(remaining)]);
                if count > remaining || bytes.len() > cap {
                    over.store(true, Ordering::SeqCst);
                }
            }
            Ok::<_, io::Error>(bytes)
        })
    });
    let started = std::time::Instant::now();
    let mut failure = None;
    let mut observed_status = None;
    let status = loop {
        if token.checkpoint().is_err() {
            failure = Some(io::Error::other(Cancelled));
        } else if oversized.load(Ordering::SeqCst) {
            failure = Some(io::Error::other("metadata evaluator exceeded output bound"));
        } else if started.elapsed() > timeout {
            failure = Some(io::Error::new(
                io::ErrorKind::TimedOut,
                "metadata evaluator exceeded time bound",
            ));
        }
        if failure.is_some() {
            child.kill()?;
            break child.wait()?;
        }
        if observed_status.is_none() {
            observed_status = child.try_wait()?;
        }
        if let Some(status) = observed_status.as_ref()
            && stdout.as_ref().is_none_or(|reader| reader.is_finished())
        {
            break *status;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let stdout = match stdout {
        Some(reader) => reader
            .join()
            .map_err(|_| io::Error::other("metadata reader panicked"))??,
        None => Vec::new(),
    };
    if let Some(error) = failure {
        return Err(error);
    }
    if stdout.len() > cap {
        return Err(io::Error::other("metadata evaluator exceeded output bound"));
    }
    Ok(Output {
        status,
        stdout,
        stderr: Vec::new(),
    })
}

#[cfg(all(test, unix))]
mod bounded_metadata_tests {
    use super::*;
    #[test]
    fn stop_after_leader_exit_reaps_descendant_holding_pipe() {
        let token = CancellationToken::new();
        let signal = token.clone();
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            signal.cancel();
        });
        let start = std::time::Instant::now();
        let result = bounded_output(
            Command::new("/bin/sh")
                .args(["-c", "sleep 30 & exit 0"])
                .stderr(Stdio::null()),
            &token,
            Duration::from_secs(5),
            262144,
        );
        canceller.join().unwrap();
        assert!(result.is_err());
        assert!(token.acknowledged());
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn evaluator_output_remains_bounded() {
        let token = CancellationToken::new();
        let error = bounded_output(
            Command::new("/bin/sh")
                .args(["-c", "while :; do printf 'xxxxxxxxxxxxxxxx'; done"])
                .stderr(Stdio::null()),
            &token,
            Duration::from_secs(5),
            1024,
        )
        .unwrap_err();
        assert!(error.to_string().contains("output bound"));
    }
}
