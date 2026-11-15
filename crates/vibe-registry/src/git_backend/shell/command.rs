//! Native child construction and bounded command execution.
use super::*;

impl ShellGit {
    pub(super) fn admit_native(&self) -> Result<(), GitError> {
        if self.native.is_some() {
            return Err(native_admission::unavailable());
        }
        if self.admission.is_some() {
            return Err(crate::ContactRefusal::NativeTransportUnavailable.into());
        }
        Ok(())
    }

    pub(super) fn run(&self, args: &[&str], cwd: Option<&Path>) -> Result<Output, GitError> {
        self.admit_native()?;
        let mut cmd = Command::new(&self.binary);
        // apply_common_env may push `-c <key>=<value>` flags that must
        // come BEFORE the subcommand — push the env / global-flag
        // layer first, then the actual git subcommand args.
        apply_common_env(&mut cmd, self.force_anonymous);
        env::apply_network(&mut cmd, &self.binary, args, cwd)?;
        cmd.args(args);
        if let Some(d) = cwd {
            cmd.current_dir(d);
        }

        tracing::debug!(target: "vibe_registry::git", argv = ?render_argv(&self.binary, args), cwd = ?cwd, "running git");

        let output = controlled::output(
            &mut cmd,
            crate::git_backend::transfers::current()
                .progress
                .unwrap_or_default()
                .cancellation(),
        )
        .map_err(|e| controlled::git_error(e, render_argv(&self.binary, args)))?;
        if output.status.success() {
            return Ok(output);
        }
        Err(classify_failure(args, &output))
    }

    /// Like [`Self::run`] but returns the raw [`Output`] on non-zero exit
    /// without classifying it. Used by callers that need to look at
    /// stdout / stderr together (e.g. `fetch_file_at_ref` distinguishing
    /// "ref missing" from "file missing in ref" from "archive
    /// unsupported").
    pub(super) fn run_raw(&self, args: &[&str], cwd: Option<&Path>) -> Result<Output, GitError> {
        self.admit_native()?;
        let mut cmd = Command::new(&self.binary);
        apply_common_env(&mut cmd, self.force_anonymous);
        env::apply_network(&mut cmd, &self.binary, args, cwd)?;
        cmd.args(args);
        if let Some(d) = cwd {
            cmd.current_dir(d);
        }

        tracing::debug!(target: "vibe_registry::git", argv = ?render_argv(&self.binary, args), cwd = ?cwd, "running git (raw)");

        controlled::output(
            &mut cmd,
            crate::git_backend::transfers::current()
                .progress
                .unwrap_or_default()
                .cancellation(),
        )
        .map_err(|e| controlled::git_error(e, render_argv(&self.binary, args)))
    }
}
