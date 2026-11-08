//! One long-lived process group owned by an operation and stopped by it.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW");
use super::*;
use command_group::{CommandGroup, GroupChild};

/// A group/job-spawned child whose stdin pipe is its parent-liveness signal.
///
/// Closing stdin asks the child to exit; [`Self::kill_and_reap`] terminates
/// the whole group and waits for it. Stdout and stderr stay inherited.
pub(crate) struct SessionProcess {
    child: GroupChild,
    liveness: Option<std::process::ChildStdin>,
    reaped: bool,
}

impl SessionProcess {
    pub(crate) fn spawn(
        program: &std::path::Path,
        args: &[OsString],
    ) -> Result<Self, ProcessError> {
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let mut child = spawn_retrying_busy(BUSY_ATTEMPTS, BUSY_PAUSE, || command.group_spawn())
            .map_err(|error| ProcessError::Spawn {
                program: program.display().to_string(),
                reason: error.to_string(),
            })?;
        let liveness = child.inner().stdin.take();
        let mut session = Self {
            child,
            liveness,
            reaped: false,
        };
        if session.liveness.is_none() {
            let cleanup = session.kill_and_reap();
            return Err(ProcessError::Spawn {
                program: program.display().to_string(),
                reason: format!("spawned helper exposed no liveness pipe; {cleanup:?}"),
            });
        }
        Ok(session)
    }

    /// The exit status once the child has exited on its own.
    pub(crate) fn exited(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        let status = self.child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            self.reaped = true;
        }
        Ok(status)
    }

    /// Terminate the owned group/job and wait until the leader is reaped.
    pub(crate) fn kill_and_reap(&mut self) -> Result<(), String> {
        self.liveness.take();
        if self.reaped {
            return Ok(());
        }
        let killed = self.child.kill().map_err(|error| error.to_string());
        let waited = self.child.wait().map_err(|error| error.to_string());
        self.reaped = waited.is_ok();
        match (killed, waited) {
            (_, Ok(_)) => Ok(()),
            (Err(kill), Err(wait)) => Err(format!("kill failed: {kill}; reap failed: {wait}")),
            (Ok(()), Err(wait)) => Err(format!("reap failed: {wait}")),
        }
    }

    /// Ask the child to exit by closing its liveness pipe, then reap it,
    /// terminating the group when it outlives `grace`.
    pub(crate) fn close(&mut self, grace: Duration) -> Result<(), String> {
        self.liveness.take();
        let deadline = std::time::Instant::now() + grace;
        while std::time::Instant::now() < deadline {
            if self.exited()?.is_some() {
                // The leader exited; still stop anything left in its group.
                let _ = self.child.kill();
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.kill_and_reap()
    }

    #[cfg(test)]
    pub(crate) fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for SessionProcess {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.kill_and_reap();
        }
    }
}
