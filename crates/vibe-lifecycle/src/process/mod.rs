//! Injectable external-process wire for lifecycle handlers.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use specmark::spec;
use thiserror::Error;

mod scratch;
use scratch::create_unique_file;
pub use scratch::{
    PendingReply, ScratchError, allocate_pending_reply, allocate_run_id, execution_scratch,
    is_valid_run_id, write_atomic_json,
};

pub const STREAM_CAP: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub enum StreamMode {
    Inherit,
    Capture,
    Null,
}

#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub struct ProcessSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub env: BTreeMap<OsString, OsString>,
    pub stdin: Option<Vec<u8>>,
    pub stdout: StreamMode,
    pub stderr: StreamMode,
    pub scratch: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub struct ProcessOutput {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
pub enum ProcessError {
    #[error(
        "spawning lifecycle handler `{program}` failed: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT; \
          fix: install or correct the selected provider-contained executable)"
    )]
    Spawn { program: String, reason: String },
    #[error(
        "writing lifecycle handler stdin failed: {0} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY; \
          fix: correct the handler's stdin transport and rerun)"
    )]
    Stdin(String),
    #[error(
        "waiting for lifecycle handler failed: {0} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT; \
          fix: correct the handler process and rerun)"
    )]
    Wait(String),
}

/// Injectable process boundary used by script and binary handlers.
///
/// ```
/// use vibe_lifecycle::process::{ProcessError, ProcessOutput, ProcessRunner, ProcessSpec};
/// struct Success;
/// impl ProcessRunner for Success {
///     fn run(&self, _: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
///         Ok(ProcessOutput { code: Some(0), ..ProcessOutput::default() })
///     }
/// }
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub trait ProcessRunner: Send + Sync {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError>;
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub struct SystemProcessRunner;

struct CaptureFile {
    path: PathBuf,
    file: File,
}

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        let mut command = Command::new(&spec.program);
        command.args(&spec.args).current_dir(&spec.cwd).env_clear();
        command.envs(&spec.env);
        command.stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let (stdout, mut stdout_capture) = stream_stdio(spec.stdout, &spec.scratch, "stdout")
            .map_err(|reason| ProcessError::Spawn {
                program: spec.program.display().to_string(),
                reason,
            })?;
        let (stderr, mut stderr_capture) = stream_stdio(spec.stderr, &spec.scratch, "stderr")
            .map_err(|reason| ProcessError::Spawn {
                program: spec.program.display().to_string(),
                reason,
            })?;
        command.stdout(stdout);
        command.stderr(stderr);
        let mut child = command.spawn().map_err(|error| ProcessError::Spawn {
            program: spec.program.display().to_string(),
            reason: error.to_string(),
        })?;
        if let Some(bytes) = &spec.stdin {
            use std::io::Write;
            let Some(mut stdin) = child.stdin.take() else {
                return Err(reap_stdin_failure(
                    &mut child,
                    "spawned process did not expose the requested pipe".into(),
                ));
            };
            if let Err(error) = stdin.write_all(bytes) {
                drop(stdin);
                return Err(reap_stdin_failure(&mut child, error.to_string()));
            }
            drop(stdin);
        }
        let status = child
            .wait()
            .map_err(|error| ProcessError::Wait(error.to_string()))?;
        let retain = !status.success();
        let (stdout, stdout_truncated) = read_capped(spec.stdout, stdout_capture.as_mut(), retain)?;
        let (stderr, stderr_truncated) = read_capped(spec.stderr, stderr_capture.as_mut(), retain)?;
        Ok(ProcessOutput {
            code: status.code(),
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
        })
    }
}

