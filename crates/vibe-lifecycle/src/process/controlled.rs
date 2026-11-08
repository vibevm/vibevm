//! The existing process wire with invocation-owned cooperative control.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use super::*;
use command_group::CommandGroup;
use vibe_core::cancellation::CancellationToken;

/// Same capture/environment contract as SystemProcessRunner, with group/job stop.
/// ```
/// let runner = vibe_lifecycle::process::ControlledProcessRunner::new(
///     vibe_core::cancellation::CancellationToken::new());
/// let _: &dyn vibe_lifecycle::process::ProcessRunner = &runner;
/// ```
pub struct ControlledProcessRunner {
    cancellation: CancellationToken,
}
impl ControlledProcessRunner {
    pub fn new(cancellation: CancellationToken) -> Self {
        Self { cancellation }
    }
}
impl ProcessRunner for ControlledProcessRunner {
    fn cancellation(&self) -> Option<&CancellationToken> {
        self.cancellation
            .has_control()
            .then_some(&self.cancellation)
    }

    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        if !self.cancellation.has_control() {
            return SystemProcessRunner.run(spec);
        }
        self.cancellation
            .checkpoint()
            .map_err(|error| ProcessError::Wait(error.to_string()))?;
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .current_dir(&spec.cwd)
            .env_clear()
            .envs(&spec.env);
        command.stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let spawn_error = |reason: String| ProcessError::Spawn {
            program: spec.program.display().to_string(),
            reason,
        };
        let (stdout, mut stdout_capture) =
            stream_stdio(spec.stdout, &spec.scratch, "stdout").map_err(spawn_error)?;
        let (stderr, mut stderr_capture) =
            stream_stdio(spec.stderr, &spec.scratch, "stderr").map_err(spawn_error)?;
        command.stdout(stdout).stderr(stderr);
        let mut child = spawn_retrying_busy(BUSY_ATTEMPTS, BUSY_PAUSE, || command.group_spawn())
            .map_err(|error| spawn_error(error.to_string()))?;
        let writer = match (spec.stdin.as_ref(), child.inner().stdin.take()) {
            (Some(bytes), Some(mut pipe)) => {
                let bytes = bytes.clone();
                Some(std::thread::spawn(move || {
                    use std::io::Write;
                    pipe.write_all(&bytes)
                }))
            }
            _ => None,
        };
        let mut stopped = false;
        let status = loop {
            if self.cancellation.checkpoint().is_err() {
                stopped = true;
                child.kill().map_err(|error| {
                    ProcessError::Wait(format!("cancellation kill failed: {error}"))
                })?;
                break child
                    .wait()
                    .map_err(|error| ProcessError::Wait(error.to_string()))?;
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|error| ProcessError::Wait(error.to_string()))?
            {
                break status;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        // A cancelled process group/job is reaped before joining its stdin writer.
        if let Some(writer) = writer {
            let written = writer
                .join()
                .map_err(|_| ProcessError::Stdin("stdin writer panicked".into()))?;
            if !stopped {
                written.map_err(|error| ProcessError::Stdin(error.to_string()))?;
            }
        }
        let retain = stopped || !status.success();
        let (stdout, stdout_truncated) = read_capped(spec.stdout, stdout_capture.as_mut(), retain)?;
        let (stderr, stderr_truncated) = read_capped(spec.stderr, stderr_capture.as_mut(), retain)?;
        if stopped {
            return Err(ProcessError::Wait("operation cancelled; process group/job reaped; arbitrary handler effects are unknown".into()));
        }
        Ok(ProcessOutput {
            code: status.code(),
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn stop_reaps_long_hook_and_unblocks_stdin_writer() {
        let temp = tempfile::tempdir().unwrap();
        let token = CancellationToken::new();
        let signal = token.clone();
        let cancel = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            signal.cancel();
        });
        let spec = ProcessSpec {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "sleep 30 & wait".into()],
            cwd: temp.path().into(),
            env: minimal_environment([]),
            stdin: Some(vec![1; 4 << 20]),
            stdout: StreamMode::Capture,
            stderr: StreamMode::Capture,
            scratch: temp.path().into(),
        };
        let begin = std::time::Instant::now();
        assert!(ControlledProcessRunner::new(token).run(&spec).is_err());
        cancel.join().unwrap();
        assert!(begin.elapsed() < Duration::from_secs(5));
    }
}
