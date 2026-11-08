//! Explicit consent and read-only observation use the same persister target as repair.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);
use super::super::{VvmEnv, env, error::VvmError, make_persister};
use crate::output;
use std::{io::IsTerminal, path::Path};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn observe(environment: &VvmEnv, shim_dir: &Path) -> env::EnvironmentObservation {
    let shell = env::Shell::detect(environment.shell.as_deref());
    match make_persister(environment, shell) {
        Ok(persister) => persister.inspect_path(shim_dir),
        Err(error) => env::EnvironmentObservation::unavailable(error.to_string()),
    }
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn confirm_fix(
    ctx: &output::Context,
    yes: bool,
    target: Option<&str>,
) -> Result<bool, VvmError> {
    if yes {
        return Ok(true);
    }
    if ctx.is_unattended() || !std::io::stdin().is_terminal() {
        return Err(VvmError::NoTty { detail: "durable environment repair requires explicit --yes or interactive consent; unattended mode alone is not consent".into() });
    }
    let prompt = format!(
        "Write shims and update PATH at {}?",
        target.unwrap_or("the unavailable durable target")
    );
    Ok(crate::output::prompts::confirm(&prompt, false).unwrap_or(false))
}

/// Consent to run the caller's login shell once: the explicit
/// `--probe-login-shell` flag, or a yes to an interactive question. An
/// unattended or non-terminal run without the flag never asks and never runs it.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn login_probe_consent(ctx: &output::Context, flag: bool) -> bool {
    if flag {
        return true;
    }
    if ctx.is_unattended() || !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return false;
    }
    crate::output::prompts::confirm(
        "Run your login shell once to check whether its PATH includes vibe? \
         This executes your shell startup files.",
        false,
    )
    .unwrap_or(false)
}

/// How long a consented login shell may take before it is killed.
const LOGIN_PROBE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
/// Marks the one stdout line the probe reads; startup-file output is ignored.
const LOGIN_PATH_MARKER: &str = "__VIBE_LOGIN_PATH__";

/// Run `$SHELL -l -c` once with no stdin and read only the marked PATH line;
/// record whether the managed shim directory is in effect for login shells.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn probe_login_shell(
    environment: &VvmEnv,
    shim_dir: &Path,
    observation: &mut env::EnvironmentObservation,
) {
    let (status, effectiveness) = login_path(environment.shell.as_deref()).map_or(
        (env::ProbeStatus::Unsupported, None),
        |result| match result {
            Ok(Some(path)) => (
                env::ProbeStatus::Ran,
                Some(std::env::split_paths(&path).any(|entry| entry == shim_dir)),
            ),
            Ok(None) => (env::ProbeStatus::Failed, None),
            Err(LoginProbeError::TimedOut) => (env::ProbeStatus::TimedOut, None),
            Err(LoginProbeError::Spawn) => (env::ProbeStatus::Failed, None),
        },
    );
    observation.login_probe = status;
    observation.login_effectiveness = match effectiveness {
        Some(true) => env::Effectiveness::Effective,
        Some(false) => env::Effectiveness::NotEffective,
        None => env::Effectiveness::Unknown,
    };
}

enum LoginProbeError {
    Spawn,
    TimedOut,
}

/// `None` when no probe exists for this platform or shell; otherwise the
/// login shell's PATH when it printed one.
fn login_path(shell: Option<&str>) -> Option<Result<Option<String>, LoginProbeError>> {
    if cfg!(windows) {
        return None;
    }
    let executable = Path::new(shell?);
    if !executable.is_absolute() {
        return None;
    }
    let script = match env::Shell::detect(shell) {
        env::Shell::Fish => format!("printf '\\n{LOGIN_PATH_MARKER}%s\\n' (string join : $PATH)"),
        env::Shell::Pwsh => return None,
        _ => format!("printf '\\n{LOGIN_PATH_MARKER}%s\\n' \"$PATH\""),
    };
    Some(run_login_shell(executable, &script, LOGIN_PROBE_DEADLINE))
}

fn run_login_shell(
    executable: &Path,
    script: &str,
    deadline: std::time::Duration,
) -> Result<Option<String>, LoginProbeError> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut child = Command::new(executable)
        .args(["-l", "-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| LoginProbeError::Spawn)?;
    // Read concurrently: startup files may print more than a pipe buffer holds.
    let mut stdout = child.stdout.take().ok_or(LoginProbeError::Spawn)?;
    let reader = std::thread::spawn(move || {
        let mut text = Vec::new();
        stdout.read_to_end(&mut text).map(|_| text)
    });
    let started = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(LoginProbeError::TimedOut);
            }
        }
    };
    let text = reader
        .join()
        .ok()
        .and_then(Result::ok)
        .ok_or(LoginProbeError::Spawn)?;
    if !status.success() {
        return Ok(None);
    }
    Ok(String::from_utf8_lossy(&text)
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(LOGIN_PATH_MARKER).map(str::to_owned)))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A stand-in login shell: runs its `-c` script under `/bin/sh` with a
    /// fixed PATH, after printing startup noise that must be ignored.
    fn fake_shell(dir: &Path, path: &str, extra: &str) -> std::path::PathBuf {
        let shell = dir.join("bash");
        std::fs::write(
            &shell,
            format!(
                "#!/bin/sh\necho startup noise\n{extra}\nPATH='{path}' exec /bin/sh -c \"$3\"\n"
            ),
        )
        .unwrap();
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
        shell
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
    fn marked_login_path_is_read_and_startup_output_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let shell = fake_shell(dir.path(), "/usr/bin:/opt/vibe/bin", "");
        let script = format!("printf '\\n{LOGIN_PATH_MARKER}%s\\n' \"$PATH\"");
        let path = run_login_shell(&shell, &script, LOGIN_PROBE_DEADLINE)
            .ok()
            .flatten();
        assert_eq!(path.as_deref(), Some("/usr/bin:/opt/vibe/bin"));
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
    fn a_hanging_login_shell_is_killed_at_the_deadline() {
        let dir = tempfile::tempdir().unwrap();
        let shell = fake_shell(dir.path(), "/usr/bin", "sleep 30");
        let started = std::time::Instant::now();
        let result = run_login_shell(&shell, "true", std::time::Duration::from_millis(300));
        assert!(matches!(result, Err(LoginProbeError::TimedOut)));
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }
}
