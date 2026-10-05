use crate::Diagnostic;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(crate) fn run(
    toolchain: &Path,
    cwd: &Path,
    program: &str,
    args: &[String],
    seconds: u64,
) -> Result<String, Diagnostic> {
    let output = tempfile::NamedTempFile::new()
        .map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?;
    let errors = tempfile::NamedTempFile::new()
        .map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?;
    let mut command = Command::new(toolchain.join("bin").join(if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_owned()
    }));
    command
        .args(args)
        .current_dir(cwd)
        .env_remove("LEAN_PATH")
        .env_remove("LEAN_SRC_PATH")
        .env_remove("LEAN_SYSROOT")
        .env("LEAN_PATH", cwd)
        .env(
            "PATH",
            std::env::join_paths(std::iter::once(toolchain.join("bin")).chain(
                std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
            ))
            .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?,
        )
        .stdin(Stdio::null())
        .stdout(
            output
                .reopen()
                .map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?,
        )
        .stderr(
            errors
                .reopen()
                .map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?,
        );
    let mut child = command
        .spawn()
        .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?;
    let started = Instant::now();
    let status = loop {
        let over = [output.path(), errors.path()].iter().any(|p| {
            fs::metadata(p)
                .map(|m| m.len() > 1024 * 1024)
                .unwrap_or(true)
        });
        if started.elapsed() > Duration::from_secs(seconds) || over {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Diagnostic::new(
                if over {
                    "checker_output_limit"
                } else {
                    "checker_timeout"
                },
                "checker exceeded its bounded execution contract",
            ));
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| Diagnostic::new("checker_failed", e.to_string()))?
        {
            break status;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = fs::read_to_string(output.path())
        .map_err(|e| Diagnostic::new("checker_failed", e.to_string()))?;
    let stderr = fs::read_to_string(errors.path())
        .map_err(|e| Diagnostic::new("checker_failed", e.to_string()))?;
    if !status.success() {
        return Err(Diagnostic::new(
            "checker_failed",
            format!("{program} exited {status}: {stdout}{stderr}"),
        ));
    }
    Ok(stdout)
}
