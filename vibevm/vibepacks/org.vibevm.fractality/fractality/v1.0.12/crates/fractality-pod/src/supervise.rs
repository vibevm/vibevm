//! Child supervision: the one property the pod exists for (plan D3/F5).
//!
//! On Windows the child is assigned to a Job Object armed with
//! `KILL_ON_JOB_CLOSE` immediately after spawn: if this pod dies — panic,
//! kill, crash — the OS closes the job handle and reaps the whole worker
//! tree. The Phase 0 spike proved this survives even a parent that exits
//! without any cleanup code (F5): **a pod crash leaks no worker.**
//!
//! On POSIX the child gets its own process group at spawn
//! (`command-group`); this campaign validates Windows only (plan §10) —
//! the POSIX path is written portable and awaits the CI matrix (DEF-8).
//!
//! The environment is `env_clear()` + exactly the spec's map: invariant
//! I1 holds structurally (see the assertion test in this crate).

use std::process::Stdio;

use fractality_core::worker::WorkerSpec;

specmark::scope!("spec://fractality/PROP-001#invariants");

/// A supervised child plus the OS-level kill guarantee that owns it.
pub struct SupervisedChild {
    #[cfg(windows)]
    child: tokio::process::Child,
    /// Held for its `Drop`: closing the job handle kills the tree. An
    /// explicit [`Self::kill_tree`] takes and drops it early — same
    /// mechanism, deliberate timing (Phase 4 kill).
    #[cfg(windows)]
    job: Option<win32job::Job>,
    #[cfg(unix)]
    child: command_group::AsyncGroupChild,
}

/// Builds the command from a spec: clean-slate env, piped stdio. Stdin
/// is piped only when the spec carries a payload (the pod feeds it and
/// closes the pipe — F14), otherwise null so no child ever waits on us.
fn build_command(spec: &WorkerSpec) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(&spec.argv[0]);
    cmd.args(&spec.argv[1..])
        .env_clear()
        .envs(&spec.env)
        .current_dir(spec.cwd.as_std_path())
        .stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);
    cmd
}

/// Spawns the worker under supervision.
pub fn spawn(spec: &WorkerSpec) -> Result<SupervisedChild, String> {
    spec.validate().map_err(|e| e.to_string())?;
    let mut cmd = build_command(spec);

    #[cfg(windows)]
    {
        let job = win32job::Job::create().map_err(|e| format!("creating job object: {e}"))?;
        let mut info = job
            .query_extended_limit_info()
            .map_err(|e| format!("querying job limits: {e}"))?;
        info.limit_kill_on_job_close();
        job.set_extended_limit_info(&info)
            .map_err(|e| format!("arming KILL_ON_JOB_CLOSE: {e}"))?;

        let child = cmd
            .spawn()
            .map_err(|e| format!("spawning `{}`: {e}", spec.argv[0]))?;
        // Assign immediately after spawn (F5); the not-yet-assigned window
        // is accepted — the worker cannot fork faster than this line runs.
        let handle = child
            .raw_handle()
            .ok_or_else(|| "child has no handle (exited during spawn?)".to_owned())?;
        job.assign_process(handle as isize)
            .map_err(|e| format!("assigning child to job: {e}"))?;
        Ok(SupervisedChild {
            child,
            job: Some(job),
        })
    }

    #[cfg(unix)]
    {
        use command_group::AsyncCommandGroup;
        let child = cmd
            .group_spawn()
            .map_err(|e| format!("spawning `{}` in a group: {e}", spec.argv[0]))?;
        Ok(SupervisedChild { child })
    }
}

impl SupervisedChild {
    pub fn pid(&self) -> Option<u32> {
        #[cfg(windows)]
        {
            self.child.id()
        }
        #[cfg(unix)]
        {
            self.child.inner().id()
        }
    }

    pub fn take_stdin(&mut self) -> Option<tokio::process::ChildStdin> {
        #[cfg(windows)]
        {
            self.child.stdin.take()
        }
        #[cfg(unix)]
        {
            self.child.inner_mut().stdin.take()
        }
    }

    pub fn take_stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        #[cfg(windows)]
        {
            self.child.stdout.take()
        }
        #[cfg(unix)]
        {
            self.child.inner_mut().stdout.take()
        }
    }

    pub fn take_stderr(&mut self) -> Option<tokio::process::ChildStderr> {
        #[cfg(windows)]
        {
            self.child.stderr.take()
        }
        #[cfg(unix)]
        {
            self.child.inner_mut().stderr.take()
        }
    }

    /// Waits for exit; `None` = terminated without a code (signal).
    pub async fn wait(&mut self) -> Result<Option<i32>, String> {
        #[cfg(windows)]
        {
            let status = self
                .child
                .wait()
                .await
                .map_err(|e| format!("waiting for worker: {e}"))?;
            Ok(status.code())
        }
        #[cfg(unix)]
        {
            let status = self
                .child
                .wait()
                .await
                .map_err(|e| format!("waiting for worker: {e}"))?;
            Ok(status.code())
        }
    }

    /// Kills the whole worker tree now (Phase 4 `kill`). Windows: drop
    /// the Job Object — `KILL_ON_JOB_CLOSE` reaps every descendant, the
    /// exact mechanism F5 proved. POSIX: signal the process group. The
    /// caller still `wait()`s to harvest the exit.
    pub fn kill_tree(&mut self) {
        #[cfg(windows)]
        {
            if let Some(job) = self.job.take() {
                drop(job);
            } else {
                tracing::warn!("kill_tree called twice; job already closed");
            }
        }
        #[cfg(unix)]
        {
            if let Err(e) = self.child.kill() {
                tracing::warn!(error = %e, "process-group kill failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// I1 structural assertion: the command is built from `env_clear()`
    /// plus exactly the spec map — there is no inherit-and-override path,
    /// so parent poison (`ANTHROPIC_*`, `CLAUDE_*`) cannot leak no matter
    /// what this process's environment holds. `get_envs()` on a cleared
    /// command yields only explicit `Some` entries; any inherit-based
    /// construction would surface removals (`None`) or extra names here.
    #[test]
    fn child_command_env_is_exactly_the_spec_map() {
        let mut env = BTreeMap::new();
        env.insert("PATH".to_owned(), "C:/bin".to_owned());
        let spec = WorkerSpec {
            argv: vec!["worker.exe".into()],
            env,
            cwd: "C:/tmp".into(),
            stdin: None,
        };
        let cmd = build_command(&spec);
        let std_cmd = cmd.as_std();
        assert!(
            std_cmd.get_envs().all(|(_, v)| v.is_some()),
            "env_clear + explicit map only — no removals, no inherits"
        );
        let names: Vec<String> = std_cmd
            .get_envs()
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["PATH".to_owned()], "exactly the spec map");
        for name in names {
            for poison in fractality_backend_claude_code::env::POISON_PREFIXES {
                assert!(!name.starts_with(poison), "{name} is poison");
            }
        }
    }
}