fn stream_stdio(
    mode: StreamMode,
    scratch: &std::path::Path,
    genre: &str,
) -> Result<(Stdio, Option<CaptureFile>), String> {
    match mode {
        StreamMode::Inherit => Ok((Stdio::inherit(), None)),
        StreamMode::Null => Ok((Stdio::null(), None)),
        StreamMode::Capture => {
            let (path, file) = create_unique_file(scratch, genre).map_err(|e| e.to_string())?;
            let child_file = file.try_clone().map_err(|e| e.to_string())?;
            Ok((Stdio::from(child_file), Some(CaptureFile { path, file })))
        }
    }
}

fn read_capped(
    mode: StreamMode,
    capture: Option<&mut CaptureFile>,
    retain: bool,
) -> Result<(Vec<u8>, bool), ProcessError> {
    if mode != StreamMode::Capture {
        return Ok((Vec::new(), false));
    }
    let capture =
        capture.ok_or_else(|| ProcessError::Wait("capture path was not allocated".into()))?;
    capture
        .file
        .seek(SeekFrom::Start(0))
        .map_err(|error| ProcessError::Wait(error.to_string()))?;
    let mut bytes = Vec::with_capacity(STREAM_CAP + 1);
    (&mut capture.file)
        .take((STREAM_CAP + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| ProcessError::Wait(error.to_string()))?;
    let truncated = bytes.len() > STREAM_CAP;
    bytes.truncate(STREAM_CAP);
    if !retain {
        let _ = std::fs::remove_file(&capture.path);
    }
    Ok((bytes, truncated))
}

fn reap_stdin_failure(child: &mut std::process::Child, primary: String) -> ProcessError {
    let kill = child.kill().err().and_then(|error| {
        (error.kind() != std::io::ErrorKind::InvalidInput).then(|| error.to_string())
    });
    let wait = child.wait().err().map(|error| error.to_string());
    let cleanup = [kill, wait].into_iter().flatten().collect::<Vec<_>>();
    if cleanup.is_empty() {
        ProcessError::Stdin(primary)
    } else {
        ProcessError::Stdin(format!(
            "{primary}; handler cleanup/reap also failed: {}",
            cleanup.join("; ")
        ))
    }
}

#[spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "the system-process adapter is the explicit composition boundary that snapshots a fixed OS allowlist before env_clear"
)]
pub fn minimal_environment(
    extra: impl IntoIterator<Item = (String, String)>,
) -> BTreeMap<OsString, OsString> {
    let mut env = BTreeMap::new();
    for key in [
        "PATH",
        "SystemRoot",
        "WINDIR",
        "HOME",
        "USERPROFILE",
        "TEMP",
        "TMP",
        "LANG",
        "LC_ALL",
    ] {
        if let Some(value) = std::env::var_os(key) {
            env.insert(key.into(), value);
        }
    }
    for (key, value) in extra {
        env.insert(key.into(), value.into());
    }
    env
}

/// Build the clean environment used for an injected client executable.
///
/// Unlike [`minimal_environment`], this boundary deliberately excludes
/// ambient `PATH`, home variables, client config roots, credentials and
/// proxy settings. The exact injected home is then installed under both
/// conventional names, with at most one client-specific config override.
#[spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "the system-process adapter is the recorded composition boundary that snapshots only the fixed OS bootstrap/locale allowlist before env_clear; injected home and client roots never come from ambient values"
)]
pub(crate) fn client_environment(
    user_home: &std::path::Path,
    client_override: Option<(&str, &std::path::Path)>,
) -> BTreeMap<OsString, OsString> {
    let mut env = BTreeMap::new();
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP", "LANG", "LC_ALL"] {
        if let Some(value) = std::env::var_os(key) {
            env.insert(OsString::from(key), value);
        }
    }
    env.insert(OsString::from("HOME"), user_home.as_os_str().to_owned());
    env.insert(
        OsString::from("USERPROFILE"),
        user_home.as_os_str().to_owned(),
    );
    if let Some((key, value)) = client_override {
        env.insert(OsString::from(key), value.as_os_str().to_owned());
    }
    env
}

#[cfg(test)]
mod tests;
